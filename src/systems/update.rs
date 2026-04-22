use std::time::Duration;

const LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/kimulaco/term-survivors/releases/latest";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);
const USER_AGENT: &str = concat!("term-survivors/", env!("CARGO_PKG_VERSION"));
const ENV_INSTALLED_VIA: &str = "TERM_SURVIVORS_INSTALLED_VIA";
#[cfg(feature = "debug-update")]
const ENV_DEBUG_VERSION: &str = "TERM_SURVIVORS_DEBUG_UPDATE_VERSION";

pub struct UpdateInfo {
    pub latest_version: String,
    pub install_command: Option<String>,
}

pub fn check() -> Option<UpdateInfo> {
    let current = env!("CARGO_PKG_VERSION");

    #[cfg(feature = "debug-update")]
    if let Ok(v) = std::env::var(ENV_DEBUG_VERSION) {
        if is_newer(&v, current) {
            let install_command = detect_install_command();
            return Some(UpdateInfo {
                latest_version: v,
                install_command,
            });
        }
        return None;
    }

    let latest = fetch_latest_version()?;

    if !is_newer(&latest, current) {
        return None;
    }

    let install_command = detect_install_command();

    Some(UpdateInfo {
        latest_version: latest,
        install_command,
    })
}

fn fetch_latest_version() -> Option<String> {
    let agent = ureq::AgentBuilder::new().timeout(REQUEST_TIMEOUT).build();

    let response = agent
        .get(LATEST_RELEASE_API)
        .set("User-Agent", USER_AGENT)
        .call()
        .ok()?;

    let json: serde_json::Value = response.into_json().ok()?;
    let tag = json["tag_name"].as_str()?;
    Some(tag.trim_start_matches('v').to_string())
}

fn is_newer(latest: &str, current: &str) -> bool {
    let parse = |v: &str| -> Option<(u64, u64, u64)> {
        let mut parts = v.splitn(3, '.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next()?.parse().ok()?;
        Some((major, minor, patch))
    };

    match (parse(latest), parse(current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    }
}

fn detect_install_command() -> Option<String> {
    if std::env::var(ENV_INSTALLED_VIA).as_deref() == Ok("npm") {
        return Some("npm install -g term-survivors".to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_newer_detects_updates() {
        assert!(is_newer("0.5.0", "0.4.2"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(is_newer("0.4.3", "0.4.2"));
    }

    #[test]
    fn is_newer_same_or_older() {
        assert!(!is_newer("0.4.2", "0.4.2"));
        assert!(!is_newer("0.3.0", "0.4.2"));
        assert!(!is_newer("0.4.1", "0.4.2"));
    }

    #[test]
    fn is_newer_handles_bad_input() {
        assert!(!is_newer("not-a-version", "0.4.2"));
        assert!(!is_newer("0.5.0", "not-a-version"));
    }
}
