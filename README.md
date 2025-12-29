# iLegalFlow Core

**Rust-based trademark search and scoring engine**

This crate provides the "brains" of the iLegalFlow system: retrieval, re-ranking, and explanation logic for trademark risk analysis.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    ilegalflow-core                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────┐  ┌──────────┐  ┌─────────┐  ┌───────────┐    │
│  │  model  │  │ features │  │  query  │  │  explain  │    │
│  │         │  │          │  │         │  │           │    │
│  │ Record  │  │ Phonetic │  │ Dialect │  │ RiskFlag  │    │
│  │ Status  │  │ N-grams  │  │ Builder │  │ Evidence  │    │
│  │ Query   │  │ EditDist │  │         │  │           │    │
│  └────┬────┘  └────┬─────┘  └────┬────┘  └─────┬─────┘    │
│       │            │             │              │          │
│       └────────────┴──────┬──────┴──────────────┘          │
│                           │                                │
│                    ┌──────▼──────┐                         │
│                    │   rerank    │                         │
│                    │             │                         │
│                    │ Score+Flag  │                         │
│                    └──────┬──────┘                         │
│                           │                                │
│              ┌────────────┴────────────┐                   │
│              │                         │                   │
│       ┌──────▼──────┐          ┌───────▼───────┐          │
│       │  backend-   │          │   (future)    │          │
│       │  manticore  │          │   backend-    │          │
│       │             │          │   tantivy     │          │
│       └─────────────┘          └───────────────┘          │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Crates

| Crate | Purpose |
|-------|---------|
| `ilegalflow-model` | Core types: TrademarkRecord, SearchQuery, RiskFlag |
| `ilegalflow-features` | Phonetics, n-grams, edit distance, normalization |
| `ilegalflow-query` | Query dialect translation (Manticore SQL, etc.) |
| `ilegalflow-explain` | Human-readable explanations for risk flags |
| `ilegalflow-rerank` | Re-ranking logic with configurable weights |
| `ilegalflow-backend-manticore` | Manticore Search HTTP adapter |
| `ilegalflow-eval` | CLI for testing and benchmarking |

## Quick Start

```bash
# Build all crates
cargo build

# Run tests
cargo test

# Run eval CLI
cargo run --bin eval -- health
cargo run --bin eval -- search "NIKE" --limit 10
```

## Design Principles

1. **Separation of concerns**: Retrieval (backend) vs. Reasoning (rerank/explain)
2. **Backend-agnostic**: `SearchBackend` trait allows swapping Manticore → Tantivy
3. **Explain > Score**: Every risk flag has evidence and explanation
4. **Pure functions**: Feature computation has no side effects
5. **Testable**: All logic can be unit tested without network

## Integration

### With ilegalflow-data

Consumes `marks.json` or `marks.parquet` produced by the data pipeline.

```rust
use ilegalflow_model::TrademarkRecord;

let records: Vec<TrademarkRecord> = serde_json::from_reader(file)?;
```

### With Manticore (on MSI)

```bash
# Manticore must be running on MSI
export MANTICORE_URL=http://msi-raider-linux.local:9308

cargo run --bin eval -- search "APPLE" --classes 9
```

## Development

### Prerequisites

- Rust 1.75+ (uses async traits)
- Manticore Search (for integration tests)

### Running Tests

```bash
# Unit tests (no network)
cargo test

# Integration tests (requires Manticore)
cargo test --features integration
```

## Related Repositories

| Repo | Relationship |
|------|--------------|
| `ilegalflow-data` | Produces: marks.json, Manticore index |
| `ilegalflow-web` | Consumes: search API (future) |
| `ilegalflow-extension` | Consumes: risk flags, explanations |

## License

MIT
