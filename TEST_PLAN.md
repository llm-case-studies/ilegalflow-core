# iLegalFlow Core Test Plan

This document outlines the testing strategy for validating the trademark search and scoring engine.

## Prerequisites

- Rust 1.75+ installed
- Manticore Search running with indexed trademarks
- Access to MSI or equivalent machine with Manticore

## 1. Unit Tests (No Network Required)

Run anywhere with just Rust installed:

```bash
# All unit tests
cargo test

# Specific crate tests
cargo test -p ilegalflow-features   # Phonetics, edit distance, n-grams
cargo test -p ilegalflow-model      # Serialization, status parsing
cargo test -p ilegalflow-rerank     # Scoring logic
cargo test -p ilegalflow-explain    # Explanation generation

# With output for debugging
cargo test -- --nocapture

# Single test
cargo test test_phonetic_match
```

### Expected Results

| Test | Expected |
|------|----------|
| `test_phonetic_match("NIKE", "NYKE")` | `Some(("soundex", ...))` |
| `test_phonetic_match("SMITH", "SMYTH")` | `Some(...)` |
| `test_normalize_text("  Hello,  World!  ")` | `"HELLO WORLD"` |
| `test_dominant_term("ACME Corporation")` | `Some("ACME")` |
| `test_edit_distance("NIKE", "NYKE")` | `1` |
| `test_class_overlap([9, 25], [25, 35])` | `[25]` |

## 2. Integration Tests (Requires Manticore)

### 2.1 Health Check

```bash
cargo run --bin eval -- health
```

**Expected**: `Checking manticore backend... OK`

### 2.2 Basic Search Validation

Test retrieval from live Manticore index:

```bash
# Famous marks - should return exact matches
cargo run --bin eval -- search "NIKE" --limit 10
cargo run --bin eval -- search "APPLE" --limit 10
cargo run --bin eval -- search "AMAZON" --limit 20
cargo run --bin eval -- search "COCA-COLA" --limit 10

# Less common marks
cargo run --bin eval -- search "ACME" --limit 10
cargo run --bin eval -- search "WIDGET" --limit 10
```

**Expected**:
- Results returned (non-empty)
- Each result has `retrieval_score` > 0
- Exact matches should appear first

### 2.3 Phonetic Matching Tests

Test that phonetically similar marks are found:

```bash
# Misspellings that sound the same
cargo run --bin eval -- search "NYKE" --limit 10      # Should find NIKE
cargo run --bin eval -- search "APLE" --limit 10      # Should find APPLE
cargo run --bin eval -- search "KOKA KOLA" --limit 10 # Should find COCA-COLA
cargo run --bin eval -- search "ADDIDAS" --limit 10   # Should find ADIDAS
cargo run --bin eval -- search "GOOGEL" --limit 10    # Should find GOOGLE
```

**Expected**:
- Results include phonetically similar marks
- `PhoneticMatch` flag present in results
- Phonetic code shown in flag details

### 2.4 Fuzzy/Edit Distance Tests

Test edit distance matching:

```bash
# One character off
cargo run --bin eval -- search "NIKEE" --limit 10     # 1 insertion
cargo run --bin eval -- search "NIK" --limit 10       # 1 deletion
cargo run --bin eval -- search "NUKE" --limit 10      # 1 substitution

# Two characters off
cargo run --bin eval -- search "NIKEES" --limit 10
cargo run --bin eval -- search "APPPLE" --limit 10
```

**Expected**:
- `FuzzyMatch` flag with correct distance
- Distance 1 matches ranked higher than distance 2

### 2.5 Class-Filtered Searches

Test Nice classification filtering:

```bash
# Class 25 = Clothing
cargo run --bin eval -- search "NIKE" --classes 25 --limit 10

# Class 9 = Electronics/Software
cargo run --bin eval -- search "APPLE" --classes 9 --limit 10

# Class 42 = Computer services
cargo run --bin eval -- search "GOOGLE" --classes 42 --limit 10

# Multiple classes
cargo run --bin eval -- search "AMAZON" --classes 9,35,42 --limit 20
```

**Expected**:
- Results filtered to specified classes
- `ClassOverlap` flag shows matching classes

### 2.6 Dominant Term Extraction

```bash
# Should match "APPLE" despite additional words
cargo run --bin eval -- search "APPLE COMPUTER INC" --limit 10

# Should match "AMAZON"
cargo run --bin eval -- search "THE AMAZON COMPANY" --limit 10
```

**Expected**:
- `DominantTermMatch` flag with extracted term
- Related marks appear in results

## 3. Pipeline Validation

For each search result, verify the full pipeline:

### 3.1 Score Propagation

| Field | Source | Expected |
|-------|--------|----------|
| `retrieval_score` | Manticore | 0.0 - 1.0+ |
| `risk_score` | Reranker | 0.0 - 1.0 |
| `flags` | Feature analysis | Non-empty for matches |

### 3.2 Flag Coverage Matrix

Test that appropriate flags are generated:

