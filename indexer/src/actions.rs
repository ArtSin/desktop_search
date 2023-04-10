use axum::{http::StatusCode, Json};
use common_lib::{
    actions::{OpenPathArgs, PickFileResult, PickFolderResult},
    search::SearchRequest,
};
use rfd::AsyncFileDialog;
use tracing_unwrap::ResultExt;

pub async fn open_path(Json(args): Json<OpenPathArgs>) -> Result<(), (StatusCode, String)> {
    open::that(args.path).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

pub async fn pick_file() -> Json<PickFileResult> {
    Json(PickFileResult {
        path: AsyncFileDialog::new()
            .pick_file()
            .await
            .map(|x| x.path().to_owned()),
    })
}

pub async fn pick_folder() -> Json<PickFolderResult> {
    Json(PickFolderResult {
        path: AsyncFileDialog::new()
            .pick_folder()
            .await
            .map(|x| x.path().to_owned()),
    })
}

pub async fn open_request() -> Result<Json<Option<SearchRequest>>, (StatusCode, String)> {
    Ok(Json(
        match AsyncFileDialog::new()
            .add_filter("JSON", &["json"])
            .pick_file()
            .await
        {
            Some(x) => serde_json::from_slice(
                &tokio::fs::read(x.path())
                    .await
                    .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?,
            )
            .map_err(|e| (StatusCode::UNPROCESSABLE_ENTITY, e.to_string()))?,
            None => None,
        },
    ))
}

pub async fn save_request(Json(request): Json<SearchRequest>) -> Result<(), (StatusCode, String)> {
    if let Some(x) = AsyncFileDialog::new()
        .add_filter("JSON", &["json"])
        .save_file()
        .await
    {
        tokio::fs::write(x.path(), serde_json::to_vec(&request).unwrap_or_log())
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
    }
    Ok(())
}
