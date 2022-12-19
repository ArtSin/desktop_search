use std::{error::Error, path::PathBuf};

use url::Url;

use tauri::{
    http::{Request, Response, ResponseBuilder},
    AppHandle,
};

pub fn get_local_file(_handle: &AppHandle, request: &Request) -> Result<Response, Box<dyn Error>> {
    let path = PathBuf::from(
        percent_encoding::percent_decode_str(Url::parse(request.uri()).unwrap().path())
            .decode_utf8()?
            .into_owned(),
    );
    let data = std::fs::read(path)?;
    Ok(ResponseBuilder::new().status(200).body(data)?)
}
