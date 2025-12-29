//! Query translation and dialect generation.
//!
//! Converts abstract `SearchQuery` into backend-specific query syntax:
//! - Manticore SQL
//! - USPTO TESS syntax (future)
//! - Tantivy query (future)

use ilegalflow_model::SearchQuery;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum QueryError {
    #[error("Empty query text")]
    EmptyQuery,
    #[error("Invalid class number: {0}")]
    InvalidClass(u16),
}

/// Trait for translating queries to backend-specific syntax.
pub trait QueryDialect {
    /// The output type (usually String or a structured query)
    type Output;

    /// Translate a SearchQuery to this dialect
    fn translate(&self, query: &SearchQuery) -> Result<Self::Output, QueryError>;
}

/// Manticore SQL dialect generator.
#[derive(Debug, Default)]
pub struct ManticoreDialect;

impl QueryDialect for ManticoreDialect {
    type Output = String;

    fn translate(&self, query: &SearchQuery) -> Result<String, QueryError> {
        if query.mark_text.trim().is_empty() {
            return Err(QueryError::EmptyQuery);
        }

        // Escape single quotes for SQL
        let escaped = query.mark_text.replace('\'', "''");

        // Build MATCH clause
        let match_clause = format!("MATCH('{}')", escaped);

        // Build WHERE conditions
        let mut conditions = vec![match_clause];

        // Add status filter if specified
        if let Some(status) = &query.status_filter {
            conditions.push(format!("status = '{:?}'", status));
        }

        // Build final query
        let where_clause = conditions.join(" AND ");
        let sql = format!(
            "SELECT * FROM trademarks WHERE {} LIMIT {}",
            where_clause, query.limit
        );

        Ok(sql)
    }
}

/// Generate phonetic variants of a query term.
pub fn generate_variants(text: &str) -> Vec<String> {
    let mut variants = vec![text.to_string()];

    // TODO: Add phonetic variants
    // This would use ilegalflow-features to generate soundex/metaphone codes
    // and query for those as well

    variants
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manticore_basic() {
        let dialect = ManticoreDialect;
        let query = SearchQuery::new("NIKE");
        let sql = dialect.translate(&query).unwrap();
        assert!(sql.contains("MATCH('NIKE')"));
        assert!(sql.contains("LIMIT 100"));
    }

    #[test]
    fn test_manticore_escaping() {
        let dialect = ManticoreDialect;
        let query = SearchQuery::new("O'REILLY");
        let sql = dialect.translate(&query).unwrap();
        assert!(sql.contains("O''REILLY"));
    }

    #[test]
    fn test_empty_query_error() {
        let dialect = ManticoreDialect;
        let query = SearchQuery::new("   ");
        assert!(matches!(
            dialect.translate(&query),
            Err(QueryError::EmptyQuery)
        ));
    }
}
