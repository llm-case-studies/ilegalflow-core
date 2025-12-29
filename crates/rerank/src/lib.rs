//! Re-ranking and scoring for trademark candidates.
//!
//! Takes raw search results and applies proprietary scoring logic
//! to produce risk-ranked results with explanations.

use ilegalflow_model::{CandidateHit, RiskFlag, SearchQuery, TrademarkRecord};
use ilegalflow_features::{
    class_overlap, edit_distance, extract_dominant_term, normalize_text, phonetic_match,
};

/// Configuration for the re-ranker.
#[derive(Debug, Clone)]
pub struct RerankConfig {
    /// Weight for phonetic similarity
    pub phonetic_weight: f32,
    /// Weight for fuzzy/edit distance
    pub fuzzy_weight: f32,
    /// Weight for class overlap
    pub class_weight: f32,
    /// Weight for dominant term match
    pub dominant_weight: f32,
    /// Maximum edit distance to consider
    pub max_edit_distance: usize,
}

impl Default for RerankConfig {
    fn default() -> Self {
        Self {
            phonetic_weight: 0.3,
            fuzzy_weight: 0.2,
            class_weight: 0.25,
            dominant_weight: 0.25,
            max_edit_distance: 3,
        }
    }
}

/// Re-rank candidates based on trademark risk analysis.
pub fn rerank(
    query: &SearchQuery,
    candidates: Vec<(TrademarkRecord, f32)>,
    config: &RerankConfig,
) -> Vec<CandidateHit> {
    let query_normalized = normalize_text(&query.mark_text);
    let query_dominant = extract_dominant_term(&query.mark_text);

    let mut hits: Vec<CandidateHit> = candidates
        .into_iter()
        .map(|(record, retrieval_score)| {
            let (risk_score, flags) =
                compute_risk(&query_normalized, &query.classes, query_dominant.as_deref(), &record, config);

            CandidateHit {
                record,
                retrieval_score,
                risk_score,
                flags,
            }
        })
        .collect();

    // Sort by risk score descending
    hits.sort_by(|a, b| b.risk_score.partial_cmp(&a.risk_score).unwrap_or(std::cmp::Ordering::Equal));

    hits
}

/// Compute risk score and flags for a single candidate.
fn compute_risk(
    query_normalized: &str,
    query_classes: &[u16],
    query_dominant: Option<&str>,
    record: &TrademarkRecord,
    config: &RerankConfig,
) -> (f32, Vec<RiskFlag>) {
    let mut flags = Vec::new();
    let mut score = 0.0_f32;

    let mark_normalized = normalize_text(&record.mark_text);

    // Check exact match
    if query_normalized == mark_normalized {
        flags.push(RiskFlag::ExactMatch);
        return (1.0, flags); // Maximum risk
    }

    // Check phonetic match
    if let Some((algorithm, code)) = phonetic_match(query_normalized, &mark_normalized) {
        flags.push(RiskFlag::PhoneticMatch { algorithm, code });
        score += config.phonetic_weight;
    }

    // Check fuzzy/edit distance
    let distance = edit_distance(query_normalized, &mark_normalized);
    if distance > 0 && distance <= config.max_edit_distance {
        flags.push(RiskFlag::FuzzyMatch {
            distance: distance as u8,
        });
        // Closer = higher risk
        let fuzzy_score = 1.0 - (distance as f32 / (config.max_edit_distance as f32 + 1.0));
        score += config.fuzzy_weight * fuzzy_score;
    }

    // Check class overlap
    let overlapping = class_overlap(query_classes, &record.classes);
    if !overlapping.is_empty() {
        flags.push(RiskFlag::ClassOverlap {
            classes: overlapping,
        });
        score += config.class_weight;
    }

    // Check dominant term match
    if let Some(query_dom) = query_dominant {
        if let Some(record_dom) = extract_dominant_term(&record.mark_text) {
            if query_dom.to_uppercase() == record_dom.to_uppercase() {
                flags.push(RiskFlag::DominantTermMatch { term: record_dom });
                score += config.dominant_weight;
            }
        }
    }

    // Normalize score to 0.0 - 1.0
    score = score.min(1.0);

    (score, flags)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ilegalflow_model::TrademarkStatus;

    fn make_record(serial: &str, mark: &str, classes: Vec<u16>) -> TrademarkRecord {
        TrademarkRecord {
            serial_number: serial.to_string(),
            registration_number: None,
            mark_text: mark.to_string(),
            mark_text_normalized: None,
            status: TrademarkStatus::Live,
            status_code: None,
            classes,
            goods_services: String::new(),
            owner_name: String::new(),
            filing_date: None,
            registration_date: None,
            status_date: None,
            is_design_mark: false,
        }
    }

    #[test]
    fn test_exact_match_highest_risk() {
        let query = SearchQuery::new("NIKE").with_classes(vec![25]);
        let candidates = vec![(make_record("001", "NIKE", vec![25]), 1.0)];
        let config = RerankConfig::default();

        let hits = rerank(&query, candidates, &config);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].risk_score, 1.0);
        assert!(hits[0].flags.contains(&RiskFlag::ExactMatch));
    }

    #[test]
    fn test_phonetic_match() {
        let query = SearchQuery::new("NIKE").with_classes(vec![25]);
        let candidates = vec![(make_record("001", "NYKE", vec![25]), 1.0)];
        let config = RerankConfig::default();

        let hits = rerank(&query, candidates, &config);
        assert!(hits[0].flags.iter().any(|f| matches!(f, RiskFlag::PhoneticMatch { .. })));
    }

    #[test]
    fn test_class_overlap() {
        let query = SearchQuery::new("ACME").with_classes(vec![9, 42]);
        let candidates = vec![(make_record("001", "WIDGET", vec![42, 35]), 1.0)];
        let config = RerankConfig::default();

        let hits = rerank(&query, candidates, &config);
        assert!(hits[0].flags.iter().any(|f| matches!(f, RiskFlag::ClassOverlap { classes } if classes.contains(&42))));
    }
}
