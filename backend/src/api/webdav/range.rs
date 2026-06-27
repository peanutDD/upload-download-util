pub(super) fn parse_single_range(range: &str, total: u64) -> Option<(u64, u64)> {
    if total == 0 {
        return None;
    }
    let value = range.strip_prefix("bytes=")?;
    let (start, end) = value.split_once('-')?;
    if start.is_empty() {
        let suffix_len = end.parse::<u64>().ok()?;
        if suffix_len == 0 {
            return None;
        }
        let start = total.saturating_sub(suffix_len);
        return Some((start, total - 1));
    }
    let start = start.parse::<u64>().ok()?;
    let end = if end.is_empty() {
        total.checked_sub(1)?
    } else {
        end.parse::<u64>().ok()?
    };
    if start > end || end >= total {
        return None;
    }
    Some((start, end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_range_supports_standard_and_suffix_ranges() {
        assert_eq!(parse_single_range("bytes=2-5", 10), Some((2, 5)));
        assert_eq!(parse_single_range("bytes=7-", 10), Some((7, 9)));
        assert_eq!(parse_single_range("bytes=-4", 10), Some((6, 9)));
        assert_eq!(parse_single_range("bytes=8-2", 10), None);
        assert_eq!(parse_single_range("bytes=0-10", 10), None);
        assert_eq!(parse_single_range("items=0-1", 10), None);
    }
}
