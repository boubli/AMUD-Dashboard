use std::collections::HashMap;

pub(crate) fn get_default_settings() -> HashMap<&'static str, &'static str> {
    let mut s = HashMap::new();
    s.insert("app_name", "AMUD");
    s.insert("tagline", "Homelab Operations Cockpit");
    s.insert("accent_color", "#cf6427");
    s.insert("custom_bg_url", "/static/wallpaper.png");
    s.insert("app_logo", "");
    s.insert("glass_blur_intensity", "16");
    s.insert("glass_opacity", "0.45");
    s.insert("wallpaper_overlay_strength", "0.85");
    s.insert("bento_radius", "16");
    s.insert("grid_columns", "3");
    s.insert("pve_api_token", "");
    s.insert("donate_enabled", "1");
    s.insert("telemetry_public", "0");
    s.insert("ha_url", "");
    s.insert("ha_token", "");
    s.insert("custom_css", "");
    s.insert("active_theme_id", "default");
    s.insert("theme_mode", "dark");
    s.insert("theme_scheduler", "off");
    s.insert("theme_light_at", "07:00");
    s.insert("theme_dark_at", "19:00");
    s.insert("guest_category_restrict", "0");
    s.insert("guest_visible_categories", "");
    s.insert("dashboard_layout", "tabs");
    s.insert("status_page_public", "0");
    s.insert("kiosk_mode", "0");
    s.insert("iframe_embeds_enabled", "0");
    s.insert("oidc_enabled", "0");
    s.insert("oidc_issuer", "");
    s.insert("oidc_client_id", "");
    s.insert("oidc_client_secret", "");
    s.insert("oidc_redirect_uri", "");
    s.insert("oidc_default_role", "Guest");
    s.insert("integration_cache_ttl_secs", "45");
    s.insert("integration_cache_max_entries", "256");
    s.insert("feeds_enabled", "1");
    s.insert("agent_telemetry_interval_secs", "5");
    s.insert("agent_lxc_poll_interval_secs", "10");
    s.insert("agent_docker_poll_interval_secs", "10");
    s.insert("status_poll_interval_secs", "15");
    s.insert("media_poll_interval_secs", "5");
    s.insert("ha_poll_interval_secs", "15");
    s.insert("telemetry_broadcast_interval_secs", "5");
    s.insert("integration_coordinator_interval_secs", "45");
    s.insert("ldap_enabled", "0");
    s.insert("ldap_url", "");
    s.insert("ldap_bind_dn", "");
    s.insert("ldap_bind_password", "");
    s.insert("ldap_base_dn", "");
    s.insert("ldap_user_filter", "(uid={username})");
    s.insert("oidc_admin_group", "");
    s.insert("agent_node_tag", "Local");
    s.insert("performance_preset", "light");
    s.insert("idle_grace_secs", "45");
    s.insert("alert_cpu_threshold", "90");
    s.insert("alert_ram_threshold", "90");
    s.insert("alert_disk_threshold", "95");
    s.insert("backup_reminder_days", "30");
    s.insert("webgl_effects_enabled", "1");
    s.insert("greeting_animations_enabled", "1");
    s.insert("dashboard_reorder_enabled", "1");

    s
}

pub(crate) const SECRET_SETTING_KEYS: &[&str] = &[
    "pve_api_token",
    "ha_token",
    "oidc_client_secret",
    "ldap_bind_password",
];

pub(crate) const EXTRA_SETTING_KEYS: &[&str] = &[
    "weather_latitude",
    "weather_longitude",
    "weather_temp_unit",
    "clock_timezone",
    "clock_time_format",
    "default_search_engine",
    "custom_search_engines",
    "accept_invalid_certs",
    "webhooks_allow_private_ips",
    "enable_proxmox",
    "last_backup_export_at",
    "telemetry_external_ifaces",
    "telemetry_internal_ifaces",
    "telemetry_disk_mounts",
    "theme_scheduler",
    "theme_light_at",
    "theme_dark_at",
    "active_theme_id",
    "guest_category_restrict",
    "guest_visible_categories",
    "dashboard_layout",
    "status_page_public",
    "kiosk_mode",
    "iframe_embeds_enabled",
    "oidc_enabled",
    "oidc_issuer",
    "oidc_client_id",
    "oidc_redirect_uri",
    "oidc_default_role",
    "integration_cache_ttl_secs",
    "integration_cache_max_entries",
    "feeds_enabled",
    "agent_telemetry_interval_secs",
    "agent_lxc_poll_interval_secs",
    "agent_docker_poll_interval_secs",
    "status_poll_interval_secs",
    "media_poll_interval_secs",
    "ha_poll_interval_secs",
    "telemetry_broadcast_interval_secs",
    "integration_coordinator_interval_secs",
    "ldap_enabled",
    "ldap_url",
    "ldap_bind_dn",
    "ldap_base_dn",
    "ldap_user_filter",
    "oidc_admin_group",
    "agent_node_tag",
    "performance_preset",
    "idle_grace_secs",
    "alert_cpu_threshold",
    "alert_ram_threshold",
    "alert_disk_threshold",
    "backup_reminder_days",
    "webgl_effects_enabled",
    "greeting_animations_enabled",
    "dashboard_reorder_enabled",
    "installed_version",
    "last_version_change_at",
    "last_update_method",
];

