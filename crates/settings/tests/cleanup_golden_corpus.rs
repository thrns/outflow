//! Current-behavior characterization for the native Rust cleanup pipeline.
//!
//! These 200 synthetic cases form an independently reviewed current-behavior
//! characterization, not the approved final AC-07 acceptance corpus or an
//! approval of complete Verbatim semantics. Keep fixture expectations
//! explicit when behavior changes.

use std::collections::HashSet;

use outflow_core::vocabulary::VocabularyEntryRef;
use outflow_settings::cleanup_policy::process_transcript_with_filler_preference;
use outflow_settings::{CleanupMode, CleanupSettings};
use serde::Deserialize;
use serde_json::Value;

const FIXTURE_JSON: &str = include_str!("fixtures/cleanup_current_behavior.json");
const REQUIRED_FIELDS: [&str; 8] = [
    "id",
    "category",
    "input",
    "mode",
    "remove_fillers",
    "commands_enabled",
    "vocabulary",
    "expected",
];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    id: String,
    category: FixtureCategory,
    input: String,
    mode: CleanupMode,
    remove_fillers: bool,
    commands_enabled: bool,
    vocabulary: Vec<VocabularyFixture>,
    expected: String,
    #[serde(default)]
    protected_tokens: Vec<ProtectedTokenFixture>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VocabularyFixture {
    spoken: String,
    written: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProtectedTokenFixture {
    kind: ProtectedTokenKind,
    text: String,
    input_bytes: [usize; 2],
    output_bytes: [usize; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ProtectedTokenKind {
    Number,
    ProperNoun,
}

impl ProtectedTokenKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Number => "number",
            Self::ProperNoun => "proper_noun",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
enum FixtureCategory {
    Whitespace,
    Commands,
    Vocabulary,
    Punctuation,
    Fillers,
    Modes,
    Unicode,
    Safety,
}

impl FixtureCategory {
    const ALL: [Self; 8] = [
        Self::Whitespace,
        Self::Commands,
        Self::Vocabulary,
        Self::Punctuation,
        Self::Fillers,
        Self::Modes,
        Self::Unicode,
        Self::Safety,
    ];

    const fn as_str(self) -> &'static str {
        match self {
            Self::Whitespace => "whitespace",
            Self::Commands => "commands",
            Self::Vocabulary => "vocabulary",
            Self::Punctuation => "punctuation",
            Self::Fillers => "fillers",
            Self::Modes => "modes",
            Self::Unicode => "unicode",
            Self::Safety => "safety",
        }
    }
}

fn fixture_values() -> Vec<Value> {
    let value: Value = serde_json::from_str(FIXTURE_JSON)
        .unwrap_or_else(|error| panic!("cleanup fixture JSON is malformed: {error}"));
    value
        .as_array()
        .cloned()
        .unwrap_or_else(|| panic!("cleanup fixture JSON must contain a top-level array"))
}

fn fixture_label(value: &Value) -> String {
    let id = value
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("<missing or invalid id>");
    let category = value
        .get("category")
        .and_then(Value::as_str)
        .unwrap_or("<missing or invalid category>");
    format!("fixture {id:?} (category {category:?})")
}

fn parse_fixture(value: &Value) -> Fixture {
    let context = fixture_label(value);
    serde_json::from_value(value.clone())
        .unwrap_or_else(|error| panic!("{context}: malformed fixture schema: {error}"))
}

fn run_fixture(fixture: &Fixture, settings: &CleanupSettings) -> Result<String, String> {
    let vocabulary = fixture
        .vocabulary
        .iter()
        .map(|entry| VocabularyEntryRef {
            spoken: &entry.spoken,
            written: &entry.written,
        })
        .collect::<Vec<_>>();

    process_transcript_with_filler_preference(
        &fixture.input,
        &vocabulary,
        fixture.commands_enabled,
        settings,
    )
    .map_err(|error| error.to_string())
}

fn protection_context(fixture_id: &str, token: &ProtectedTokenFixture) -> String {
    format!(
        "fixture {:?}, protected token kind {:?}, input range [{}, {}), output range [{}, {})",
        fixture_id,
        token.kind.as_str(),
        token.input_bytes[0],
        token.input_bytes[1],
        token.output_bytes[0],
        token.output_bytes[1]
    )
}

fn checked_slice<'a>(
    value: &'a str,
    range: [usize; 2],
    label: &str,
    context: &str,
) -> Result<&'a str, String> {
    let [start, end] = range;
    if start >= end {
        return Err(format!(
            "{context}: {label} range [{start}, {end}) must be nonempty and ordered"
        ));
    }
    if end > value.len() {
        return Err(format!(
            "{context}: {label} range [{start}, {end}) exceeds the string's {} UTF-8 bytes",
            value.len()
        ));
    }
    if !value.is_char_boundary(start) || !value.is_char_boundary(end) {
        return Err(format!(
            "{context}: {label} range [{start}, {end}) must start and end on UTF-8 character boundaries"
        ));
    }

    value.get(start..end).ok_or_else(|| {
        format!("{context}: {label} range [{start}, {end}) is not a valid UTF-8 slice")
    })
}

