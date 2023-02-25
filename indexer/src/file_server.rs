use std::sync::Arc;

use axum::{
    body::{boxed, Body, BoxBody},
    extract::{Query, State},
    http::{HeaderMap, Request, StatusCode, Uri},
    response::Response,
};
use common_lib::elasticsearch::ELASTICSEARCH_INDEX;
use rust_embed::RustEmbed;
use serde::Deserialize;
use serde_json::Value;
use tower::ServiceExt;
use tower_http::services::ServeFile;
use tracing_unwrap::{OptionExt, ResultExt};

use crate::{thumbnails::get_thumbnail, ServerState};

#[derive(RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/../client_ui/dist"]
struct Assets;

#[derive(Deserialize)]
pub struct FileQuery {
    path: String,
    content_type: Option<String>,
    thumbnail: bool,
}

#[derive(Deserialize)]
pub struct DocumentQuery {
    id: String,
}

#[derive(Deserialize)]
pub struct DocumentContent {
    content: String,
}

pub async fn get_client_file(uri: Uri) -> Result<Response<BoxBody>, (StatusCode, String)> {
    let mut path = uri.path().trim_start_matches('/');
    if path.is_empty() {
        path = "index.html";
    }

    match Assets::get(path) {
        Some(content) => {
            let body = boxed(axum::body::Full::from(content.data));
            let mime = mime_guess::from_path(path).first_or_octet_stream();

            Ok(Response::builder()
                .header(axum::http::header::CONTENT_TYPE, mime.as_ref())
                .header(
                    axum::http::header::CACHE_CONTROL,
                    if path == "index.html" {
                        "no-cache"
                    } else {
                        "public, max-age=31536000, immutable"
                    },
                )
                .body(body)
                .unwrap_or_log())
        }
        None => Err((StatusCode::NOT_FOUND, "Not Found".to_owned())),
    }
}

pub async fn get_file(
    headers: HeaderMap,
    Query(params): Query<FileQuery>,
) -> Result<Response<BoxBody>, (StatusCode, String)> {
    if params.thumbnail {
        match get_thumbnail(&params.path, &params.content_type).await {
            Ok((res, out_content_type)) => Ok(Response::builder()
                .header("Content-Type", out_content_type)
                .body(boxed(Body::from(res)))
                .unwrap_or_log()),
            Err(err) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Can't create thumbnail: {err}"),
            )),
        }
    } else {
        let mut request_builder = Request::builder();
        *request_builder.headers_mut().unwrap_or_log() = headers;
        let request = request_builder.body(()).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("File request error: {e}"),
            )
        })?;

        let file_mime = match params.content_type {
            Some(x) => x.parse().unwrap_or_log(),
            None => {
                let mut tmp = mime_guess::from_path(&params.path).first_or_octet_stream();
                if tmp.type_() == mime::TEXT && tmp.essence_str() != mime::TEXT_HTML {
                    tmp = mime::TEXT_PLAIN;
                };
                tmp
            }
        };

        let res = match ServeFile::new_with_mime(params.path, &file_mime)
            .oneshot(request)
            .await
        {
            Ok(res) => Ok(res.map(boxed)),
            Err(err) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Can't read file: {err}"),
            )),
        }?;

        if res.status() == StatusCode::NOT_FOUND {
            Err((res.status(), "Not Found".to_owned()))
        } else {
            Ok(res)
        }
    }
}

pub async fn get_document_content(
    State(state): State<Arc<ServerState>>,
    Query(params): Query<DocumentQuery>,
) -> Result<String, (StatusCode, String)> {
    let es_response_body = state
        .es_client
        .get(elasticsearch::GetParts::IndexId(
            ELASTICSEARCH_INDEX,
            &params.id,
        ))
        ._source(&["content"])
        .send()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .json::<Value>()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(
        serde_json::from_value::<DocumentContent>(es_response_body["_source"].clone())
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
            .content,
    )
}
