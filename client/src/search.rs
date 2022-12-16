use common_lib::{
    elasticsearch::{FileES, ELASTICSEARCH_INDEX},
    search::{SearchRequest, SearchResponse},
};
use elasticsearch::{Elasticsearch, SearchParts};
use serde_json::{json, Value};
use tauri::async_runtime::RwLock;

use crate::ClientState;

use self::query::{range, simple_query_string};

const RESULTS_PER_PAGE: u32 = 20;

fn get_request_body(search_request: SearchRequest) -> Value {
    let es_request_must = [
        Some(simple_query_string(search_request.query, &["path", "hash"])),
        (search_request.modified_from.is_some() || search_request.modified_to.is_some()).then(
            || {
                range(
                    "modified",
                    search_request.modified_from.map(|d| d.timestamp()),
                    search_request.modified_to.map(|d| d.timestamp()),
                )
            },
        ),
        (search_request.size_from.is_some() || search_request.size_to.is_some())
            .then(|| range("size", search_request.size_from, search_request.size_to)),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    json!({
        "query": {
            "bool": {
                "must": es_request_must,
            }
        }
    })
}

async fn get_es_response(
    es_client: &Elasticsearch,
    page: u32,
    es_request_body: Value,
) -> Result<Value, elasticsearch::Error> {
    es_client
        .search(SearchParts::Index(&[ELASTICSEARCH_INDEX]))
        .from((page * RESULTS_PER_PAGE).into())
        .size(RESULTS_PER_PAGE.into())
        .body(es_request_body)
        .send()
        .await?
        .json::<Value>()
        .await
}

fn get_results(es_response_body: &Value) -> Vec<FileES> {
    es_response_body["hits"]["hits"]
        .as_array()
        .unwrap()
        .iter()
        .map(|val| {
            let mut file_es: FileES = serde_json::from_value(val["_source"].clone()).unwrap();
            file_es._id = Some(val["_id"].as_str().unwrap().to_owned());
            file_es
        })
        .collect()
}

#[tauri::command]
pub async fn search(
    state: tauri::State<'_, RwLock<ClientState>>,
    search_request: SearchRequest,
) -> Result<SearchResponse, String> {
    let es_request_body = get_request_body(search_request);
    let es_response_body = get_es_response(&state.read().await.es_client, 0, es_request_body)
        .await
        .map_err(|e| e.to_string())?;
    let results = get_results(&es_response_body);
    Ok(SearchResponse { results })
}

mod query {
    use serde::Serialize;
    use serde_json::{json, Value};

    pub fn simple_query_string(mut query: String, fields: &[&str]) -> Value {
        if query.is_empty() {
            query = "*".to_owned();
        }
        json!({
            "simple_query_string": {
                "query": query,
                "fields": fields,
            }
        })
    }

    // pub fn term(field: &str, value: impl Serialize) -> Value {
    //     json!({
    //         "term": {
    //             field: {
    //                 "value": value,
    //             }
    //         }
    //     })
    // }

    // pub fn match_(field: &str, query: impl Serialize) -> Value {
    //     json!({
    //         "match": {
    //             field: {
    //                 "query": query,
    //             }
    //         }
    //     })
    // }

    pub fn range(field: &str, gte: impl Serialize, lte: impl Serialize) -> Value {
        json!({
            "range": {
                field: {
                    "gte": gte,
                    "lte": lte,
                }
            }
        })
    }

    // pub fn suggest(query: Option<String>, field: &str) -> Value {
    //     json!({
    //         "text": query.unwrap_or_else(|| "*".to_owned()),
    //         "simple_phrase": {
    //             "phrase": {
    //                 "field": field,
    //                 "size": 1,
    //                 "gram_size": 3,
    //                 "direct_generator": [
    //                     {
    //                         "field": field,
    //                         "suggest_mode": "missing"
    //                     }
    //                 ],
    //                 "highlight": {
    //                     "pre_tag": "<i>",
    //                     "post_tag": "</i>"
    //                 }
    //             }
    //         }
    //     })
    // }
}
