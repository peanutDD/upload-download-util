use axum::http::HeaderMap;

pub(super) fn header_lock_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get("lock-token")
        .and_then(|v| v.to_str().ok())
        .map(|value| value.trim().trim_matches(['<', '>']).to_string())
}

pub(super) fn request_lock_tokens(headers: &HeaderMap) -> Vec<String> {
    let mut tokens = Vec::new();
    if let Some(token) = header_lock_token(headers) {
        tokens.push(token);
    }
    if let Some(value) = headers.get("if").and_then(|v| v.to_str().ok()) {
        let mut rest = value;
        while let Some(start) = rest.find('<') {
            rest = &rest[start + 1..];
            let Some(end) = rest.find('>') else {
                break;
            };
            tokens.push(rest[..end].to_string());
            rest = &rest[end + 1..];
        }
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_lock_tokens_collects_lock_token_and_if_header_tokens() {
        let mut headers = HeaderMap::new();
        headers.insert("lock-token", "<opaquelocktoken:main>".parse().unwrap());
        headers.insert(
            "if",
            "(<opaquelocktoken:a>) (Not <opaquelocktoken:b>)"
                .parse()
                .unwrap(),
        );

        assert_eq!(
            request_lock_tokens(&headers),
            vec![
                "opaquelocktoken:main".to_string(),
                "opaquelocktoken:a".to_string(),
                "opaquelocktoken:b".to_string()
            ]
        );
    }
}
