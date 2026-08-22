//! Core, dependency-free logic for logsift: filtering and summarizing huge
//! log files without loading them into memory.
//!
//! Timestamp filtering relies on a property of the ISO 8601
//! `YYYY-MM-DDTHH:MM:SS` format: for two timestamps in that exact format,
//! lexical (byte-wise) string comparison gives the same result as
//! chronological comparison. That means `--since`/`--until` can be plain
//! string comparisons against the first 19 characters of a line, with no
//! date-parsing library required.

use std::collections::HashMap;

#[derive(Default, Debug, Clone)]
pub struct Filters {
    pub contains: Option<String>,
    pub exclude: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
}

#[derive(Default, Debug)]
pub struct Summary {
    pub total_lines: usize,
    pub matched_lines: usize,
    pub level_counts: HashMap<String, usize>,
}

const KNOWN_LEVELS: [&str; 5] = ["ERROR", "WARN", "INFO", "DEBUG", "TRACE"];

/// Returns the first 19 characters of `line` if they look like a
/// `YYYY-MM-DDTHH:MM:SS` (or `YYYY-MM-DD HH:MM:SS`) timestamp prefix,
/// otherwise None. Validates digit positions, not calendar correctness.
pub fn extract_timestamp_prefix(line: &str) -> Option<&str> {
    if line.len() < 19 {
        return None;
    }
    let prefix = &line[..19];
    let bytes = prefix.as_bytes();

    let is_digit = |i: usize| bytes[i].is_ascii_digit();
    let expect_digits = |range: std::ops::Range<usize>| range.clone().all(is_digit);

    let separator_ok = (bytes[4] == b'-')
        && (bytes[7] == b'-')
        && (bytes[10] == b'T' || bytes[10] == b' ')
        && (bytes[13] == b':')
        && (bytes[16] == b':');

    let digits_ok = expect_digits(0..4)
        && expect_digits(5..7)
        && expect_digits(8..10)
        && expect_digits(11..13)
        && expect_digits(14..16)
        && expect_digits(17..19);

    if separator_ok && digits_ok {
        Some(prefix)
    } else {
        None
    }
}

/// Returns the first known log level token found in `line`, matched as a
/// whole whitespace-delimited word (so "ERRORS" doesn't match "ERROR").
pub fn classify_level(line: &str) -> Option<&'static str> {
    for word in line.split_whitespace() {
        for level in KNOWN_LEVELS {
            if word == level {
                return Some(level);
            }
        }
    }
    None
}

/// Returns true if `line` passes all configured filters.
pub fn matches_filters(line: &str, filters: &Filters) -> bool {
    if let Some(needle) = &filters.contains
        && !line.contains(needle.as_str())
    {
        return false;
    }
    if let Some(needle) = &filters.exclude
        && line.contains(needle.as_str())
    {
        return false;
    }

    if filters.since.is_some() || filters.until.is_some() {
        let Some(ts) = extract_timestamp_prefix(line) else {
            // No timestamp on this line -- can't evaluate a time filter, so exclude it.
            return false;
        };
        if let Some(since) = &filters.since
            && ts < since.as_str()
        {
            return false;
        }
        if let Some(until) = &filters.until
            && ts > until.as_str()
        {
            return false;
        }
    }

    true
}

