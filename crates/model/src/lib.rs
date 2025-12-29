//! Core domain model for iLegalFlow trademark analysis.
//!
//! This crate defines the fundamental types used throughout the system:
//! - `TrademarkRecord`: The normalized trademark data from USPTO
//! - `TrademarkStatus`: Live, Dead, Pending status
//! - `CandidateHit`: A search result with score
//! - `RiskFlag`: Types of trademark risks identified

use serde::{Deserialize, Serialize};

/// Status of a trademark registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TrademarkStatus {
    /// Active registration
    Live,
    /// Cancelled, expired, or abandoned
    Dead,
    /// Application in progress
    Pending,
    /// Unknown status
    Unknown,
}

impl Default for TrademarkStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

impl From<&str> for TrademarkStatus {
    fn from(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "LIVE" => Self::Live,
            "DEAD" => Self::Dead,
            "PENDING" => Self::Pending,
            _ => Self::Unknown,
        }
    }
}

/// A normalized trademark record from USPTO data.
///
/// This is the canonical representation consumed by all downstream systems.
/// Produced by `ilegalflow-data` pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrademarkRecord {
    /// USPTO serial number (8 digits, zero-padded)
    pub serial_number: String,

    /// Registration number (if registered)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registration_number: Option<String>,

    /// The mark text (word mark)
    #[serde(default)]
    pub mark_text: String,

    /// Normalized/cleaned mark text for matching
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mark_text_normalized: Option<String>,

    /// Current status
    #[serde(default)]
    pub status: TrademarkStatus,

    /// USPTO status code (e.g., "800" for dead)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_code: Option<String>,

    /// Nice classification codes
    #[serde(default)]
    pub classes: Vec<u16>,

    /// Goods and services description
    #[serde(default)]
    pub goods_services: String,

    /// Owner/registrant name
    #[serde(default)]
    pub owner_name: String,

    /// Filing date (ISO format)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filing_date: Option<String>,

    /// Registration date (ISO format)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registration_date: Option<String>,

    /// Status change date (ISO format)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_date: Option<String>,

    /// Whether this is a design mark (has visual elements)
    #[serde(default)]
    pub is_design_mark: bool,
}

impl TrademarkRecord {
    /// Create a minimal record for testing.
    pub fn new(serial_number: impl Into<String>, mark_text: impl Into<String>) -> Self {
        Self {
            serial_number: serial_number.into(),
            registration_number: None,
            mark_text: mark_text.into(),
            mark_text_normalized: None,
            status: TrademarkStatus::Unknown,
            status_code: None,
            classes: Vec::new(),
            goods_services: String::new(),
            owner_name: String::new(),
            filing_date: None,
            registration_date: None,
            status_date: None,
            is_design_mark: false,
        }
    }

    /// Get the effective mark text for matching (normalized if available).
    pub fn effective_mark_text(&self) -> &str {
        self.mark_text_normalized
            .as_deref()
            .unwrap_or(&self.mark_text)
    }
}

/// A candidate hit from search retrieval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateHit {
    /// The trademark record
    pub record: TrademarkRecord,

    /// Raw retrieval score from backend
    pub retrieval_score: f32,

    /// Re-ranked risk score (0.0 = no risk, 1.0 = high risk)
    #[serde(default)]
    pub risk_score: f32,

    /// Risk flags identified
    #[serde(default)]
    pub flags: Vec<RiskFlag>,
}

/// Types of trademark risk flags.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "detail")]
pub enum RiskFlag {
    /// Exact text match
    ExactMatch,

    /// Phonetically similar (sounds like)
    PhoneticMatch {
        /// Algorithm used (soundex, metaphone, etc.)
        algorithm: String,
        /// The phonetic code that matched
        code: String,
    },

    /// Similar spelling (edit distance)
    FuzzyMatch {
        /// Edit distance
        distance: u8,
    },

    /// Same Nice classification
    ClassOverlap {
        /// Overlapping class numbers
        classes: Vec<u16>,
    },

    /// Similar goods/services description
    GoodsServicesSimilar {
        /// Similarity score
        similarity: f32,
    },

    /// Dominant term match
    DominantTermMatch {
        /// The dominant term that matched
        term: String,
    },

    /// Well-known/famous mark
    FamousMark,

    /// Common law usage concern
    CommonLawRisk,
}

impl RiskFlag {
    /// Get a human-readable label for this flag.
    pub fn label(&self) -> &'static str {
        match self {
            Self::ExactMatch => "Exact Match",
            Self::PhoneticMatch { .. } => "Sounds Similar",
            Self::FuzzyMatch { .. } => "Spelled Similarly",
            Self::ClassOverlap { .. } => "Same Class",
            Self::GoodsServicesSimilar { .. } => "Similar Goods/Services",
            Self::DominantTermMatch { .. } => "Dominant Term Match",
            Self::FamousMark => "Famous Mark",
            Self::CommonLawRisk => "Common Law Risk",
        }
    }

    /// Get severity weight (higher = more concerning).
    pub fn severity(&self) -> f32 {
        match self {
            Self::ExactMatch => 1.0,
            Self::FamousMark => 0.95,
            Self::PhoneticMatch { .. } => 0.8,
            Self::DominantTermMatch { .. } => 0.7,
            Self::ClassOverlap { .. } => 0.6,
            Self::FuzzyMatch { distance } => 0.5 - (*distance as f32 * 0.1),
            Self::GoodsServicesSimilar { similarity } => *similarity * 0.5,
            Self::CommonLawRisk => 0.4,
        }
    }
}

/// Query parameters for trademark search.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchQuery {
    /// The mark text to search for
    pub mark_text: String,

    /// Optional Nice classes to filter by
    #[serde(default)]
    pub classes: Vec<u16>,

    /// Filter by status
    #[serde(default)]
    pub status_filter: Option<TrademarkStatus>,

    /// Maximum results to retrieve
    #[serde(default = "default_limit")]
    pub limit: usize,

    /// Enable phonetic matching
    #[serde(default = "default_true")]
    pub phonetic: bool,

    /// Enable fuzzy matching
    #[serde(default = "default_true")]
    pub fuzzy: bool,
}

fn default_limit() -> usize {
    100
}

fn default_true() -> bool {
    true
}

impl SearchQuery {
    pub fn new(mark_text: impl Into<String>) -> Self {
        Self {
            mark_text: mark_text.into(),
            ..Default::default()
        }
    }

    pub fn with_classes(mut self, classes: Vec<u16>) -> Self {
        self.classes = classes;
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_from_str() {
        assert_eq!(TrademarkStatus::from("LIVE"), TrademarkStatus::Live);
        assert_eq!(TrademarkStatus::from("dead"), TrademarkStatus::Dead);
        assert_eq!(TrademarkStatus::from("Pending"), TrademarkStatus::Pending);
        assert_eq!(TrademarkStatus::from("unknown"), TrademarkStatus::Unknown);
    }

    #[test]
    fn test_record_serialization() {
        let record = TrademarkRecord::new("12345678", "ACME");
        let json = serde_json::to_string(&record).unwrap();
        let parsed: TrademarkRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.serial_number, "12345678");
        assert_eq!(parsed.mark_text, "ACME");
    }

    #[test]
    fn test_risk_flag_severity() {
        assert!(RiskFlag::ExactMatch.severity() > RiskFlag::PhoneticMatch {
            algorithm: "soundex".into(),
            code: "A250".into()
        }.severity());
    }
}