pub(crate) const AGENT_CONFIG_SETTING_KEYS: &[&str] = &[
    "pve_api_token",
    "telemetry_external_ifaces",
    "telemetry_internal_ifaces",
    "telemetry_disk_mounts",
    "enable_proxmox",
    "agent_telemetry_interval_secs",
    "agent_lxc_poll_interval_secs",
    "agent_docker_poll_interval_secs",
    "agent_node_tag",
];

pub(crate) fn allowed_setting_keys() -> std::collections::HashSet<String> {
    let mut keys: std::collections::HashSet<String> = get_default_settings()
        .keys()
        .map(|k| (*k).to_string())
        .collect();
    for key in EXTRA_SETTING_KEYS {
        keys.insert((*key).to_string());
    }
    keys
}

pub(crate) fn setting_key_allowed(key: &str) -> bool {
    allowed_setting_keys().contains(key)
}

pub(crate) fn sanitize_wallpaper_overlay_strength(value: &str) -> String {
    let v: f64 = value.trim().parse().unwrap_or(0.85);
    format!("{:.2}", v.clamp(0.0, 1.0))
}

pub(crate) fn sanitize_custom_css(value: &str) -> String {
    let mut cleaned = String::new();
    let chars: Vec<char> = value.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] != '<' {
            cleaned.push(chars[i]);
            i += 1;
            continue;
        }

        let mut temp_idx = i + 1;
        while temp_idx < chars.len() && chars[temp_idx].is_whitespace() {
            temp_idx += 1;
        }

        let mut is_slash = false;
        if temp_idx < chars.len() && chars[temp_idx] == '/' {
            is_slash = true;
            temp_idx += 1;
            while temp_idx < chars.len() && chars[temp_idx].is_whitespace() {
                temp_idx += 1;
            }
        }

        let tag_start = temp_idx;
        while temp_idx < chars.len() && chars[temp_idx].is_alphabetic() {
            temp_idx += 1;
        }
        let tag_name: String = chars[tag_start..temp_idx]
            .iter()
            .collect::<String>()
            .to_ascii_lowercase();

        let dangerous = matches!(
            tag_name.as_str(),
            "style" | "script" | "iframe" | "object" | "html" | "body"
        );

        if dangerous {
            // Remove the full dangerous tag token instead of leaving broken fragments like "/style>".
            while temp_idx < chars.len() && chars[temp_idx] != '>' {
                temp_idx += 1;
            }
            if temp_idx < chars.len() && chars[temp_idx] == '>' {
                temp_idx += 1;
            }
            i = temp_idx;
            continue;
        }

        // Keep benign "<" content unchanged (e.g. @media (width < 900px)).
        cleaned.push('<');
        if is_slash {
            cleaned.push('/');
        }
        i += 1;
    }

    cleaned
}

/// Integration base URLs must be empty or absolute http(s).
pub(crate) fn sanitize_integration_url(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return trimmed.to_string();
    }
    String::new()
}

pub(crate) fn sanitize_setting_url(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.starts_with('/') && !trimmed.contains("..") {
        return trimmed.to_string();
    }
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return trimmed.to_string();
    }
    String::new()
}

/// Light/dark only — invalid values fall back to dark.
pub(crate) fn sanitize_theme_mode(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "light" => "light".to_string(),
        _ => "dark".to_string(),
    }
}

/// Theme scheduler mode: off, sunrise_sunset, or manual.
pub(crate) fn sanitize_theme_scheduler(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "sunrise_sunset" => "sunrise_sunset".to_string(),
        "manual" => "manual".to_string(),
        _ => "off".to_string(),
    }
}

