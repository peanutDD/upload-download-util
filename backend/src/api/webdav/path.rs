use axum::http::{HeaderMap, StatusCode};
use percent_encoding::{percent_decode_str, utf8_percent_encode, AsciiSet, CONTROLS};
use url::Url;

const DAV_PATH_SEGMENT_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'`')
    .add(b'{')
    .add(b'}');

pub(super) fn path_segments(raw_path: &str) -> Result<Vec<String>, StatusCode> {
    let decoded = percent_decode_str(raw_path)
        .decode_utf8()
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let mut segments = Vec::new();
    for part in decoded.split('/') {
        if part.is_empty() {
            continue;
        }
        if part == "." || part == ".." || part.contains('\\') || part.contains('\0') {
            return Err(StatusCode::BAD_REQUEST);
        }
        segments.push(part.to_string());
    }
    Ok(segments)
}

// WebDAV clients disagree on Destination shape: absolute URLs, reverse-proxy
// prefixes before /dav, and relative targets are all seen in the wild.
pub(super) fn destination_segments(
    headers: &HeaderMap,
    source_segments: &[String],
) -> Option<String> {
    let value = headers.get("destination")?.to_str().ok()?;
    if let Ok(url) = Url::parse(value) {
        return dav_relative_path_from_absolute_path(url.path());
    }
    if value.starts_with('/') {
        return dav_relative_path_from_absolute_path(value);
    }
    let relative = value.trim_start_matches('/');
    let mut destination = source_segments
        .iter()
        .take(source_segments.len().saturating_sub(1))
        .cloned()
        .collect::<Vec<_>>();
    destination.extend(
        relative
            .split('/')
            .filter(|segment| !segment.is_empty())
            .map(ToOwned::to_owned),
    );
    Some(destination.join("/"))
}

fn dav_relative_path_from_absolute_path(path: &str) -> Option<String> {
    let mut segments = path.trim_start_matches('/').split('/').peekable();
    while let Some(segment) = segments.next() {
        if segment == "dav" {
            return Some(segments.collect::<Vec<_>>().join("/"));
        }
    }
    None
}

pub(super) fn lock_key(segments: &[String]) -> String {
    if segments.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", segments.join("/"))
    }
}

pub(super) fn dav_href(segments: &[String], child: Option<&str>, is_collection: bool) -> String {
    let mut href = String::from("/dav");
    for segment in segments {
        href.push('/');
        href.push_str(&utf8_percent_encode(segment, DAV_PATH_SEGMENT_ENCODE_SET).to_string());
    }
    if let Some(child) = child {
        href.push('/');
        href.push_str(&utf8_percent_encode(child, DAV_PATH_SEGMENT_ENCODE_SET).to_string());
    }
    if is_collection {
        href.push('/');
    }
    href
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_segments_decodes_and_rejects_unsafe_segments() {
        assert_eq!(
            path_segments("docs/%E6%8A%A5%E5%91%8A.txt").unwrap(),
            vec!["docs", "报告.txt"]
        );
        assert_eq!(
            path_segments("../secret").unwrap_err(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            path_segments("bad\\name").unwrap_err(),
            StatusCode::BAD_REQUEST
        );
    }

    #[test]
    fn destination_segments_handles_absolute_proxy_and_relative_targets() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "destination",
            "https://files.example.com/proxy/prefix/dav/folder/new.txt"
                .parse()
                .unwrap(),
        );
        assert_eq!(
            destination_segments(&headers, &["old.txt".to_string()]).unwrap(),
            "folder/new.txt"
        );

        headers.insert("destination", "archive/new.txt".parse().unwrap());
        assert_eq!(
            destination_segments(&headers, &["folder".to_string(), "old.txt".to_string()]).unwrap(),
            "folder/archive/new.txt"
        );
    }

    #[test]
    fn dav_href_encodes_segments_and_marks_collections() {
        assert_eq!(
            dav_href(&["space dir".into()], Some("a#b.txt"), false),
            "/dav/space%20dir/a%23b.txt"
        );
        assert_eq!(dav_href(&["folder".into()], None, true), "/dav/folder/");
    }
}
