use super::imports::*;

pub async fn settings_page_handler(
    Extension(csp): Extension<CspNonce>,
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let session = match require_admin_session(&headers, &state.sessions) {
        Ok(s) => s,
        Err(_resp) => return Redirect::to("/login").into_response(),
    };
    crate::activity::signal_gui_session_start(&state);

    let settings = state.settings_cache.read().unwrap().clone();
    let branding = branding_from_settings(&settings);
    let app_name = branding.app_name.as_str();
    let tagline = branding
        .tagline
        .as_deref()
        .unwrap_or("Homelab Operations Cockpit");
    let custom_bg_url = branding.custom_bg_url.as_str();
    let app_logo = branding.app_logo.as_str();
    let accent_color = branding.accent_color.as_str();
    let glass_blur = branding.glass_blur.as_str();
    let glass_opacity = branding.glass_opacity.as_str();
    let bento_radius = branding.bento_radius.as_str();
    let grid_columns = branding.grid_columns.as_deref().unwrap_or("3");
    let weather_latitude = settings
        .get("weather_latitude")
        .map(|s| s.as_str())
        .unwrap_or("");
    let weather_longitude = settings
        .get("weather_longitude")
        .map(|s| s.as_str())
        .unwrap_or("");
    let weather_temp_unit = settings
        .get("weather_temp_unit")
        .map(|s| s.as_str())
        .unwrap_or("celsius");
    let clock_timezone = crate::settings::sanitize_clock_timezone(
        settings
            .get("clock_timezone")
            .map(|s| s.as_str())
            .unwrap_or("auto"),
    );
    let clock_time_format = crate::settings::sanitize_clock_time_format(
        settings
            .get("clock_time_format")
            .map(|s| s.as_str())
            .unwrap_or("12h"),
    );
    let custom_search_engines = settings
        .get("custom_search_engines")
        .map(|s| s.as_str())
        .unwrap_or("[]");
    let default_search_engine = crate::settings::sanitize_default_search_engine(
        settings
            .get("default_search_engine")
            .map(|s| s.as_str())
            .unwrap_or("google"),
        custom_search_engines,
    );
    let timezone_options_html = crate::settings::build_timezone_options_html(&clock_timezone);
    let default_search_options_html = crate::settings::build_search_engine_options_html(&settings);
    let donate_enabled = settings
        .get("donate_enabled")
        .map(|s| s.as_str())
        .unwrap_or("1");
    let custom_css = settings.get("custom_css").map(|s| s.as_str()).unwrap_or("");
    let active_theme_id = sanitize_active_theme_id(
        settings
            .get("active_theme_id")
            .map(|s| s.as_str())
            .unwrap_or("default"),
    );
    let ha_url = settings.get("ha_url").map(|s| s.as_str()).unwrap_or("");
    let ha_token_placeholder = secret_field_placeholder(
        secret_setting_configured(&settings, "ha_token"),
        "Paste Home Assistant long-lived token",
    );
    let pve_api_token_placeholder = secret_field_placeholder(
        secret_setting_configured(&settings, "pve_api_token"),
        "PVEAPIToken=root@pam!tokenid=xxxx-xxxx-xxxx-xxxx",
    );
    let csrf_token = csrf_token_for_session(&headers, &state.sessions);
    let root_css = build_root_css(&branding);

    let settings_tmpl = include_str!("../../../ui/templates/settings.html");
    let username = session.username.as_str();
    let app_version = option_env!("GIT_TAG").unwrap_or(env!("CARGO_PKG_VERSION"));
    let proxmox_enabled = settings
        .get("enable_proxmox")
        .map(|s| s.as_str())
        .unwrap_or("0");
    let feeds_enabled_flag = crate::settings::feeds_enabled(&settings);

    let mut result = settings_tmpl
        .replace("/* ROOT_CSS */", &root_css)
        .replace("{{app_name}}", app_name)
        .replace("{{app_version}}", app_version)
        .replace(
            "{{proxmox_enabled}}",
            if proxmox_enabled == "1" {
                "true"
            } else {
                "false"
            },
        )
        .replace(
            "{{feeds_enabled}}",
            if feeds_enabled_flag { "true" } else { "false" },
        )
        .replace("{{tagline}}", tagline)
        .replace("{{custom_bg_url}}", custom_bg_url);

    result = apply_app_logo_template(result, app_logo, app_name);
    result = result.replace("{{app_logo}}", &escape_html(app_logo));

    result = result
        .replace("{{accent_color}}", accent_color)
        .replace("{{glass_blur_intensity}}", glass_blur)
        .replace("{{glass_opacity}}", glass_opacity)
        .replace(
            "{{wallpaper_overlay_strength}}",
            branding.wallpaper_overlay_strength.as_str(),
        )
        .replace("{{bento_radius}}", bento_radius)
        .replace(
            "{{eq_grid_2}}",
            crate::templates::theme_eq_attr(grid_columns, "2"),
        )
        .replace(
            "{{eq_grid_3}}",
            crate::templates::theme_eq_attr(grid_columns, "3"),
        )
        .replace(
            "{{eq_grid_4}}",
            crate::templates::theme_eq_attr(grid_columns, "4"),
        )
        .replace(
            "{{eq_grid_5}}",
            crate::templates::theme_eq_attr(grid_columns, "5"),
        )
        .replace("{{weather_latitude}}", weather_latitude)
        .replace("{{weather_longitude}}", weather_longitude)
        .replace(
            "{{eq_weather_temp_celsius}}",
            crate::templates::theme_eq_attr(weather_temp_unit, "celsius"),
        )
        .replace(
            "{{eq_weather_temp_fahrenheit}}",
            crate::templates::theme_eq_attr(weather_temp_unit, "fahrenheit"),
        )
        .replace("{{timezone_options}}", &timezone_options_html)
        .replace(
            "{{eq_clock_12h}}",
            crate::templates::theme_eq_attr(&clock_time_format, "12h"),
        )
        .replace(
            "{{eq_clock_24h}}",
            crate::templates::theme_eq_attr(&clock_time_format, "24h"),
        )
        .replace(
            "{{custom_search_engines}}",
            &escape_html(custom_search_engines),
        )
        .replace("{{default_search_options}}", &default_search_options_html)
        .replace(
            "{{default_search_engine}}",
            &escape_html(&default_search_engine),
        )
        .replace(
            "{{pve_api_token_placeholder}}",
            &escape_html(&pve_api_token_placeholder),
        )
        .replace("{{ha_url}}", ha_url)
        .replace(
            "{{ha_token_placeholder}}",
            &escape_html(&ha_token_placeholder),
        )
        .replace("{{custom_css}}", custom_css)
        .replace("{{active_theme_id}}", &escape_html(&active_theme_id))
        .replace("{{csrf_token}}", &csrf_token)
        .replace("{{username}}", &escape_html(username))
        .replace(
            "{{eq_donate_on}}",
            if donate_enabled == "1" {
                "selected"
            } else {
                ""
            },
        )
        .replace(
            "{{eq_donate_off}}",
            if donate_enabled != "1" {
                "selected"
            } else {
                ""
            },
        );
    let telemetry_public = settings
        .get("telemetry_public")
        .map(|s| s.as_str())
        .unwrap_or("0");
    let accept_invalid = settings
        .get("accept_invalid_certs")
        .map(|s| s.as_str())
        .unwrap_or("0");
    let webhooks_allow_private = settings
        .get("webhooks_allow_private_ips")
        .map(|s| s.as_str())
        .unwrap_or("0");
    let last_backup_export_at = settings
        .get("last_backup_export_at")
        .cloned()
        .unwrap_or_else(|| "Never".to_string());
    let backup_overdue_banner = if crate::settings::backup_export_overdue(&settings) {
        r#"<div class="backup-reminder-banner" style="margin-bottom:1rem;padding:0.75rem 1rem;border-radius:8px;background:rgba(255,180,0,0.12);border:1px solid rgba(255,180,0,0.35);font-size:0.85rem;"><strong>Backup reminder:</strong> No recent database export. Download a backup below and store <code>amud.db</code> with <code>.amud-secrets-key</code> safely.</div>"#.to_string()
    } else {
        String::new()
    };
    let alert_cpu_threshold = settings
        .get("alert_cpu_threshold")
        .map(|s| s.as_str())
        .unwrap_or("90");
    let alert_ram_threshold = settings
        .get("alert_ram_threshold")
        .map(|s| s.as_str())
        .unwrap_or("90");
    let alert_disk_threshold = settings
        .get("alert_disk_threshold")
        .map(|s| s.as_str())
        .unwrap_or("95");
    let backup_reminder_days = settings
        .get("backup_reminder_days")
        .map(|s| s.as_str())
        .unwrap_or("30");
    result = result
        .replace(
            "{{eq_telemetry_on}}",
            if telemetry_public == "1" {
                "selected"
            } else {
                ""
            },
        )
        .replace(
            "{{eq_telemetry_off}}",
            if telemetry_public != "1" {
                "selected"
            } else {
                ""
            },
        )
        .replace(
            "{{eq_accept_invalid_certs_on}}",
            if accept_invalid == "1" {
                "selected"
            } else {
                ""
            },
        )
        .replace(
            "{{eq_accept_invalid_certs_off}}",
            if accept_invalid != "1" {
                "selected"
            } else {
                ""
            },
        )
        .replace(
            "{{eq_theme_dark}}",
            crate::templates::theme_eq_attr(&branding.theme_mode, "dark"),
        )
        .replace(
            "{{eq_theme_light}}",
            crate::templates::theme_eq_attr(&branding.theme_mode, "light"),
        )
        .replace("{{theme_mode}}", &escape_html(&branding.theme_mode))
        .replace(
            "{{eq_enable_proxmox_on}}",
            if proxmox_enabled == "1" {
                "selected"
            } else {
                ""
            },
        )
        .replace(
            "{{eq_enable_proxmox_off}}",
            if proxmox_enabled != "1" {
                "selected"
            } else {
                ""
            },
        )
        .replace(
            "{{eq_webhooks_private_on}}",
            if webhooks_allow_private == "1" {
                "selected"
            } else {
                ""
            },
        )
        .replace(
            "{{eq_webhooks_private_off}}",
            if webhooks_allow_private != "1" {
                "selected"
            } else {
                ""
            },
        )
        .replace(
            "{{last_backup_export_at}}",
            &escape_html(&last_backup_export_at),
        )
        .replace("{{backup_overdue_banner}}", &backup_overdue_banner)
        .replace("{{alert_cpu_threshold}}", &escape_html(alert_cpu_threshold))
        .replace("{{alert_ram_threshold}}", &escape_html(alert_ram_threshold))
        .replace(
            "{{alert_disk_threshold}}",
            &escape_html(alert_disk_threshold),
        )
        .replace(
            "{{backup_reminder_days}}",
            &escape_html(backup_reminder_days),
        )
        .replace(
            "{{telemetry_external_ifaces}}",
            &escape_html(
                settings
                    .get("telemetry_external_ifaces")
                    .map(String::as_str)
                    .unwrap_or(""),
            ),
        )
        .replace(
            "{{telemetry_internal_ifaces}}",
            &escape_html(
                settings
                    .get("telemetry_internal_ifaces")
                    .map(String::as_str)
                    .unwrap_or(""),
            ),
        )
        .replace(
            "{{telemetry_disk_mounts}}",
            &escape_html(
                settings
                    .get("telemetry_disk_mounts")
                    .map(String::as_str)
                    .unwrap_or(""),
            ),
        )
        .replace(
            "{{integration_cache_ttl_secs}}",
            &escape_html(
                settings
                    .get("integration_cache_ttl_secs")
                    .map(String::as_str)
                    .unwrap_or("45"),
            ),
        )
        .replace(
            "{{integration_cache_max_entries}}",
            &escape_html(
                settings
                    .get("integration_cache_max_entries")
                    .map(String::as_str)
                    .unwrap_or("256"),
            ),
        )
        .replace(
            "{{agent_telemetry_interval_secs}}",
            &escape_html(
                settings
                    .get("agent_telemetry_interval_secs")
                    .map(String::as_str)
                    .unwrap_or("5"),
            ),
        )
        .replace(
            "{{agent_lxc_poll_interval_secs}}",
            &escape_html(
                settings
                    .get("agent_lxc_poll_interval_secs")
                    .map(String::as_str)
                    .unwrap_or("10"),
            ),
        )
        .replace(
            "{{agent_docker_poll_interval_secs}}",
            &escape_html(
                settings
                    .get("agent_docker_poll_interval_secs")
                    .map(String::as_str)
                    .unwrap_or("10"),
            ),
        )
        .replace(
            "{{status_poll_interval_secs}}",
            &escape_html(
                settings
                    .get("status_poll_interval_secs")
                    .map(String::as_str)
                    .unwrap_or("15"),
            ),
        )
        .replace(
            "{{media_poll_interval_secs}}",
            &escape_html(
                settings
                    .get("media_poll_interval_secs")
                    .map(String::as_str)
                    .unwrap_or("5"),
            ),
        )
        .replace(
            "{{ha_poll_interval_secs}}",
            &escape_html(
                settings
                    .get("ha_poll_interval_secs")
                    .map(String::as_str)
                    .unwrap_or("15"),
            ),
        )
        .replace(
            "{{telemetry_broadcast_interval_secs}}",
            &escape_html(
                settings
                    .get("telemetry_broadcast_interval_secs")
                    .map(String::as_str)
                    .unwrap_or("5"),
            ),
        )
        .replace(
            "{{integration_coordinator_interval_secs}}",
            &escape_html(
                settings
                    .get("integration_coordinator_interval_secs")
                    .map(String::as_str)
                    .unwrap_or("45"),
            ),
        )
        .replace(
            "{{eq_feeds_enabled_on}}",
            if crate::settings::feeds_enabled(&settings) {
                "selected"
            } else {
                ""
            },
        )
        .replace(
            "{{eq_feeds_enabled_off}}",
            if !crate::settings::feeds_enabled(&settings) {
                "selected"
            } else {
                ""
            },
        );

    let perf_preset = settings
        .get("performance_preset")
        .map(|s| s.as_str())
        .unwrap_or("light");
    result = result
        .replace(
            "{{idle_grace_secs}}",
            &escape_html(
                settings
                    .get("idle_grace_secs")
                    .map(String::as_str)
                    .unwrap_or("45"),
            ),
        )
        .replace(
            "{{agent_node_tag}}",
            &escape_html(
                settings
                    .get("agent_node_tag")
                    .map(String::as_str)
                    .unwrap_or("Local"),
            ),
        )
        .replace(
            "{{eq_performance_preset_light}}",
            if perf_preset == "light" {
                "checked"
            } else {
                ""
            },
        )
        .replace(
            "{{eq_performance_preset_balanced}}",
            if perf_preset == "balanced" {
                "checked"
            } else {
                ""
            },
        )
        .replace(
            "{{eq_performance_preset_active}}",
            if perf_preset == "active" {
                "checked"
            } else {
                ""
            },
        )
        .replace(
            "{{eq_performance_preset_custom}}",
            if perf_preset == "custom" {
                "checked"
            } else {
                ""
            },
        )
        .replace(
            "{{last_version_change_at}}",
            &escape_html(
                settings
                    .get("last_version_change_at")
                    .map(String::as_str)
                    .unwrap_or("Never"),
            ),
        )
        .replace(
            "{{eq_webgl_effects_on}}",
            if settings
                .get("webgl_effects_enabled")
                .map(|s| s.as_str() != "0")
                .unwrap_or(true)
            {
                "selected"
            } else {
                ""
            },
        )
        .replace(
            "{{eq_webgl_effects_off}}",
            if settings
                .get("webgl_effects_enabled")
                .map(|s| s.as_str() == "0")
                .unwrap_or(false)
            {
                "selected"
            } else {
                ""
            },
        )
        .replace(
            "{{eq_greeting_animations_on}}",
            if settings
                .get("greeting_animations_enabled")
                .map(|s| s.as_str() != "0")
                .unwrap_or(true)
            {
                "selected"
            } else {
                ""
            },
        )
        .replace(
            "{{eq_greeting_animations_off}}",
            if settings
                .get("greeting_animations_enabled")
                .map(|s| s.as_str() == "0")
                .unwrap_or(false)
            {
                "selected"
            } else {
                ""
            },
        )
        .replace(
            "{{eq_dashboard_reorder_on}}",
            if settings
                .get("dashboard_reorder_enabled")
                .map(|s| s.as_str() != "0")
                .unwrap_or(true)
            {
                "selected"
            } else {
                ""
            },
        )
        .replace(
            "{{eq_dashboard_reorder_off}}",
            if settings
                .get("dashboard_reorder_enabled")
                .map(|s| s.as_str() == "0")
                .unwrap_or(false)
            {
                "selected"
            } else {
                ""
            },
        );

    let db_categories = with_db(state.db.clone(), load_categories).await;
    let guest_category_restrict = settings
        .get("guest_category_restrict")
        .map(|s| s.as_str())
        .unwrap_or("0");
    let guest_visible_categories = settings
        .get("guest_visible_categories")
        .map(|s| s.as_str())
        .unwrap_or("");
    let theme_scheduler = sanitize_theme_scheduler(
        settings
            .get("theme_scheduler")
            .map(|s| s.as_str())
            .unwrap_or("off"),
    );
    let theme_light_at = sanitize_time_hhmm(
        settings
            .get("theme_light_at")
            .map(|s| s.as_str())
            .unwrap_or("07:00"),
        "07:00",
    );
    let theme_dark_at = sanitize_time_hhmm(
        settings
            .get("theme_dark_at")
            .map(|s| s.as_str())
            .unwrap_or("19:00"),
        "19:00",
    );
    let theme_scheduler_config = build_theme_scheduler_json(&settings, &branding.theme_mode);
    let guest_category_controls = render_guest_category_controls(
        &db_categories,
        guest_category_restrict == "1",
        guest_visible_categories,
    );

    let result = result
        .replace("{{theme_scheduler_config}}", &theme_scheduler_config)
        .replace("{{theme_light_at}}", &escape_html(&theme_light_at))
        .replace("{{theme_dark_at}}", &escape_html(&theme_dark_at))
        .replace(
            "{{eq_theme_scheduler_off}}",
            if theme_scheduler == "off" {
                "selected"
            } else {
                ""
            },
        )
        .replace(
            "{{eq_theme_scheduler_sunrise}}",
            if theme_scheduler == "sunrise_sunset" {
                "selected"
            } else {
                ""
            },
        )
        .replace(
            "{{eq_theme_scheduler_manual}}",
            if theme_scheduler == "manual" {
                "selected"
            } else {
                ""
            },
        )
        .replace(
            "{{eq_guest_category_restrict_on}}",
            if guest_category_restrict == "1" {
                "selected"
            } else {
                ""
            },
        )
        .replace(
            "{{eq_guest_category_restrict_off}}",
            if guest_category_restrict != "1" {
                "selected"
            } else {
                ""
            },
        )
        .replace("{{guest_category_controls}}", &guest_category_controls);

    let dashboard_layout = sanitize_dashboard_layout(
        settings
            .get("dashboard_layout")
            .map(|s| s.as_str())
            .unwrap_or("tabs"),
    );
    let oidc_enabled = settings
        .get("oidc_enabled")
        .map(|s| s.as_str())
        .unwrap_or("0");
    let oidc_issuer = settings
        .get("oidc_issuer")
        .map(|s| s.as_str())
        .unwrap_or("");
    let oidc_client_id = settings
        .get("oidc_client_id")
        .map(|s| s.as_str())
        .unwrap_or("");
    let oidc_redirect_uri = settings
        .get("oidc_redirect_uri")
        .map(|s| s.as_str())
        .unwrap_or("");
    let oidc_admin_group = settings
        .get("oidc_admin_group")
        .map(|s| s.as_str())
        .unwrap_or("");
    let ldap_enabled = settings
        .get("ldap_enabled")
        .map(|s| s.as_str())
        .unwrap_or("0");
    let ldap_url = settings.get("ldap_url").map(|s| s.as_str()).unwrap_or("");
    let ldap_bind_dn = settings
        .get("ldap_bind_dn")
        .map(|s| s.as_str())
        .unwrap_or("");
    let ldap_base_dn = settings
        .get("ldap_base_dn")
        .map(|s| s.as_str())
        .unwrap_or("");
    let ldap_user_filter = settings
        .get("ldap_user_filter")
        .map(|s| s.as_str())
        .unwrap_or("(uid={username})");
    let kiosk_mode = settings
        .get("kiosk_mode")
        .map(|s| s.as_str())
        .unwrap_or("0");
    let status_page_public = settings
        .get("status_page_public")
        .map(|s| s.as_str())
        .unwrap_or("0");
    let iframe_embeds_enabled = settings
        .get("iframe_embeds_enabled")
        .map(|s| s.as_str())
        .unwrap_or("0");

    let result = result
        .replace(
            "{{eq_dashboard_layout_tabs}}",
            if dashboard_layout == "tabs" {
                "selected"
            } else {
                ""
            },
        )
        .replace(
            "{{eq_dashboard_layout_sections}}",
            if dashboard_layout == "sections" {
                "selected"
            } else {
                ""
            },
        )
        .replace("{{oidc_issuer}}", &escape_html(oidc_issuer))
        .replace("{{oidc_client_id}}", &escape_html(oidc_client_id))
        .replace("{{oidc_redirect_uri}}", &escape_html(oidc_redirect_uri))
        .replace("{{oidc_admin_group}}", &escape_html(oidc_admin_group))
        .replace("{{ldap_url}}", &escape_html(ldap_url))
        .replace("{{ldap_bind_dn}}", &escape_html(ldap_bind_dn))
        .replace("{{ldap_base_dn}}", &escape_html(ldap_base_dn))
        .replace("{{ldap_user_filter}}", &escape_html(ldap_user_filter))
        .replace(
            "{{eq_ldap_enabled_on}}",
            if ldap_enabled == "1" { "selected" } else { "" },
        )
        .replace(
            "{{eq_ldap_enabled_off}}",
            if ldap_enabled != "1" { "selected" } else { "" },
        )
        .replace(
            "{{eq_oidc_enabled_on}}",
            if oidc_enabled == "1" { "selected" } else { "" },
        )
        .replace(
            "{{eq_oidc_enabled_off}}",
            if oidc_enabled != "1" { "selected" } else { "" },
        )
        .replace(
            "{{eq_kiosk_on}}",
            if kiosk_mode == "1" { "selected" } else { "" },
        )
        .replace(
            "{{eq_kiosk_off}}",
            if kiosk_mode != "1" { "selected" } else { "" },
        )
        .replace(
            "{{eq_status_public_on}}",
            if status_page_public == "1" {
                "selected"
            } else {
                ""
            },
        )
        .replace(
            "{{eq_status_public_off}}",
            if status_page_public != "1" {
                "selected"
            } else {
                ""
            },
        )
        .replace(
            "{{eq_iframe_embeds_on}}",
            if iframe_embeds_enabled == "1" {
                "selected"
            } else {
                ""
            },
        )
        .replace(
            "{{eq_iframe_embeds_off}}",
            if iframe_embeds_enabled != "1" {
                "selected"
            } else {
                ""
            },
        );

    let result = apply_branding_head(result, &branding);

    Html(apply_csp_nonce(result, &csp.0)).into_response()
}

