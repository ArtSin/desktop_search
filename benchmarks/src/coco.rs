use std::{path::PathBuf, time::Instant};

use common_lib::search::{QueryType, SearchRequest, SearchResponse, TextQuery};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tracing_unwrap::{OptionExt, ResultExt};

use crate::get_reqwest_client;

const MAX_RANK: usize = 100;

#[derive(Debug, Deserialize)]
struct Captions {
    annotations: Vec<Caption>,
}

#[derive(Debug, Deserialize)]
struct Caption {
    image_id: u32,
    id: u32,
    caption: String,
}

#[derive(Debug, Serialize)]
struct ImageCaptionResult {
    image_id: u32,
    id: u32,
    rank: Option<u32>,
    duration_s: f32,
}

async fn process_caption(
    reqwest_client: &reqwest_middleware::ClientWithMiddleware,
    search_url: Url,
    caption: Caption,
) -> ImageCaptionResult {
    let search_request = SearchRequest {
        page: 0,
        query: QueryType::Text(TextQuery {
            query: caption.caption,
            content_enabled: false,
            text_search_enabled: false,
            image_search_enabled: true,
            reranking_enabled: false,
            text_search_pages: 1,
            image_search_pages: 1,
            query_coeff: 1.0,
            text_search_coeff: 1.0,
            image_search_coeff: 1.0,
            reranking_coeff: 1.0,
        }),
        path_prefix: None,
        content_type: None,
        path_enabled: false,
        hash_enabled: false,
        modified_from: None,
        modified_to: None,
        size_from: None,
        size_to: None,
        image_data: Default::default(),
        multimedia_data: Default::default(),
        document_data: Default::default(),
    };

    let start_time = Instant::now();
    let response = reqwest_client
        .post(search_url)
        .json(&search_request)
        .send()
        .await
        .expect_or_log("Error sending request")
        .error_for_status()
        .expect_or_log("Server returned error");
    let duration = Instant::now() - start_time;

    let search_response: SearchResponse = response
        .json()
        .await
        .expect_or_log("Error parsing response");
    assert_eq!(
        search_response.results.len(),
        MAX_RANK,
        "Search must return exactly {} results",
        MAX_RANK
    );

    let rank = search_response
        .results
        .into_iter()
        .enumerate()
        .find_map(|(i, res)| {
            (res.file
                .path
                .file_stem()
                .unwrap_or_log()
                .to_str()
                .unwrap_or_log()
                .parse::<u32>()
                .unwrap_or_log()
                == caption.image_id)
                .then_some((i + 1) as u32)
        });

    ImageCaptionResult {
        image_id: caption.image_id,
        id: caption.id,
        rank,
        duration_s: duration.as_secs_f32(),
    }
}

fn calculate_recall(results: &[ImageCaptionResult]) -> ([u32; MAX_RANK], [f32; MAX_RANK]) {
    let mut recall_cnt = [0; MAX_RANK];
    for res in results {
        if let Some(rank) = res.rank {
            recall_cnt[(rank - 1) as usize] += 1;
        }
    }
    for i in 1..MAX_RANK {
        recall_cnt[i] += recall_cnt[i - 1];
    }
    let recall_percent = recall_cnt.map(|x| ((100 * x) as f32) / (results.len() as f32));
    (recall_cnt, recall_percent)
}

fn write_all_results(results: &[ImageCaptionResult], mut results_dir: PathBuf) -> csv::Result<()> {
    results_dir.push("all_results.csv");
    let mut writer = csv::Writer::from_path(results_dir)?;
    for result in results {
        writer.serialize(result)?;
    }
    Ok(())
}

fn write_recall(
    recall: ([u32; MAX_RANK], [f32; MAX_RANK]),
    mut results_dir: PathBuf,
) -> csv::Result<()> {
    results_dir.push("recall.csv");
    let mut writer = csv::Writer::from_path(results_dir)?;
    writer.write_record((1..=MAX_RANK).map(|i| format!("Recall@{i}")))?;
    writer.write_record(recall.0.map(|x| x.to_string()))?;
    writer.write_record(recall.1.map(|x| x.to_string()))
}

pub async fn benchmark(captions_path: PathBuf, results_dir: PathBuf, indexer_address: Url) {
    // Read captions from JSON file
    let json_str = tokio::fs::read_to_string(captions_path)
        .await
        .expect_or_log("Error reading file");
    let captions: Captions = serde_json::from_str(&json_str).expect_or_log("Error parsing file");

    // Create reqwest client for HTTP requests
    let reqwest_client = get_reqwest_client();
    let mut search_url = indexer_address.clone();
    search_url.set_path("/search");

    // Process all captions
    let mut results = Vec::new();
    let captions_cnt = captions.annotations.len();
    for (i, caption) in captions.annotations.into_iter().enumerate() {
        results.push(process_caption(&reqwest_client, search_url.clone(), caption).await);
        tracing::info!("Processed {}/{}", i + 1, captions_cnt);
    }

    // Write results
    if let Err(err) = std::fs::create_dir(&results_dir) {
        tracing::warn!("Error creating results directory: {}", err);
    }
    write_all_results(&results, results_dir.clone()).expect_or_log("Error writing all results");
    write_recall(calculate_recall(&results), results_dir)
        .expect_or_log("Error writing recall results");
    tracing::info!("The results were written to files");
}