fn ranges_overlap(left: [usize; 2], right: [usize; 2]) -> bool {
    left[0] < right[1] && right[0] < left[1]
}

fn validate_protected_tokens(
    fixture_id: &str,
    input: &str,
    expected_output: &str,
    actual_output: &str,
    protected_tokens: &[ProtectedTokenFixture],
) -> Result<(), String> {
    let mut input_ranges = Vec::with_capacity(protected_tokens.len());
    let mut output_ranges = Vec::with_capacity(protected_tokens.len());

    for token in protected_tokens {
        let context = protection_context(fixture_id, token);
        if token.text.is_empty() {
            return Err(format!("{context}: declared token text must be nonempty"));
        }

        let input_slice = checked_slice(input, token.input_bytes, "input", &context)?;
        let expected_slice = checked_slice(
            expected_output,
            token.output_bytes,
            "expected output",
            &context,
        )?;
        let actual_slice =
            checked_slice(actual_output, token.output_bytes, "actual output", &context)?;

        if input_slice.as_bytes() != token.text.as_bytes() {
            return Err(format!(
                "{context}: input bytes do not equal the declared token"
            ));
        }
        if actual_slice.as_bytes() != token.text.as_bytes() {
            return Err(format!(
                "{context}: actual cleanup output bytes do not equal the declared token"
            ));
        }
        if expected_slice.as_bytes() != token.text.as_bytes() {
            return Err(format!(
                "{context}: expected output bytes do not identify the declared token"
            ));
        }

        for previous in &input_ranges {
            if ranges_overlap(token.input_bytes, *previous) {
                return Err(format!(
                    "{context}: input range overlaps previous protected input range [{}, {})",
                    previous[0], previous[1]
                ));
            }
        }
        for previous in &output_ranges {
            if ranges_overlap(token.output_bytes, *previous) {
                return Err(format!(
                    "{context}: output range overlaps previous protected output range [{}, {})",
                    previous[0], previous[1]
                ));
            }
        }

        input_ranges.push(token.input_bytes);
        output_ranges.push(token.output_bytes);
    }

    Ok(())
}

fn assert_fixture_schema_ids_categories_and_mode_flags(values: &[Value]) {
    assert_eq!(
        values.len(),
        200,
        "the cleanup current-behavior corpus must contain exactly 200 fixtures"
    );

    let mut ids = HashSet::new();
    let mut categories = HashSet::new();
    let mut has_clean_fillers_enabled = false;
    let mut has_clean_fillers_disabled = false;
    let mut has_verbatim_fillers_enabled = false;
    let mut has_verbatim_fillers_disabled = false;
    let mut has_commands_enabled = false;
    let mut has_commands_disabled = false;

    for value in values {
        let context = fixture_label(value);
        let object = value
            .as_object()
            .unwrap_or_else(|| panic!("{context}: fixture must be a JSON object"));
        for field in REQUIRED_FIELDS {
            assert!(
                object.contains_key(field),
                "{context}: required field {field:?} is missing"
            );
        }

        let fixture = parse_fixture(value);
        assert!(
            !fixture.id.trim().is_empty(),
            "{context}: ID must be nonempty"
        );
        assert!(
            ids.insert(fixture.id.clone()),
            "{context}: ID must be unique"
        );
        categories.insert(fixture.category);

        match (fixture.mode, fixture.remove_fillers) {
            (CleanupMode::Clean, true) => has_clean_fillers_enabled = true,
            (CleanupMode::Clean, false) => has_clean_fillers_disabled = true,
            (CleanupMode::Verbatim, true) => has_verbatim_fillers_enabled = true,
            (CleanupMode::Verbatim, false) => has_verbatim_fillers_disabled = true,
        }
        if fixture.commands_enabled {
            has_commands_enabled = true;
        } else {
            has_commands_disabled = true;
        }
    }

    for category in FixtureCategory::ALL {
        assert!(
            categories.contains(&category),
            "no fixture covers category {:?}",
            category.as_str()
        );
    }
    assert_eq!(categories.len(), FixtureCategory::ALL.len());
    assert!(
        has_clean_fillers_enabled,
        "Clean/remove_fillers=true is missing"
    );
    assert!(
        has_clean_fillers_disabled,
        "Clean/remove_fillers=false is missing"
    );
    assert!(
        has_verbatim_fillers_enabled,
        "Verbatim/remove_fillers=true is missing"
    );
    assert!(
        has_verbatim_fillers_disabled,
        "Verbatim/remove_fillers=false is missing"
    );
    assert!(has_commands_enabled, "commands_enabled=true is missing");
    assert!(has_commands_disabled, "commands_enabled=false is missing");
}

