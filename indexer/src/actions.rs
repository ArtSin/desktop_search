use axum::{http::StatusCode, Json};
use common_lib::actions::{OpenPathArgs, PickFileResult, PickFolderResult};
use rfd::AsyncFileDialog;

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