pub async fn manifest_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let settings = state.settings_cache.read().unwrap().clone();
    let branding = branding_from_settings(&settings);
    let body = crate::templates::build_web_manifest_json(&branding);
    (
        [
            (header::CONTENT_TYPE, "application/manifest+json"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        body,
    )
}

fn render_guest_category_controls(
    categories: &[(i64, String)],
    restrict: bool,
    visible: &str,
) -> String {
    let visible_set: std::collections::HashSet<&str> = visible
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    let mut checkboxes = String::new();
    if categories.is_empty() {
        checkboxes.push_str(
            r#"<p style="font-size:0.78rem; color:var(--text-muted);">No categories defined yet. Add categories under Management → Categories.</p>"#,
        );
    } else {
        checkboxes.push_str(
            r#"<div style="display:flex; flex-direction:column; gap:0.5rem; margin-top:0.75rem;">"#,
        );
        for (_id, name) in categories {
            let checked = if !restrict || visible_set.contains(name.as_str()) {
                "checked"
            } else {
                ""
            };
            checkboxes.push_str(&format!(
                r#"<label style="display:flex; align-items:center; gap:0.5rem; font-size:0.9rem; cursor:pointer;"><input type="checkbox" class="guest-category-checkbox" data-category="{}" {}> {}</label>"#,
                escape_html(name),
                checked,
                escape_html(name)
            ));
        }
        checkboxes.push_str("</div>");
    }

    format!(
        r#"<div class="form-group">
            <label for="setting-guest-category-restrict">Guest category tabs</label>
            <select id="setting-guest-category-restrict" name="guest_category_restrict" class="form-control" style="height:3rem; cursor:pointer;">
                <option value="0" {}>Show all category tabs to guests</option>
                <option value="1" {}>Restrict to selected categories only</option>
            </select>
            <p style="font-size:0.78rem; color:var(--text-muted); margin-top:0.45rem;">Applies to anonymous visitors and Guest accounts on the dashboard and feeds pages. Hidden categories also hide their apps.</p>
            <input type="hidden" name="guest_visible_categories" id="guest-visible-categories-input" value="{}">
            <div id="guest-category-picker" style="margin-top:1rem;">
                <div style="font-size:0.85rem; font-weight:600; margin-bottom:0.25rem;">Visible categories when restricted</div>
                {}
            </div>
        </div>"#,
        if restrict { "" } else { "selected" },
        if restrict { "selected" } else { "" },
        escape_html(visible),
        checkboxes
    )
}
