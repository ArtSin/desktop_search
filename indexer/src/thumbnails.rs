use std::process::Stdio;

use tokio::process::Command;

pub async fn get_thumbnail(
    path: &str,
    content_type: &Option<String>,
) -> std::io::Result<(Vec<u8>, &'static str)> {
    let (output_format, out_content_type) = match content_type.as_deref() {
        Some("image/png") => ("png", "image/png"),
        _ => ("mjpeg", "image/jpeg"),
    };

    Command::new("ffmpeg")
        .args([
            "-i",
            path,
            "-threads",
            "1",
            "-vf",
            r#"select='eq(pict_type\,I)',scale='512:512:force_original_aspect_ratio=decrease'"#,
            "-vframes",
            "1",
            "-c:v",
            output_format,
            "-f",
            "image2pipe",
            "-",
        ])
        .stdin(Stdio::null())
        .output()
        .await
        .map(|data| (data.stdout, out_content_type))
}