/// Bundled theme id from manifest (alphanumeric + hyphens).
pub(crate) fn sanitize_active_theme_id(value: &str) -> String {
    let trimmed = value.trim().to_ascii_lowercase();
    if trimmed.is_empty() || trimmed == "default" {
        return "default".to_string();
    }
    // Removed in v1.8.9 — fall back so saved settings stay valid.
    const REMOVED: &[&str] = &[
        "sunset-warm",
        "vaporwave-grid",
        "ocean-depths",
        "terminal-amber",
        "arctic-frost",
    ];
    if REMOVED.contains(&trimmed.as_str()) {
        return "default".to_string();
    }
    let valid = trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-')
        && trimmed.len() <= 64;
    if valid {
        trimmed
    } else {
        "default".to_string()
    }
}

/// HH:MM clock time for theme scheduler manual mode.
pub(crate) fn sanitize_time_hhmm(value: &str, default: &str) -> String {
    let parts: Vec<&str> = value.trim().split(':').collect();
    if parts.len() == 2 {
        if let (Ok(h), Ok(m)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
            if h < 24 && m < 60 {
                return format!("{h:02}:{m:02}");
            }
        }
    }
    default.to_string()
}

pub(crate) fn sanitize_bool_setting(value: &str) -> String {
    if value.trim() == "1" || value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("on")
    {
        "1".to_string()
    } else {
        "0".to_string()
    }
}

/// Comma-separated dashboard category names visible to guests when restriction is enabled.
pub(crate) fn sanitize_guest_visible_categories(value: &str) -> String {
    value
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty() && s.len() <= 64)
        .filter(|s| s.chars().all(|c| !c.is_control()))
        .collect::<Vec<_>>()
        .join(",")
}

/// When restriction is off, returns None (all categories). When on, returns allowed names (may be empty).
pub(crate) fn parse_guest_visible_categories(
    settings: &HashMap<String, String>,
) -> Option<std::collections::HashSet<String>> {
    if settings
        .get("guest_category_restrict")
        .map(|s| s.as_str())
        .unwrap_or("0")
        != "1"
    {
        return None;
    }
    let list = settings
        .get("guest_visible_categories")
        .map(|s| s.as_str())
        .unwrap_or("");
    Some(
        list.split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect(),
    )
}

/// Comma-separated network interface names (e.g. `eth0,vmbr0`).
pub(crate) fn sanitize_iface_list(value: &str) -> String {
    let mut seen = std::collections::HashSet::new();
    value
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_ascii_lowercase())
        .map(|s| {
            s.trim_matches(|c: char| c.is_whitespace() || c == ',')
                .to_string()
        })
        .filter(|s| !s.is_empty())
        .filter(|s| {
            s.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
        })
        .filter(|s| seen.insert(s.clone()))
        .collect::<Vec<_>>()
        .join(",")
}

/// Comma-separated absolute mount paths (e.g. `/,/mnt/user`).
pub(crate) fn sanitize_disk_mount_list(value: &str) -> String {
    let mut seen = std::collections::HashSet::new();
    value
        .split(',')
        .map(str::trim)
        .filter(|s| s.starts_with('/') && !s.contains(".."))
        .map(|s| {
            let mut v = s.to_string();
            while v.ends_with('/') && v.len() > 1 {
                v.pop();
            }
            v
        })
        .filter(|s| seen.insert(s.clone()))
        .collect::<Vec<_>>()
        .join(",")
}

/// Bento card span — unknown values become 1x1.
pub(crate) fn sanitize_card_span(value: &str) -> String {
    match value.trim() {
        "2x1" | "1x2" => value.trim().to_string(),
        _ => "1x1".to_string(),
    }
}

/// Integration + container metrics need a tall card when API metrics are shown.
pub(crate) fn resolve_card_span(
    integration_type: &str,
    show_container_metrics: bool,
    integration_visible_metrics: &str,
    requested: &str,
) -> String {
    if integration_type.is_empty() || integration_type == "rss" {
        return sanitize_card_span(requested);
    }
    if integration_api_metrics_hidden(integration_visible_metrics) {
        if show_container_metrics {
            return "1x1".to_string();
        }
        return sanitize_card_span(requested);
    }
    "1x2".to_string()
}

/// Empty JSON array hides API integration metrics on the app card (CPU/RAM may still show).
pub(crate) fn integration_api_metrics_hidden(integration_visible_metrics: &str) -> bool {
    integration_visible_metrics.trim() == "[]"
}

pub(crate) fn default_integration_visible_metrics(integration_type: &str) -> String {
    match integration_type.trim().to_ascii_lowercase().as_str() {
        "jellyfin" | "plex" | "emby" => "[]".to_string(),
        _ => String::new(),
    }
}

