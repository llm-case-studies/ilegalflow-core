# Agent Instructions for ilegalflow-core

This document provides everything an AI agent needs to continue development on the Rust trademark search/scoring engine.

## Quick Start

```bash
# Clone and build
cd ~/Projects
git clone https://github.com/llm-case-studies/ilegalflow-core.git
cd ilegalflow-core

# Build all crates
cargo build

# Run tests
cargo test

# Check against Manticore (must be running on MSI)
cargo run --bin eval -- health --manticore-url http://127.0.0.1:9308
cargo run --bin eval -- search "NIKE" --limit 10
```

## Environment

### Recommended Machine: MSI Raider

The MSI laptop has:
- Manticore Search running (Docker, ports 9306/9308)
- 10K+ trademarks indexed
- Full USPTO data available
- Rust toolchain

```bash
# SSH to MSI
ssh alex@msi-raider-linux.local

# Check Manticore
curl -s 'http://127.0.0.1:9308/cli' -d "SELECT COUNT(*) FROM trademarks"
```

### Alternative: Local Development

For pure logic work (no Manticore needed):

```bash
# Unit tests work anywhere
cargo test --lib

# Skip integration tests
cargo test -- --skip manticore
```

## Crate Structure

```
ilegalflow-core/
├── Cargo.toml              # Workspace root
├── crates/
│   ├── model/              # Core types (TrademarkRecord, RiskFlag)
│   ├── features/           # Phonetics, n-grams, edit distance
│   ├── query/              # Query dialect translation
│   ├── explain/            # Human-readable explanations
│   ├── rerank/             # Scoring and re-ranking logic
│   ├── backend-manticore/  # Manticore HTTP adapter
│   └── eval/               # CLI tool
├── ARCHITECTURE.md
└── AGENT.md                # This file
```

## Current State

### Implemented
- [x] Core types (TrademarkRecord, SearchQuery, RiskFlag, CandidateHit)
- [x] Feature extraction (phonetics, edit distance, n-grams)
- [x] Query dialect (Manticore SQL generation)
- [x] Explanation generation
- [x] Re-ranking pipeline
- [x] Manticore backend (SearchBackend trait)
- [x] Eval CLI skeleton

### Not Yet Implemented
- [ ] Full integration testing with Manticore
- [ ] Benchmark suite with golden tests
- [ ] JSON API output schema
- [ ] TantivyBackend
- [ ] Famous marks detection
- [ ] Goods/services similarity scoring

## Key Traits

### SearchBackend

```rust
pub trait SearchBackend {
    async fn search(&self, query: &SearchQuery)
        -> Result<Vec<(TrademarkRecord, f32)>, BackendError>;
    async fn health_check(&self) -> Result<(), BackendError>;
    fn name(&self) -> &'static str;
}
```

Current implementation: `ManticoreBackend`

### QueryDialect

```rust
pub trait QueryDialect {
    type Output;
    fn translate(&self, query: &SearchQuery) -> Result<Self::Output, QueryError>;
}
```

Current implementation: `ManticoreDialect`

## Development Tasks

### Phase 0: Stabilization (Current)

1. **Verify builds on MSI**
   ```bash
   cargo build --release
   cargo test
   ```

2. **Test against live Manticore**
   ```bash
   cargo run --bin eval -- search "JUICY JUICE" --limit 20
   ```

3. **Fix any compilation issues**
   - Check rphonetic crate compatibility
   - Verify async trait syntax

### Phase 1: Integration

1. **Create golden test suite**
   ```yaml
   # tests/golden/basic.yaml
   queries:
     - text: "NIKE"
       expected_flags: [ExactMatch]
     - text: "NYKE"
       expected_flags: [PhoneticMatch]
   ```

2. **Add benchmark command**
   ```bash
   cargo run --bin eval -- benchmark --test-file tests/golden/basic.yaml
   ```

3. **Measure quality metrics**
   - Recall@50
   - Precision@10
   - Flag coverage

### Phase 2: Tantivy

1. **Add backend-tantivy crate**
2. **Implement SearchBackend for Tantivy**
3. **Add CustomScorer for risk scoring**
4. **Compare against Manticore results**

## Git Workflow

```bash
# Always pull latest
git pull origin main

# Create feature branch for significant changes
git checkout -b feat/tantivy-backend

# Commit with standard format
git commit -m "feat(backend): Add Tantivy implementation

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: [Agent] <noreply@anthropic.com>"

# Push and create PR
git push -u origin feat/tantivy-backend
```

## Troubleshooting

### Cargo build fails

```bash
# Update dependencies
cargo update

# Check for version conflicts
cargo tree -d
```

### rphonetic issues

The `rphonetic` crate may have API changes. Check:
```bash
cargo doc --open -p rphonetic
```

### Manticore connection refused

```bash
# Check if running
docker ps | grep manticore

# Start if needed
docker start manticore

# Check logs
docker logs manticore
```

### Async trait errors

Requires Rust 1.75+. Check version:
```bash
rustc --version
rustup update stable
```

## Testing Commands

```bash
# All tests
cargo test

# Specific crate
cargo test -p ilegalflow-features

# With output
cargo test -- --nocapture

# Single test
cargo test test_phonetic_match
```

## Dependencies

Key crates and their purposes:

| Crate | Purpose |
|-------|---------|
| `serde` | Serialization |
| `tokio` | Async runtime |
| `reqwest` | HTTP client for Manticore |
| `rphonetic` | Soundex/Metaphone |
| `thiserror` | Error types |
| `clap` | CLI parsing |
| `tracing` | Logging |

## Related Repos

| Repo | URL | Relationship |
|------|-----|--------------|
| ilegalflow-data | github.com/llm-case-studies/ilegalflow-data | Produces trademark data |
| ilegalflow (main) | github.com/??? | Documentation hub |

## Contact

Repository: https://github.com/llm-case-studies/ilegalflow-core
