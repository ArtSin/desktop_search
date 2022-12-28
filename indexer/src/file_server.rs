use axum::{
    body::{boxed, Body, BoxBody},
    extract::Query,
    http::{HeaderMap, Request, StatusCode},
    response::Response,
};
use serde::Deserialize;
use tokio::process::Command;
use tower::ServiceExt;
use tower_http::services::ServeFile;

#[derive(Deserialize)]
pub struct FileQuery {
    path: String,
    thumbnail: bool,
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
        .output()
        .await
        .map(|data| data.stdout)
}