pub(crate) fn sanitize_integration_visible_metrics(raw: Option<&str>) -> String {
    let Some(value) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return String::new();
    };
    if value == "[]" {
        return "[]".to_string();
    }
    let Ok(parsed) = serde_json::from_str::<Vec<String>>(value) else {
        return String::new();
    };
    let cleaned: Vec<String> = parsed
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if cleaned.is_empty() {
        return "[]".to_string();
    }
    serde_json::to_string(&cleaned).unwrap_or_default()
}

/// Per-app embed mode: link, iframe, or tab.
pub(crate) fn sanitize_embed_mode(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "iframe" | "tab" => value.trim().to_ascii_lowercase(),
        _ => "link".to_string(),
    }
}

/// Dashboard layout: tabs or sections.
pub(crate) fn sanitize_dashboard_layout(value: &str) -> String {
    if value.trim().eq_ignore_ascii_case("sections") {
        "sections".to_string()
    } else {
        "tabs".to_string()
    }
}

/// Widget type whitelist.
pub(crate) fn sanitize_widget_type(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "links" | "html" | "calendar_ics" | "arr_calendar" | "datetime" | "resources" => {
            value.trim().to_ascii_lowercase()
        }
        _ => "note".to_string(),
    }
}

/// Per-app CPU/RAM row on cards — default on for existing apps.
pub(crate) fn parse_show_container_metrics(value: Option<&str>) -> i64 {
    match value.map(str::trim) {
        Some("1") | Some("true") | Some("on") => 1,
        _ => 0,
    }
}

