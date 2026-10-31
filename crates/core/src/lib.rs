//! Dependency-free Rust foundation for Outflow.
//!
//! This crate is the platform-independent core boundary described by
//! `docs/ARCHITECTURE.md`. It intentionally has no frontend, Tauri, operating
//! system, or external-service dependencies.

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
