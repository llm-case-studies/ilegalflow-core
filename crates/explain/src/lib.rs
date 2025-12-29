//! Explanation generation for trademark risk analysis.
//!
//! Converts risk flags into human-readable explanations suitable for
//! display in the extension and web interface.

use ilegalflow_model::{CandidateHit, RiskFlag};
use serde::{Deserialize, Serialize};

/// A structured explanation for a trademark risk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Explanation {
    /// Short summary (1 line)
    pub summary: String,

    /// Detailed explanation (2-3 sentences)
    pub detail: String,

    /// Severity level (0.0 - 1.0)
    pub severity: f32,

    /// Evidence items supporting this explanation
    pub evidence: Vec<EvidenceItem>,
}

/// A piece of evidence supporting a risk flag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceItem {
    /// Type of evidence
    pub kind: String,

    /// The specific value or match
    pub value: String,

    /// Optional context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

/// Generate explanations for a candidate hit.
pub fn explain_hit(hit: &CandidateHit, query_text: &str) -> Vec<Explanation> {
    hit.flags.iter().map(|flag| explain_flag(flag, query_text, &hit.record.mark_text)).collect()
}

/// Generate explanation for a single risk flag.
pub fn explain_flag(flag: &RiskFlag, query_text: &str, mark_text: &str) -> Explanation {
    match flag {
        RiskFlag::ExactMatch => Explanation {
            summary: "Exact match found".to_string(),
            detail: format!(
                "The mark '{}' is an exact match for your query '{}'. \
                 This represents the highest level of potential conflict.",
                mark_text, query_text
            ),
            severity: 1.0,
            evidence: vec![EvidenceItem {
                kind: "exact_match".to_string(),
                value: mark_text.to_string(),
                context: None,
            }],
        },

        RiskFlag::PhoneticMatch { algorithm, code } => Explanation {
            summary: "Sounds similar".to_string(),
            detail: format!(
                "The mark '{}' sounds phonetically similar to '{}'. \
                 Consumers may confuse the two when spoken aloud.",
                mark_text, query_text
            ),
            severity: 0.8,
            evidence: vec![EvidenceItem {
                kind: format!("phonetic_{}", algorithm),
                value: code.clone(),
                context: Some(format!("Both encode to: {}", code)),
            }],
        },

        RiskFlag::FuzzyMatch { distance } => Explanation {
            summary: "Spelled similarly".to_string(),
            detail: format!(
                "The mark '{}' differs from '{}' by only {} character(s). \
                 This minor spelling difference may not prevent consumer confusion.",
                mark_text, query_text, distance
            ),
            severity: 0.5 - (*distance as f32 * 0.1),
            evidence: vec![EvidenceItem {
                kind: "edit_distance".to_string(),
                value: distance.to_string(),
                context: None,
            }],
        },

        RiskFlag::ClassOverlap { classes } => Explanation {
            summary: format!("Same class ({})", classes.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(", ")),
            detail: format!(
                "Both marks are registered in the same Nice classification(s): {}. \
                 This increases the likelihood of confusion in the marketplace.",
                classes.iter().map(|c| format!("Class {}", c)).collect::<Vec<_>>().join(", ")
            ),
            severity: 0.6,
            evidence: classes.iter().map(|c| EvidenceItem {
                kind: "nice_class".to_string(),
                value: c.to_string(),
                context: None,
            }).collect(),
        },

        RiskFlag::GoodsServicesSimilar { similarity } => Explanation {
            summary: "Similar goods/services".to_string(),
            detail: format!(
                "The goods and services descriptions are {:.0}% similar. \
                 Even with different marks, similar goods increase confusion risk.",
                similarity * 100.0
            ),
            severity: similarity * 0.5,
            evidence: vec![EvidenceItem {
                kind: "goods_similarity".to_string(),
                value: format!("{:.2}", similarity),
                context: None,
            }],
        },

        RiskFlag::DominantTermMatch { term } => Explanation {
            summary: format!("Dominant term '{}' matches", term),
            detail: format!(
                "The dominant/distinctive element '{}' appears in both marks. \
                 Courts often focus on dominant terms when assessing confusion.",
                term
            ),
            severity: 0.7,
            evidence: vec![EvidenceItem {
                kind: "dominant_term".to_string(),
                value: term.clone(),
                context: None,
            }],
        },

        RiskFlag::FamousMark => Explanation {
            summary: "Famous mark".to_string(),
            detail: format!(
                "The mark '{}' may be considered famous/well-known. \
                 Famous marks receive broader protection against dilution.",
                mark_text
            ),
            severity: 0.95,
            evidence: vec![EvidenceItem {
                kind: "famous_mark".to_string(),
                value: mark_text.to_string(),
                context: None,
            }],
        },

        RiskFlag::CommonLawRisk => Explanation {
            summary: "Common law usage".to_string(),
            detail: "There may be unregistered common law trademark rights. \
                     Consider conducting a comprehensive common law search."
                .to_string(),
            severity: 0.4,
            evidence: vec![],
        },
    }
}

/// Generate a combined risk summary for all flags.
pub fn summarize_risk(hit: &CandidateHit) -> String {
    if hit.flags.is_empty() {
        return "Low risk - no significant matches found.".to_string();
    }

    let max_severity = hit.flags.iter().map(|f| f.severity()).fold(0.0_f32, f32::max);

    let level = if max_severity >= 0.8 {
        "HIGH RISK"
    } else if max_severity >= 0.5 {
        "MODERATE RISK"
    } else {
        "LOW RISK"
    };

    let flag_labels: Vec<_> = hit.flags.iter().map(|f| f.label()).collect();
    format!("{}: {}", level, flag_labels.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_explain_exact_match() {
        let explanation = explain_flag(&RiskFlag::ExactMatch, "NIKE", "NIKE");
        assert_eq!(explanation.severity, 1.0);
        assert!(explanation.summary.contains("Exact"));
    }

    #[test]
    fn test_explain_phonetic() {
        let flag = RiskFlag::PhoneticMatch {
            algorithm: "soundex".to_string(),
            code: "N200".to_string(),
        };
        let explanation = explain_flag(&flag, "NIKE", "NYKE");
        assert!(explanation.detail.contains("sounds"));
    }
}