// Donation links are locked to the author; toggle via show_donation setting.
pub(crate) const DONATION_MESSAGE: &str = "AMUD is completely free and you already have every feature unlocked. A donation is not required and unlocks nothing extra - it is simply a kind way to support continued development. Thank you!";
pub(crate) const DONATION_LINKS: [(&str, &str, &str); 3] = [
    (
        "https://github.com/sponsors/boubli",
        "GitHub Sponsors",
        "github",
    ),
    (
        "https://buy.stripe.com/cNi14n6b9a7v5Jg4Rq4ko00",
        "Donate via Card",
        "credit-card",
    ),
    ("https://ko-fi.com/Youssefboubli", "Ko-fi", "coffee"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_wallpaper_overlay_strength() {
        assert_eq!(sanitize_wallpaper_overlay_strength("0.85"), "0.85");
        assert_eq!(sanitize_wallpaper_overlay_strength("1.5"), "1.00");
        assert_eq!(sanitize_wallpaper_overlay_strength("-0.2"), "0.00");
    }

    #[test]
    fn test_sanitize_custom_css() {
        assert_eq!(
            sanitize_custom_css("body { color: red; }"),
            "body { color: red; }"
        );
        assert_eq!(
            sanitize_custom_css("div > p { color: blue; }"),
            "div > p { color: blue; }"
        );
        assert_eq!(
            sanitize_custom_css("@media (max-width < 600px) { }"),
            "@media (max-width < 600px) { }"
        );
        assert_eq!(
            sanitize_custom_css("</style><script>alert(1)</script>"),
            "alert(1)"
        );
        assert_eq!(sanitize_custom_css("< sCrIpt >"), "");
        assert_eq!(
            sanitize_custom_css("@media (max-width < 900px) { .x { color: red; } }"),
            "@media (max-width < 900px) { .x { color: red; } }"
        );
    }

    #[test]
    fn test_sanitize_theme_mode() {
        assert_eq!(sanitize_theme_mode("light"), "light");
        assert_eq!(sanitize_theme_mode("LIGHT"), "light");
        assert_eq!(sanitize_theme_mode("dark"), "dark");
        assert_eq!(sanitize_theme_mode("invalid"), "dark");
        assert_eq!(sanitize_theme_mode(""), "dark");
    }

    #[test]
    fn test_sanitize_theme_scheduler() {
        assert_eq!(sanitize_theme_scheduler("off"), "off");
        assert_eq!(sanitize_theme_scheduler("sunrise_sunset"), "sunrise_sunset");
        assert_eq!(sanitize_theme_scheduler("manual"), "manual");
        assert_eq!(sanitize_theme_scheduler("bogus"), "off");
    }

    #[test]
    fn test_sanitize_active_theme_id() {
        assert_eq!(sanitize_active_theme_id("default"), "default");
        assert_eq!(sanitize_active_theme_id("nord"), "nord");
        assert_eq!(
            sanitize_active_theme_id("terminal-matrix"),
            "terminal-matrix"
        );
        assert_eq!(sanitize_active_theme_id(""), "default");
        assert_eq!(sanitize_active_theme_id("bad id!"), "default");
        assert_eq!(sanitize_active_theme_id("sunset-warm"), "default");
        assert_eq!(sanitize_active_theme_id("vaporwave-grid"), "default");
        assert_eq!(sanitize_active_theme_id("ocean-depths"), "default");
        assert_eq!(sanitize_active_theme_id("terminal-amber"), "default");
        assert_eq!(sanitize_active_theme_id("arctic-frost"), "default");
        assert_eq!(sanitize_active_theme_id("ember-hearth"), "ember-hearth");
    }

    #[test]
    fn test_sanitize_time_hhmm() {
        assert_eq!(sanitize_time_hhmm("7:5", "07:00"), "07:05");
        assert_eq!(sanitize_time_hhmm("25:00", "07:00"), "07:00");
        assert_eq!(sanitize_time_hhmm("", "19:00"), "19:00");
    }

    #[test]
    fn test_parse_guest_visible_categories() {
        let mut settings = HashMap::new();
        assert!(parse_guest_visible_categories(&settings).is_none());

        settings.insert("guest_category_restrict".to_string(), "1".to_string());
        settings.insert(
            "guest_visible_categories".to_string(),
            "Media,General".to_string(),
        );
        let allowed = parse_guest_visible_categories(&settings).unwrap();
        assert!(allowed.contains("Media"));
        assert!(allowed.contains("General"));
    }

    #[test]
    fn test_sanitize_card_span() {
        assert_eq!(sanitize_card_span("1x1"), "1x1");
        assert_eq!(sanitize_card_span("2x1"), "2x1");
        assert_eq!(sanitize_card_span("1x2"), "1x2");
        assert_eq!(sanitize_card_span("2x2"), "1x1");
        assert_eq!(sanitize_card_span(""), "1x1");
    }

    #[test]
    fn test_resolve_card_span_integration_metrics() {
        assert_eq!(resolve_card_span("radarr", true, "", "1x1"), "1x2");
        assert_eq!(resolve_card_span("radarr", false, "", "2x1"), "1x2");
        assert_eq!(resolve_card_span("jellyfin", true, "[]", "1x1"), "1x1");
        assert_eq!(resolve_card_span("jellyfin", false, "[]", "2x1"), "2x1");
        assert_eq!(resolve_card_span("rss", true, "", "1x1"), "1x1");
        assert_eq!(resolve_card_span("", true, "", "2x1"), "2x1");
    }

    #[test]
    fn test_parse_show_container_metrics() {
        assert_eq!(parse_show_container_metrics(Some("1")), 1);
        assert_eq!(parse_show_container_metrics(Some("true")), 1);
        assert_eq!(parse_show_container_metrics(Some("0")), 0);
        assert_eq!(parse_show_container_metrics(None), 0);
    }

    #[test]
    fn test_sanitize_iface_list() {
        assert_eq!(sanitize_iface_list("eth0, vmbr0"), "eth0,vmbr0");
        assert_eq!(sanitize_iface_list("ETH0, eth0 , vmbr0 "), "eth0,vmbr0");
        assert_eq!(sanitize_iface_list("eth0,bad!name"), "eth0");
    }

    #[test]
    fn test_sanitize_disk_mount_list() {
        assert_eq!(sanitize_disk_mount_list("/,/mnt/user"), "/,/mnt/user");
        assert_eq!(
            sanitize_disk_mount_list("/mnt/user/, /mnt/user, /mnt/user/cache/"),
            "/mnt/user,/mnt/user/cache"
        );
        assert_eq!(sanitize_disk_mount_list("relative,/../etc"), "");
    }
}

pub(crate) fn setting_flag(settings: &HashMap<String, String>, key: &str, default: bool) -> bool {
    settings.get(key).map(|s| s == "1").unwrap_or(default)
}

pub(crate) fn feeds_enabled(settings: &HashMap<String, String>) -> bool {
    setting_flag(settings, "feeds_enabled", true)
}

pub(crate) fn setting_u64_bounded(
    settings: &HashMap<String, String>,
    key: &str,
    default: u64,
    min: u64,
    max: u64,
) -> u64 {
    let v = settings
        .get(key)
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(default);
    v.clamp(min, max)
}

pub(crate) fn sanitize_interval_setting(value: &str, default: u64, min: u64, max: u64) -> String {
    let v = value.trim().parse::<u64>().unwrap_or(default);
    v.clamp(min, max).to_string()
}

pub(crate) fn sanitize_cache_max_entries(value: &str) -> String {
    sanitize_interval_setting(value, 256, 16, 512)
}

pub(crate) fn sanitize_cache_ttl_secs(value: &str) -> String {
    sanitize_interval_setting(value, 45, 5, 600)
}

pub(crate) fn apply_integration_cache_limits(
    cache: &crate::integration_cache::IntegrationCache,
    settings: &HashMap<String, String>,
) {
    let max = setting_u64_bounded(settings, "integration_cache_max_entries", 256, 16, 512) as usize;
    let ttl = setting_u64_bounded(settings, "integration_cache_ttl_secs", 45, 5, 600);
    cache.set_limits(max, ttl);
}

pub(crate) fn apply_performance_preset(db: &rusqlite::Connection, preset: &str) {
    let (coord, status, cache_max, cache_ttl, tel_bcast, media, ha, agent_tel) = match preset {
        "balanced" => (45, 15, 48, 45, 5, 5, 15, 5),
        "active" => (20, 10, 48, 30, 3, 3, 10, 3),
        "custom" => return,
        _ => (90, 30, 32, 60, 10, 10, 30, 10),
    };
    for (key, val) in [
        ("integration_coordinator_interval_secs", coord),
        ("status_poll_interval_secs", status),
        ("integration_cache_max_entries", cache_max),
        ("integration_cache_ttl_secs", cache_ttl),
        ("telemetry_broadcast_interval_secs", tel_bcast),
        ("media_poll_interval_secs", media),
        ("ha_poll_interval_secs", ha),
        ("agent_telemetry_interval_secs", agent_tel),
    ] {
        let _ = db.execute(
            "INSERT INTO settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![key, val.to_string()],
        );
    }
}

pub(crate) fn sanitize_performance_preset(value: &str) -> String {
    match value.trim().to_lowercase().as_str() {
        "balanced" => "balanced".into(),
        "active" => "active".into(),
        "custom" => "custom".into(),
        _ => "light".into(),
    }
}

/// Built-in web search engines: (id, label, url template with `{query}`).
pub(crate) const BUILTIN_SEARCH_ENGINES: &[(&str, &str, &str)] = &[
    (
        "google",
        "Google",
        "https://www.google.com/search?q={query}",
    ),
    ("bing", "Bing", "https://www.bing.com/search?q={query}"),
    (
        "duckduckgo",
        "DuckDuckGo",
        "https://duckduckgo.com/?q={query}",
    ),
    (
        "youtube",
        "YouTube",
        "https://www.youtube.com/results?search_query={query}",
    ),
    ("github", "GitHub", "https://github.com/search?q={query}"),
];

/// Curated IANA timezones for the clock setting (plus empty/`auto` for browser local).
pub(crate) const CLOCK_TIMEZONE_OPTIONS: &[&str] = &[
    "UTC",
    "America/New_York",
    "America/Chicago",
    "America/Denver",
    "America/Los_Angeles",
    "America/Toronto",
    "America/Mexico_City",
    "America/Sao_Paulo",
    "America/Argentina/Buenos_Aires",
    "Europe/London",
    "Europe/Paris",
    "Europe/Berlin",
    "Europe/Madrid",
    "Europe/Rome",
    "Europe/Amsterdam",
    "Europe/Brussels",
    "Europe/Warsaw",
    "Europe/Moscow",
    "Europe/Istanbul",
    "Africa/Casablanca",
    "Africa/Cairo",
    "Africa/Johannesburg",
    "Asia/Dubai",
    "Asia/Karachi",
    "Asia/Kolkata",
    "Asia/Bangkok",
    "Asia/Shanghai",
    "Asia/Hong_Kong",
    "Asia/Tokyo",
    "Asia/Seoul",
    "Asia/Singapore",
    "Australia/Sydney",
    "Australia/Melbourne",
    "Pacific/Auckland",
    "Pacific/Honolulu",
];

pub(crate) fn sanitize_clock_timezone(value: &str) -> String {
    let v = value.trim();
    if v.is_empty() || v.eq_ignore_ascii_case("auto") || v.eq_ignore_ascii_case("local") {
        return "auto".into();
    }
    if CLOCK_TIMEZONE_OPTIONS.iter().any(|z| *z == v) {
        return v.to_string();
    }
    // Allow other valid-looking IANA ids (Area/Location) so users aren't blocked.
    let ok = v.len() <= 64
        && v
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '/' || c == '_' || c == '+' || c == '-');
    if ok && v.contains('/') {
        v.to_string()
    } else {
        "auto".into()
    }
}

