# iLegalFlow Core Architecture

## Design Philosophy

> "Keep retrieval and reasoning separate."

This architecture enables:
1. Starting with Manticore (validated, running)
2. Swapping to Tantivy later (performance, control)
3. Stable interfaces for extension/web consumers

## System Context

```
                    ┌─────────────────┐
                    │   USPTO Data    │
                    │  (ilegalflow-   │
                    │     data)       │
                    └────────┬────────┘
                             │
                    marks.json / parquet
                             │
                             ▼
┌─────────────────────────────────────────────────────────────┐
│                    ilegalflow-core                          │
│                                                             │
│   Query ──► Backend ──► Candidates ──► Rerank ──► Hits     │
│              (Manticore)              (Scoring)   (Flags)   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
                             │
              ┌──────────────┴──────────────┐
              ▼                              ▼
       Extension API                    Web Dashboard
       (risk flags)                    (analytics)
```

## Core Abstractions

### 1. SearchBackend Trait

```rust
pub trait SearchBackend {
    async fn search(&self, query: &SearchQuery)
        -> Result<Vec<(TrademarkRecord, f32)>, BackendError>;

    async fn health_check(&self) -> Result<(), BackendError>;

    fn name(&self) -> &'static str;
}
```

**Implementations:**
- `ManticoreBackend` - HTTP/SQL to Manticore (current)
- `TantivyBackend` - Embedded Tantivy (future)

**Why trait-based:**
- Test scoring logic against mock backend
- A/B test Manticore vs Tantivy
- Swap backends without changing consumers

### 2. RiskFlag Enum

```rust
pub enum RiskFlag {
    ExactMatch,
    PhoneticMatch { algorithm: String, code: String },
    FuzzyMatch { distance: u8 },
    ClassOverlap { classes: Vec<u16> },
    DominantTermMatch { term: String },
    FamousMark,
    // ...
}
```

**Design decisions:**
- Enum with associated data (not just strings)
- Each flag has `severity()` weight
- Each flag maps to `Explanation`

### 3. Rerank Pipeline

```
Raw Candidates        Scored Hits
(from backend)   ──►  (with flags)

┌────────────┐        ┌────────────┐
│ Record     │        │ Record     │
│ score: 0.8 │   ──►  │ risk: 0.95 │
│            │        │ flags: [   │
└────────────┘        │   Phonetic │
                      │   ClassOvr │
                      │ ]          │
                      └────────────┘
```

**Algorithm:**
1. Normalize query text
2. For each candidate:
   - Check exact match → flag + max score
   - Check phonetic match → flag + weight
   - Check edit distance → flag + weight
   - Check class overlap → flag + weight
   - Check dominant term → flag + weight
3. Sum weights, cap at 1.0
4. Sort by risk score descending

### 4. Explanation Generation

Every `RiskFlag` produces an `Explanation`:

```rust
pub struct Explanation {
    pub summary: String,      // "Sounds similar"
    pub detail: String,       // "The mark 'NYKE' sounds phonetically..."
    pub severity: f32,        // 0.8
    pub evidence: Vec<EvidenceItem>,
}
```

**Why explain > score:**
- Users need to understand *why* something is risky
- Legal decisions require justification
- Builds trust in automated analysis

## Data Flow

### Search Request

```
User: "Search for NIKE in Class 25"
          │
          ▼
┌─────────────────────────────────┐
│ SearchQuery {                   │
│   mark_text: "NIKE",            │
│   classes: [25],                │
│   limit: 100,                   │
│   phonetic: true,               │
│ }                               │
└─────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────┐
│ ManticoreBackend.search()       │
│                                 │
│ SELECT * FROM trademarks        │
│ WHERE MATCH('NIKE')             │
│ LIMIT 100                       │
└─────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────┐
│ Vec<(TrademarkRecord, f32)>     │
│ - ("NIKE", 1.0)                 │
│ - ("NYKE", 0.9)                 │
│ - ("NIKE SPORTS", 0.85)         │
│ - ...                           │
└─────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────┐
│ rerank::rerank()                │
│                                 │
│ For each candidate:             │
│ - Compute risk flags            │
│ - Calculate risk score          │
│ - Generate explanations         │
└─────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────┐
│ Vec<CandidateHit>               │
│ - NIKE: risk=1.0, [ExactMatch]  │
│ - NYKE: risk=0.85, [Phonetic,   │
│         ClassOverlap]           │
│ - ...                           │
└─────────────────────────────────┘
```

## Configuration

### RerankConfig

```rust
pub struct RerankConfig {
    pub phonetic_weight: f32,    // 0.3
    pub fuzzy_weight: f32,       // 0.2
    pub class_weight: f32,       // 0.25
    pub dominant_weight: f32,    // 0.25
    pub max_edit_distance: usize, // 3
}
```

These weights are the "algorithmic moat" - tunable based on:
- Legal precedent analysis
- User feedback
- A/B testing results

## Future: Tantivy Backend

When Manticore limitations are hit:

```rust
pub struct TantivyBackend {
    index: tantivy::Index,
    reader: tantivy::IndexReader,
}

impl SearchBackend for TantivyBackend {
    async fn search(&self, query: &SearchQuery) -> Result<...> {
        // Native Tantivy query with CustomScorer
    }
}
```

**Migration path:**
1. Implement `TantivyBackend` behind same trait
2. Run both backends in parallel (A/B)
3. Compare quality metrics
4. Switch when confident

## Testing Strategy

### Unit Tests (no network)

```rust
#[test]
fn test_phonetic_match() {
    let match = phonetic_match("NIKE", "NYKE");
    assert!(match.is_some());
}

#[test]
fn test_rerank_exact_match() {
    let query = SearchQuery::new("NIKE");
    let candidates = vec![(make_record("NIKE"), 1.0)];
    let hits = rerank(&query, candidates, &config);
    assert_eq!(hits[0].risk_score, 1.0);
}
```

### Integration Tests (with Manticore)

```rust
#[tokio::test]
#[ignore] // Requires running Manticore
async fn test_manticore_search() {
    let backend = ManticoreBackend::new(config);
    let results = backend.search(&query).await.unwrap();
    assert!(!results.is_empty());
}
```

### Golden Tests (regression)

```yaml
# tests/golden/nike.yaml
query: "NIKE"
classes: [25]
expected_top_5:
  - serial: "12345678"
    mark: "NIKE"
    must_have_flags: [ExactMatch]
  - serial: "87654321"
    mark: "NYKE"
    must_have_flags: [PhoneticMatch]
```

## Performance Considerations

1. **Retrieval** (Manticore): ~1ms per query
2. **Reranking** (Rust): ~0.1ms for 100 candidates
3. **Explanation**: Generated lazily on demand

Bottleneck is network latency to Manticore, not computation.

## Security

- No secrets in code (Manticore URL from env)
- No user data storage (stateless)
- Input sanitization in query builder (SQL escaping)
