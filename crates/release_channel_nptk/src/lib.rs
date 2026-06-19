//! Provides constructs for NPTK app version and identity.

#![deny(missing_docs)]

use std::env;

use gpui::{App, Global};
use semver::Version;

/// Application ID used by Wayland and WM_CLASS on X11.
pub fn app_id() -> &'static str {
    "com.nptk.app"
}

/// The Git commit SHA that the application was built at.
#[derive(Clone, Eq, Debug, PartialEq)]
pub struct AppCommitSha(String);

struct GlobalAppCommitSha(AppCommitSha);

impl Global for GlobalAppCommitSha {}

impl AppCommitSha {
    /// Creates a new [`AppCommitSha`].
    pub fn new(sha: String) -> Self {
        AppCommitSha(sha)
    }

    /// Returns the global [`AppCommitSha`], if one is set.
    pub fn try_global(cx: &App) -> Option<AppCommitSha> {
        cx.try_global::<GlobalAppCommitSha>()
            .map(|sha| sha.0.clone())
    }

    /// Sets the global [`AppCommitSha`].
    pub fn set_global(sha: AppCommitSha, cx: &mut App) {
        cx.set_global(GlobalAppCommitSha(sha))
    }

    /// Returns the full commit SHA.
    pub fn full(&self) -> String {
        self.0.to_string()
    }

    /// Returns the short (7 character) commit SHA.
    pub fn short(&self) -> String {
        self.0.chars().take(7).collect()
    }
}

struct GlobalAppVersion(Version);

impl Global for GlobalAppVersion {}

/// The application version.
pub struct AppVersion;

impl AppVersion {
    /// Load the app version from env.
    pub fn load(
        pkg_version: &str,
        build_id: Option<&str>,
        commit_sha: Option<AppCommitSha>,
    ) -> Version {
        let mut version: Version = if let Ok(from_env) = env::var("ZED_APP_VERSION") {
            from_env.parse().expect("invalid ZED_APP_VERSION")
        } else {
            pkg_version.parse().expect("invalid version in Cargo.toml")
        };

        let mut build_metadata = String::new();
        if let Some(build_id) = build_id {
            build_metadata.push_str(build_id);
        }
        if let Some(sha) = commit_sha {
            if !build_metadata.is_empty() {
                build_metadata.push('.');
            }
            build_metadata.push_str(&sha.0);
        }
        if !build_metadata.is_empty() {
            if let Ok(build) = semver::BuildMetadata::new(&build_metadata) {
                version.build = build;
            }
        }

        version
    }

    /// Returns the global version number.
    pub fn global(cx: &App) -> Version {
        if cx.has_global::<GlobalAppVersion>() {
            cx.global::<GlobalAppVersion>().0.clone()
        } else {
            Version::new(0, 0, 0)
        }
    }
}

/// Initializes the global app version.
pub fn init(app_version: Version, cx: &mut App) {
    cx.set_global(GlobalAppVersion(app_version));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_id() {
        assert_eq!(app_id(), "com.nptk.app");
    }

    #[test]
    fn test_app_version_load() {
        let version = AppVersion::load("1.2.3", None, None);
        assert_eq!(version, Version::new(1, 2, 3));

        let version = AppVersion::load(
            "1.2.3",
            Some("build42"),
            Some(AppCommitSha::new("abcdef0".to_string())),
        );
        assert_eq!(version.build.to_string(), "build42.abcdef0");
    }
}