fn protected_token(
    kind: ProtectedTokenKind,
    text: &str,
    input_bytes: [usize; 2],
    output_bytes: [usize; 2],
) -> ProtectedTokenFixture {
    ProtectedTokenFixture {
        kind,
        text: text.to_owned(),
        input_bytes,
        output_bytes,
    }
}

fn assert_checker_rejects_synthetic_cases() {
    let incorrect_value = protected_token(ProtectedTokenKind::ProperNoun, "other", [0, 5], [0, 5]);
    let error = validate_protected_tokens(
        "negative-incorrect-token-value",
        "Alpha",
        "Alpha",
        "Alpha",
        &[incorrect_value],
    )
    .expect_err("a declared value different from the input must be rejected");
    assert!(error.contains("negative-incorrect-token-value"), "{error}");
    assert!(error.contains("proper_noun"), "{error}");
    assert!(error.contains("input range [0, 5)"), "{error}");

    let changed_output = protected_token(ProtectedTokenKind::ProperNoun, "Alpha", [0, 5], [0, 5]);
    let error = validate_protected_tokens(
        "negative-changed-output-token",
        "Alpha",
        "Alpha",
        "alpha",
        &[changed_output],
    )
    .expect_err("a changed output token must be rejected");
    assert!(error.contains("negative-changed-output-token"), "{error}");
    assert!(error.contains("proper_noun"), "{error}");
    assert!(error.contains("output range [0, 5)"), "{error}");

    let out_of_bounds = protected_token(ProtectedTokenKind::Number, "Alpha", [0, 6], [0, 5]);
    let error = validate_protected_tokens(
        "negative-out-of-bounds-range",
        "Alpha",
        "Alpha",
        "Alpha",
        &[out_of_bounds],
    )
    .expect_err("an out-of-bounds range must be rejected");
    assert!(error.contains("negative-out-of-bounds-range"), "{error}");
    assert!(error.contains("number"), "{error}");
    assert!(error.contains("input range [0, 6)"), "{error}");

    let invalid_boundary = protected_token(ProtectedTokenKind::ProperNoun, "é", [1, 2], [0, 2]);
    let error = validate_protected_tokens(
        "negative-invalid-utf8-boundary",
        "é",
        "é",
        "é",
        &[invalid_boundary],
    )
    .expect_err("a range inside a multi-byte character must be rejected");
    assert!(error.contains("negative-invalid-utf8-boundary"), "{error}");
    assert!(error.contains("proper_noun"), "{error}");
    assert!(error.contains("input range [1, 2)"), "{error}");

    let overlapping = [
        protected_token(ProtectedTokenKind::ProperNoun, "alph", [0, 4], [0, 4]),
        protected_token(ProtectedTokenKind::ProperNoun, "ha", [3, 5], [3, 5]),
    ];
    let error = validate_protected_tokens(
        "negative-overlapping-ranges",
        "alpha",
        "alpha",
        "alpha",
        &overlapping,
    )
    .expect_err("overlapping protected ranges must be rejected");
    assert!(error.contains("negative-overlapping-ranges"), "{error}");
    assert!(error.contains("proper_noun"), "{error}");
    assert!(error.contains("input range [3, 5)"), "{error}");
}