/// Processes one line: updates `summary` and returns whether the line
/// matched the filters (i.e. should be printed by the caller).
pub fn process_line(line: &str, filters: &Filters, summary: &mut Summary) -> bool {
    summary.total_lines += 1;

    if let Some(level) = classify_level(line) {
        *summary.level_counts.entry(level.to_string()).or_insert(0) += 1;
    }

    let matched = matches_filters(line, filters);
    if matched {
        summary.matched_lines += 1;
    }
    matched
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_filter_matches() {
        let f = Filters {
            contains: Some("boom".to_string()),
            ..Default::default()
        };
        assert!(matches_filters("something boom happened", &f));
        assert!(!matches_filters("all quiet", &f));
    }

    #[test]
    fn exclude_filter_removes_matching_lines() {
        let f = Filters {
            exclude: Some("healthcheck".to_string()),
            ..Default::default()
        };
        assert!(!matches_filters("GET /healthcheck 200", &f));
        assert!(matches_filters("GET /orders 200", &f));
    }

    #[test]
    fn contains_and_exclude_combine() {
        let f = Filters {
            contains: Some("ERROR".to_string()),
            exclude: Some("timeout".to_string()),
            ..Default::default()
        };
        assert!(matches_filters(
            "2024-01-01T00:00:00 ERROR db connection lost",
            &f
        ));
        assert!(!matches_filters(
            "2024-01-01T00:00:00 ERROR request timeout",
            &f
        ));
        assert!(!matches_filters("2024-01-01T00:00:00 INFO all good", &f));
    }

    #[test]
    fn extract_timestamp_prefix_valid_t_separator() {
        let ts = extract_timestamp_prefix("2024-03-15T10:30:00 INFO started");
        assert_eq!(ts, Some("2024-03-15T10:30:00"));
    }

    #[test]
    fn extract_timestamp_prefix_valid_space_separator() {
        let ts = extract_timestamp_prefix("2024-03-15 10:30:00 INFO started");
        assert_eq!(ts, Some("2024-03-15 10:30:00"));
    }

    #[test]
    fn extract_timestamp_prefix_rejects_non_timestamp_lines() {
        assert_eq!(extract_timestamp_prefix("not a timestamp at all"), None);
        assert_eq!(extract_timestamp_prefix("short"), None);
        assert_eq!(
            extract_timestamp_prefix("2024-3-15T10:30:00 bad month digits"),
            None
        );
    }

    #[test]
    fn since_until_filter_lexical_ordering_matches_chronological() {
        let f = Filters {
            since: Some("2024-01-02T00:00:00".to_string()),
            until: Some("2024-01-04T00:00:00".to_string()),
            ..Default::default()
        };
        assert!(!matches_filters(
            "2024-01-01T23:59:59 INFO before range",
            &f
        ));
        assert!(matches_filters("2024-01-02T00:00:00 INFO at start", &f));
        assert!(matches_filters("2024-01-03T12:00:00 INFO inside range", &f));
        assert!(matches_filters("2024-01-04T00:00:00 INFO at end", &f));
        assert!(!matches_filters("2024-01-04T00:00:01 INFO after range", &f));
    }

    #[test]
    fn time_filter_excludes_lines_with_no_timestamp() {
        let f = Filters {
            since: Some("2024-01-01T00:00:00".to_string()),
            ..Default::default()
        };
        assert!(!matches_filters("no timestamp on this line", &f));
    }

    #[test]
    fn classify_level_matches_whole_word_only() {
        assert_eq!(classify_level("2024-01-01 ERROR db down"), Some("ERROR"));
        assert_eq!(classify_level("2024-01-01 WARN retrying"), Some("WARN"));
        // "ERRORS" must NOT match the ERROR token.
        assert_eq!(classify_level("ERRORS is a field name here"), None);
        assert_eq!(classify_level("no level token in this line"), None);
    }

    #[test]
    fn process_line_updates_summary_counts() {
        let f = Filters {
            contains: Some("ERROR".to_string()),
            ..Default::default()
        };
        let mut summary = Summary::default();

        let lines = [
            "2024-01-01T00:00:00 INFO started",
            "2024-01-01T00:00:01 ERROR db down",
            "2024-01-01T00:00:02 WARN retrying",
            "2024-01-01T00:00:03 ERROR still down",
        ];

        let matched: Vec<bool> = lines
            .iter()
            .map(|l| process_line(l, &f, &mut summary))
            .collect();

        assert_eq!(matched, vec![false, true, false, true]);
        assert_eq!(summary.total_lines, 4);
        assert_eq!(summary.matched_lines, 2);
        assert_eq!(summary.level_counts.get("ERROR"), Some(&2));
        assert_eq!(summary.level_counts.get("WARN"), Some(&1));
        assert_eq!(summary.level_counts.get("INFO"), Some(&1));
    }

    #[test]
    fn no_filters_means_everything_matches() {
        let f = Filters::default();
        assert!(matches_filters("literally anything", &f));
        assert!(matches_filters("", &f));
    }
}
