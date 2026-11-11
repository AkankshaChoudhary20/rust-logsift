use logsift::classify_level;

#[test]
fn recognizes_common_level_delimiters_without_substring_matches() {
    assert_eq!(classify_level("timestamp [ERROR] failure"), Some("ERROR"));
    assert_eq!(classify_level("WARN: retry"), Some("WARN"));
    assert_eq!(classify_level("[ERRORS]"), None);
    assert_eq!(classify_level("prefixERRORsuffix"), None);
}
