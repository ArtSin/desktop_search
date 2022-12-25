use std::{error::Error, path::PathBuf};

use url::Url;

use tauri::{
    http::{Request, Response, ResponseBuilder},
    AppHandle,
};

pub fn get_local_file(_handle: &AppHandle, request: &Request) -> Result<Response, Box<dyn Error>> {
    let url = Url::parse(request.uri()).unwrap();
    let path = PathBuf::from(
        percent_encoding::percent_decode_str(url.path())
            .decode_utf8()?
            .into_owned(),
    );
    let mut builder = ResponseBuilder::new().status(200);
    for (key, val) in url.query_pairs() {
        if key == "content_type" {
            builder = builder.mimetype(&val);
        }
    }
    let data = std::fs::read(path)?;
    builder.body(data)
}
