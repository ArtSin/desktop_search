use fluent_bundle::{FluentArgs, FluentValue};

use crate::app::get_translation;

pub fn duration_str_from_seconds(total_float_s: f32) -> String {
    let total_s = total_float_s.floor() as u64;
    let (h, m, s) = (
        total_s / 3600,
        (total_s / 60) % 60,
        (total_s % 60) as f32 + total_float_s.fract(),
    );

    let args = FluentArgs::from_iter([
        ("hours", Into::<FluentValue>::into(h)),
        ("minutes", m.into()),
        ("seconds", format!("{s:.3}").into()),
    ]);
    let format_str = if h > 0 {
        "duration_h_m_s"
    } else if m > 0 {
        "duration_m_s"
    } else {
        "duration_s"
    };
    get_translation(format_str, Some(&args)).to_string()
}

pub fn file_size_str(size: u64) -> String {
    let (format_size, format_str): (FluentValue, _) = if size < 1024 {
        ((size).into(), "file_size_b")
    } else if size < 1024 * 1024 {
        (
            format!("{:.3}", (size as f64) / 1024.0).into(),
            "file_size_kib",
        )
    } else if size < 1024 * 1024 * 1024 {
        (
            format!("{:.3}", (size as f64) / (1024.0 * 1024.0)).into(),
            "file_size_mib",
        )
    } else {
        (
            format!("{:.3}", (size as f64) / (1024.0 * 1024.0 * 1024.0)).into(),
            "file_size_gib",
        )
    };

    let args = FluentArgs::from_iter([("size", format_size)]);
    get_translation(format_str, Some(&args)).to_string()
}
