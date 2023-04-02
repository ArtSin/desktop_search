use std::{process::ExitStatus, time::Duration};

use clap::{ArgAction, Parser};
use common_lib::settings::Settings;
use reqwest::Url;
use tokio::process::Command;
use tracing_subscriber::{
    filter::LevelFilter, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
};
use tracing_unwrap::ResultExt;

const SETTINGS_FILE_PATH: &str = "Settings.toml";
const ELASTICSEARCH_FOLDER: &str = "elasticsearch-8.7.0";
const TIKA_JAR: &str = "tika-server-standard-2.7.0.jar";
const TIKA_CONFIG: &str = "tika-config.xml";
const NN_SERVER_PATH: &str = "nn_server/nn_server";
const ONNX_RUNTIME_LIB_FOLDER: &str = "onnxruntime-linux-x64-gpu-1.14.1/lib";
const INDEXER_PATH: &str = "./indexer";

const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
const REQUEST_RETRIES: u32 = 120;
const REQUEST_RETRY_DURATION: Duration = Duration::from_secs(1);

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Don't run Apache Tika
    #[arg(long = "disable-tika", action = ArgAction::SetFalse)]
    tika_enabled: bool,
    /// Don't run nn_server
    #[arg(long = "disable-nn-server", action = ArgAction::SetFalse)]
    nn_server_enabled: bool,
}

pub async fn read_settings_file() -> Settings {
    match tokio::fs::read_to_string(SETTINGS_FILE_PATH).await {
        Ok(s) => toml::from_str(&s).expect_or_log("Error reading settings"),
        Err(e) => {
            tracing::warn!("Error reading settings file: {}, using defaults", e);
            Default::default()
        }
    }
}

async fn run_elasticsearch() -> tokio::io::Result<ExitStatus> {
    let mut es_path = ELASTICSEARCH_FOLDER.to_owned() + "/bin/elasticsearch";
    if cfg!(windows) {
        es_path += ".bat";
    }
    Command::new(es_path).spawn().unwrap_or_log().wait().await
}

async fn run_tika() -> tokio::io::Result<ExitStatus> {
    if cfg!(windows) {
        let tika_path = "tika.bat".to_owned();
        Command::new(tika_path).spawn().unwrap_or_log().wait().await
    } else {
        let java_path = ELASTICSEARCH_FOLDER.to_owned() + "/jdk/bin/java";
        Command::new(java_path)
            .args(["-jar", TIKA_JAR, "-c", TIKA_CONFIG])
            .spawn()
            .unwrap_or_log()
            .wait()
            .await
    }
}

async fn run_nn_server() -> tokio::io::Result<ExitStatus> {
    if cfg!(windows) {
        let nn_server_path = NN_SERVER_PATH.to_owned() + ".exe";
        Command::new(nn_server_path)
            .spawn()
            .unwrap_or_log()
            .wait()
            .await
    } else {
        let env_name = "LD_LIBRARY_PATH";
        let env_value = std::fs::canonicalize(ONNX_RUNTIME_LIB_FOLDER).unwrap_or_log();
        Command::new(NN_SERVER_PATH)
            .env(env_name, env_value)
            .spawn()
            .unwrap_or_log()
            .wait()
            .await
    }
}

async fn run_indexer() -> tokio::io::Result<ExitStatus> {
    let mut indexer_path = INDEXER_PATH.to_owned();
    if cfg!(windows) {
        indexer_path += ".exe";
    }
    Command::new(indexer_path)
        .spawn()
        .unwrap_or_log()
        .wait()
        .await
}

async fn retry_request(reqwest_client: &reqwest::Client, url: Url) -> reqwest::Result<()> {
    let mut res = Ok(());
    for _ in 0..REQUEST_RETRIES {
        let url = url.clone();
        res = async {
            reqwest_client.get(url).send().await?.error_for_status()?;
            Ok(())
        }
        .await;
        if res.is_ok() {
            return res;
        }
        tokio::time::sleep(REQUEST_RETRY_DURATION).await;
    }
    res
}

async fn await_elasticsearch(
    reqwest_client: &reqwest::Client,
    mut elasticsearch_url: Url,
) -> reqwest::Result<()> {
    elasticsearch_url.set_path("/_cluster/health");
    elasticsearch_url.set_query(Some("wait_for_status=yellow&timeout=2m"));
    retry_request(reqwest_client, elasticsearch_url).await
}

async fn await_tika(reqwest_client: &reqwest::Client, mut tika_url: Url) -> reqwest::Result<()> {
    tika_url.set_path("/tika");
    retry_request(reqwest_client, tika_url).await
}

async fn await_nn_server(
    reqwest_client: &reqwest::Client,
    mut nn_server_url: Url,
) -> reqwest::Result<()> {
    nn_server_url.set_path("/health");
    retry_request(reqwest_client, nn_server_url).await
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

    let settings = read_settings_file().await;

    let elasticsearch_task = tokio::spawn(async { run_elasticsearch().await });
    let tika_task = args
        .tika_enabled
        .then(|| tokio::spawn(async { run_tika().await }));
    let nn_server_task = args
        .nn_server_enabled
        .then(|| tokio::spawn(async { run_nn_server().await }));

    let reqwest_client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .unwrap_or_log();

    await_elasticsearch(&reqwest_client, settings.elasticsearch_url)
        .await
        .expect_or_log("Elasticsearch didn't start");
    tracing::info!("Elasticsearch started");
    if args.tika_enabled {
        await_tika(&reqwest_client, settings.tika_url)
            .await
            .expect_or_log("Apache Tika didn't start");
        tracing::info!("Apache Tika started");
    }
    if args.nn_server_enabled {
        await_nn_server(&reqwest_client, settings.nn_server_url)
            .await
            .expect_or_log("nn_server didn't start");
        tracing::info!("nn_server started");
    }

    let indexer_task = tokio::spawn(async { run_indexer().await });

    elasticsearch_task
        .await
        .unwrap_or_log()
        .expect_or_log("Failed to start Elasticsearch");
    if let Some(task) = tika_task {
        task.await
            .unwrap_or_log()
            .expect_or_log("Failed to start Apache Tika");
    }
    if let Some(task) = nn_server_task {
        task.await
            .unwrap_or_log()
            .expect_or_log("Failed to start nn_server");
    }
    indexer_task
        .await
        .unwrap_or_log()
        .expect_or_log("Failed to start indexer");
}
