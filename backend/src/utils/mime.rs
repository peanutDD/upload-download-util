pub fn effective_file_mime_type(filename: &str, supplied_mime_type: Option<&str>) -> String {
    let supplied = supplied_mime_type
        .map(str::trim)
        .filter(|mime| !mime.is_empty())
        .unwrap_or("application/octet-stream");

    if supplied.eq_ignore_ascii_case("application/octet-stream") {
        if let Some(guessed) = mime_guess::from_path(filename).first_raw() {
            return guessed.to_string();
        }
    }

    supplied.to_string()
}

pub fn is_macos_appledouble_filename(filename: &str) -> bool {
    filename.starts_with("._") || filename == ".DS_Store"
}
