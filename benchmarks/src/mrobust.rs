use std::{path::PathBuf, time::Instant};

use common_lib::search::{QueryType, SearchRequest, SearchResponse, TextQuery};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tracing_unwrap::{OptionExt, ResultExt};

use crate::get_reqwest_client;

const MAX_RANK: usize = 100;

#[derive(Debug, Deserialize)]
struct Document {
    id: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct Query {
    id: String,
    text: String,
}

#[derive(Debug, Serialize)]
struct QueryResult {
    query_id: String,
    iter: String,
    doc_id: String,
    rank: u32,
    similarity: u32,
    run_id: String,
    duration_s: f32,
}

pub async fn create_files(collection_path: PathBuf, output_dir: PathBuf) {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .has_headers(false)
        .from_path(collection_path)
        .expect_or_log("Error reading file");
    if let Err(err) = std::fs::create_dir(&output_dir) {
        tracing::warn!("Error creating output directory: {}", err);
    }

    for res in reader.deserialize() {
        let doc: Document = res.expect_or_log("Error reading record");

        let mut doc_path = output_dir.clone();
        if doc.id.len() >= 2 {
            doc_path.push(&doc.id[0..2]);
            _ = tokio::fs::create_dir(&doc_path).await;
            if doc.id.len() >= 4 {
                doc_path.push(&doc.id[2..4]);
                _ = tokio::fs::create_dir(&doc_path).await;
            }
        }
        doc_path.push(format!("{}.txt", doc.id));

        tokio::fs::write(doc_path, doc.text)
            .await
            .expect_or_log("Error writing file");
    }
}

async fn process_query(
    reqwest_client: &reqwest_middleware::ClientWithMiddleware,
    search_url: Url,
    content_enabled: bool,
    text_search_enabled: bool,
    reranking_enabled: bool,
    text_search_coeff: f64,
    reranking_coeff: f32,
    query: Query,
) -> Vec<QueryResult> {
    let search_request = SearchRequest {
        page: 0,
        query: QueryType::Text(TextQuery {
            query: query.text,
            content_enabled,
            text_search_enabled,
            image_search_enabled: false,
            reranking_enabled,
            text_search_pages: 1,
            image_search_pages: 1,
            query_coeff: 1.0,
            text_search_coeff,
            image_search_coeff: 1.0,
            reranking_coeff,
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
    if search_response.results.len() < MAX_RANK {
        tracing::warn!(
            "Search returned {} results instead of {}",
            search_response.results.len(),
            MAX_RANK
        );
    }
    assert!(
        search_response.results.len() <= MAX_RANK,
        "Search must return no more than {} results",
        MAX_RANK
    );

    search_response
        .results
        .into_iter()
        .enumerate()
        .map(|(i, res)| QueryResult {
            query_id: query.id.clone(),
            iter: "0".to_owned(),
            doc_id: res
                .file
                .path
                .file_stem()
                .unwrap_or_log()
                .to_str()
                .unwrap_or_log()
                .to_owned(),
            rank: i as u32,
            similarity: (MAX_RANK - i) as u32,
            run_id: "0".to_owned(),
            duration_s: duration.as_secs_f32(),
        })
        .collect()
}

pub async fn benchmark(
    content_enabled: bool,
    text_search_enabled: bool,
    reranking_enabled: bool,
    text_search_coeff: f64,
    reranking_coeff: f32,
    queries_path: PathBuf,
    result_path: PathBuf,
    indexer_address: Url,
) {
    // Create reader for queries
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .has_headers(false)
        .from_path(queries_path)
        .expect_or_log("Error reading file");
    // Create writer for results
    let mut writer = csv::WriterBuilder::new()
        .delimiter(b' ')
        .has_headers(false)
        .from_path(result_path)
        .expect_or_log("Error writing file");

    // Create reqwest client for HTTP requests
    let reqwest_client = get_reqwest_client();
    let mut search_url = indexer_address.clone();
    search_url.set_path("/search");

    // Process all queries
    for (i, res) in reader.deserialize().enumerate() {
        let query: Query = res.expect_or_log("Error reading record");
        // Write all query results
        for q_res in process_query(
            &reqwest_client,
            search_url.clone(),
            content_enabled,
            text_search_enabled,
            reranking_enabled,
            text_search_coeff,
            reranking_coeff,
            query,
        )
        .await
        {
            writer
                .serialize(q_res)
                .expect_or_log("Error writing record");
        }
        tracing::info!("Processed {}", i + 1);
    }
}