pub(crate) fn sanitize_clock_time_format(value: &str) -> String {
    match value.trim().to_lowercase().as_str() {
        "24h" | "24" | "HH" => "24h".into(),
        _ => "12h".into(),
    }
}

fn slugify_engine_id(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if (c == '-' || c == '_' || c.is_whitespace()) && !out.ends_with('-') {
            out.push('-');
        }
        if out.len() >= 32 {
            break;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "custom".into()
    } else {
        trimmed
    }
}

fn normalize_search_url_template(raw: &str) -> Option<String> {
    let url = raw.trim();
    if !url.starts_with("https://") || url.len() > 512 {
        return None;
    }
    if url.contains("{query}") || url.contains("%s") {
        Some(url.to_string())
    } else {
        None
    }
}

/// Sanitize custom search engines JSON to a compact array of `{id,name,url}`.
pub(crate) fn sanitize_custom_search_engines(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return "[]".into();
    }
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(trimmed) else {
        return "[]".into();
    };
    let Some(arr) = parsed.as_array() else {
        return "[]".into();
    };
    let mut out = Vec::new();
    let mut used_ids = std::collections::HashSet::new();
    for builtin in BUILTIN_SEARCH_ENGINES {
        used_ids.insert(builtin.0.to_string());
    }
    for item in arr.iter().take(20) {
        let name = item
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        let name: String = name
            .chars()
            .filter(|c| !matches!(c, '<' | '>' | '"' | '\'' | '&' | '`'))
            .take(64)
            .collect();
        let name = name.trim();
        let url_raw = item
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        if name.is_empty() || name.len() > 64 {
            continue;
        }
        let Some(url) = normalize_search_url_template(url_raw) else {
            continue;
        };
        let mut id = item
            .get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| slugify_engine_id(name));
        if !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            || id.len() > 40
        {
            id = slugify_engine_id(name);
        }
        let mut candidate = id.clone();
        let mut n = 2u32;
        while used_ids.contains(&candidate) {
            candidate = format!("{id}-{n}");
            n += 1;
            if n > 99 {
                break;
            }
        }
        used_ids.insert(candidate.clone());
        out.push(serde_json::json!({
            "id": candidate,
            "name": name,
            "url": url,
        }));
    }
    serde_json::to_string(&out).unwrap_or_else(|_| "[]".into())
}

