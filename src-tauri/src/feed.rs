//! Where this build looks for updates, and the release page of a version.
//!
//! Both come from the first of `plugins.updater.endpoints` in tauri.conf.json,
//! or of the config a fork lays over it with `tauri build --config` (README,
//! 「フォークで使うとき」). A fork then only sets the endpoint, and the tag
//! prefix of its releases if they aren't `v<version>`: the proxy lookup and
//! the release page follow them.

use tauri::{AppHandle, Runtime};

/// What this build's release tags start with before `v<version>`
/// (`VIDEO_STUDIO_TAG_PREFIX` at build time, e.g. `corp-` for `corp-v1.2.3`).
/// Empty = `v1.2.3`.
pub const TAG_PREFIX: &str = match option_env!("VIDEO_STUDIO_TAG_PREFIX") {
    Some(p) => p,
    None => "",
};

/// The update feed of this build (`releases/latest/download/latest.json`).
pub fn update_feed<R: Runtime>(app: &AppHandle<R>) -> Option<String> {
    let updater = app.config().plugins.0.get("updater")?;
    updater
        .get("endpoints")?
        .get(0)?
        .as_str()
        .map(str::to_owned)
}

/// The release page of `version` in the same repository as `feed`
/// (`https://github.com/<owner>/<repo>/releases/...`), tagged
/// `<prefix>v<version>`. Only a version goes into the URL, so the page can't be
/// sent anywhere else. A feed outside GitHub Releases has no page.
pub fn release_page(feed: &str, prefix: &str, version: &str) -> Result<String, String> {
    let version = version.strip_prefix('v').unwrap_or(version);
    let ok = !version.is_empty()
        && version
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+'));
    if !ok {
        return Err(format!("not a version: {version}"));
    }
    if !prefix
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
    {
        return Err(format!("not a tag prefix: {prefix}"));
    }
    let name_ok = |p: &str| {
        !p.is_empty()
            && p.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    };
    let repo = feed
        .strip_prefix("https://github.com/")
        .and_then(|rest| rest.split_once("/releases/"))
        .map(|(repo, _)| repo)
        .filter(|repo| matches!(repo.split('/').collect::<Vec<_>>()[..], [owner, name] if name_ok(owner) && name_ok(name)))
        .ok_or_else(|| "このビルドの更新の配信先には、更新内容のページがありません".to_string())?;
    Ok(format!(
        "https://github.com/{repo}/releases/tag/{prefix}v{version}"
    ))
}

#[cfg(test)]
mod tests {
    use super::release_page;

    const FEED: &str =
        "https://github.com/someone/video-studio/releases/latest/download/latest.json";

    #[test]
    fn takes_only_a_version() {
        let page = "https://github.com/someone/video-studio/releases/tag/v0.2.0";
        assert_eq!(release_page(FEED, "", "0.2.0").unwrap(), page);
        assert_eq!(release_page(FEED, "", "v0.2.0").unwrap(), page);
        assert!(release_page(FEED, "", "1.0.0-beta.1").is_ok());
        assert!(release_page(FEED, "", "").is_err());
        assert!(release_page(FEED, "", "0.2.0/../../evil").is_err());
        assert!(release_page(FEED, "", "0.2.0?x=1").is_err());
        assert!(release_page(FEED, "../", "0.2.0").is_err());
    }

    #[test]
    fn follows_the_feed_and_the_prefix() {
        let fork =
            "https://github.com/someone/video-studio-fork/releases/latest/download/latest.json";
        assert_eq!(
            release_page(fork, "corp-", "1.2.3").unwrap(),
            "https://github.com/someone/video-studio-fork/releases/tag/corp-v1.2.3"
        );
        for feed in [
            "",
            "https://example.com/someone/video-studio/releases/latest/download/latest.json",
            "https://github.com/someone/releases/latest/download/latest.json",
            "https://github.com/some one/video-studio/releases/latest/download/latest.json",
        ] {
            assert!(release_page(feed, "", "1.2.3").is_err(), "{feed}");
        }
    }

    #[test]
    fn the_shipped_config_has_a_release_page() {
        let conf: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let feed = conf["plugins"]["updater"]["endpoints"][0].as_str().unwrap();
        assert_eq!(
            feed,
            "https://github.com/hamirilo/video-studio/releases/latest/download/latest.json"
        );
        assert_eq!(
            release_page(feed, "", "1.2.3").unwrap(),
            "https://github.com/hamirilo/video-studio/releases/tag/v1.2.3"
        );
    }
}
