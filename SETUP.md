# iLegalFlow Core - Development Environment Setup

This guide helps new team members or automated agents set up a complete development environment.

## Quick Start (TL;DR)

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Clone and build
git clone git@github.com:llm-case-studies/ilegalflow-core.git
cd ilegalflow-core
cargo build

# Run tests
cargo test
```

## Detailed Setup

### 1. Prerequisites

#### Operating System Support

| OS | Status | Notes |
|----|--------|-------|
| Ubuntu 22.04+ | Fully supported | Primary dev environment |
| macOS 13+ | Fully supported | Works on Intel and Apple Silicon |
| Windows 11 + WSL2 | Supported | Use Ubuntu WSL |
| Docker | Supported | See container section |

#### Required Software

| Software | Minimum Version | Install Command |
|----------|-----------------|-----------------|
| Rust | 1.75+ | See below |
| Git | 2.0+ | `sudo apt install git` |
| curl | Any | Pre-installed |

### 2. Install Rust

#### Linux/macOS

```bash
# Install rustup (Rust toolchain manager)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Follow prompts, select default installation
# Then reload your shell:
source ~/.cargo/env

# Verify installation
rustc --version  # Should be 1.75+
cargo --version
```

#### Update Existing Rust

```bash
rustup update stable
rustc --version
```

### 3. Clone Repository

```bash
# Via SSH (recommended for contributors)
git clone git@github.com:llm-case-studies/ilegalflow-core.git

# Via HTTPS (read-only)
git clone https://github.com/llm-case-studies/ilegalflow-core.git

cd ilegalflow-core
```

### 4. Build Project

```bash
# Debug build (fast compile, slower runtime)
cargo build

# Release build (slower compile, optimized)
cargo build --release

# Check without building (faster feedback)
cargo check
```

Expected output:
```
   Compiling ilegalflow-model v0.1.0
   Compiling ilegalflow-features v0.1.0
   ...
    Finished `dev` profile in X.XXs
```

### 5. Run Tests

```bash
# All unit tests
cargo test

# Specific crate
cargo test -p ilegalflow-features

# With output visible
cargo test -- --nocapture
```

### 6. Manticore Setup (for Integration Tests)

Manticore Search is required for full integration testing. Options:

#### Option A: Use Existing MSI Instance

If you have network access to MSI:

```bash
# Set environment variable
export MANTICORE_URL=http://msi-raider-linux.local:9308

# Test connection
curl -s "$MANTICORE_URL/cli" -d "SHOW TABLES"
```

#### Option B: Run Local Manticore (Docker)

```bash
# Pull Manticore image
docker pull manticoresearch/manticore

# Run container
docker run -d \
  --name manticore \
  -p 9306:9306 \
  -p 9308:9308 \
  -v manticore_data:/var/lib/manticore \
  manticoresearch/manticore

# Verify it's running
curl -s 'http://127.0.0.1:9308/cli' -d "SHOW TABLES"
```

Note: You'll need to index trademark data. See `ilegalflow-data` repo for indexing scripts.

#### Option C: Skip Integration Tests

For pure logic work:

```bash
# Unit tests work without Manticore
cargo test --lib
```

### 7. IDE Setup

#### VS Code (Recommended)

Install extensions:
- **rust-analyzer**: Rust language server
- **Even Better TOML**: Cargo.toml editing
- **CodeLLDB**: Debugging support

Settings (`.vscode/settings.json`):
```json
{
  "rust-analyzer.cargo.features": "all",
  "rust-analyzer.checkOnSave.command": "clippy"
}
```

#### IntelliJ/CLion

Install the Rust plugin, then open the project root folder.

#### Neovim

With rust-analyzer LSP:
```lua
require('lspconfig').rust_analyzer.setup{}
```

### 8. Development Workflow

```bash
# Check code (fast)
cargo check

# Run clippy (lints)
cargo clippy