pub(crate) fn sanitize_default_search_engine(
    value: &str,
    custom_engines_json: &str,
) -> String {
    let v = value.trim().to_lowercase();
    if BUILTIN_SEARCH_ENGINES.iter().any(|(id, _, _)| *id == v) {
        return v;
    }
    if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(custom_engines_json) {
        if arr.iter().any(|e| {
            e.get("id")
                .and_then(|x| x.as_str())
                .is_some_and(|id| id.eq_ignore_ascii_case(&v))
        }) {
            return v;
        }
    }
    "google".into()
}

pub(crate) fn parse_custom_search_engines(
    settings: &HashMap<String, String>,
) -> Vec<(String, String, String)> {
    let raw = settings
        .get("custom_search_engines")
        .map(|s| s.as_str())
        .unwrap_or("[]");
    let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(raw) else {
        return Vec::new();
    };
    arr.into_iter()
        .filter_map(|e| {
            let id = e.get("id")?.as_str()?.to_string();
            let name = e.get("name")?.as_str()?.to_string();
            let url = e.get("url")?.as_str()?.to_string();
            Some((id, name, url))
        })
        .collect()
}

pub(crate) fn build_search_engines_json(settings: &HashMap<String, String>) -> String {
    let custom = parse_custom_search_engines(settings);
    let default = settings
        .get("default_search_engine")
        .map(|s| s.as_str())
        .unwrap_or("google");
    let mut engines = Vec::new();
    for (id, name, url) in BUILTIN_SEARCH_ENGINES {
        engines.push(serde_json::json!({
            "id": id,
            "name": name,
            "url": url,
            "builtin": true,
        }));
    }
    for (id, name, url) in &custom {
        engines.push(serde_json::json!({
            "id": id,
            "name": name,
            "url": url,
            "builtin": false,
        }));
    }
    serde_json::to_string(&serde_json::json!({
        "default": default,
        "engines": engines,
    }))
    .unwrap_or_else(|_| {
        r#"{"default":"google","engines":[]}"#.into()
    })
}

