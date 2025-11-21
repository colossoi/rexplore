use rexplore_core::{load_rustdoc_json_from_str, public_api_in_crate, BuilderOptions};

const REGEX_RUSTDOC_JSON: &str = include_str!("fixtures/regex.json");

#[test]
fn test_parse_regex_crate() {
    let crate_data =
        load_rustdoc_json_from_str(REGEX_RUSTDOC_JSON).expect("Failed to parse regex rustdoc JSON");

    assert_eq!(crate_data.crate_version.as_deref(), Some("1.11.3"));

    let options = BuilderOptions::default();
    let api = public_api_in_crate(&crate_data, options);

    // Verify we got some public items
    assert!(
        !api.items.is_empty(),
        "Should have public items, got {} items",
        api.items.len()
    );

    // Verify we have the main Regex type
    let has_regex_type = api.items.iter().any(|item| {
        let s = item.to_string();
        s.contains("Regex") && s.contains("struct")
    });
    assert!(
        has_regex_type,
        "Should have Regex struct. Total items: {}",
        api.items.len()
    );
}

#[test]
fn test_regex_crate_has_expected_items() {
    let crate_data =
        load_rustdoc_json_from_str(REGEX_RUSTDOC_JSON).expect("Failed to parse regex rustdoc JSON");

    let options = BuilderOptions::default();
    let api = public_api_in_crate(&crate_data, options);

    let items_as_strings: Vec<String> = api.items.iter().map(|item| item.to_string()).collect();

    // Check for some well-known types
    let has_regex = items_as_strings
        .iter()
        .any(|s| s.contains("regex::Regex") && s.contains("struct"));
    let has_regexset = items_as_strings
        .iter()
        .any(|s| s.contains("regex::RegexSet") && s.contains("struct"));
    let has_captures = items_as_strings
        .iter()
        .any(|s| s.contains("regex::Captures") && s.contains("struct"));

    assert!(has_regex, "Should have Regex struct");
    assert!(has_regexset, "Should have RegexSet struct");
    assert!(has_captures, "Should have Captures struct");
}

#[test]
fn test_regex_enum_and_variants() {
    let crate_data =
        load_rustdoc_json_from_str(REGEX_RUSTDOC_JSON).expect("Failed to parse regex rustdoc JSON");

    let options = BuilderOptions::default();
    let api = public_api_in_crate(&crate_data, options);

    let items_str: Vec<String> = api.items.iter().map(|i| i.to_string()).collect();

    // Should have root module
    assert!(
        items_str.iter().any(|s| s.contains("pub mod regex")),
        "Should have root regex module"
    );

    // Should have Error enum
    assert!(
        items_str
            .iter()
            .any(|s| s.contains("pub enum regex::Error")),
        "Should have Error enum"
    );

    // Should have Error variants
    assert!(
        items_str.iter().any(|s| s.contains("Error::Syntax")),
        "Should have Error::Syntax variant"
    );
    assert!(
        items_str
            .iter()
            .any(|s| s.contains("Error::CompiledTooBig")),
        "Should have Error::CompiledTooBig variant"
    );
}