# Format code
cargo fmt

# Build and test cycle
cargo test && cargo build

# Run eval CLI
cargo run --bin eval -- health
cargo run --bin eval -- search "NIKE" --limit 10
```

### 9. Troubleshooting

#### "async trait" errors

Requires Rust 1.75+:
```bash
rustc --version  # Check version
rustup update stable  # Update if needed
```

#### rphonetic compilation fails

Update dependencies:
```bash
cargo update
cargo build
```

#### Linker errors on Linux

Install build essentials:
```bash
sudo apt install build-essential pkg-config libssl-dev
```

#### Manticore connection refused

Check Docker container:
```bash
docker ps | grep manticore
docker start manticore  # if stopped
docker logs manticore   # check for errors
```

### 10. Container Development

#### Dockerfile

```dockerfile
FROM rust:1.83-slim

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifests first (cache dependencies)
COPY Cargo.toml Cargo.lock ./
COPY crates/*/Cargo.toml ./crates/

# Build dependencies (cached layer)
RUN mkdir -p crates/model/src crates/features/src crates/query/src \
    crates/explain/src crates/rerank/src crates/backend-manticore/src \
    crates/eval/src \
    && echo "fn main() {}" > crates/eval/src/main.rs \
    && touch crates/model/src/lib.rs crates/features/src/lib.rs \
    crates/query/src/lib.rs crates/explain/src/lib.rs \
    crates/rerank/src/lib.rs crates/backend-manticore/src/lib.rs \
    && cargo build --release \
    && rm -rf crates/*/src

# Copy source
COPY crates ./crates

# Build for real
RUN cargo build --release

# Run tests
RUN cargo test --release
```

#### Docker Compose (with Manticore)

```yaml
# docker-compose.yml
version: "3.8"

services:
  manticore:
    image: manticoresearch/manticore
    ports:
      - "9306:9306"
      - "9308:9308"
    volumes:
      - manticore_data:/var/lib/manticore

  ilegalflow-core:
    build: .
    depends_on:
      - manticore
    environment:
      - MANTICORE_URL=http://manticore:9308
    command: cargo run --bin eval -- health

volumes:
  manticore_data:
```

### 11. Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `MANTICORE_URL` | `http://127.0.0.1:9308` | Manticore HTTP endpoint |
| `RUST_LOG` | `info` | Logging level (debug, info, warn, error) |
| `RUST_BACKTRACE` | `0` | Set to `1` for stack traces on panic |

Example:
```bash
export MANTICORE_URL=http://msi-raider-linux.local:9308
export RUST_LOG=debug
cargo run --bin eval -- search "NIKE" --limit 10
```

### 12. Machine-Specific Notes

#### MSI Raider (Primary Dev)

- Manticore running in Docker
- Full USPTO data indexed
- SSH: `alex@msi-raider-linux.local`

```bash
ssh alex@msi-raider-linux.local
cd ~/Projects/ilegalflow-core
cargo build && cargo test
```

#### iMac / Mac Mini

- Remote connection to MSI's Manticore
- Apple Silicon: Rust works natively

```bash
export MANTICORE_URL=http://msi-raider-linux.local:9308
cargo test
```

#### GitHub Codespaces / Cloud IDEs

- Use Docker Compose for Manticore
- Or skip integration tests with `cargo test --lib`

### 13. Getting Help

- **AGENT.md**: Instructions for AI agents
- **ARCHITECTURE.md**: System design
- **TEST_PLAN.md**: Testing strategy
- **Issues**: https://github.com/llm-case-studies/ilegalflow-core/issues

### 14. Verification Checklist

After setup, verify everything works:

- [ ] `rustc --version` shows 1.75+
- [ ] `cargo build` succeeds
- [ ] `cargo test` passes all unit tests
- [ ] `cargo run --bin eval -- health` shows OK (if Manticore available)
- [ ] `cargo clippy` shows no errors
