//! Dependency-free Rust foundation for Outflow.
//!
//! This crate is the platform-independent core boundary described by
//! `docs/ARCHITECTURE.md`. It intentionally has no frontend, Tauri, operating
//! system, or external-service dependencies.

/// Deterministic whitespace normalization for transcript text.
pub mod text_cleanup;
/// Pure validation for the spoken and written sides of a vocabulary entry.
pub mod vocabulary;
/// Versioned Unicode canonical-caseless comparison keys for spoken phrases.
pub mod vocabulary_matching;
/// Read-only compatibility inspection before adopting a newer vocabulary policy.
pub mod vocabulary_migration;
/// Pure, boundary-aware single-entry and collection vocabulary correction.
pub mod vocabulary_post_correction;

/// Returns a development-only status message for the foundation smoke test.
pub const fn foundation_status() -> &'static str {
    "Outflow Rust foundation ready"
}

#[cfg(test)]
mod tests {
    use super::foundation_status;

    #[test]
    fn foundation_status_returns_expected_message() {
        assert_eq!(foundation_status(), "Outflow Rust foundation ready");
    }

    #[test]
    fn foundation_status_has_no_frontend_runtime_dependency() {
        // A compile-time value keeps this core smoke API independent of
        // frontend types, runtime state, and external resources.
        const MESSAGE: &str = foundation_status();
        assert_eq!(MESSAGE, "Outflow Rust foundation ready");
    }
}
