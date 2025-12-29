//! Evaluation CLI for testing trademark search quality.
//!
//! Usage:
//!     eval search "NIKE" --limit 20
//!     eval benchmark --test-file tests.yaml
//!     eval health

use anyhow::Result;
use clap::{Parser, Subcommand};
use ilegalflow_backend_manticore::{ManticoreBackend, ManticoreConfig, SearchBackend};
use ilegalflow_explain::summarize_risk;
use ilegalflow_model::SearchQuery;
use ilegalflow_rerank::{rerank, RerankConfig};

#[derive(Parser)]
#[command(name = "eval")]
#[command(about = "Evaluate trademark search quality")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Manticore URL
    #[arg(long, default_value = "http://127.0.0.1:9308")]
    manticore_url: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Search for a trademark
    Search {
        /// Mark text to search
        query: String,

        /// Maximum results
        #[arg(short, long, default_value = "20")]
        limit: usize,

        /// Nice classes to filter (comma-separated)
        #[arg(short, long)]
        classes: Option<String>,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Check backend health
    Health,

    /// Run benchmark against test file
    Benchmark {
        /// Path to test YAML file
        #[arg(short, long)]
        test_file: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("ilegalflow=debug".parse()?),
        )
        .init();

    let cli = Cli::parse();

    let config = ManticoreConfig {
        base_url: cli.manticore_url,
        ..Default::default()
    };
    let backend = ManticoreBackend::new(config);

    match cli.command {
        Commands::Search {
            query,
            limit,
            classes,
            format,
        } => {
            run_search(&backend, &query, limit, classes, &format).await?;
        }
        Commands::Health => {
            run_health(&backend).await?;
        }
        Commands::Benchmark { test_file } => {
            run_benchmark(&backend, &test_file).await?;
        }
    }

    Ok(())
}

async fn run_search(
    backend: &ManticoreBackend,
    query_text: &str,
    limit: usize,
    classes: Option<String>,
    format: &str,
) -> Result<()> {
    let classes: Vec<u16> = classes
        .map(|s| {
            s.split(',')
                .filter_map(|c| c.trim().parse().ok())
                .collect()
        })
        .unwrap_or_default();

    let query = SearchQuery {
        mark_text: query_text.to_string(),
        classes: classes.clone(),
        limit,
        ..Default::default()
    };

    println!("Searching for: {}", query_text);
    if !classes.is_empty() {
        println!("Classes: {:?}", classes);
    }
    println!("---");

    // Retrieve from backend
    let candidates = backend.search(&query).await?;
    println!("Retrieved {} candidates from Manticore", candidates.len());

    // Re-rank with our scoring logic
    let config = RerankConfig::default();
    let hits = rerank(&query, candidates, &config);

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&hits)?);
    } else {
        for (i, hit) in hits.iter().enumerate() {
            println!(
                "\n{}. {} (Serial: {})",
                i + 1,
                hit.record.mark_text,
                hit.record.serial_number
            );
            println!("   Status: {:?}", hit.record.status);
            println!(
                "   Risk Score: {:.2} | Retrieval Score: {:.2}",
                hit.risk_score, hit.retrieval_score
            );
            println!("   {}", summarize_risk(hit));

            if !hit.flags.is_empty() {
                println!("   Flags: {:?}", hit.flags.iter().map(|f| f.label()).collect::<Vec<_>>());
            }
        }
    }

    println!("\n---");
    println!("Total: {} results", hits.len());

    Ok(())
}

async fn run_health(backend: &ManticoreBackend) -> Result<()> {
    print!("Checking {} backend... ", backend.name());

    match backend.health_check().await {
        Ok(()) => {
            println!("OK");
            Ok(())
        }
        Err(e) => {
            println!("FAILED: {}", e);
            std::process::exit(1);
        }
    }
}

async fn run_benchmark(_backend: &ManticoreBackend, test_file: &str) -> Result<()> {
    // TODO: Implement benchmark loading and execution
    println!("Benchmark not yet implemented");
    println!("Would load tests from: {}", test_file);

    // Expected format:
    // queries:
    //   - text: "NIKE"
    //     expected_top: ["NIKE", "NYKE"]
    //     expected_flags: [phonetic]
    //   - text: "APPLE"
    //     classes: [9]
    //     expected_top: ["APPLE"]

    Ok(())
}
