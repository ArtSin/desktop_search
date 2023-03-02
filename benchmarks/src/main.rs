use std::path::PathBuf;

use clap::{Parser, Subcommand};
use reqwest::Url;
use tracing_subscriber::{
    filter::LevelFilter, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

mod coco;
mod mrobust;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value_t = Url::parse("http://127.0.0.1:11000/").unwrap())]
    indexer_address: Url,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Evaluate text-to-image search on COCO dataset.
    /// Before running, you must index all images and set number of results per page to 100
    Coco {
        /// Path to the captions file (captions_val2017.json)
        captions_path: PathBuf,
        /// Directory for storing results
        results_dir: PathBuf,
    },
    /// Evaluate text-to-text search on mRobust dataset
    MRobust(MRobust),
}

#[derive(Debug, Parser)]
struct MRobust {
    #[command(subcommand)]
    command: MRobustCommands,
}

#[derive(Debug, Subcommand)]
enum MRobustCommands {
    /// Create file for each document in collection
    CreateFiles {
        /// Path to the collection file
        collection_path: PathBuf,
        /// Directory for storing output
        output_dir: PathBuf,
    },
    /// Run benchmark.
    /// Before running, you must index all documents and set number of results per page to 100
    Run {
        /// Enable content search
        #[arg(short, long, action)]
        content_enabled: bool,
        /// Enable semantic text search
        #[arg(short, long, action)]
        text_search_enabled: bool,
        /// Semantic text search coefficient
        #[arg(short = 'k', long, default_value_t = 1.0)]
        text_search_coeff: f64,
        /// Path to the queries file
        queries_path: PathBuf,
        /// Path to the results file
        result_path: PathBuf,
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
        Commands::Coco {
            captions_path,
            results_dir,
        } => coco::benchmark(captions_path, results_dir, args.indexer_address).await,
        Commands::MRobust(MRobust { command }) => match command {
            MRobustCommands::CreateFiles {
                collection_path,
                output_dir,
            } => mrobust::create_files(collection_path, output_dir).await,
            MRobustCommands::Run {
                content_enabled,
                text_search_enabled,
                text_search_coeff,
                queries_path,
                result_path,
            } => {
                mrobust::benchmark(
                    content_enabled,
                    text_search_enabled,
                    text_search_coeff,
                    queries_path,
                    result_path,
                    args.indexer_address,
                )
                .await
            }
        },
    }
}
