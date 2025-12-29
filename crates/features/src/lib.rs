//! Feature extraction for trademark analysis.
//!
//! Provides pure functions for computing features used in scoring:
//! - Phonetic encodings (Soundex, Metaphone)
//! - Text normalization
//! - N-gram generation
//! - Dominant term extraction

use rphonetic::{Encoder, Soundex, Metaphone};

/// Phonetic encoding results for a mark.
#[derive(Debug, Clone, Default)]
pub struct PhoneticCodes {
    pub soundex: Option<String>,
    pub metaphone: Option<String>,
}

/// Compute phonetic encodings for a mark text.
pub fn compute_phonetics(text: &str) -> PhoneticCodes {
    let soundex = Soundex::default();
    let metaphone = Metaphone::default();

    // rphonetic encode() returns String directly
    let soundex_code = soundex.encode(text);
    let metaphone_code = metaphone.encode(text);

    PhoneticCodes {
        soundex: if soundex_code.is_empty() { None } else { Some(soundex_code) },
        metaphone: if metaphone_code.is_empty() { None } else { Some(metaphone_code) },
    }
}

/// Check if two texts are phonetically similar.
pub fn phonetic_match(text1: &str, text2: &str) -> Option<(String, String)> {
    let codes1 = compute_phonetics(text1);
    let codes2 = compute_phonetics(text2);

    // Check Soundex match
    if let (Some(s1), Some(s2)) = (&codes1.soundex, &codes2.soundex) {
        if s1 == s2 {
            return Some(("soundex".to_string(), s1.clone()));
        }
    }

    // Check Metaphone match
    if let (Some(m1), Some(m2)) = (&codes1.metaphone, &codes2.metaphone) {
        if m1 == m2 {
            return Some(("metaphone".to_string(), m1.clone()));
        }
    }

    None
}

/// Normalize text for comparison.
pub fn normalize_text(text: &str) -> String {
    text.to_uppercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Extract dominant term(s) from a mark.
///
/// Heuristic: longest word, excluding common suffixes like INC, LLC, CORP.
pub fn extract_dominant_term(text: &str) -> Option<String> {
    let stopwords = ["INC", "LLC", "CORP", "CO", "LTD", "THE", "A", "AN", "AND", "OF", "FOR"];

    let normalized = normalize_text(text);
    let words: Vec<&str> = normalized
        .split_whitespace()
        .filter(|w| !stopwords.contains(&w.to_uppercase().as_str()))
        .collect();

    words.into_iter()
        .max_by_key(|w| w.len())
        .map(|s| s.to_string())
}

/// Generate character n-grams.
pub fn generate_ngrams(text: &str, n: usize) -> Vec<String> {
    let normalized = normalize_text(text).replace(' ', "");
    if normalized.len() < n {
        return vec![normalized];
    }

    normalized
        .chars()
        .collect::<Vec<_>>()
        .windows(n)
        .map(|w| w.iter().collect())
        .collect()
}

/// Compute Levenshtein edit distance between two strings.
pub fn edit_distance(s1: &str, s2: &str) -> usize {
    let s1: Vec<char> = s1.chars().collect();
    let s2: Vec<char> = s2.chars().collect();
    let len1 = s1.len();
    let len2 = s2.len();

    let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

    for i in 0..=len1 {
        matrix[i][0] = i;
    }
    for j in 0..=len2 {
        matrix[0][j] = j;
    }

    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = if s1[i - 1] == s2[j - 1] { 0 } else { 1 };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }

    matrix[len1][len2]
}

/// Check Nice class overlap.
pub fn class_overlap(classes1: &[u16], classes2: &[u16]) -> Vec<u16> {
    classes1
        .iter()
        .filter(|c| classes2.contains(c))
        .copied()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phonetic_match() {
        // These should match phonetically
        assert!(phonetic_match("SMITH", "SMYTH").is_some());
        assert!(phonetic_match("NIKE", "NYKE").is_some());
    }

    #[test]
    fn test_normalize_text() {
        assert_eq!(normalize_text("  Hello,  World!  "), "HELLO WORLD");
        assert_eq!(normalize_text("ACME Inc."), "ACME INC");
    }

    #[test]
    fn test_dominant_term() {
        assert_eq!(extract_dominant_term("ACME Corporation"), Some("ACME".to_string()));
        assert_eq!(extract_dominant_term("The Widget Company Inc"), Some("WIDGET".to_string()));
    }

    #[test]
    fn test_ngrams() {
        let ngrams = generate_ngrams("NIKE", 2);
        assert_eq!(ngrams, vec!["NI", "IK", "KE"]);
    }

    #[test]
    fn test_edit_distance() {
        assert_eq!(edit_distance("NIKE", "NIKE"), 0);
        assert_eq!(edit_distance("NIKE", "NYKE"), 1);
        assert_eq!(edit_distance("NIKE", "ADIDAS"), 6);
    }

    #[test]
    fn test_class_overlap() {
        assert_eq!(class_overlap(&[9, 25, 42], &[25, 35, 42]), vec![25, 42]);
        assert_eq!(class_overlap(&[1, 2], &[3, 4]), Vec::<u16>::new());
    }
}
