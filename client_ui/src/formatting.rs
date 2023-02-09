pub fn duration_str_from_seconds(total_float_s: f32) -> String {
    let total_s = total_float_s.floor() as u64;
    let (h, m, s) = (
        total_s / 3600,
        (total_s / 60) % 60,
        (total_s % 60) as f32 + total_float_s.fract(),
    );
    if h > 0 {
        format!("{} ч {} мин {:.3} с", h, m, s)
    } else if m > 0 {
        format!("{} мин {:.3} с", m, s)
    } else {
        format!("{:.3} с", s)
    }
}

pub fn file_size_str(size: u64) -> String {
    if size < 1024 {
        format!("{} Б", size)
    } else if size < 1024 * 1024 {
        format!("{:.3} КиБ", (size as f64) / 1024.0)
    } else if size < 1024 * 1024 * 1024 {
        format!("{:.3} МиБ", (size as f64) / (1024.0 * 1024.0))
    } else {
        format!("{:.3} ГиБ", (size as f64) / (1024.0 * 1024.0 * 1024.0))
    }
}
