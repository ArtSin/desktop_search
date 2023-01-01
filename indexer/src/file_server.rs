use std::process::Stdio;

use axum::{
    body::{boxed, Body, BoxBody},
    extract::Query,
    http::{HeaderMap, Request, StatusCode, Uri},
    response::Response,
};
use rust_embed::RustEmbed;
use serde::Deserialize;
use tokio::process::Command;
use tower::ServiceExt;
use tower_http::services::ServeFile;

#[derive(RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/../client_ui/dist"]
struct Assets;

#[derive(Deserialize)]
pub struct FileQuery {
    path: String,
    thumbnail: bool,
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
                .body(body)
                .unwrap())
        }
        None => Err((StatusCode::NOT_FOUND, "Not Found".to_owned())),
    }
}

pub async fn get_file(
    headers: HeaderMap,
    Query(params): Query<FileQuery>,
) -> Result<Response<BoxBody>, (StatusCode, String)> {
    if params.thumbnail {
        match get_thumbnail(&params.path).await {
            Ok(res) => Ok(Response::builder()
                .header("Content-Type", "image/jpeg")
                .body(boxed(Body::from(res)))
                .unwrap()),
            Err(err) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Can't create thumbnail: {}", err),
            )),
        }
    } else {
        let mut request_builder = Request::builder();
        *request_builder.headers_mut().unwrap() = headers;
        let request = request_builder.body(()).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("File request error: {}", e),
            )
        })?;
        let res = match ServeFile::new(params.path).oneshot(request).await {
            Ok(res) => Ok(res.map(boxed)),
            Err(err) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Can't read file: {}", err),
            )),
        }?;

        if res.status() == StatusCode::NOT_FOUND {
            Err((res.status(), "Not Found".to_owned()))
        } else {
            Ok(res)
        }
    }
}

async fn get_thumbnail(path: &str) -> std::io::Result<Vec<u8>> {
    Command::new("ffmpeg")
        .args([
            "-i",
            path,
            "-vf",
            r#"select='eq(pict_type\,I)',scale='512:512:force_original_aspect_ratio=decrease'"#,
            "-vframes",
            "1",
            "-f",
            "mjpeg",
            "-",
        ])
        .stdin(Stdio::null())
        .output()
        .await
        .map(|data| data.stdout)
}