| Query | Target Mark | Expected Flags |
|-------|-------------|----------------|
| "NIKE" | NIKE | `ExactMatch` |
| "NYKE" | NIKE | `PhoneticMatch` |
| "NIKEE" | NIKE | `FuzzyMatch(1)` |
| "NIKE" | NIKE (Class 25) | `ExactMatch`, `ClassOverlap` |
| "NIKE SPORTS" | NIKE | `DominantTermMatch` |

### 3.3 Explanation Generation

Each flag should produce valid explanation:

```bash
# Run with verbose output
cargo run --bin eval -- search "NIKE" --limit 5 --verbose
```

**Expected explanation fields**:
- `summary`: Short label (e.g., "Exact Match")
- `detail`: Full explanation
- `severity`: 0.0 - 1.0 weight
- `evidence`: Supporting data

## 4. Regression Tests (Golden Tests)

### 4.1 Create Golden File

```yaml
# tests/golden/famous_marks.yaml
queries:
  - text: "NIKE"
    classes: [25]
    expected:
      - serial: "73170572"  # Actual NIKE registration
        must_have_flags: [ExactMatch, ClassOverlap]
        min_risk_score: 0.9

  - text: "NYKE"
    classes: [25]
    expected:
      - mark_contains: "NIKE"
        must_have_flags: [PhoneticMatch]
        min_risk_score: 0.7
```

### 4.2 Run Golden Tests

```bash
cargo run --bin eval -- benchmark --test-file tests/golden/famous_marks.yaml
```

## 5. Performance Benchmarks

### 5.1 Latency

```bash
# Measure search latency
time cargo run --release --bin eval -- search "NIKE" --limit 100

# Multiple queries
for mark in NIKE APPLE AMAZON GOOGLE MICROSOFT; do
  time cargo run --release --bin eval -- search "$mark" --limit 100
done
```

**Expected**: < 100ms per query (including network to Manticore)

### 5.2 Throughput

```bash
# Sequential queries
cargo run --bin eval -- benchmark --queries 100 --concurrency 1

# Concurrent queries
cargo run --bin eval -- benchmark --queries 100 --concurrency 10
```

## 6. Quality Metrics

### 6.1 Recall@K

For known marks, verify they appear in top K results:

| Query | Known Target | K=10 | K=50 | K=100 |
|-------|--------------|------|------|-------|
| "NIKE" | NIKE | Y | Y | Y |
| "NYKE" | NIKE | Y | Y | Y |
| "NIKEY" | NIKE | ? | Y | Y |

### 6.2 Precision@K

Verify top K results are relevant:

```bash
# Manual inspection of top 10
cargo run --bin eval -- search "NIKE" --limit 10 --format json | jq '.hits[].record.mark_text'
```

### 6.3 Flag Accuracy

| Scenario | Expected Flag | Test |
|----------|---------------|------|
| Exact text | `ExactMatch` | Query="NIKE", Mark="NIKE" |
| Same sound | `PhoneticMatch` | Query="NYKE", Mark="NIKE" |
| Close spelling | `FuzzyMatch` | Edit distance < 3 |
| Same class | `ClassOverlap` | Classes intersect |

## 7. Error Handling

### 7.1 Manticore Unavailable

```bash
# Stop Manticore, then:
cargo run --bin eval -- search "NIKE" --limit 10
```

**Expected**: Clear error message, not panic

### 7.2 Invalid Queries

```bash
# Empty query
cargo run --bin eval -- search "" --limit 10

# Very long query
cargo run --bin eval -- search "$(printf 'A%.0s' {1..10000})" --limit 10
```

**Expected**: Graceful handling with error message

### 7.3 Invalid Classes

```bash
cargo run --bin eval -- search "NIKE" --classes 999 --limit 10
```

**Expected**: Empty results or appropriate message

## 8. Test Environments

| Environment | Manticore | Tests |
|-------------|-----------|-------|
| MSI Raider | Local (127.0.0.1:9308) | All |
| iMac/Mac Mini | Remote (msi-raider-linux.local:9308) | Integration |
| CI/GitHub Actions | Mock backend | Unit only |
| Docker container | Sidecar Manticore | All |

## 9. Continuous Integration

### GitHub Actions Workflow

```yaml
# .github/workflows/test.yml
name: Tests
on: [push, pull_request]

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test

  integration-tests:
    runs-on: ubuntu-latest
    services:
      manticore:
        image: manticoresearch/manticore
        ports:
          - 9306:9306
          - 9308:9308
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --features integration
```

## 10. Test Checklist

Before release:

- [ ] All unit tests pass (`cargo test`)
- [ ] Health check succeeds against Manticore
- [ ] Famous marks (NIKE, APPLE, etc.) return expected results
- [ ] Phonetic matching works (NYKE → NIKE)
- [ ] Fuzzy matching works (NIKEE → NIKE)
- [ ] Class filtering works
- [ ] Explanations are human-readable
- [ ] No panics on edge cases
- [ ] Performance within targets
