use std::path::PathBuf;

use clap::{Parser, Subcommand};
use reqwest::Url;
use tracing_subscriber::{
    filter::LevelFilter, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

mod coco;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value_t = Url::parse("http://127.0.0.1:11000/").unwrap())]
    indexer_address: Url,
    #[command(subcommand)]
    command: Commands,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Subcommand)]
enum Commands {
    /// Evaluate text-to-image search on COCO dataset.
    /// Before running, you must index all images and set number of results per page to 100
    COCO {
        /// Path to the captions file (captions_val2017.json)
        captions_path: PathBuf,
        /// Directory for storing results
        results_dir: PathBuf,
    },
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::DEBUG.into())
                .from_env_lossy(),
        )
        .init();

    match args.command {
        Commands::COCO {
            captions_path,
            results_dir,
        } => coco::benchmark_coco(captions_path, results_dir, args.indexer_address).await,
    }
}
