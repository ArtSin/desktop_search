use std::{cmp::min, sync::Arc};

use axum::{extract::State, http::StatusCode, Json};
use common_lib::{
    elasticsearch::{FileES, ELASTICSEARCH_INDEX, ELASTICSEARCH_MAX_SIZE},
    search::{
        ContentTypeRequestItem, DocumentHighlightedFields, HighlightedFields,
        ImageHighlightedFields, ImageQuery, MultimediaHighlightedFields, PageType, QueryType,
        SearchRequest, SearchResponse, SearchResult, TextQuery,
    },
};
use elasticsearch::{Elasticsearch, SearchParts};
use serde_json::{json, Value};
use tracing_unwrap::{OptionExt, ResultExt};
use url::Url;
use uuid::Uuid;

use crate::{
    embeddings::{
        get_image_search_image_embedding, get_image_search_text_embedding,
        get_text_search_embedding,
    },
    ServerState,
};

use self::query::{range, simple_query_string, suggest, term, terms};

mod query;

const ADJACENT_PAGES: u32 = 3;

fn get_es_request_filter(search_request: &SearchRequest) -> Vec<Value> {
    [
        search_request
            .path_prefix
            .as_ref()
            .map(|x| term("path.hierarchy", x.to_string_lossy().replace('\\', "/"))),
        search_request.content_type.as_ref().map(|v| {
            let mut include_type = Vec::new();
            let mut include_subtypes = Vec::new();
            let mut exclude_type = Vec::new();
            let mut exclude_subtypes = Vec::new();

            for x in v {
                match x {
                    ContentTypeRequestItem::IncludeType { type_ } => include_type.push(type_),
                    ContentTypeRequestItem::IncludeSubtypes { subtypes } => {
                        include_subtypes.extend(subtypes)
                    }
                    ContentTypeRequestItem::ExcludeType { type_ } => exclude_type.push(type_),
                    ContentTypeRequestItem::ExcludeSubtypes { type_, subtypes } => {
                        include_type.push(type_);
                        exclude_subtypes.extend(subtypes)
                    }
                };
            }

            json!({
                "bool": {
                    "should": [
                        terms("content_type_mime_type", include_type),
                        terms("content_type_mime_essence", include_subtypes)
                    ],
                    "must_not": [
                        terms("content_type_mime_type", exclude_type),
                        terms("content_type_mime_essence", exclude_subtypes)
                    ]
                }
            })
        }),
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
        // Fields for image files
        (search_request.image_data.width_from.is_some()
            || search_request.image_data.width_to.is_some())
        .then(|| {
            range(
                "width",
                search_request.image_data.width_from,
                search_request.image_data.width_to,
            )
        }),
        (search_request.image_data.height_from.is_some()
            || search_request.image_data.height_to.is_some())
        .then(|| {
            range(
                "height",
                search_request.image_data.height_from,
                search_request.image_data.height_to,
            )
        }),
        (search_request.image_data.x_resolution_from.is_some()
            || search_request.image_data.x_resolution_to.is_some()
            || search_request.image_data.y_resolution_from.is_some()
            || search_request.image_data.y_resolution_to.is_some())
        .then(|| term("resolution_unit", search_request.image_data.resolution_unit)),
        (search_request.image_data.x_resolution_from.is_some()
            || search_request.image_data.x_resolution_to.is_some())
        .then(|| {
            range(
                "x_resolution",
                search_request.image_data.x_resolution_from,
                search_request.image_data.x_resolution_to,
            )
        }),
        (search_request.image_data.y_resolution_from.is_some()
            || search_request.image_data.y_resolution_to.is_some())
        .then(|| {
            range(
                "y_resolution",
                search_request.image_data.y_resolution_from,
                search_request.image_data.y_resolution_to,
            )
        }),
        (search_request.image_data.f_number_from.is_some()
            || search_request.image_data.f_number_to.is_some())
        .then(|| {
            range(
                "f_number",
                search_request.image_data.f_number_from,
                search_request.image_data.f_number_to,
            )
        }),
        (search_request.image_data.focal_length_from.is_some()
            || search_request.image_data.focal_length_to.is_some())
        .then(|| {
            range(
                "focal_length",
                search_request.image_data.focal_length_from,
                search_request.image_data.focal_length_to,
            )
        }),
        (search_request.image_data.exposure_time_from.is_some()
            || search_request.image_data.exposure_time_to.is_some())
        .then(|| {
            range(
                "exposure_time",
                search_request.image_data.exposure_time_from,
                search_request.image_data.exposure_time_to,
            )
        }),
        search_request
            .image_data
            .flash_fired
            .map(|x| term("flash_fired", x)),
        // Fields for multimedia files
        (search_request.multimedia_data.duration_min_from.is_some()
            || search_request.multimedia_data.duration_min_to.is_some())
        .then(|| {
            range(
                "duration",
                search_request
                    .multimedia_data
                    .duration_min_from
                    .map(|x| x * 60.0),
                search_request
                    .multimedia_data
                    .duration_min_to
                    .map(|x| x * 60.0),
            )
        }),
        (search_request
            .multimedia_data
            .audio_sample_rate_from
            .is_some()
            || search_request
                .multimedia_data
                .audio_sample_rate_to
                .is_some())
        .then(|| {
            range(
                "audio_sample_rate",
                search_request.multimedia_data.audio_sample_rate_from,
                search_request.multimedia_data.audio_sample_rate_to,
            )
        }),
        search_request
            .multimedia_data
            .audio_channel_type
            .map(|x| term("audio_channel_type", x)),
        // Fields for document files
        (search_request.document_data.doc_created_from.is_some()
            || search_request.document_data.doc_created_to.is_some())
        .then(|| {
            range(
                "doc_created",
                search_request
                    .document_data
                    .doc_created_from
                    .map(|d| d.timestamp()),
                search_request
                    .document_data
                    .doc_created_to
                    .map(|d| d.timestamp()),
            )
        }),
        (search_request.document_data.doc_modified_from.is_some()
            || search_request.document_data.doc_modified_to.is_some())
        .then(|| {
            range(
                "doc_modified",
                search_request
                    .document_data
                    .doc_modified_from
                    .map(|d| d.timestamp()),
                search_request
                    .document_data
                    .doc_modified_to
                    .map(|d| d.timestamp()),
            )
        }),
        (search_request.document_data.num_pages_from.is_some()
            || search_request.document_data.num_pages_to.is_some())
        .then(|| {
            range(
                "num_pages",
                search_request.document_data.num_pages_from,
                search_request.document_data.num_pages_to,
            )
        }),
        (search_request.document_data.num_words_from.is_some()
            || search_request.document_data.num_words_to.is_some())
        .then(|| {
            range(
                "num_words",
                search_request.document_data.num_words_from,
                search_request.document_data.num_words_to,
            )
        }),
        (search_request.document_data.num_characters_from.is_some()
            || search_request.document_data.num_characters_to.is_some())
        .then(|| {
            range(
                "num_characters",
                search_request.document_data.num_characters_from,
                search_request.document_data.num_characters_to,
            )
        }),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn get_es_request_must(search_request: &SearchRequest) -> Vec<Value> {
    let query_string = match search_request.query {
        QueryType::Text(TextQuery {
            ref query,
            content_enabled,
            ..
        }) => {
            let query_fields = [
                search_request.path_enabled.then_some("path"),
                search_request.hash_enabled.then_some("hash"),
                content_enabled.then_some("content"),
                // Fields for image files
                search_request
                    .image_data
                    .image_make_enabled
                    .then_some("image_make"),
                search_request
                    .image_data
                    .image_model_enabled
                    .then_some("image_model"),
                search_request
                    .image_data
                    .image_software_enabled
                    .then_some("image_software"),
                // Fields for multimedia files
                search_request
                    .multimedia_data
                    .artist_enabled
                    .then_some("artist"),
                search_request
                    .multimedia_data
                    .album_enabled
                    .then_some("album"),
                search_request
                    .multimedia_data
                    .genre_enabled
                    .then_some("genre"),
                search_request
                    .multimedia_data
                    .track_number_enabled
                    .then_some("track_number"),
                search_request
                    .multimedia_data
                    .disc_number_enabled
                    .then_some("disc_number"),
                search_request
                    .multimedia_data
                    .release_date_enabled
                    .then_some("release_date"),
                // Fields for document files
                search_request
                    .document_data
                    .title_enabled
                    .then_some("title"),
                search_request
                    .document_data
                    .creator_enabled
                    .then_some("creator"),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

            if query_fields.is_empty() {
                None
            } else {
                Some(simple_query_string(query.clone(), &query_fields))
            }
        }
        _ => None,
    };
    [query_string].into_iter().flatten().collect()
}

async fn get_request_body(
    max_sentences: u32,
    sentences_per_paragraph: u32,
    results_per_page: u32,
    reqwest_client: &reqwest::Client,
    nnserver_url: Url,
    knn_candidates_multiplier: u32,
    search_request: &SearchRequest,
) -> anyhow::Result<Value> {
    let mut request_body = Value::Object(serde_json::Map::new());
    let mut request_body_knn = Vec::new();

    let es_request_must = get_es_request_must(search_request);
    let es_request_filter = get_es_request_filter(search_request);

    match search_request.query {
        QueryType::Text(TextQuery {
            ref query,
            text_search_enabled,
            image_search_enabled,
            text_search_pages,
            image_search_pages,
            query_coeff,
            text_search_coeff,
            image_search_coeff,
            ..
        }) => {
            if text_search_enabled && !query.is_empty() {
                let text_search_embedding = get_text_search_embedding(
                    max_sentences,
                    sentences_per_paragraph,
                    reqwest_client,
                    nnserver_url.clone(),
                    query,
                )
                .await?;

                let k = min(
                    results_per_page * text_search_pages,
                    ELASTICSEARCH_MAX_SIZE as u32,
                );
                let num_candidates = min(
                    results_per_page * text_search_pages * knn_candidates_multiplier,
                    ELASTICSEARCH_MAX_SIZE as u32,
                );
                request_body_knn.push(json!({
                    "field": "text_embedding",
                    "query_vector": text_search_embedding.embedding,
                    "k": k,
                    "num_candidates": num_candidates,
                    "filter": es_request_filter,
                    "boost": text_search_coeff
                }));
            }

            if image_search_enabled && !query.is_empty() {
                let image_search_text_embedding =
                    get_image_search_text_embedding(reqwest_client, nnserver_url, query).await?;

                let k = min(
                    results_per_page * image_search_pages,
                    ELASTICSEARCH_MAX_SIZE as u32,
                );
                let num_candidates = min(
                    results_per_page * image_search_pages * knn_candidates_multiplier,
                    ELASTICSEARCH_MAX_SIZE as u32,
                );
                request_body_knn.push(json!({
                    "field": "image_embedding",
                    "query_vector": image_search_text_embedding.embedding,
                    "k": k,
                    "num_candidates": num_candidates,
                    "filter": es_request_filter,
                    "boost": image_search_coeff
                }));
            }

            request_body.as_object_mut().unwrap_or_log().insert(
                "query".to_owned(),
                json!({
                    "bool": {
                        "must": es_request_must,
                        "filter": es_request_filter,
                        "boost": query_coeff
                    }
                }),
            );

            request_body.as_object_mut().unwrap_or_log().insert(
                "highlight".to_owned(),
                json!({
                    "pre_tags": ["<b>"],
                    "post_tags": ["</b>"],
                    "encoder": "html",
                    "number_of_fragments": 0,
                    "max_analyzed_offset": 1000000,
                    "fields": {
                        "path": {},
                        "hash": {},
                        "content": {
                            "fragment_size": 300,
                            "no_match_size": 300,
                            "number_of_fragments": 1
                        },
                        // Fields for image files
                        "image_make": {},
                        "image_model": {},
                        "image_software": {},
                        // Fields for multimedia files
                        "artist": {},
                        "album": {},
                        "genre": {},
                        "track_number": {},
                        "disc_number": {},
                        "release_date": {},
                        // Fields for document files
                        "title": {},
                        "creator": {}
                    }
                }),
            );

            request_body.as_object_mut().unwrap_or_log().insert(
                "suggest".to_owned(),
                suggest(
                    query.clone(),
                    "content.shingles",
                    &["content.shingles", "path.shingles"],
                ),
            );
        }
        QueryType::Image(ImageQuery {
            ref image_path,
            image_search_pages,
        }) => {
            let image_search_image_embedding =
                get_image_search_image_embedding(reqwest_client, nnserver_url, image_path).await?;
            let embedding = image_search_image_embedding
                .embedding
                .ok_or_else(|| anyhow::anyhow!("Incorrect image"))?;

            let k = min(
                results_per_page * image_search_pages,
                ELASTICSEARCH_MAX_SIZE as u32,
            );
            let num_candidates = min(
                results_per_page * image_search_pages * knn_candidates_multiplier,
                ELASTICSEARCH_MAX_SIZE as u32,
            );
            request_body_knn.push(json!({
                "field": "image_embedding",
                "query_vector": embedding,
                "k": k,
                "num_candidates": num_candidates,
                "filter": es_request_filter
            }));
        }
    }

    if !request_body_knn.is_empty() {
        request_body
            .as_object_mut()
            .unwrap_or_log()
            .insert("knn".to_owned(), Value::Array(request_body_knn));
    }
    Ok(request_body)
}

async fn get_es_response(
    results_per_page: u32,
    es_client: &Elasticsearch,
    page: u32,
    es_request_body: Value,
) -> Result<Value, elasticsearch::Error> {
    es_client
        .search(SearchParts::Index(&[ELASTICSEARCH_INDEX]))
        .from((page * results_per_page).into())
        .size(results_per_page.into())
        .body(es_request_body)
        .send()
        .await?
        .json::<Value>()
        .await
}

fn get_highlighted_field(result_value: &Value, field: &str, field_value: &str) -> String {
    result_value["highlight"][field].as_array().map_or_else(
        || html_escape::encode_text(field_value).to_string(),
        |s| s[0].as_str().unwrap_or_default().to_owned(),
    )
}

fn get_highlighted_optional_field(
    result_value: &Value,
    field: &str,
    field_value: Option<&str>,
) -> Option<String> {
    field_value.map(|field_val| get_highlighted_field(result_value, field, field_val))
}

fn get_results(es_response_body: &Value) -> Vec<SearchResult> {
    es_response_body["hits"]["hits"]
        .as_array()
        .unwrap_or_log()
        .iter()
        .map(|val| {
            let mut file_es: FileES =
                serde_json::from_value(val["_source"].clone()).unwrap_or_log();
            file_es._id = Some(val["_id"].as_str().unwrap_or_log().to_owned());
            let highlights = HighlightedFields {
                path: get_highlighted_field(val, "path", file_es.path.to_str().unwrap_or_log()),
                hash: get_highlighted_optional_field(val, "hash", file_es.hash.as_deref()),
                content: get_highlighted_optional_field(val, "content", file_es.content.as_deref()),
                image_data: ImageHighlightedFields {
                    image_make: get_highlighted_optional_field(
                        val,
                        "image_make",
                        file_es.image_data.image_make.as_deref(),
                    ),
                    image_model: get_highlighted_optional_field(
                        val,
                        "image_model",
                        file_es.image_data.image_model.as_deref(),
                    ),
                    image_software: get_highlighted_optional_field(
                        val,
                        "image_software",
                        file_es.image_data.image_software.as_deref(),
                    ),
                },
                multimedia_data: MultimediaHighlightedFields {
                    artist: get_highlighted_optional_field(
                        val,
                        "artist",
                        file_es.multimedia_data.artist.as_deref(),
                    ),
                    album: get_highlighted_optional_field(
                        val,
                        "album",
                        file_es.multimedia_data.album.as_deref(),
                    ),
                    genre: get_highlighted_optional_field(
                        val,
                        "genre",
                        file_es.multimedia_data.genre.as_deref(),
                    ),
                    track_number: get_highlighted_optional_field(
                        val,
                        "track_number",
                        file_es.multimedia_data.track_number.as_deref(),
                    ),
                    disc_number: get_highlighted_optional_field(
                        val,
                        "disc_number",
                        file_es.multimedia_data.disc_number.as_deref(),
                    ),
                    release_date: get_highlighted_optional_field(
                        val,
                        "release_date",
                        file_es.multimedia_data.release_date.as_deref(),
                    ),
                },
                document_data: DocumentHighlightedFields {
                    title: get_highlighted_optional_field(
                        val,
                        "title",
                        file_es.document_data.title.as_deref(),
                    ),
                    creator: get_highlighted_optional_field(
                        val,
                        "creator",
                        file_es.document_data.creator.as_deref(),
                    ),
                },
            };

            // Don't send big fields to client
            file_es.content = None;
            file_es.text_data.text_embedding = None;
            file_es.image_data.image_embedding = None;

            SearchResult {
                file: file_es,
                highlights,
                id: Uuid::new_v4(),
            }
        })
        .collect()
}

fn get_pages(results_per_page: u32, es_response_body: &Value, page: u32) -> Vec<PageType> {
    let total_pages = (es_response_body["hits"]["total"]["value"]
        .as_u64()
        .unwrap_or_log() as u32
        + results_per_page
        - 1)
        / results_per_page;

    let mut pages = Vec::new();
    if page > 1 {
        pages.push(PageType::First);
    }
    if page > 0 {
        pages.push(PageType::Previous(page - 1));
    }
    pages.append(
        &mut (page.saturating_sub(ADJACENT_PAGES)..min(page + ADJACENT_PAGES + 1, total_pages))
            .map(|i| {
                if i == page {
                    PageType::Current(i)
                } else {
                    PageType::Other(i)
                }
            })
            .collect(),
    );
    if page + 1 < total_pages {
        pages.push(PageType::Next(page + 1));
    }
    if page + 2 < total_pages {
        pages.push(PageType::Last(total_pages - 1));
    }
    pages
}

fn get_suggestion(es_response_body: &Value) -> Option<(String, String)> {
    let suggest_json = &es_response_body["suggest"]["simple_phrase"][0]["options"][0];
    suggest_json["highlighted"].as_str().and_then(|highlight| {
        suggest_json["text"]
            .as_str()
            .map(|text| (highlight.to_owned(), text.to_owned()))
    })
}

pub async fn search(
    State(state): State<Arc<ServerState>>,
    Json(search_request): Json<SearchRequest>,
) -> Result<Json<SearchResponse>, (StatusCode, String)> {
    let (
        max_sentences,
        sentences_per_paragraph,
        nnserver_url,
        results_per_page,
        knn_candidates_multiplier,
    ) = {
        let tmp = state.settings.read().await;
        (
            tmp.other.max_sentences,
            tmp.other.sentences_per_paragraph,
            tmp.other.nnserver_url.clone(),
            tmp.other.results_per_page,
            tmp.other.knn_candidates_multiplier,
        )
    };
    let es_request_body = get_request_body(
        max_sentences,
        sentences_per_paragraph,
        results_per_page,
        &state.reqwest_client,
        nnserver_url,
        knn_candidates_multiplier,
        &search_request,
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let es_response_body = get_es_response(
        results_per_page,
        &state.es_client,
        search_request.page,
        es_request_body,
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let results = get_results(&es_response_body);
    let pages = get_pages(results_per_page, &es_response_body, search_request.page);
    let suggestion = get_suggestion(&es_response_body);
    Ok(Json(SearchResponse {
        results,
        pages,
        suggestion,
    }))
}
