//! Current-behavior characterization for the native Rust cleanup pipeline.
//!
//! These 48 synthetic cases are an independently reviewed baseline, not the
//! approved final AC-07 acceptance corpus or an approval of complete Verbatim
//! semantics. Keep fixture expectations explicit when behavior changes.

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
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VocabularyFixture {
    spoken: String,
    written: String,
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

#[test]
fn current_behavior_fixtures_match_exactly_and_are_deterministic() {
    let values = fixture_values();
    let mut seen_ids = HashSet::new();

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

    println!(
        "Cleanup golden corpus: {} fixtures passed\nCategories covered: {}\nCurrent behavior baseline: true",
        values.len(),
        FixtureCategory::ALL.len()
    );
}

#[test]
fn fixture_schema_ids_categories_and_mode_flags_are_complete() {
    let values = fixture_values();
    assert_eq!(
        values.len(),
        48,
        "the cleanup current-behavior corpus must contain exactly 48 fixtures"
    );

    let mut ids = HashSet::new();
    let mut categories = HashSet::new();
    let mut has_clean_fillers_enabled = false;
    let mut has_clean_fillers_disabled = false;
    let mut has_verbatim_fillers_enabled = false;
    let mut has_verbatim_fillers_disabled = false;
    let mut has_commands_enabled = false;
    let mut has_commands_disabled = false;

    for value in &values {
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