fn assert_protected_token_schema_is_strict() {
    let valid = r#"{
        "id":"schema-probe",
        "category":"safety",
        "input":"A",
        "mode":"clean",
        "remove_fillers":false,
        "commands_enabled":false,
        "vocabulary":[],
        "expected":"A",
        "protected_tokens":[{"kind":"proper_noun","text":"A","input_bytes":[0,1],"output_bytes":[0,1]}]
    }"#;
    let fixture: Fixture = serde_json::from_str(valid)
        .unwrap_or_else(|error| panic!("valid protection metadata was rejected: {error}"));
    assert_eq!(fixture.protected_tokens.len(), 1);

    let unknown_token_field = valid.replace(
        "\"output_bytes\":[0,1]}",
        "\"output_bytes\":[0,1],\"unexpected\":true}",
    );
    assert!(
        serde_json::from_str::<Fixture>(&unknown_token_field).is_err(),
        "unknown protected-token fields must be rejected"
    );

    let unknown_fixture_field = valid.replace(
        "\"expected\":\"A\",",
        "\"expected\":\"A\",\"unexpected\":true,",
    );
    assert!(
        serde_json::from_str::<Fixture>(&unknown_fixture_field).is_err(),
        "unknown fixture fields must still be rejected"
    );
}

#[test]
fn current_behavior_fixtures_match_exactly_and_are_deterministic() {
    let values = fixture_values();
    assert_fixture_schema_ids_categories_and_mode_flags(&values);
    assert_protected_token_schema_is_strict();
    assert_checker_rejects_synthetic_cases();

    let mut seen_ids = HashSet::new();
    let mut annotated_fixtures = HashSet::new();
    let mut protected_numbers = 0;
    let mut protected_proper_nouns = 0;
    let mut multibyte_annotations = 0;

    for value in &values {
        let fixture = parse_fixture(value);
        let context = format!(
            "fixture {:?} (category {:?})",
            fixture.id,
            fixture.category.as_str()
        );

        assert!(
            !fixture.id.trim().is_empty(),
            "{context}: ID must be nonempty"
        );
        assert!(
            seen_ids.insert(fixture.id.clone()),
            "{context}: ID must be unique"
        );

        let input_before = fixture.input.clone();
        let vocabulary_before = fixture
            .vocabulary
            .iter()
            .map(|entry| (entry.spoken.clone(), entry.written.clone()))
            .collect::<Vec<_>>();
        let settings = CleanupSettings {
            mode: fixture.mode,
            remove_fillers: fixture.remove_fillers,
        };
        let settings_before = settings.clone();

        let actual = run_fixture(&fixture, &settings)
            .unwrap_or_else(|error| panic!("{context}: cleanup rejected fixture: {error}"));
        assert_eq!(
            actual, fixture.expected,
            "{context}: output differs from the explicit current-behavior expectation"
        );
        validate_protected_tokens(
            &fixture.id,
            &fixture.input,
            &fixture.expected,
            &actual,
            &fixture.protected_tokens,
        )
        .unwrap_or_else(|error| panic!("{error}"));
        if !fixture.protected_tokens.is_empty() {
            annotated_fixtures.insert(fixture.id.clone());
        }
        for token in &fixture.protected_tokens {
            match token.kind {
                ProtectedTokenKind::Number => protected_numbers += 1,
                ProtectedTokenKind::ProperNoun => protected_proper_nouns += 1,
            }
            if token.text.len() > token.text.chars().count() {
                multibyte_annotations += 1;
            }
        }

        let repeated = run_fixture(&fixture, &settings)
            .unwrap_or_else(|error| panic!("{context}: repeated cleanup failed: {error}"));
        assert_eq!(
            repeated, actual,
            "{context}: identical arguments produced different outputs"
        );
        assert_eq!(fixture.input, input_before, "{context}: input was changed");
        assert_eq!(
            fixture
                .vocabulary
                .iter()
                .map(|entry| (entry.spoken.clone(), entry.written.clone()))
                .collect::<Vec<_>>(),
            vocabulary_before,
            "{context}: vocabulary values were changed"
        );
        assert_eq!(
            settings, settings_before,
            "{context}: settings were changed"
        );
    }

    assert_eq!(
        protected_numbers, 8,
        "the corpus must annotate exactly eight numeric tokens"
    );
    assert_eq!(
        protected_proper_nouns, 8,
        "the corpus must annotate exactly eight proper nouns"
    );
    assert!(
        annotated_fixtures.len() >= 12,
        "at least 12 distinct fixtures must contain protected-token annotations"
    );
    assert!(
        multibyte_annotations >= 2,
        "at least two protected tokens must contain multibyte UTF-8 characters"
    );

    println!(
        "Cleanup golden corpus: {} fixtures passed\nCategories covered: {}\nProtected numeric tokens: {} (0 altered)\nProtected proper nouns: {} (0 altered)\nCurrent behavior baseline: true\nAC-07 acceptance: NOT EVALUATED",
        values.len(),
        FixtureCategory::ALL.len(),
        protected_numbers,
        protected_proper_nouns
    );
}