pub(crate) fn build_search_engine_options_html(settings: &HashMap<String, String>) -> String {
    let default = settings
        .get("default_search_engine")
        .map(|s| s.as_str())
        .unwrap_or("google");
    let custom = parse_custom_search_engines(settings);
    let mut html = String::new();
    for (id, name, _) in BUILTIN_SEARCH_ENGINES {
        let selected = if *id == default { " selected" } else { "" };
        html.push_str(&format!(
            r#"<option value="{id}"{selected}>{name}</option>"#,
            id = crate::templates::escape_html(id),
            selected = selected,
            name = crate::templates::escape_html(name),
        ));
    }
    for (id, name, _) in &custom {
        let selected = if id == default { " selected" } else { "" };
        html.push_str(&format!(
            r#"<option value="{}"{}>{}</option>"#,
            crate::templates::escape_html(id),
            selected,
            crate::templates::escape_html(name),
        ));
    }
    html
}

pub(crate) fn build_clock_config_json(settings: &HashMap<String, String>) -> String {
    let timezone = sanitize_clock_timezone(
        settings
            .get("clock_timezone")
            .map(|s| s.as_str())
            .unwrap_or("auto"),
    );
    let format = sanitize_clock_time_format(
        settings
            .get("clock_time_format")
            .map(|s| s.as_str())
            .unwrap_or("12h"),
    );
    serde_json::to_string(&serde_json::json!({
        "timezone": timezone,
        "format": format,
    }))
    .unwrap_or_else(|_| r#"{"timezone":"auto","format":"12h"}"#.into())
}

pub(crate) fn build_timezone_options_html(selected: &str) -> String {
    let sel = sanitize_clock_timezone(selected);
    let mut html = format!(
        r#"<option value="auto"{}>Browser local</option>"#,
        if sel == "auto" { " selected" } else { "" }
    );
    for zone in CLOCK_TIMEZONE_OPTIONS {
        let selected_attr = if *zone == sel { " selected" } else { "" };
        html.push_str(&format!(
            r#"<option value="{z}"{selected_attr}>{z}</option>"#,
            z = crate::templates::escape_html(zone),
            selected_attr = selected_attr,
        ));
    }
    if sel != "auto" && !CLOCK_TIMEZONE_OPTIONS.iter().any(|z| *z == sel) {
        html.push_str(&format!(
            r#"<option value="{z}" selected>{z}</option>"#,
            z = crate::templates::escape_html(&sel),
        ));
    }
    html
}

pub(crate) fn backup_export_overdue(settings: &HashMap<String, String>) -> bool {
    let reminder_days = settings
        .get("backup_reminder_days")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(30);
    if reminder_days <= 0 {
        return false;
    }
    let last = settings
        .get("last_backup_export_at")
        .map(|s| s.as_str())
        .unwrap_or("");
    if last.is_empty() || last.eq_ignore_ascii_case("never") {
        return true;
    }
    chrono::DateTime::parse_from_rfc3339(last)
        .ok()
        .map(|dt| (chrono::Utc::now() - dt.with_timezone(&chrono::Utc)).num_days() >= reminder_days)
        .unwrap_or(true)
}

#[cfg(test)]
mod v177_tests {
    use super::*;

    #[test]
    fn feeds_enabled_defaults_true() {
        assert!(feeds_enabled(&HashMap::new()));
    }

    #[test]
    fn feeds_disabled_when_zero() {
        let mut s = HashMap::new();
        s.insert("feeds_enabled".to_string(), "0".to_string());
        assert!(!feeds_enabled(&s));
    }

    #[test]
    fn setting_u64_bounded_clamps_high() {
        let mut s = HashMap::new();
        s.insert(
            "agent_telemetry_interval_secs".to_string(),
            "999".to_string(),
        );
        assert_eq!(
            setting_u64_bounded(&s, "agent_telemetry_interval_secs", 5, 3, 60),
            60
        );
    }

    #[test]
    fn apply_integration_cache_limits_updates_ttl() {
        let cache = crate::integration_cache::IntegrationCache::new(64, 45);
        let mut s = HashMap::new();
        s.insert("integration_cache_ttl_secs".to_string(), "90".to_string());
        s.insert(
            "integration_cache_max_entries".to_string(),
            "32".to_string(),
        );
        apply_integration_cache_limits(&cache, &s);
        assert_eq!(cache.default_ttl(), std::time::Duration::from_secs(90));
        assert_eq!(cache.len(), 0);
    }
}
