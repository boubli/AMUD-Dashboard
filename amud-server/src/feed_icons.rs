use std::collections::HashMap;

/// Resolve the logo path for an RSS feed card or settings preview.
pub(crate) fn resolve_feed_logo(
    icon: &str,
    _name: &str,
    site_url: &str,
    feed_url: &str,
    manifest: &HashMap<String, String>,
) -> String {
    if icon.starts_with("http") || icon.starts_with('/') {
        return icon.to_string();
    }

    let key = icon.trim().to_lowercase();
    if key.is_empty() || key == "rss" || key == "auto" {
        return auto_feed_icon_url(site_url, feed_url);
    }

    if let Some(path) = lookup_preset_logo(&key) {
        return path;
    }
    if let Some(path) = manifest.get(&key) {
        return path.clone();
    }
    let dashed = key.replace(' ', "-");
    if let Some(path) = manifest.get(&dashed) {
        return path.clone();
    }

    auto_feed_icon_url(site_url, feed_url)
}

/// Favicon URL derived from the feed or site hostname (stored in `apps.icon` for new feeds).
pub(crate) fn auto_feed_icon_url(site_url: &str, feed_url: &str) -> String {
    for url in [site_url, feed_url] {
        let host = host_from_url(url);
        if let Some(logo) = preset_logo_for_host(&host) {
            return logo;
        }
    }
    for url in [site_url, feed_url] {
        let host = host_from_url(url);
        if !host.is_empty() {
            return favicon_url_for_host(&host);
        }
    }
    "/static/feeds/icons/rss.svg".to_string()
}

pub(crate) fn favicon_url_for_host(host: &str) -> String {
    let host = host.trim().trim_start_matches("www.");
    if host.is_empty() {
        return "/static/feeds/icons/rss.svg".to_string();
    }
    format!("/api/rss/favicon?host={host}")
}

pub(crate) fn host_from_url(url: &str) -> String {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let without_scheme = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .unwrap_or(trimmed);
    let host = without_scheme.split('/').next().unwrap_or("");
    host.strip_prefix("www.").unwrap_or(host).to_lowercase()
}

fn lookup_preset_logo(key: &str) -> Option<String> {
    PRESET_LOGOS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, path)| path.to_string())
}

fn host_matches_domain(host: &str, domain: &str) -> bool {
    host == domain || host.ends_with(&format!(".{domain}"))
}

fn preset_logo_for_host(host: &str) -> Option<String> {
    let host = host.trim().trim_start_matches("www.").to_lowercase();
    if host.is_empty() {
        return None;
    }
    for (preset_id, domains) in PRESET_DOMAINS {
        if domains.iter().any(|d| host_matches_domain(&host, d)) {
            return lookup_preset_logo(preset_id);
        }
    }
    None
}

const PRESET_LOGOS: &[(&str, &str)] = &[
    ("rss", "/static/feeds/icons/rss.svg"),
    ("bbc", "/static/feeds/icons/bbc.svg"),
    ("cnn", "/static/feeds/icons/cnn.svg"),
    ("reuters", "/static/feeds/icons/reuters.svg"),
    ("nytimes", "/static/feeds/icons/nytimes.svg"),
    ("guardian", "/static/feeds/icons/guardian.svg"),
    ("techcrunch", "/static/feeds/icons/techcrunch.svg"),
    ("the-verge", "/static/feeds/icons/the-verge.svg"),
    ("verge", "/static/feeds/icons/the-verge.svg"),
    ("espn", "/static/feeds/icons/espn.svg"),
    ("bloomberg", "/static/feeds/icons/bloomberg.svg"),
    ("hackernews", "/static/feeds/icons/hackernews.svg"),
    ("hn", "/static/feeds/icons/hackernews.svg"),
    ("ars-technica", "/static/feeds/icons/ars-technica.svg"),
    ("ars", "/static/feeds/icons/ars-technica.svg"),
    ("substack", "/static/feeds/icons/substack.svg"),
    ("fox-news", "/static/feeds/icons/fox-news.svg"),
    ("ap-news", "/static/feeds/icons/ap-news.svg"),
    ("wired", "/static/feeds/icons/wired.svg"),
    ("engadget", "/static/feeds/icons/engadget.svg"),
    ("politico", "/static/feeds/icons/politico.svg"),
    ("inoreader", "/static/logos/inoreader.svg"),
    ("reddit", "/static/logos/reddit.svg"),
    ("nasa", "/static/logos/nasa.svg"),
    ("youtube", "/static/logos/youtube.svg"),
    ("github", "/static/logos/github.svg"),
    ("x", "/static/logos/x.svg"),
    ("twitter", "/static/logos/twitter.svg"),
    ("medium", "/static/logos/medium-dark.svg"),
    ("google", "/static/logos/google.svg"),
    ("apple", "/static/logos/apple.svg"),
    ("netflix", "/static/logos/netflix.svg"),
];

const PRESET_DOMAINS: &[(&str, &[&str])] = &[
    ("bbc", &["bbc.co.uk", "bbci.co.uk", "bbc.com"]),
    ("cnn", &["cnn.com"]),
    ("reuters", &["reuters.com"]),
    ("nytimes", &["nytimes.com"]),
    ("guardian", &["theguardian.com", "guardian.co.uk"]),
    ("techcrunch", &["techcrunch.com"]),
    ("the-verge", &["theverge.com"]),
    ("hackernews", &["news.ycombinator.com", "hnrss.org"]),
    ("ars-technica", &["arstechnica.com"]),
    ("espn", &["espn.com"]),
    ("bloomberg", &["bloomberg.com"]),
    ("nasa", &["nasa.gov"]),
    ("reddit", &["reddit.com"]),
    ("youtube", &["youtube.com"]),
    ("github", &["github.com", "github.blog"]),
    ("x", &["x.com", "twitter.com"]),
    ("medium", &["medium.com"]),
    ("substack", &["substack.com"]),
    ("fox-news", &["foxnews.com"]),
    ("ap-news", &["apnews.com", "ap.org"]),
    ("wired", &["wired.com"]),
    ("engadget", &["engadget.com"]),
    ("politico", &["politico.com"]),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_icon_uses_feed_host() {
        let url = auto_feed_icon_url("", "https://feeds.bbci.co.uk/news/rss.xml");
        assert!(url.contains("bbc"));
    }

    #[test]
    fn auto_icon_uses_preset_domain() {
        let url = auto_feed_icon_url("https://www.bbc.com/", "");
        assert!(url.contains("bbc"));
    }

    #[test]
    fn resolve_prefers_stored_favicon_url() {
        let manifest = HashMap::new();
        let logo = resolve_feed_logo(
            "/api/rss/favicon?host=example.com",
            "Example",
            "",
            "",
            &manifest,
        );
        assert_eq!(logo, "/api/rss/favicon?host=example.com");
    }

    #[test]
    fn resolve_auto_from_feed_url() {
        let manifest = HashMap::new();
        let logo = resolve_feed_logo(
            "rss",
            "Tech Blog",
            "",
            "https://news.ycombinator.com/rss",
            &manifest,
        );
        assert!(logo.contains("hackernews"));
    }

    #[test]
    fn legacy_preset_icon_still_works() {
        let manifest = HashMap::new();
        let logo = resolve_feed_logo("bbc", "BBC", "", "", &manifest);
        assert!(logo.contains("bbc"));
    }
}
