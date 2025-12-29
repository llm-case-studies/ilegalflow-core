//! Manticore Search backend implementation.
//!
//! Provides the `SearchBackend` trait and its Manticore implementation.
//! This allows retrieval from Manticore while keeping the core logic
//! backend-agnostic for future Tantivy migration.

use ilegalflow_model::{SearchQuery, TrademarkRecord, TrademarkStatus};
use std::future::Future;
use thiserror::Error;

/// Errors from search backend operations.
#[derive(Debug, Error)]
pub enum BackendError {
    #[error("Connection failed: {0}")]
    Connection(String),

    #[error("Query execution failed: {0}")]
    QueryFailed(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Backend not available")]
    Unavailable,
}

/// Trait for search backends (Manticore, Tantivy, etc.)
///
/// This abstraction allows swapping backends without changing scoring logic.
pub trait SearchBackend {
    /// Search for candidates matching the query.
    fn search(
        &self,
        query: &SearchQuery,
    ) -> impl Future<Output = Result<Vec<(TrademarkRecord, f32)>, BackendError>> + Send;

    /// Check if the backend is healthy.
    fn health_check(&self) -> impl Future<Output = Result<(), BackendError>> + Send;

    /// Get the backend name for logging.
    fn name(&self) -> &'static str;
}

/// Manticore Search backend configuration.
#[derive(Debug, Clone)]
pub struct ManticoreConfig {
    /// Base URL for Manticore HTTP API
    pub base_url: String,
    /// Table/index name
    pub table_name: String,
    /// Request timeout in seconds
    pub timeout_secs: u64,
}

impl Default for ManticoreConfig {
    fn default() -> Self {
        Self {
            base_url: "http://127.0.0.1:9308".to_string(),
            table_name: "trademarks".to_string(),
            timeout_secs: 30,
        }
    }
}

/// Manticore Search backend.
pub struct ManticoreBackend {
    config: ManticoreConfig,
    client: reqwest::Client,
}

impl ManticoreBackend {
    /// Create a new Manticore backend.
    pub fn new(config: ManticoreConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .expect("Failed to create HTTP client");

        Self { config, client }
    }

    /// Build SQL query for Manticore.
    fn build_query(&self, query: &SearchQuery) -> String {
        let escaped = query.mark_text.replace('\'', "''");

        let mut sql = format!(
            "SELECT *, WEIGHT() as _score FROM {} WHERE MATCH('{}')",
            self.config.table_name, escaped
        );

        if let Some(status) = &query.status_filter {
            sql.push_str(&format!(" AND status = '{:?}'", status));
        }

        sql.push_str(&format!(" LIMIT {}", query.limit));

        sql
    }

    /// Parse Manticore response into records.
    fn parse_response(
        &self,
        response: serde_json::Value,
    ) -> Result<Vec<(TrademarkRecord, f32)>, BackendError> {
        // Manticore /cli returns plain text, /sql returns JSON
        // We'll handle the JSON format from /sql endpoint

        let hits = response
            .get("hits")
            .and_then(|h| h.get("hits"))
            .and_then(|h| h.as_array())
            .ok_or_else(|| BackendError::ParseError("Missing hits array".to_string()))?;

        let mut results = Vec::new();

        for hit in hits {
            let source = hit.get("_source").ok_or_else(|| {
                BackendError::ParseError("Missing _source".to_string())
            })?;

            let score = hit
                .get("_score")
                .and_then(|s| s.as_f64())
                .unwrap_or(0.0) as f32;

            let record = TrademarkRecord {
                serial_number: source
                    .get("serial_number")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                registration_number: source
                    .get("registration_number")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                mark_text: source
                    .get("mark_text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                mark_text_normalized: None,
                status: source
                    .get("status")
                    .and_then(|v| v.as_str())
                    .map(TrademarkStatus::from)
                    .unwrap_or_default(),
                status_code: source
                    .get("status_code")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                classes: Vec::new(), // TODO: Parse from response
                goods_services: source
                    .get("goods_services")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                owner_name: source
                    .get("owner_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                filing_date: None,
                registration_date: None,
                status_date: None,
                is_design_mark: false,
            };

            results.push((record, score));
        }

        Ok(results)
    }
}

impl SearchBackend for ManticoreBackend {
    async fn search(
        &self,
        query: &SearchQuery,
    ) -> Result<Vec<(TrademarkRecord, f32)>, BackendError> {
        let sql = self.build_query(query);

        tracing::debug!(sql = %sql, "Executing Manticore query");

        // Use /sql endpoint with mode=raw for JSON response
        let response = self
            .client
            .post(format!("{}/sql", self.config.base_url))
            .query(&[("mode", "raw")])
            .body(format!("query={}", sql))
            .send()
            .await
            .map_err(|e| BackendError::Connection(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(BackendError::QueryFailed(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| BackendError::ParseError(e.to_string()))?;

        self.parse_response(json)
    }

    async fn health_check(&self) -> Result<(), BackendError> {
        let response = self
            .client
            .post(format!("{}/cli", self.config.base_url))
            .body("SHOW STATUS")
            .send()
            .await
            .map_err(|e| BackendError::Connection(e.to_string()))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(BackendError::Unavailable)
        }
    }

    fn name(&self) -> &'static str {
        "manticore"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_query() {
        let backend = ManticoreBackend::new(ManticoreConfig::default());
        let query = SearchQuery::new("NIKE").with_limit(50);
        let sql = backend.build_query(&query);

        assert!(sql.contains("MATCH('NIKE')"));
        assert!(sql.contains("LIMIT 50"));
        assert!(sql.contains("trademarks"));
    }

    #[test]
    fn test_query_escaping() {
        let backend = ManticoreBackend::new(ManticoreConfig::default());
        let query = SearchQuery::new("O'REILLY");
        let sql = backend.build_query(&query);

        assert!(sql.contains("O''REILLY"));
    }
}
