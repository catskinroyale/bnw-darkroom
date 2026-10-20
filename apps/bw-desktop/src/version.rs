//! The app version: semantic versioning (MAJOR.MINOR.PATCH) from the workspace
//! `Cargo.toml`, plus the git commit the build was made from.

/// Semantic version, e.g. "0.2.0".
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Short git commit hash, or "unknown" outside a git checkout.
pub const COMMIT: &str = env!("BW_GIT_COMMIT");

/// "0.2.0 (cf32c9d)".
pub fn full() -> String {
    format!("{VERSION} ({COMMIT})")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_semver() {
        let core = VERSION.split(['-', '+']).next().unwrap();
        let parts: Vec<&str> = core.split('.').collect();
        assert_eq!(parts.len(), 3, "{VERSION}");
        assert!(parts.iter().all(|p| p.parse::<u64>().is_ok()), "{VERSION}");
        assert!(!COMMIT.is_empty());
    }
}
