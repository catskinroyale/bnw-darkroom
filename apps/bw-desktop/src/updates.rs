//! Update notice: asks GitHub for the latest release and compares its version with this
//! build's. Nothing is downloaded or installed; the user is offered the release page.
//!
//! The check runs on its own thread with a short timeout, sends no data beyond the
//! request itself (GitHub requires a User-Agent naming the program) and fails quietly
//! when offline.

use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

use eframe::egui;
use serde::Deserialize;

use crate::version;

const LATEST_URL: &str = "https://api.github.com/repos/catskinroyale/bnw-darkroom/releases/latest";
/// Where "Download" goes when a release has no page of its own.
pub const RELEASES_URL: &str = "https://github.com/catskinroyale/bnw-darkroom/releases/latest";
/// Automatic checks happen at most this often.
pub const CHECK_INTERVAL_S: u64 = 24 * 60 * 60;

/// A published release newer than this build.
#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    /// Version without the leading "v", e.g. "0.7.0".
    pub version: String,
    /// The release page on GitHub.
    pub url: String,
    /// Release notes (the changelog section for that version).
    pub notes: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CheckResult {
    UpToDate,
    Available(Release),
    Failed(String),
}

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: Option<String>,
    body: Option<String>,
}

/// MAJOR.MINOR.PATCH and whether it is a pre-release ("1.2.0-beta.1").
fn parse_version(v: &str) -> Option<([u64; 3], bool)> {
    let v = v.trim().trim_start_matches('v');
    let (core, pre) = match v.split_once('-') {
        Some((core, _)) => (core, true),
        None => (v.split('+').next().unwrap_or(v), false),
    };
    let mut parts = core.split('.').map(|p| p.parse::<u64>().ok());
    let version = [parts.next()??, parts.next()??, parts.next()??];
    parts.next().is_none().then_some((version, pre))
}

/// Whether `candidate` is a newer version than `current`. Unparsable versions never are.
pub fn is_newer(candidate: &str, current: &str) -> bool {
    match (parse_version(candidate), parse_version(current)) {
        // At the same MAJOR.MINOR.PATCH a release is newer than its pre-release.
        (Some((c, c_pre)), Some((v, v_pre))) => c > v || (c == v && v_pre && !c_pre),
        _ => false,
    }
}

/// Interpret GitHub's "latest release" reply for a build of version `current`.
pub fn interpret(json: &str, current: &str) -> CheckResult {
    let release: GithubRelease = match serde_json::from_str(json) {
        Ok(r) => r,
        Err(e) => return CheckResult::Failed(format!("unexpected reply from GitHub: {e}")),
    };
    let version = release.tag_name.trim_start_matches('v').to_string();
    if parse_version(&version).is_none() {
        return CheckResult::Failed(format!("unrecognised version {:?}", release.tag_name));
    }
    if !is_newer(&version, current) {
        return CheckResult::UpToDate;
    }
    CheckResult::Available(Release {
        version,
        url: release.html_url.unwrap_or_else(|| RELEASES_URL.into()),
        notes: release.body.unwrap_or_default().trim().to_string(),
    })
}

fn fetch() -> Result<String, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(10))
        .build();
    agent
        .get(LATEST_URL)
        .set("User-Agent", &format!("BW-Darkroom/{}", version::VERSION))
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(code, _) => format!("GitHub replied with status {code}"),
            ureq::Error::Transport(t) => format!("could not reach GitHub ({})", t.kind()),
        })?
        .into_string()
        .map_err(|e| format!("could not read GitHub's reply: {e}"))
}

/// Check in the background; the result arrives on the returned channel and wakes the UI.
pub fn spawn_check(ctx: &egui::Context) -> Receiver<CheckResult> {
    let (tx, rx) = channel();
    let ctx = ctx.clone();
    std::thread::Builder::new()
        .name("update-check".into())
        .spawn(move || {
            let result = match fetch() {
                Ok(json) => interpret(&json, version::VERSION),
                Err(e) => CheckResult::Failed(e),
            };
            let _ = tx.send(result);
            ctx.request_repaint();
        })
        .expect("spawn update-check thread");
    rx
}

/// Open a web page in the default browser.
pub fn open_in_browser(url: &str) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    let mut command = std::process::Command::new("open");
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", "start", ""]);
        c
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let mut command = std::process::Command::new("xdg-open");
    command.arg(url).spawn().map(|_| ())
}

/// Seconds since the Unix epoch.
pub fn now_s() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_numerically() {
        assert!(is_newer("0.7.0", "0.6.2"));
        assert!(is_newer("v0.6.10", "0.6.9"));
        assert!(is_newer("1.0.0", "0.99.99"));
        assert!(!is_newer("0.6.2", "0.6.2"));
        assert!(!is_newer("0.6.1", "0.6.2"));
        // Pre-releases: the release is newer than its beta, never the other way round.
        assert!(is_newer("0.7.0", "0.7.0-beta.1"));
        assert!(!is_newer("0.7.0-beta.1", "0.7.0"));
        assert!(!is_newer("nightly", "0.6.2"));
        assert!(!is_newer("0.7", "0.6.2"));
    }

    /// Real request to GitHub: `cargo test -p bw-desktop -- --ignored live`.
    #[test]
    #[ignore = "needs the network"]
    fn live_check_finds_a_release() {
        let json = fetch().expect("GitHub reachable");
        match interpret(&json, "0.0.1") {
            CheckResult::Available(r) => assert!(r.url.starts_with("https://github.com/")),
            other => panic!("expected a release, got {other:?}"),
        }
        assert_eq!(interpret(&json, "999.0.0"), CheckResult::UpToDate);
    }

    #[test]
    fn interprets_githubs_reply() {
        let json = r#"{"tag_name":"v0.7.0","html_url":"https://github.com/x/y/releases/tag/v0.7.0",
                       "body":"Added\n- Things\n","draft":false}"#;
        assert_eq!(
            interpret(json, "0.6.2"),
            CheckResult::Available(Release {
                version: "0.7.0".into(),
                url: "https://github.com/x/y/releases/tag/v0.7.0".into(),
                notes: "Added\n- Things".into(),
            })
        );
        assert_eq!(interpret(json, "0.7.0"), CheckResult::UpToDate);
        assert!(matches!(
            interpret("<html>", "0.6.2"),
            CheckResult::Failed(_)
        ));
        assert!(matches!(
            interpret(r#"{"tag_name":"latest"}"#, "0.6.2"),
            CheckResult::Failed(_)
        ));
    }
}
