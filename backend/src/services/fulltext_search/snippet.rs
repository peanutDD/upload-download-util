pub(super) fn snippet<const N: usize>(query: &str, parts: [&str; N]) -> String {
    let query_lower = query.to_lowercase();
    for part in parts {
        if let Some(match_start) = case_insensitive_char_match_start(part, &query_lower) {
            let start = match_start.saturating_sub(48);
            return part
                .chars()
                .skip(start)
                .take((match_start - start + query.chars().count() + 96).min(160))
                .collect();
        }
    }
    parts
        .into_iter()
        .find(|part| !part.trim().is_empty())
        .unwrap_or("")
        .chars()
        .take(160)
        .collect()
}

fn case_insensitive_char_match_start(part: &str, query_lower: &str) -> Option<usize> {
    part.char_indices()
        .enumerate()
        .find_map(|(char_index, (byte_index, _))| {
            part[byte_index..]
                .to_lowercase()
                .starts_with(query_lower)
                .then_some(char_index)
        })
}

pub(super) fn match_source(query: &str, filename: &str, ocr: &str, category: &str) -> String {
    let q = query.to_lowercase();
    if filename.to_lowercase().contains(&q) {
        "filename"
    } else if ocr.to_lowercase().contains(&q) {
        "ocr"
    } else if category.to_lowercase().contains(&q) {
        "category"
    } else {
        "content"
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snippet_prefers_matching_text_and_falls_back_to_first_content() {
        assert!(snippet("zebra", ["", "invoice zebra-445"]).contains("zebra-445"));
        assert_eq!(snippet("missing", ["abcdef", ""]), "abcdef");
    }

    #[test]
    fn snippet_is_utf8_safe_and_bounded_for_long_queries() {
        let query = "界".repeat(240);
        let content = format!("prefix {query} suffix");

        let result = snippet(&query, [&content]);

        assert!(result.chars().count() <= 160);
        assert!(std::str::from_utf8(result.as_bytes()).is_ok());
    }

    #[test]
    fn match_source_prefers_filename_then_ocr_then_category_then_content() {
        assert_eq!(
            match_source("report", "report.pdf", "report", "report"),
            "filename"
        );
        assert_eq!(match_source("scan", "file.pdf", "scan text", "scan"), "ocr");
        assert_eq!(match_source("tax", "file.pdf", "", "tax"), "category");
        assert_eq!(match_source("body", "file.pdf", "", ""), "content");
    }
}
