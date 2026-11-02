//! Versioned spoken-phrase comparison keys for C-06 and C-08.
//!
//! Keys are temporary derived data used only for uniqueness checks. Callers
//! retain and display the original vocabulary text. This module uses ICU4X's
//! compiled Unicode data and creates no runtime data provider or locale state.

use icu_casemap::{CaseMapper, CaseMapperBorrowed};
use icu_normalizer::DecomposingNormalizerBorrowed;

/// The current comparison policy used by collection validation.
pub const CURRENT_VOCABULARY_MATCHING_VERSION: VocabularyMatchingVersion =
    VocabularyMatchingVersion::Unicode18CanonicalCaselessV1;

// These const constructors point at ICU4X's compiled static data. Reusing the
// borrowed values avoids constructing an owned data provider for each phrase.
const DEFAULT_CASE_MAPPER: CaseMapperBorrowed<'static> = CaseMapper::new();
const NFD_NORMALIZER: DecomposingNormalizerBorrowed<'static> =
    DecomposingNormalizerBorrowed::new_nfd();

/// Identifies the algorithm and Unicode data used to derive a vocabulary key.
///
/// Add a new variant when either the transformation or Unicode data version
/// changes; never silently change the meaning of an existing variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VocabularyMatchingVersion {
    /// Unicode 18.0.0 canonical caseless matching using D145.
    Unicode18CanonicalCaselessV1,
}

impl VocabularyMatchingVersion {
    /// Returns the stable user-facing identifier for this policy version.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Unicode18CanonicalCaselessV1 => "Unicode 18.0.0 canonical caseless V1",
        }
    }
}

/// Derives a comparison key from an original spoken phrase under `version`.
///
/// `Unicode18CanonicalCaselessV1` implements Unicode D145 as
/// `NFD(toCasefold(NFD(phrase)))`, using default full (non-Turkic) case folding.
/// It does not trim whitespace, apply compatibility normalization, strip
/// accents, or change the borrowed input. The returned key may contain a
/// different number of scalars than the original phrase.
pub fn comparison_key(phrase: &str, version: VocabularyMatchingVersion) -> String {
    match version {
        VocabularyMatchingVersion::Unicode18CanonicalCaselessV1 => {
            // ASCII has no decompositions or multi-character folds; Unicode
            // default folding maps only A-Z among these code points. The
            // exhaustive pairwise test below keeps this path aligned with the
            // pinned ICU4X behavior.
            if phrase.is_ascii() {
                return phrase.to_ascii_lowercase();
            }

            let decomposed = NFD_NORMALIZER.normalize(phrase);
            let folded = DEFAULT_CASE_MAPPER.fold_string(decomposed.as_ref());
            NFD_NORMALIZER.normalize(folded.as_ref()).into_owned()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CURRENT_VOCABULARY_MATCHING_VERSION, DEFAULT_CASE_MAPPER, NFD_NORMALIZER,
        VocabularyMatchingVersion, comparison_key,
    };

    fn key(value: &str) -> String {
        comparison_key(value, CURRENT_VOCABULARY_MATCHING_VERSION)
    }

    #[test]
    fn implements_the_task_43_comparison_matrix() {
        let cases = [
            ("rust", "Rust", true),
            ("CAFÉ", "café", true),
            ("straße", "STRASSE", true),
            ("é", "e\u{301}", true),
            ("Σ", "σ", true),
            ("σ", "ς", true),
            ("North Parkade", "north parkade", true),
            ("north parkade", "north parkade ", false),
            ("I", "ı", false),
            ("İ", "i", false),
            ("①", "1", false),
            ("a", "а", false),
        ];

        for (left, right, expected_equal) in cases {
            assert_eq!(
                key(left) == key(right),
                expected_equal,
                "unexpected comparison result for {left:?} and {right:?}"
            );
        }
    }

    #[test]
    fn applies_full_default_folding_for_sharp_s_and_ligatures() {
        // Unicode 18.0.0 CaseFolding.txt defines U+00DF -> "ss" and
        // U+FB00 -> "ff" with full (F) mappings.
        assert_eq!(key("Straße"), key("STRASSE"));
        assert_eq!(key("ﬀ"), key("FF"));
    }

    #[test]
    fn applies_default_non_turkic_folding_to_dotted_and_dotless_i() {
        assert_eq!(key("I"), key("i"));
        assert_ne!(key("I"), key("ı"));
        assert_ne!(key("İ"), key("i"));
        assert_eq!(key("İ"), key("i\u{307}"));
    }

    #[test]
    fn canonical_decomposition_reorders_combining_marks() {
        // Unicode 18.0.0 NormalizationTest.txt Part 0 includes the canonical
        // equivalence U+1E0A U+0323 == U+1E0C U+0307 and reordered marks.
        assert_eq!(key("\u{1E0A}\u{0323}"), key("\u{1E0C}\u{0307}"));
        assert_eq!(key("a\u{0315}\u{0300}"), key("a\u{0300}\u{0315}"));
    }

    #[test]
    fn keeps_accents_and_compatibility_distinctions() {
        assert_ne!(key("é"), key("e"));
        assert_ne!(key("①"), key("1"));
    }

    #[test]
    fn supports_cyrillic_and_leaves_uncased_scripts_unchanged() {
        assert_eq!(key("Привет"), key("ПРИВЕТ"));
        assert_eq!(key("日本語"), "日本語");
        assert_eq!(key("नमस्ते"), "नमस्ते");
    }

    #[test]
    fn folds_supplementary_plane_letters_and_preserves_emoji() {
        assert_eq!(key("𐐀"), key("𐐨"));
        assert_eq!(key("😀🧑‍💻"), "😀🧑‍💻");
    }

    #[test]
    fn does_not_trim_whitespace_or_remove_format_characters() {
        assert_ne!(key("north parkade"), key(" north parkade"));
        assert_ne!(key("north parkade"), key("north parkade "));
        assert_ne!(key("ab"), key("a\u{200B}b"));
        assert_ne!(key("👩‍💻"), key("👩💻"));
    }

    #[test]
    fn matching_key_generation_is_deterministic_and_preserves_input() {
        let original = String::from("  STRAẞE e\u{301}  ");
        let snapshot = original.clone();
        let expected = key(&original);

        for _ in 0..10 {
            assert_eq!(key(&original), expected);
        }
        assert_eq!(original, snapshot);
    }

    #[test]
    fn matching_version_identifier_is_stable() {
        assert_eq!(
            CURRENT_VOCABULARY_MATCHING_VERSION,
            VocabularyMatchingVersion::Unicode18CanonicalCaselessV1
        );
        assert_eq!(
            CURRENT_VOCABULARY_MATCHING_VERSION.label(),
            "Unicode 18.0.0 canonical caseless V1"
        );
    }

    #[test]
    fn ascii_fast_path_matches_icu_for_every_pair_of_ascii_scalars() {
        for first in 0_u8..=0x7f {
            for second in 0_u8..=0x7f {
                let input = [char::from(first), char::from(second)]
                    .into_iter()
                    .collect::<String>();
                let decomposed = NFD_NORMALIZER.normalize(&input);
                let folded = DEFAULT_CASE_MAPPER.fold_string(decomposed.as_ref());
                let expected = NFD_NORMALIZER.normalize(folded.as_ref()).into_owned();

                assert_eq!(
                    key(&input),
                    expected,
                    "ASCII fast path differed from ICU for bytes {first:#04x}, {second:#04x}"
                );
            }
        }
    }
}
