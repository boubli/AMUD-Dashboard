use super::imports::*;
use super::share::share_session_from_headers;
use crate::models::App;

#[derive(Clone, Copy)]
enum PageMode {
    Dashboard,
    Feeds,
}

pub async fn dashboard_handler(
    Extension(csp): Extension<CspNonce>,
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    render_page(PageMode::Dashboard, &csp.0, &headers, &state).await
}

pub async fn feeds_page_handler(
    Extension(csp): Extension<CspNonce>,
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    {
        let settings = state.settings_cache.read().unwrap();
        if !crate::settings::feeds_enabled(&settings) {
            return Redirect::to("/").into_response();
        }
    }
    render_page(PageMode::Feeds, &csp.0, &headers, &state)
        .await
        .into_response()
}

async fn render_page(
    mode: PageMode,
    nonce: &str,
    headers: &HeaderMap,
    state: &Arc<AppState>,
) -> Html<String> {
    let session = get_session(headers, &state.sessions);
    if session.is_some() {
        crate::activity::signal_gui_session_start(state);
    }
    let share_session = share_session_from_headers(headers, state);

    let settings = state.settings_cache.read().unwrap().clone();

    let mode_path = match mode {
        PageMode::Dashboard => "/",
        PageMode::Feeds => "/feeds",
    };
    if let Some(share) = &share_session {
        if !share
            .allowed_paths
            .split(',')
            .map(str::trim)
            .any(|p| p == mode_path)
        {
            return Html("<html><body><p>Share link does not allow this page.</p><a href=\"/\">Home</a></body></html>".to_string());
        }
    }

    let iframe_embeds_enabled = settings
        .get("iframe_embeds_enabled")
        .map(|s| s.as_str())
        .unwrap_or("0")
        == "1";
    let branding = branding_from_settings(&settings);
    let tagline = branding
        .tagline
        .as_deref()
        .unwrap_or("Homelab Operations Cockpit");
    let weather_lat = settings
        .get("weather_latitude")
        .map(|s| s.as_str())
        .unwrap_or("");
    let weather_lon = settings
        .get("weather_longitude")
        .map(|s| s.as_str())
        .unwrap_or("");
    let weather_temp_unit = settings
        .get("weather_temp_unit")
        .map(|s| s.as_str())
        .unwrap_or("celsius");
    let clock_config_json = crate::settings::build_clock_config_json(&settings);
    let search_config_json = crate::settings::build_search_engines_json(&settings);
    let search_engine_options_html = crate::settings::build_search_engine_options_html(&settings);
    let is_admin = session.as_ref().map(|s| s.role == "Admin").unwrap_or(false);
    let telemetry_public = settings
        .get("telemetry_public")
        .map(|s| s.as_str())
        .unwrap_or("0");
    let hide_telemetry = match mode {
        PageMode::Feeds => true,
        PageMode::Dashboard => match &session {
            None => telemetry_public != "1",
            Some(s) if s.role == "Guest" => telemetry_public != "1",
            _ => false,
        },
    };
    let logo_manifest = state.logo_manifest.clone();
    let custom_css = settings.get("custom_css").map(|s| s.as_str()).unwrap_or("");
    let active_theme_id = sanitize_active_theme_id(
        settings
            .get("active_theme_id")
            .map(|s| s.as_str())
            .unwrap_or("default"),
    );
    let safe_active_theme_id = escape_html(&active_theme_id);
    let csrf_token = csrf_token_for_session(headers, &state.sessions);
    let csrf_attr = escape_html(&csrf_token);

    let (db_categories, all_apps, total_apps, wol_devices) = with_db(state.db.clone(), move |db| {
        let wol = match mode {
            PageMode::Dashboard => load_wol_devices_from_db(db),
            PageMode::Feeds => vec![],
        };
        let (apps, total) = match mode {
            PageMode::Dashboard => (
                crate::db::load_dashboard_apps_page(db, 0, 50),
                crate::db::count_dashboard_apps(db),
            ),
            PageMode::Feeds => (
                crate::db::load_rss_apps_page(db, 0, 50),
                crate::db::count_rss_apps(db),
            ),
        };
        (load_categories(db), apps, total, wol)
    })
    .await;

    let mut apps: Vec<App> = all_apps;

    let is_guest = !is_admin && is_guest_session(&session);
    if is_guest {
        if let Some(allowed) = parse_guest_visible_categories(&settings) {
            apps.retain(|app| {
                let cat = if app.category.is_empty() {
                    "General"
                } else {
                    app.category.as_str()
                };
                allowed.contains(cat)
            });
        }
        apps.retain(|app| app.guest_visible);
    }

    let mut category_options_html = String::new();
    for (_id, cat_name) in &db_categories {
        category_options_html.push_str(&format!(
            r#"<option value="{}">{}</option>"#,
            escape_html(cat_name),
            escape_html(cat_name)
        ));
    }
    if category_options_html.is_empty() {
        category_options_html = r#"<option value="General">General</option>"#.to_string();
    }
    let app_names_json = serde_json::to_string(
        &apps
            .iter()
            .map(|app| app.name.to_lowercase())
            .collect::<Vec<_>>(),
    )
    .unwrap_or_else(|_| "[]".to_string());

    let feed_categories = match mode {
        PageMode::Feeds => with_db(state.db.clone(), load_feed_categories_json).await,
        PageMode::Dashboard => Vec::new(),
    };

    let apps_html = match mode {
        PageMode::Feeds => render_feeds_grid(
            &apps,
            is_admin,
            &csrf_attr,
            &logo_manifest,
            &feed_categories,
        ),
        PageMode::Dashboard => {
            // Last known in-memory state, embedded into the SSR HTML so a page
            // reload shows statuses/metrics instantly instead of placeholders.
            let known_statuses = state.app_statuses.read().unwrap().clone();
            let known_containers = state
                .latest_telemetry
                .read()
                .unwrap()
                .lxc_containers
                .clone();
            render_apps_grid(
                &apps,
                is_admin,
                &csrf_token,
                &csrf_attr,
                &logo_manifest,
                iframe_embeds_enabled,
                &known_statuses,
                &known_containers,
                active_theme_id.as_str(),
            )
        }
    };

    let wol_html = render_wol_devices(&wol_devices, is_admin, &csrf_attr);

    let kiosk_mode = settings
        .get("kiosk_mode")
        .map(|s| s.as_str())
        .unwrap_or("0")
        == "1";
    let auth_buttons = render_auth_buttons(&session, &csrf_attr, mode, kiosk_mode);
    let streams_html = match mode {
        PageMode::Dashboard => render_streams(
            &apps,
            &session,
            is_admin,
            &csrf_attr,
            logo_manifest.as_ref(),
        ),
        PageMode::Feeds => String::new(),
    };
    let category_tabs_html = match mode {
        PageMode::Feeds => render_feed_category_tabs(&apps, &feed_categories),
        PageMode::Dashboard => {
            let layout = sanitize_dashboard_layout(
                settings
                    .get("dashboard_layout")
                    .map(|s| s.as_str())
                    .unwrap_or("tabs"),
            );
            if layout == "sections" {
                render_category_sections(&apps)
            } else {
                render_category_tabs(&apps)
            }
        }
    };
    let widgets_html = match mode {
        PageMode::Dashboard => {
            let widgets = with_db(state.db.clone(), load_dashboard_widgets).await;
            super::widgets::render_dashboard_widgets(&widgets, is_guest).await
        }
        PageMode::Feeds => String::new(),
    };
    let support_html = match mode {
        PageMode::Dashboard => render_support_section(&settings),
        PageMode::Feeds => String::new(),
    };
    let feed_hero_html = match mode {
        PageMode::Feeds if !apps.is_empty() => {
            r#"<section id="feed-hero" class="glass-panel feed-hero" hidden aria-live="polite"></section>"#
                .to_string()
        }
        _ => String::new(),
    };

    let root_css = build_root_css(&branding);

    let index_tmpl = include_str!("../../../ui/templates/index.html");
    let username = session
        .as_ref()
        .map(|s| s.username.as_str())
        .unwrap_or("guest");
    let proxmox_enabled = settings
        .get("enable_proxmox")
        .map(|s| s.as_str())
        .unwrap_or("0")
        == "1";

    let app_version = option_env!("GIT_TAG").unwrap_or(env!("CARGO_PKG_VERSION"));
    let app_version_formatted = if app_version.starts_with('v') {
        app_version.to_string()
    } else {
        format!("v{}", app_version)
    };

    let mut update_banner = String::new();
    let mut update_status_class = String::new();

    if is_admin {
        let cache = {
            let lock = crate::handlers::RELEASE_CACHE.read().unwrap();
            lock.clone()
        };

        if let Some(cached) = cache {
            if crate::handlers::semver_newer(&app_version_formatted, &cached.latest) {
                update_status_class = "update-available".to_string();
                update_banner = format!(
                    r#"<div id="update-banner" class="update-banner">
    <div class="update-banner-content">
        <span class="update-banner-dot animate-pulse"></span>
        <span class="update-banner-text">A new update is available! You are running <strong>{}</strong>, latest is <strong>{}</strong>.</span>
    </div>
    <div class="update-banner-actions">
        <a href="/admin/settings?tab=system" class="btn-update-banner">Go to System &rarr;</a>
        <button type="button" onclick="dismissUpdateBanner()" class="btn-update-dismiss">&times;</button>
    </div>
</div>"#,
                    escape_html(&app_version_formatted),
                    escape_html(&cached.latest)
                );
            }
        }
    }

    let safe_app_name = escape_html(&branding.app_name);
    let safe_tagline = escape_html(tagline);
    let safe_accent = branding.accent_color.clone();
    let safe_glass_blur = branding
        .glass_blur
        .parse::<u16>()
        .map(|n| n.to_string())
        .unwrap_or_else(|_| "16".to_string());
    let safe_glass_opacity = branding
        .glass_opacity
        .parse::<f64>()
        .map(|n| n.to_string())
        .unwrap_or_else(|_| "0.45".to_string());
    let safe_bento_radius = branding
        .bento_radius
        .parse::<u16>()
        .map(|n| n.to_string())
        .unwrap_or_else(|_| "16".to_string());
    let safe_weather_lat = safe_weather_coord(weather_lat);
    let safe_weather_lon = safe_weather_coord(weather_lon);
    let safe_csrf_meta = escape_html(&csrf_token);
    let safe_app_version = escape_html(&app_version_formatted);

    // Page mode variables
    let page_title_suffix = match mode {
        PageMode::Dashboard => "DASHBOARD",
        PageMode::Feeds => "FEEDS",
    };
    let feeds_on = crate::settings::feeds_enabled(&settings);
    let feeds_nav = if !feeds_on {
        String::new()
    } else {
        match mode {
            PageMode::Dashboard => {
                r#"<a href="/feeds" class="glass-panel topbar-action"><i data-lucide="rss"></i> Feeds</a>"#
                    .to_string()
            }
            PageMode::Feeds => {
                r#"<a href="/" class="glass-panel topbar-action"><i data-lucide="arrow-left"></i> Dashboard</a>"#
                    .to_string()
            }
        }
    };
    let main_grid_class = match mode {
        PageMode::Feeds => "feeds-grid",
        PageMode::Dashboard if !is_admin => "bento-grid bento-grid--guest-compact",
        PageMode::Dashboard => "bento-grid",
    };
    let body_page_class = match mode {
        PageMode::Feeds => "page-feeds",
        PageMode::Dashboard => "",
    };
    let webgl_preload_script = if active_theme_id == "taghawsa"
        && settings
            .get("webgl_effects_enabled")
            .map(|s| s.as_str() != "0")
            .unwrap_or(true)
    {
        format!(
            r#"<link rel="preload" href="/static/vendor/three.min.js?v={ver}" as="script">
    <script src="/static/vendor/three.min.js?v={ver}"></script>"#,
            ver = safe_app_version
        )
    } else {
        String::new()
    };

    let mut result = index_tmpl
        .replace("/* ROOT_CSS */", &root_css)
        .replace("{{app_name}}", &safe_app_name)
        .replace("{{tagline}}", &safe_tagline)
        .replace("{{page_title_suffix}}", page_title_suffix)
        .replace("<!-- FEEDS_NAV -->", &feeds_nav)
        .replace("{{main_grid_class}}", main_grid_class)
        .replace("{{body_page_class}}", body_page_class)
        .replace(
            "{{proxmox_enabled}}",
            if proxmox_enabled { "true" } else { "false" },
        )
        .replace("{{custom_bg_url}}", &safe_css_url(&branding.custom_bg_url));

    result = apply_app_logo_template(result, &branding.app_logo, &branding.app_name);

    let result = result
        .replace("{{accent_color}}", &safe_accent)
        .replace("{{glass_blur_intensity}}", &safe_glass_blur)
        .replace("{{glass_opacity}}", &safe_glass_opacity)
        .replace("{{bento_radius}}", &safe_bento_radius)
        .replace("<!-- APPS_GRID -->", &apps_html)
        .replace("<!-- WOL_SECTION -->", &wol_html)
        .replace("<!-- STREAMS_ROW -->", &streams_html)
        .replace("<!-- FEED_HERO -->", &feed_hero_html)
        .replace("<!-- CATEGORY_TABS -->", &category_tabs_html)
        .replace("<!-- DASHBOARD_WIDGETS -->", &widgets_html)
        .replace("<!-- SUPPORT_SECTION -->", &support_html)
        .replace("<!-- AUTH_BUTTONS -->", &auth_buttons)
        .replace("{{username}}", &escape_html(username))
        .replace("{{weather_latitude}}", &safe_weather_lat)
        .replace("{{weather_longitude}}", &safe_weather_lon)
        .replace("{{weather_temp_unit}}", &escape_html(weather_temp_unit))
        .replace("{{clock_config_json}}", &clock_config_json)
        .replace("{{search_config_json}}", &search_config_json)
        .replace("<!-- SEARCH_ENGINE_OPTIONS -->", &search_engine_options_html)
        .replace("<!-- CATEGORY_OPTIONS -->", &category_options_html)
        .replace("{{csrf_token}}", &safe_csrf_meta)
        .replace("{{telemetry_public}}", telemetry_public)
        .replace(
            "{{hide_telemetry}}",
            if hide_telemetry { "true" } else { "false" },
        )
        .replace("{{total_apps}}", &total_apps.to_string())
        .replace("{{custom_css}}", custom_css)
        .replace("{{app_names_json}}", &app_names_json)
        .replace("{{is_admin}}", if is_admin { "true" } else { "false" })
        .replace(
            "{{webgl_effects_enabled}}",
            if settings
                .get("webgl_effects_enabled")
                .map(|s| s.as_str() != "0")
                .unwrap_or(true)
            {
                "1"
            } else {
                "0"
            },
        )
        .replace(
            "{{greeting_animations_enabled}}",
            if settings
                .get("greeting_animations_enabled")
                .map(|s| s.as_str() != "0")
                .unwrap_or(true)
            {
                "1"
            } else {
                "0"
            },
        )
        .replace("{{webgl_preload_script}}", &webgl_preload_script)
        .replace(
            "{{admin_drag_script}}",
            if is_admin
                && settings
                    .get("dashboard_reorder_enabled")
                    .map(|s| s.as_str() != "0")
                    .unwrap_or(true)
            {
                r#"<script src="/static/drag.js"></script>"#
            } else {
                ""
            },
        )
        .replace("{{app_version}}", &safe_app_version)
        .replace(
            "{{update_status_class}}",
            &escape_html(&update_status_class),
        )
        .replace("{{update_banner}}", &update_banner);

    // Theme mode (light/dark)
    let theme_mode = &branding.theme_mode;
    let theme_scheduler_config = build_theme_scheduler_json(&settings, theme_mode);
    let result = result
        .replace("{{theme_mode}}", &escape_html(theme_mode))
        .replace("{{active_theme_id}}", &safe_active_theme_id)
        .replace("{{theme_scheduler_config}}", &theme_scheduler_config);

    // Video wallpaper support
    let bg_url = &branding.custom_bg_url;
    let is_video_bg = bg_url.ends_with(".mp4") || bg_url.ends_with(".webm");
    let video_bg_element = if is_video_bg {
        format!(
            r#"<video class="video-bg" autoplay muted loop playsinline><source src="{}" type="video/{}"></video>"#,
            escape_html(bg_url),
            if bg_url.ends_with(".webm") {
                "webm"
            } else {
                "mp4"
            }
        )
    } else {
        String::new()
    };
    let result = result
        .replace(
            "{{video_bg_class}}",
            if is_video_bg { "has-video-bg" } else { "" },
        )
        .replace("{{video_bg_element}}", &video_bg_element);

    let result = apply_branding_head(result, &branding);

    Html(apply_csp_nonce(result, nonce))
}

fn is_guest_session(session: &Option<Session>) -> bool {
    match session {
        None => true,
        Some(s) => s.role == "Guest",
    }
}

fn filled_cpu_ram_cells() -> &'static str {
    r#"<div class="metric-block" data-lxc-cpu><span class="metric-value">—</span><span class="metric-label">CPU</span></div><div class="metric-block" data-lxc-ram><span class="metric-value">—</span><span class="metric-label">RAM</span></div>"#
}

fn filled_loading_grid() -> &'static str {
    r#"<div class="metric-block"><span class="metric-value">—</span><span class="metric-label">…</span></div><div class="metric-block"><span class="metric-value">—</span><span class="metric-label">…</span></div><div class="metric-block"><span class="metric-value">—</span><span class="metric-label">…</span></div><div class="metric-block"><span class="metric-value">—</span><span class="metric-label">…</span></div><div class="metric-block"><span class="metric-value">—</span><span class="metric-label">…</span></div><div class="metric-block"><span class="metric-value">—</span><span class="metric-label">…</span></div><div class="metric-block"><span class="metric-value">—</span><span class="metric-label">…</span></div><div class="metric-block"><span class="metric-value">—</span><span class="metric-label">…</span></div>"#
}

fn build_filled_integration_widget(show_cpu_ram: bool) -> String {
    let cpu_ram = if show_cpu_ram {
        filled_cpu_ram_cells()
    } else {
        ""
    };
    format!(
        r#"
                <div class="integration-widget integration-widget--filled">
                    <div class="integration-metrics-grid app-card-metrics-fallback" x-show="!integrationData || !integrationData.type">{loading}</div>
                    <div x-show="integrationData && integrationData.type">
                    <template x-if="integrationData.type === 'pihole' || integrationData.type === 'adguard'">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block" x-show="metricVisible('ads_blocked_today')"><span class="metric-value" x-text="integrationData.ads_blocked_today ?? '—'"></span><span class="metric-label">Blocked</span></div>
                            <div class="metric-block" x-show="metricVisible('dns_queries_today')"><span class="metric-value" x-text="integrationData.dns_queries_today ?? '—'"></span><span class="metric-label">Queries</span></div>
                            <div class="metric-block" x-show="metricVisible('ads_percentage_today')"><span class="metric-value" x-text="integrationData.ads_percentage_today ?? integrationData.avg_processing_time ?? '—'"></span><span class="metric-label" x-text="integrationData.type === 'pihole' ? 'Block %' : 'Avg time'"></span></div>
                            <div class="metric-block" x-show="metricVisible('domains_being_blocked')"><span class="metric-value" x-text="integrationData.domains_being_blocked ?? integrationData.status ?? '—'"></span><span class="metric-label" x-text="integrationData.type === 'pihole' ? 'Domains' : 'Status'"></span></div>
                            <div class="metric-block" x-show="metricVisible('status')"><span class="metric-value" x-text="integrationData.status ?? '—'"></span><span class="metric-label">Status</span></div>
                            <div class="metric-block" x-show="metricVisible('unique_clients')"><span class="metric-value" x-text="integrationData.unique_clients ?? integrationData.block_pct ?? '—'"></span><span class="metric-label" x-text="integrationData.type === 'pihole' ? 'Clients' : 'Block %'"></span></div>
                            <div class="metric-block" x-show="metricVisible('gravity_updated')"><span class="metric-value" x-text="integrationData.gravity_updated ?? integrationData.dns_rewrites ?? '—'"></span><span class="metric-label" x-text="integrationData.type === 'pihole' ? 'Gravity' : 'Rewrites'"></span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'radarr'">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block" x-show="metricVisible('queue_size')"><span class="metric-value" x-text="integrationData.queue_size ?? '—'"></span><span class="metric-label">Queue</span></div>
                            <div class="metric-block" x-show="metricVisible('missing')"><span class="metric-value" x-text="integrationData.missing ?? '—'"></span><span class="metric-label">Missing</span></div>
                            <div class="metric-block" x-show="metricVisible('library_count')"><span class="metric-value" x-text="integrationData.library_count ?? '—'"></span><span class="metric-label">Movies</span></div>
                            <div class="metric-block" x-show="metricVisible('disk_free')"><span class="metric-value" x-text="integrationData.disk_free ?? '—'"></span><span class="metric-label">Disk free</span></div>
                            <div class="metric-block" x-show="metricVisible('version')"><span class="metric-value" x-text="integrationData.version ?? '—'"></span><span class="metric-label">Version</span></div>
                            <div class="metric-block" x-show="metricVisible('health')"><span class="metric-value" x-text="integrationData.health ?? '—'"></span><span class="metric-label">Health</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'sonarr'">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block" x-show="metricVisible('queue_size')"><span class="metric-value" x-text="integrationData.queue_size ?? '—'"></span><span class="metric-label">Queue</span></div>
                            <div class="metric-block" x-show="metricVisible('missing')"><span class="metric-value" x-text="integrationData.missing ?? '—'"></span><span class="metric-label">Missing</span></div>
                            <div class="metric-block" x-show="metricVisible('series_count')"><span class="metric-value" x-text="integrationData.series_count ?? '—'"></span><span class="metric-label">Series</span></div>
                            <div class="metric-block" x-show="metricVisible('episode_count')"><span class="metric-value" x-text="integrationData.episode_count ?? '—'"></span><span class="metric-label">Episodes</span></div>
                            <div class="metric-block" x-show="metricVisible('disk_free')"><span class="metric-value" x-text="integrationData.disk_free ?? '—'"></span><span class="metric-label">Disk free</span></div>
                            <div class="metric-block" x-show="metricVisible('version')"><span class="metric-value" x-text="integrationData.version ?? '—'"></span><span class="metric-label">Version</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'lidarr'">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block" x-show="metricVisible('queue_size')"><span class="metric-value" x-text="integrationData.queue_size ?? '—'"></span><span class="metric-label">Queue</span></div>
                            <div class="metric-block" x-show="metricVisible('missing')"><span class="metric-value" x-text="integrationData.missing ?? '—'"></span><span class="metric-label">Missing</span></div>
                            <div class="metric-block" x-show="metricVisible('library_count')"><span class="metric-value" x-text="integrationData.library_count ?? '—'"></span><span class="metric-label">Artists</span></div>
                            <div class="metric-block" x-show="metricVisible('album_count')"><span class="metric-value" x-text="integrationData.album_count ?? '—'"></span><span class="metric-label">Albums</span></div>
                            <div class="metric-block" x-show="metricVisible('disk_free')"><span class="metric-value" x-text="integrationData.disk_free ?? '—'"></span><span class="metric-label">Disk free</span></div>
                            <div class="metric-block" x-show="metricVisible('version')"><span class="metric-value" x-text="integrationData.version ?? '—'"></span><span class="metric-label">Version</span></div>
                            <div class="metric-block" x-show="metricVisible('health')"><span class="metric-value" x-text="integrationData.health ?? '—'"></span><span class="metric-label">Health</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'readarr'">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block" x-show="metricVisible('queue_size')"><span class="metric-value" x-text="integrationData.queue_size ?? '—'"></span><span class="metric-label">Queue</span></div>
                            <div class="metric-block" x-show="metricVisible('missing')"><span class="metric-value" x-text="integrationData.missing ?? '—'"></span><span class="metric-label">Missing</span></div>
                            <div class="metric-block" x-show="metricVisible('library_count')"><span class="metric-value" x-text="integrationData.library_count ?? '—'"></span><span class="metric-label">Books</span></div>
                            <div class="metric-block" x-show="metricVisible('author_count')"><span class="metric-value" x-text="integrationData.author_count ?? '—'"></span><span class="metric-label">Authors</span></div>
                            <div class="metric-block" x-show="metricVisible('disk_free')"><span class="metric-value" x-text="integrationData.disk_free ?? '—'"></span><span class="metric-label">Disk free</span></div>
                            <div class="metric-block" x-show="metricVisible('version')"><span class="metric-value" x-text="integrationData.version ?? '—'"></span><span class="metric-label">Version</span></div>
                            <div class="metric-block" x-show="metricVisible('health')"><span class="metric-value" x-text="integrationData.health ?? '—'"></span><span class="metric-label">Health</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'whisparr'">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block" x-show="metricVisible('queue_size')"><span class="metric-value" x-text="integrationData.queue_size ?? '—'"></span><span class="metric-label">Queue</span></div>
                            <div class="metric-block" x-show="metricVisible('missing')"><span class="metric-value" x-text="integrationData.missing ?? '—'"></span><span class="metric-label">Missing</span></div>
                            <div class="metric-block" x-show="metricVisible('series_count')"><span class="metric-value" x-text="integrationData.series_count ?? '—'"></span><span class="metric-label">Series</span></div>
                            <div class="metric-block" x-show="metricVisible('episode_count')"><span class="metric-value" x-text="integrationData.episode_count ?? '—'"></span><span class="metric-label">Episodes</span></div>
                            <div class="metric-block" x-show="metricVisible('disk_free')"><span class="metric-value" x-text="integrationData.disk_free ?? '—'"></span><span class="metric-label">Disk free</span></div>
                            <div class="metric-block" x-show="metricVisible('version')"><span class="metric-value" x-text="integrationData.version ?? '—'"></span><span class="metric-label">Version</span></div>
                            <div class="metric-block" x-show="metricVisible('health')"><span class="metric-value" x-text="integrationData.health ?? '—'"></span><span class="metric-label">Health</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'overseerr' || integrationData.type === 'jellyseerr'">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block" x-show="metricVisible('pending_requests')"><span class="metric-value" x-text="integrationData.pending_requests ?? '—'"></span><span class="metric-label">Pending</span></div>
                            <div class="metric-block" x-show="metricVisible('approved_requests')"><span class="metric-value" x-text="integrationData.approved_requests ?? '—'"></span><span class="metric-label">Approved</span></div>
                            <div class="metric-block" x-show="metricVisible('processing_requests')"><span class="metric-value" x-text="integrationData.processing_requests ?? '—'"></span><span class="metric-label">Processing</span></div>
                            <div class="metric-block" x-show="metricVisible('total_requests')"><span class="metric-value" x-text="integrationData.total_requests ?? '—'"></span><span class="metric-label">Total</span></div>
                            <div class="metric-block" x-show="metricVisible('declined_requests')"><span class="metric-value" x-text="integrationData.declined_requests ?? '—'"></span><span class="metric-label">Declined</span></div>
                            <div class="metric-block" x-show="metricVisible('available_requests')"><span class="metric-value" x-text="integrationData.available_requests ?? '—'"></span><span class="metric-label">Available</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'prowlarr'">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block"><span class="metric-value"><span x-text="integrationData.indexers_enabled ?? '—'"></span>/<span x-text="integrationData.indexers_total ?? '—'"></span></span><span class="metric-label">Indexers</span></div>
                            <div class="metric-block" x-show="metricVisible('queue_size')"><span class="metric-value" x-text="integrationData.queue_size ?? '—'"></span><span class="metric-label">Queue</span></div>
                            <div class="metric-block" x-show="metricVisible('failed_indexers')"><span class="metric-value" x-text="integrationData.failed_indexers ?? '—'"></span><span class="metric-label">Failed</span></div>
                            <div class="metric-block" x-show="metricVisible('version')"><span class="metric-value" x-text="integrationData.version ?? '—'"></span><span class="metric-label">Version</span></div>
                            <div class="metric-block" x-show="metricVisible('health')"><span class="metric-value" x-text="integrationData.health ?? '—'"></span><span class="metric-label">Health</span></div>
                            <div class="metric-block" x-show="metricVisible('app_count')"><span class="metric-value" x-text="integrationData.app_count ?? '—'"></span><span class="metric-label">Apps</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'uptime_kuma'">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block" x-show="metricVisible('monitors_up')"><span class="metric-value" x-text="integrationData.monitors_up ?? '—'"></span><span class="metric-label">Up</span></div>
                            <div class="metric-block" x-show="metricVisible('monitors_down')"><span class="metric-value" x-text="integrationData.monitors_down ?? '—'"></span><span class="metric-label">Down</span></div>
                            <div class="metric-block" x-show="metricVisible('monitors_total')"><span class="metric-value" x-text="integrationData.monitors_total ?? '—'"></span><span class="metric-label">Total</span></div>
                            <div class="metric-block" x-show="metricVisible('maintenance')"><span class="metric-value" x-text="integrationData.maintenance ?? '—'"></span><span class="metric-label">Maint.</span></div>
                            <div class="metric-block" x-show="metricVisible('avg_ping')"><span class="metric-value" x-text="integrationData.avg_ping ?? '—'"></span><span class="metric-label">Avg ping</span></div>
                            <div class="metric-block" x-show="metricVisible('cert_expiring')"><span class="metric-value" x-text="integrationData.cert_expiring ?? '—'"></span><span class="metric-label">Incidents</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'cloudflare_tunnel'">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block" x-show="metricVisible('tunnel_status')"><span class="metric-value" style="font-size:0.75rem;text-transform:capitalize;" x-text="integrationData.tunnel_status ?? '—'"></span><span class="metric-label">Status</span></div>
                            <div class="metric-block" x-show="metricVisible('connections')"><span class="metric-value" x-text="integrationData.connections ?? '—'"></span><span class="metric-label">Connections</span></div>
                            <div class="metric-block" x-show="metricVisible('colo_count')"><span class="metric-value" x-text="integrationData.colo_count ?? '—'"></span><span class="metric-label">Colos</span></div>
                            <div class="metric-block" x-show="metricVisible('tunnel_name')"><span class="metric-value" style="font-size:0.7rem;" x-text="integrationData.tunnel_name ?? '—'"></span><span class="metric-label">Tunnel</span></div>
                            <div class="metric-block" x-show="metricVisible('connector_version')"><span class="metric-value" style="font-size:0.7rem;" x-text="integrationData.connector_version ?? '—'"></span><span class="metric-label">Version</span></div>
                            <div class="metric-block" x-show="metricVisible('origin_count')"><span class="metric-value" x-text="integrationData.origin_count ?? '—'"></span><span class="metric-label">Origins</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'peanut'">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block"><span class="metric-value"><span x-text="integrationData.battery_percent ?? '—'"></span>%</span><span class="metric-label">Battery</span></div>
                            <div class="metric-block" x-show="metricVisible('ups_load')"><span class="metric-value" x-text="integrationData.ups_load ?? '—'"></span><span class="metric-label">Load</span></div>
                            <div class="metric-block" x-show="metricVisible('battery_runtime')"><span class="metric-value" x-text="integrationData.battery_runtime ?? '—'"></span><span class="metric-label">Runtime</span></div>
                            <div class="metric-block" x-show="metricVisible('ups_status')"><span class="metric-value" style="font-size:0.75rem;" x-text="integrationData.ups_status ?? '—'"></span><span class="metric-label">UPS</span></div>
                            <div class="metric-block" x-show="metricVisible('input_voltage')"><span class="metric-value" x-text="integrationData.input_voltage ?? '—'"></span><span class="metric-label">Input V</span></div>
                            <div class="metric-block" x-show="metricVisible('output_power')"><span class="metric-value" x-text="integrationData.output_power ?? '—'"></span><span class="metric-label">Output</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'qbittorrent'">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block" x-show="metricVisible('download_speed')"><span class="metric-value" x-text="integrationData.download_speed ?? '—'"></span><span class="metric-label">Download</span></div>
                            <div class="metric-block" x-show="metricVisible('upload_speed')"><span class="metric-value" x-text="integrationData.upload_speed ?? '—'"></span><span class="metric-label">Upload</span></div>
                            <div class="metric-block"><span class="metric-value"><span x-text="integrationData.active_downloads ?? '—'"></span>↓ <span x-text="integrationData.seeding ?? '—'"></span>↑</span><span class="metric-label">Active</span></div>
                            <div class="metric-block" x-show="metricVisible('free_disk')"><span class="metric-value" x-text="integrationData.free_disk ?? '—'"></span><span class="metric-label">Free disk</span></div>
                            <div class="metric-block" x-show="metricVisible('total_torrents')"><span class="metric-value" x-text="integrationData.total_torrents ?? '—'"></span><span class="metric-label">Torrents</span></div>
                            <div class="metric-block" x-show="metricVisible('paused_torrents')"><span class="metric-value" x-text="integrationData.paused_torrents ?? '—'"></span><span class="metric-label">Paused</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'sabnzbd' || integrationData.type === 'nzbget'">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block" x-show="metricVisible('queue_size')"><span class="metric-value" x-text="integrationData.queue_size ?? '—'"></span><span class="metric-label">Queue</span></div>
                            <div class="metric-block" x-show="metricVisible('download_speed')"><span class="metric-value" x-text="integrationData.download_speed ?? '—'"></span><span class="metric-label">Download</span></div>
                            <div class="metric-block" x-show="metricVisible('free_disk')"><span class="metric-value" x-text="integrationData.free_disk ?? '—'"></span><span class="metric-label">Free disk</span></div>
                            <div class="metric-block" x-show="metricVisible('paused')"><span class="metric-value" x-text="integrationData.paused ?? '—'"></span><span class="metric-label">Paused</span></div>
                            <div class="metric-block" x-show="metricVisible('version')"><span class="metric-value" x-text="integrationData.version ?? '—'"></span><span class="metric-label">Version</span></div>
                            <div class="metric-block" x-show="metricVisible('status')"><span class="metric-value" x-text="integrationData.status ?? '—'"></span><span class="metric-label">Status</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'transmission'">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block" x-show="metricVisible('download_speed')"><span class="metric-value" x-text="integrationData.download_speed ?? '—'"></span><span class="metric-label">Download</span></div>
                            <div class="metric-block" x-show="metricVisible('upload_speed')"><span class="metric-value" x-text="integrationData.upload_speed ?? '—'"></span><span class="metric-label">Upload</span></div>
                            <div class="metric-block"><span class="metric-value"><span x-text="integrationData.active_downloads ?? '—'"></span>↓ <span x-text="integrationData.seeding ?? '—'"></span>↑</span><span class="metric-label">Active</span></div>
                            <div class="metric-block" x-show="metricVisible('free_disk')"><span class="metric-value" x-text="integrationData.free_disk ?? '—'"></span><span class="metric-label">Free disk</span></div>
                            <div class="metric-block" x-show="metricVisible('total_torrents')"><span class="metric-value" x-text="integrationData.total_torrents ?? '—'"></span><span class="metric-label">Torrents</span></div>
                            <div class="metric-block" x-show="metricVisible('paused_torrents')"><span class="metric-value" x-text="integrationData.paused_torrents ?? '—'"></span><span class="metric-label">Paused</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'jackett'">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block"><span class="metric-value"><span x-text="integrationData.indexers_enabled ?? '—'"></span>/<span x-text="integrationData.indexers_total ?? '—'"></span></span><span class="metric-label">Indexers</span></div>
                            <div class="metric-block" x-show="metricVisible('failed_indexers')"><span class="metric-value" x-text="integrationData.failed_indexers ?? '—'"></span><span class="metric-label">Failed</span></div>
                            <div class="metric-block" x-show="metricVisible('version')"><span class="metric-value" x-text="integrationData.version ?? '—'"></span><span class="metric-label">Version</span></div>
                            <div class="metric-block" x-show="metricVisible('health')"><span class="metric-value" x-text="integrationData.health ?? '—'"></span><span class="metric-label">Health</span></div>
                            <div class="metric-block" x-show="metricVisible('indexers_total')"><span class="metric-value" x-text="integrationData.indexers_total ?? '—'"></span><span class="metric-label">Total</span></div>
                            <div class="metric-block" x-show="metricVisible('indexers_enabled')"><span class="metric-value" x-text="integrationData.indexers_enabled ?? '—'"></span><span class="metric-label">Enabled</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'tautulli'">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block" x-show="metricVisible('stream_count')"><span class="metric-value" x-text="integrationData.stream_count ?? '—'"></span><span class="metric-label">Streams</span></div>
                            <div class="metric-block" x-show="metricVisible('bandwidth')"><span class="metric-value" x-text="integrationData.bandwidth ?? '—'"></span><span class="metric-label">Bandwidth</span></div>
                            <div class="metric-block" x-show="metricVisible('library_count')"><span class="metric-value" x-text="integrationData.library_count ?? '—'"></span><span class="metric-label">Libraries</span></div>
                            <div class="metric-block" x-show="metricVisible('sessions')"><span class="metric-value" x-text="integrationData.sessions ?? '—'"></span><span class="metric-label">Sessions</span></div>
                            <div class="metric-block" x-show="metricVisible('status')"><span class="metric-value" x-text="integrationData.status ?? '—'"></span><span class="metric-label">Status</span></div>
                            <div class="metric-block" x-show="metricVisible('stream_count')"><span class="metric-value" x-text="integrationData.stream_count ?? '—'"></span><span class="metric-label">Active</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'audiobookshelf'">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block" x-show="metricVisible('library_count')"><span class="metric-value" x-text="integrationData.library_count ?? '—'"></span><span class="metric-label">Libraries</span></div>
                            <div class="metric-block" x-show="metricVisible('item_count')"><span class="metric-value" x-text="integrationData.item_count ?? '—'"></span><span class="metric-label">Items</span></div>
                            <div class="metric-block" x-show="metricVisible('status')"><span class="metric-value" x-text="integrationData.status ?? '—'"></span><span class="metric-label">Status</span></div>
                            <div class="metric-block" x-show="metricVisible('library_count')"><span class="metric-value" x-text="integrationData.library_count ?? '—'"></span><span class="metric-label">Libs</span></div>
                            <div class="metric-block" x-show="metricVisible('item_count')"><span class="metric-value" x-text="integrationData.item_count ?? '—'"></span><span class="metric-label">Media</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'immich'">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block" x-show="metricVisible('photos')"><span class="metric-value" x-text="integrationData.photos ?? '—'"></span><span class="metric-label">Photos</span></div>
                            <div class="metric-block" x-show="metricVisible('videos')"><span class="metric-value" x-text="integrationData.videos ?? '—'"></span><span class="metric-label">Videos</span></div>
                            <div class="metric-block" x-show="metricVisible('assets')"><span class="metric-value" x-text="integrationData.assets ?? '—'"></span><span class="metric-label">Assets</span></div>
                            <div class="metric-block" x-show="metricVisible('storage_used')"><span class="metric-value" x-text="integrationData.storage_used ?? '—'"></span><span class="metric-label">Storage</span></div>
                            <div class="metric-block" x-show="metricVisible('version')"><span class="metric-value" x-text="integrationData.version ?? '—'"></span><span class="metric-label">Version</span></div>
                            <div class="metric-block" x-show="metricVisible('status')"><span class="metric-value" x-text="integrationData.status ?? '—'"></span><span class="metric-label">Status</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'tdarr'">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block" x-show="metricVisible('queue_size')"><span class="metric-value" x-text="integrationData.queue_size ?? '—'"></span><span class="metric-label">Staged</span></div>
                            <div class="metric-block" x-show="metricVisible('workers')"><span class="metric-value" x-text="integrationData.workers ?? '—'"></span><span class="metric-label">Workers</span></div>
                            <div class="metric-block" x-show="metricVisible('health')"><span class="metric-value" x-text="integrationData.health ?? '—'"></span><span class="metric-label">Health</span></div>
                            <div class="metric-block" x-show="metricVisible('staged')"><span class="metric-value" x-text="integrationData.staged ?? '—'"></span><span class="metric-label">Queue</span></div>
                            <div class="metric-block" x-show="metricVisible('status')"><span class="metric-value" x-text="integrationData.status ?? '—'"></span><span class="metric-label">Status</span></div>
                            <div class="metric-block" x-show="metricVisible('workers')"><span class="metric-value" x-text="integrationData.workers ?? '—'"></span><span class="metric-label">Nodes</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'maintainerr'">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block" x-show="metricVisible('issue_count')"><span class="metric-value" x-text="integrationData.issue_count ?? '—'"></span><span class="metric-label">Issues</span></div>
                            <div class="metric-block" x-show="metricVisible('rule_count')"><span class="metric-value" x-text="integrationData.rule_count ?? '—'"></span><span class="metric-label">Rules</span></div>
                            <div class="metric-block" x-show="metricVisible('user_count')"><span class="metric-value" x-text="integrationData.user_count ?? '—'"></span><span class="metric-label">Users</span></div>
                            <div class="metric-block" x-show="metricVisible('issues')"><span class="metric-value" x-text="integrationData.issues ?? '—'"></span><span class="metric-label">Open</span></div>
                            <div class="metric-block" x-show="metricVisible('rules')"><span class="metric-value" x-text="integrationData.rules ?? '—'"></span><span class="metric-label">Active</span></div>
                            <div class="metric-block" x-show="metricVisible('status')"><span class="metric-value" x-text="integrationData.status ?? '—'"></span><span class="metric-label">Status</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'frigate'">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block"><span class="metric-value"><span x-text="integrationData.cameras_up ?? '—'"></span>/<span x-text="integrationData.cameras_total ?? '—'"></span></span><span class="metric-label">Cameras</span></div>
                            <div class="metric-block" x-show="metricVisible('detection_fps')"><span class="metric-value" x-text="integrationData.detection_fps ?? '—'"></span><span class="metric-label">Det. FPS</span></div>
                            <div class="metric-block" x-show="metricVisible('online')"><span class="metric-value" x-text="integrationData.online ?? '—'"></span><span class="metric-label">Online</span></div>
                            <div class="metric-block" x-show="metricVisible('cameras')"><span class="metric-value" x-text="integrationData.cameras ?? '—'"></span><span class="metric-label">Total</span></div>
                            <div class="metric-block" x-show="metricVisible('status')"><span class="metric-value" x-text="integrationData.status ?? '—'"></span><span class="metric-label">Status</span></div>
                            <div class="metric-block" x-show="metricVisible('cameras_up')"><span class="metric-value" x-text="integrationData.cameras_up ?? '—'"></span><span class="metric-label">Up</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'bazarr'">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block" x-show="metricVisible('missing_episodes')"><span class="metric-value" x-text="integrationData.missing_episodes ?? '—'"></span><span class="metric-label">Ep. missing</span></div>
                            <div class="metric-block" x-show="metricVisible('missing_movies')"><span class="metric-value" x-text="integrationData.missing_movies ?? '—'"></span><span class="metric-label">Mov. missing</span></div>
                            <div class="metric-block" x-show="metricVisible('version')"><span class="metric-value" x-text="integrationData.version ?? '—'"></span><span class="metric-label">Version</span></div>
                            <div class="metric-block" x-show="metricVisible('health')"><span class="metric-value" x-text="integrationData.health ?? '—'"></span><span class="metric-label">Health</span></div>
                            <div class="metric-block" x-show="metricVisible('language_count')"><span class="metric-value" x-text="integrationData.language_count ?? '—'"></span><span class="metric-label">Languages</span></div>
                            <div class="metric-block" x-show="metricVisible('provider_count')"><span class="metric-value" x-text="integrationData.provider_count ?? '—'"></span><span class="metric-label">Providers</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'fritz'">
                        <div class="integration-metrics-grid">
                            <div class="metric-block" x-show="metricVisible('status')"><span class="metric-value" style="font-size:0.75rem;text-transform:capitalize;" x-text="integrationData.status ?? '—'"></span><span class="metric-label">Status</span></div>
                            <div class="metric-block" x-show="metricVisible('download_speed')"><span class="metric-value" x-text="integrationData.download_speed ?? '—'"></span><span class="metric-label">Down</span></div>
                            <div class="metric-block" x-show="metricVisible('upload_speed')"><span class="metric-value" x-text="integrationData.upload_speed ?? '—'"></span><span class="metric-label">Up</span></div>
                            <div class="metric-block" x-show="metricVisible('external_ip')"><span class="metric-value" style="font-size:0.7rem;" x-text="integrationData.external_ip ?? '—'"></span><span class="metric-label">Ext IP</span></div>
                            <div class="metric-block" x-show="metricVisible('uptime')"><span class="metric-value" x-text="integrationData.uptime ?? '—'"></span><span class="metric-label">Uptime</span></div>
                            <div class="metric-block" x-show="metricVisible('devices')"><span class="metric-value" x-text="integrationData.devices ?? '—'"></span><span class="metric-label">Devices</span></div>
                            <div class="metric-block" x-show="metricVisible('version')"><span class="metric-value" style="font-size:0.7rem;" x-text="integrationData.version ?? '—'"></span><span class="metric-label">Version</span></div>
                            <div class="metric-block" x-show="metricVisible('model')"><span class="metric-value" style="font-size:0.7rem;" x-text="integrationData.model ?? '—'"></span><span class="metric-label">Model</span></div>
                            <div class="metric-block" x-show="metricVisible('down_link')"><span class="metric-value" x-text="integrationData.down_link ?? '—'"></span><span class="metric-label">Down link</span></div>
                            <div class="metric-block" x-show="metricVisible('up_link')"><span class="metric-value" x-text="integrationData.up_link ?? '—'"></span><span class="metric-label">Up link</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'portainer'">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block" x-show="metricVisible('containers_running')"><span class="metric-value" x-text="integrationData.containers_running ?? '—'"></span><span class="metric-label">Running</span></div>
                            <div class="metric-block" x-show="metricVisible('containers_stopped')"><span class="metric-value" x-text="integrationData.containers_stopped ?? '—'"></span><span class="metric-label">Stopped</span></div>
                            <div class="metric-block" x-show="metricVisible('stacks')"><span class="metric-value" x-text="integrationData.stacks ?? '—'"></span><span class="metric-label">Stacks</span></div>
                            <div class="metric-block" x-show="metricVisible('endpoints')"><span class="metric-value" x-text="integrationData.endpoints ?? '—'"></span><span class="metric-label">Endpoints</span></div>
                            <div class="metric-block" x-show="metricVisible('version')"><span class="metric-value" style="font-size:0.7rem;" x-text="integrationData.version ?? '—'"></span><span class="metric-label">Version</span></div>
                            <div class="metric-block" x-show="metricVisible('status')"><span class="metric-value" x-text="integrationData.status ?? '—'"></span><span class="metric-label">Status</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'opnsense' || integrationData.type === 'pfsense'">
                        <div class="integration-metrics-grid">
                            <div class="metric-block" x-show="metricVisible('cpu')"><span class="metric-value" x-text="integrationData.cpu ?? '—'"></span><span class="metric-label">CPU</span></div>
                            <div class="metric-block" x-show="metricVisible('memory')"><span class="metric-value" x-text="integrationData.memory ?? '—'"></span><span class="metric-label">Memory</span></div>
                            <div class="metric-block" x-show="metricVisible('states')"><span class="metric-value" x-text="integrationData.states ?? '—'"></span><span class="metric-label">States</span></div>
                            <div class="metric-block" x-show="metricVisible('uptime')"><span class="metric-value" x-text="integrationData.uptime ?? '—'"></span><span class="metric-label">Uptime</span></div>
                            <div class="metric-block" x-show="metricVisible('version')"><span class="metric-value" style="font-size:0.7rem;" x-text="integrationData.version ?? '—'"></span><span class="metric-label">Version</span></div>
                            <div class="metric-block" x-show="metricVisible('gateways_up')"><span class="metric-value" x-text="integrationData.gateways_up ?? integrationData.status ?? '—'"></span><span class="metric-label" x-text="integrationData.type === 'opnsense' ? 'GW up' : 'Status'"></span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'truenas'">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block" x-show="metricVisible('pools_healthy')"><span class="metric-value" x-text="integrationData.pools_healthy ?? '—'"></span><span class="metric-label">Pools OK</span></div>
                            <div class="metric-block" x-show="metricVisible('pools_degraded')"><span class="metric-value" x-text="integrationData.pools_degraded ?? '—'"></span><span class="metric-label">Degraded</span></div>
                            <div class="metric-block" x-show="metricVisible('storage_used')"><span class="metric-value" x-text="integrationData.storage_used ?? '—'"></span><span class="metric-label">Used</span></div>
                            <div class="metric-block" x-show="metricVisible('storage_free')"><span class="metric-value" x-text="integrationData.storage_free ?? '—'"></span><span class="metric-label">Free</span></div>
                            <div class="metric-block" x-show="metricVisible('version')"><span class="metric-value" style="font-size:0.7rem;" x-text="integrationData.version ?? '—'"></span><span class="metric-label">Version</span></div>
                            <div class="metric-block" x-show="metricVisible('status')"><span class="metric-value" x-text="integrationData.status ?? '—'"></span><span class="metric-label">Status</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'unifi'">
                        <div class="integration-metrics-grid">
                            <div class="metric-block" x-show="metricVisible('wan_status')"><span class="metric-value" style="font-size:0.7rem;" x-text="integrationData.wan_status ?? '—'"></span><span class="metric-label">WAN</span></div>
                            <div class="metric-block" x-show="metricVisible('clients')"><span class="metric-value" x-text="integrationData.clients ?? '—'"></span><span class="metric-label">Clients</span></div>
                            <div class="metric-block" x-show="metricVisible('aps_online')"><span class="metric-value" x-text="integrationData.aps_online ?? '—'"></span><span class="metric-label">APs up</span></div>
                            <div class="metric-block" x-show="metricVisible('devices')"><span class="metric-value" x-text="integrationData.devices ?? '—'"></span><span class="metric-label">Devices</span></div>
                            <div class="metric-block" x-show="metricVisible('latency')"><span class="metric-value" x-text="integrationData.latency ?? '—'"></span><span class="metric-label">Latency</span></div>
                            <div class="metric-block" x-show="metricVisible('status')"><span class="metric-value" x-text="integrationData.status ?? '—'"></span><span class="metric-label">Status</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'grafana'">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block" x-show="metricVisible('dashboards')"><span class="metric-value" x-text="integrationData.dashboards ?? '—'"></span><span class="metric-label">Dashboards</span></div>
                            <div class="metric-block" x-show="metricVisible('datasources')"><span class="metric-value" x-text="integrationData.datasources ?? '—'"></span><span class="metric-label">Sources</span></div>
                            <div class="metric-block" x-show="metricVisible('version')"><span class="metric-value" style="font-size:0.7rem;" x-text="integrationData.version ?? '—'"></span><span class="metric-label">Version</span></div>
                            <div class="metric-block" x-show="metricVisible('database')"><span class="metric-value" x-text="integrationData.database ?? '—'"></span><span class="metric-label">Database</span></div>
                            <div class="metric-block" x-show="metricVisible('organization')"><span class="metric-value" style="font-size:0.7rem;" x-text="integrationData.organization ?? '—'"></span><span class="metric-label">Org</span></div>
                            <div class="metric-block" x-show="metricVisible('status')"><span class="metric-value" x-text="integrationData.status ?? '—'"></span><span class="metric-label">Status</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'netdata'">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block" x-show="metricVisible('cpu')"><span class="metric-value" x-text="integrationData.cpu ?? '—'"></span><span class="metric-label">Host CPU</span></div>
                            <div class="metric-block" x-show="metricVisible('alarms')"><span class="metric-value" x-text="integrationData.alarms ?? '—'"></span><span class="metric-label">Alarms</span></div>
                            <div class="metric-block" x-show="metricVisible('charts')"><span class="metric-value" x-text="integrationData.charts ?? '—'"></span><span class="metric-label">Charts</span></div>
                            <div class="metric-block" x-show="metricVisible('version')"><span class="metric-value" style="font-size:0.7rem;" x-text="integrationData.version ?? '—'"></span><span class="metric-label">Version</span></div>
                            <div class="metric-block" x-show="metricVisible('hostname')"><span class="metric-value" style="font-size:0.7rem;" x-text="integrationData.hostname ?? '—'"></span><span class="metric-label">Host</span></div>
                            <div class="metric-block" x-show="metricVisible('status')"><span class="metric-value" x-text="integrationData.status ?? '—'"></span><span class="metric-label">Status</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'glances'">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block" x-show="metricVisible('cpu')"><span class="metric-value" x-text="integrationData.cpu ?? '—'"></span><span class="metric-label">Host CPU</span></div>
                            <div class="metric-block" x-show="metricVisible('memory')"><span class="metric-value" x-text="integrationData.memory ?? '—'"></span><span class="metric-label">Host RAM</span></div>
                            <div class="metric-block" x-show="metricVisible('load')"><span class="metric-value" x-text="integrationData.load ?? '—'"></span><span class="metric-label">Load</span></div>
                            <div class="metric-block" x-show="metricVisible('status')"><span class="metric-value" x-text="integrationData.status ?? '—'"></span><span class="metric-label">Status</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'beszel'">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block" x-show="metricVisible('systems_up')"><span class="metric-value" x-text="integrationData.systems_up ?? '—'"></span><span class="metric-label">Up</span></div>
                            <div class="metric-block" x-show="metricVisible('systems_down')"><span class="metric-value" x-text="integrationData.systems_down ?? '—'"></span><span class="metric-label">Down</span></div>
                            <div class="metric-block" x-show="metricVisible('systems')"><span class="metric-value" x-text="integrationData.systems ?? '—'"></span><span class="metric-label">Systems</span></div>
                            <div class="metric-block" x-show="metricVisible('avg_cpu')"><span class="metric-value" x-text="integrationData.avg_cpu ?? '—'"></span><span class="metric-label">Avg CPU</span></div>
                            <div class="metric-block" x-show="metricVisible('status')"><span class="metric-value" x-text="integrationData.status ?? '—'"></span><span class="metric-label">Status</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'paperless'">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block" x-show="metricVisible('documents')"><span class="metric-value" x-text="integrationData.documents ?? '—'"></span><span class="metric-label">Documents</span></div>
                            <div class="metric-block" x-show="metricVisible('inbox')"><span class="metric-value" x-text="integrationData.inbox ?? '—'"></span><span class="metric-label">Inbox</span></div>
                            <div class="metric-block" x-show="metricVisible('correspondents')"><span class="metric-value" x-text="integrationData.correspondents ?? '—'"></span><span class="metric-label">Contacts</span></div>
                            <div class="metric-block" x-show="metricVisible('tags')"><span class="metric-value" x-text="integrationData.tags ?? '—'"></span><span class="metric-label">Tags</span></div>
                            <div class="metric-block" x-show="metricVisible('storage')"><span class="metric-value" x-text="integrationData.storage ?? '—'"></span><span class="metric-label">Storage</span></div>
                            <div class="metric-block" x-show="metricVisible('status')"><span class="metric-value" x-text="integrationData.status ?? '—'"></span><span class="metric-label">Status</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'mealie'">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block" x-show="metricVisible('recipes')"><span class="metric-value" x-text="integrationData.recipes ?? '—'"></span><span class="metric-label">Recipes</span></div>
                            <div class="metric-block" x-show="metricVisible('users')"><span class="metric-value" x-text="integrationData.users ?? '—'"></span><span class="metric-label">Users</span></div>
                            <div class="metric-block" x-show="metricVisible('version')"><span class="metric-value" style="font-size:0.7rem;" x-text="integrationData.version ?? '—'"></span><span class="metric-label">Version</span></div>
                            <div class="metric-block" x-show="metricVisible('status')"><span class="metric-value" x-text="integrationData.status ?? '—'"></span><span class="metric-label">Status</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'nextcloud'">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block" x-show="metricVisible('users_active')"><span class="metric-value" x-text="integrationData.users_active ?? '—'"></span><span class="metric-label">Active 24h</span></div>
                            <div class="metric-block" x-show="metricVisible('users_online')"><span class="metric-value" x-text="integrationData.users_online ?? '—'"></span><span class="metric-label">Online</span></div>
                            <div class="metric-block" x-show="metricVisible('free_space')"><span class="metric-value" x-text="integrationData.free_space ?? '—'"></span><span class="metric-label">Free</span></div>
                            <div class="metric-block" x-show="metricVisible('version')"><span class="metric-value" style="font-size:0.7rem;" x-text="integrationData.version ?? '—'"></span><span class="metric-label">Version</span></div>
                            <div class="metric-block" x-show="metricVisible('status')"><span class="metric-value" x-text="integrationData.status ?? '—'"></span><span class="metric-label">Status</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'vaultwarden'">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block" x-show="metricVisible('users')"><span class="metric-value" x-text="integrationData.users ?? '—'"></span><span class="metric-label">Users</span></div>
                            <div class="metric-block" x-show="metricVisible('organizations')"><span class="metric-value" x-text="integrationData.organizations ?? '—'"></span><span class="metric-label">Orgs</span></div>
                            <div class="metric-block" x-show="metricVisible('version')"><span class="metric-value" style="font-size:0.7rem;" x-text="integrationData.version ?? '—'"></span><span class="metric-label">Version</span></div>
                            <div class="metric-block" x-show="metricVisible('server')"><span class="metric-value" style="font-size:0.7rem;" x-text="integrationData.server ?? '—'"></span><span class="metric-label">Server</span></div>
                            <div class="metric-block" x-show="metricVisible('status')"><span class="metric-value" x-text="integrationData.status ?? '—'"></span><span class="metric-label">Status</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'deluge'">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block" x-show="metricVisible('downloading')"><span class="metric-value" x-text="integrationData.downloading ?? '—'"></span><span class="metric-label">Downloading</span></div>
                            <div class="metric-block" x-show="metricVisible('seeding')"><span class="metric-value" x-text="integrationData.seeding ?? '—'"></span><span class="metric-label">Seeding</span></div>
                            <div class="metric-block" x-show="metricVisible('torrents')"><span class="metric-value" x-text="integrationData.torrents ?? '—'"></span><span class="metric-label">Torrents</span></div>
                            <div class="metric-block" x-show="metricVisible('free_space')"><span class="metric-value" x-text="integrationData.free_space ?? '—'"></span><span class="metric-label">Free disk</span></div>
                            <div class="metric-block" x-show="metricVisible('status')"><span class="metric-value" x-text="integrationData.status ?? '—'"></span><span class="metric-label">Status</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'navidrome'">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block" x-show="metricVisible('artists')"><span class="metric-value" x-text="integrationData.artists ?? '—'"></span><span class="metric-label">Artists</span></div>
                            <div class="metric-block" x-show="metricVisible('version')"><span class="metric-value" style="font-size:0.7rem;" x-text="integrationData.version ?? '—'"></span><span class="metric-label">Version</span></div>
                            <div class="metric-block" x-show="metricVisible('status')"><span class="metric-value" x-text="integrationData.status ?? '—'"></span><span class="metric-label">Status</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'komga'">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block" x-show="metricVisible('series')"><span class="metric-value" x-text="integrationData.series ?? '—'"></span><span class="metric-label">Series</span></div>
                            <div class="metric-block" x-show="metricVisible('books')"><span class="metric-value" x-text="integrationData.books ?? '—'"></span><span class="metric-label">Books</span></div>
                            <div class="metric-block" x-show="metricVisible('libraries')"><span class="metric-value" x-text="integrationData.libraries ?? '—'"></span><span class="metric-label">Libraries</span></div>
                            <div class="metric-block" x-show="metricVisible('status')"><span class="metric-value" x-text="integrationData.status ?? '—'"></span><span class="metric-label">Status</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'photoprism'">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block" x-show="metricVisible('photos')"><span class="metric-value" x-text="integrationData.photos ?? '—'"></span><span class="metric-label">Photos</span></div>
                            <div class="metric-block" x-show="metricVisible('videos')"><span class="metric-value" x-text="integrationData.videos ?? '—'"></span><span class="metric-label">Videos</span></div>
                            <div class="metric-block" x-show="metricVisible('albums')"><span class="metric-value" x-text="integrationData.albums ?? '—'"></span><span class="metric-label">Albums</span></div>
                            <div class="metric-block" x-show="metricVisible('index_status')"><span class="metric-value" style="font-size:0.7rem;" x-text="integrationData.index_status ?? '—'"></span><span class="metric-label">Index</span></div>
                            <div class="metric-block" x-show="metricVisible('version')"><span class="metric-value" style="font-size:0.7rem;" x-text="integrationData.version ?? '—'"></span><span class="metric-label">Version</span></div>
                            <div class="metric-block" x-show="metricVisible('status')"><span class="metric-value" x-text="integrationData.status ?? '—'"></span><span class="metric-label">Status</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'proxmox'">
                        <div class="integration-metrics-grid">
                            <div class="metric-block" x-show="metricVisible('nodes')"><span class="metric-value" x-text="integrationData.nodes ?? '—'"></span><span class="metric-label">Nodes</span></div>
                            <div class="metric-block" x-show="metricVisible('vms')"><span class="metric-value" x-text="integrationData.vms ?? '—'"></span><span class="metric-label">VMs</span></div>
                            <div class="metric-block" x-show="metricVisible('lxcs')"><span class="metric-value" x-text="integrationData.lxcs ?? '—'"></span><span class="metric-label">LXCs</span></div>
                            <div class="metric-block" x-show="metricVisible('cluster_cpu')"><span class="metric-value" x-text="integrationData.cluster_cpu ?? '—'"></span><span class="metric-label">CPU</span></div>
                            <div class="metric-block" x-show="metricVisible('cluster_mem')"><span class="metric-value" x-text="integrationData.cluster_mem ?? '—'"></span><span class="metric-label">Memory</span></div>
                            <div class="metric-block" x-show="metricVisible('version')"><span class="metric-value" style="font-size:0.7rem;" x-text="integrationData.version ?? '—'"></span><span class="metric-label">Version</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'tailscale'">
                        <div class="integration-metrics-grid">
                            <div class="metric-block" x-show="metricVisible('devices_online')"><span class="metric-value" x-text="integrationData.devices_online ?? '—'"></span><span class="metric-label">Online</span></div>
                            <div class="metric-block" x-show="metricVisible('devices')"><span class="metric-value" x-text="integrationData.devices ?? '—'"></span><span class="metric-label">Devices</span></div>
                            <div class="metric-block" x-show="metricVisible('exit_nodes')"><span class="metric-value" x-text="integrationData.exit_nodes ?? '—'"></span><span class="metric-label">Exit nodes</span></div>
                            <div class="metric-block" x-show="metricVisible('status')"><span class="metric-value" x-text="integrationData.status ?? '—'"></span><span class="metric-label">Status</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'netbird'">
                        <div class="integration-metrics-grid">
                            <div class="metric-block" x-show="metricVisible('peers_connected')"><span class="metric-value" x-text="integrationData.peers_connected ?? '—'"></span><span class="metric-label">Connected</span></div>
                            <div class="metric-block" x-show="metricVisible('peers')"><span class="metric-value" x-text="integrationData.peers ?? '—'"></span><span class="metric-label">Peers</span></div>
                            <div class="metric-block" x-show="metricVisible('setup_keys')"><span class="metric-value" x-text="integrationData.setup_keys ?? '—'"></span><span class="metric-label">Setup keys</span></div>
                            <div class="metric-block" x-show="metricVisible('status')"><span class="metric-value" x-text="integrationData.status ?? '—'"></span><span class="metric-label">Status</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.tier2">
                        <div class="integration-metrics-grid" data-lxc-metrics>
                            {cpu_ram}
                            <div class="metric-block" x-show="metricVisible('status')"><span class="metric-value" x-text="integrationData.status ?? '—'"></span><span class="metric-label">Status</span></div>
                            <template x-for="key in ['version','users','flows','devices','stacks','zones','plugins','entities','sessions','missing','queue','library','datastores','proxy_hosts','routers','array_state','blocking','volumes','model','models','running','certificates','middlewares','services','lights_on','server_name','wanted','total','pending','containers','parity_status','luci','active_streams','releases','watches','applications','teams','active','unread','nodes','pods','jobs']" :key="key">
                                <div class="metric-block" x-show="metricVisible(key) && integrationData[key] !== undefined && integrationData[key] !== null && integrationData[key] !== '—'">
                                    <span class="metric-value" x-text="integrationData[key]"></span>
                                    <span class="metric-label" x-text="key.replace(/_/g, ' ')"></span>
                                </div>
                            </template>
                        </div>
                    </template>
                    <template x-if="integrationData.health_only">
                        <div class="integration-metrics-grid cols-2">
                            <div class="metric-block" x-show="metricVisible('status')"><span class="metric-value" x-text="integrationData.status ?? '—'"></span><span class="metric-label">Status</span></div>
                            <div class="metric-block" x-show="metricVisible('version')"><span class="metric-value" x-text="integrationData.version ?? integrationData.latency_ms ?? '—'"></span><span class="metric-label" x-text="integrationData.version ? 'Version' : 'Latency'"></span></div>
                        </div>
                    </template>
                    </div>
                </div>"#,
        loading = filled_loading_grid(),
        cpu_ram = cpu_ram,
    )
}

fn normalize_container_token(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

/// Server-side mirror of dashboard-live.js findContainerByNames, so SSR can
/// pre-fill the same status/metrics the WebSocket would send.
fn find_container_for_tokens<'a>(
    containers: &'a [crate::models::LxcContainer],
    alias_tokens: &[String],
) -> Option<&'a crate::models::LxcContainer> {
    let tokens: Vec<String> = alias_tokens
        .iter()
        .flat_map(|n| {
            n.to_lowercase()
                .split(|c: char| !c.is_ascii_alphanumeric())
                .map(normalize_container_token)
                .collect::<Vec<_>>()
        })
        .filter(|t| !t.is_empty())
        .collect();
    containers.iter().find(|lxc| {
        let cname = normalize_container_token(&lxc.name);
        if cname.is_empty() {
            return false;
        }
        tokens
            .iter()
            .any(|t| cname == *t || t.contains(&cname) || cname.contains(t.as_str()))
    })
}

fn format_bytes_short(bytes: f64) -> String {
    if !bytes.is_finite() || bytes <= 0.0 {
        return "—".to_string();
    }
    if bytes >= 1_000_000_000_000.0 {
        format!("{:.1} TB", bytes / 1_000_000_000_000.0)
    } else if bytes >= 1_000_000_000.0 {
        format!("{:.1} GB", bytes / 1_000_000_000.0)
    } else if bytes >= 1_000_000.0 {
        format!("{:.0} MB", bytes / 1_000_000.0)
    } else {
        format!("{:.0} KB", bytes / 1_000.0)
    }
}

fn status_badge_class(status: &str) -> &'static str {
    match status.to_lowercase().as_str() {
        "running" | "online" => "status-online",
        "checking" => "status-checking",
        "not configured" | "unknown" => "status-unknown",
        _ => "status-offline",
    }
}

#[allow(clippy::too_many_arguments)]
fn render_apps_grid(
    apps: &[App],
    is_admin: bool,
    csrf_token: &str,
    csrf_attr: &str,
    logo_manifest: &HashMap<String, String>,
    iframe_embeds_enabled: bool,
    known_statuses: &HashMap<String, crate::models::AppStatus>,
    known_containers: &[crate::models::LxcContainer],
    theme_id: &str,
) -> String {
    if apps.is_empty() {
        return r#"
        <div class="glass-panel app-card app-card--empty">
            <p style="font-weight: 600; color: var(--text-secondary);">No services configured yet</p>
            <p style="font-size: 0.8rem; color: var(--text-muted); margin-top: 0.5rem;">Log in as Admin and click "Add App" to register your infrastructure.</p>
        </div>"#
            .to_string();
    }

    let mut cards_html = String::new();
    for app in apps.iter() {
        let lowercase_icon = app.icon.to_lowercase();
        let resolved_logo = resolve_logo_from_manifest(&app.icon, logo_manifest);
        let brand_logo = if !resolved_logo.is_empty() {
            resolved_logo
        } else if !lowercase_icon.is_empty() {
            fallback_brand_logo(&lowercase_icon)
        } else {
            "/static/fallback.svg".to_string()
        };

        let cat_slug = category_slug(&app.category);

        let name_lower = app.name.to_lowercase();

        let mut alias_tokens = vec![name_lower.clone()];
        if !lowercase_icon.is_empty() && lowercase_icon != name_lower {
            alias_tokens.push(lowercase_icon.clone());
        }
        let url_lower = app.url.to_lowercase();
        for token in url_lower
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|t| t.len() >= 3)
        {
            if !alias_tokens.iter().any(|t| t == token) {
                alias_tokens.push(token.to_string());
            }
        }

        // Instant status: pre-fill the badge from the server's last known
        // container state or URL health check, so a reload never shows an
        // empty CHECKING... badge when data already exists in memory.
        let container_match = find_container_for_tokens(known_containers, &alias_tokens);
        let url_status = known_statuses
            .get(&app.name)
            .or_else(|| known_statuses.get(&name_lower));

        let (known_status, status_title) = if let Some(container) = container_match {
            let status = if is_admin {
                container.status.clone()
            } else if matches!(
                container.status.to_lowercase().as_str(),
                "running" | "online"
            ) {
                "online".to_string()
            } else {
                "offline".to_string()
            };
            (
                Some(status),
                if is_admin {
                    "Container runtime status"
                } else {
                    "Service availability"
                },
            )
        } else if let Some(status) = url_status {
            (
                Some(status.status.clone()),
                if is_admin {
                    "URL health check"
                } else {
                    "Public availability check"
                },
            )
        } else {
            (
                None,
                if is_admin {
                    "Waiting for Proxmox / agent…"
                } else {
                    "Waiting for live status…"
                },
            )
        };
        let status_badge = match &known_status {
            Some(status) => format!(
                r#"<span class="status-badge {}" title="{}" aria-label="{}" data-last-status="{}">{}</span>"#,
                status_badge_class(status),
                status_title,
                status_title,
                escape_html(&status.to_lowercase()),
                escape_html(&status.to_uppercase())
            ),
            None => format!(
                r#"<span class="status-badge status-checking" title="{}" aria-label="{}" data-last-status="">CHECKING...</span>"#,
                status_title, status_title
            ),
        };
        let api_metrics_hidden =
            crate::settings::integration_api_metrics_hidden(&app.integration_visible_metrics);
        let use_filled = app.card_span == "1x2"
            && is_admin
            && !app.integration_type.is_empty()
            && app.integration_type != "rss"
            && !api_metrics_hidden;
        let sub_metrics = if is_admin {
            if name_lower.contains("home") && name_lower.contains("assistant") {
                r#"
                <div class="nested-metrics-grid cols-3" id="ha-metrics-grid">
                    <div class="metric-block">
                        <span class="metric-value" id="ha-lights">—</span>
                        <span class="metric-label">Lights</span>
                    </div>
                    <div class="metric-block">
                        <span class="metric-value" id="ha-switches">—</span>
                        <span class="metric-label">Switches</span>
                    </div>
                    <div class="metric-block">
                        <span class="metric-value" id="ha-temp">—</span>
                        <span class="metric-label">Avg Temp</span>
                    </div>
                </div>"#
                    .to_string()
            } else if app.show_container_metrics && (!use_filled || api_metrics_hidden) {
                // Instant metrics: seed CPU/RAM from the last agent telemetry
                // instead of empty dashes when the container is already known.
                let (cpu_display, ram_display) = match container_match {
                    Some(container) => {
                        let cpu = container.cpu.unwrap_or(0.0);
                        let cpu_display = if cpu > 0.0 {
                            format!("{:.1}%", cpu * 100.0)
                        } else {
                            "0%".to_string()
                        };
                        let ram_display = container
                            .mem
                            .filter(|m| *m > 0)
                            .map(|m| format_bytes_short(m as f64))
                            .unwrap_or_else(|| "—".to_string());
                        (cpu_display, ram_display)
                    }
                    None => ("—".to_string(), "—".to_string()),
                };
                format!(
                    r#"
                <div class="nested-metrics-grid cols-2" data-lxc-metrics>
                    <div class="metric-block">
                        <span class="metric-value">{}</span>
                        <span class="metric-label">CPU</span>
                    </div>
                    <div class="metric-block">
                        <span class="metric-value">{}</span>
                        <span class="metric-label">RAM</span>
                    </div>
                </div>"#,
                    cpu_display, ram_display
                )
            } else {
                String::new()
            }
        } else {
            "".to_string()
        };

        let delete_btn = if is_admin {
            let mut app_for_json = app.clone();
            if !app_for_json.api_key.is_empty() {
                app_for_json.api_key = "Configured — leave blank to keep unchanged".to_string();
            }
            let app_json = serde_json::to_string(&app_for_json).unwrap_or_default();
            let escaped_json = app_json
                .replace('&', "&amp;")
                .replace('"', "&quot;")
                .replace('\'', "&#39;");
            let edit_control = format!(
                r#"<button type="button" class="btn-edit-app" title="Edit application" data-app="{}" @click="editApp = JSON.parse($el.getAttribute('data-app')); window.editingAppOriginalName = (editApp.name || '').toLowerCase(); if (window.amudHydrateAppMetrics) amudHydrateAppMetrics(editApp); editAppModalOpen = true; setTimeout(checkDuplicateAppName, 0); setTimeout(function(){{ if (window.amudRefreshIntegrationPicker) window.amudRefreshIntegrationPicker('edit-app-integration-picker', editApp.integration_type || ''); }}, 50);"><i data-lucide="edit-2"></i></button>"#,
                escaped_json
            );
            format!(
                r#"
                <div style="display: inline-flex; align-items: center; gap: 0.25rem;">
                    {}
                    <form action="/apps/delete" method="POST" style="margin: 0; display: inline-flex; align-items: center;">
                        <input type="hidden" name="id" value="{}">
                        <input type="hidden" name="csrf_token" value="{}">
                        <button type="submit" class="btn-delete-app" title="Delete application">
                            <i data-lucide="trash-2"></i>
                        </button>
                    </form>
                </div>
                "#,
                edit_control, app.id, csrf_attr
            )
        } else {
            "".to_string()
        };
        let ctrl_container = if is_admin {
            r#"
            <div class="container-controls" style="display: none; align-items: center; gap: 0.25rem;" data-id="" data-provider="">
                <button type="button" class="btn-ctrl start" title="Start Container" @click="triggerContainerAction($el, 'start')">
                    <i data-lucide="circle-play" style="width:0.9rem; height:0.9rem;"></i>
                </button>
                <button type="button" class="btn-ctrl stop" title="Stop Container" @click="triggerContainerAction($el, 'stop')">
                    <i data-lucide="circle-stop" style="width:0.9rem; height:0.9rem;"></i>
                </button>
                <button type="button" class="btn-ctrl restart" title="Restart Container" @click="triggerContainerAction($el, 'restart')">
                    <i data-lucide="rotate-cw" style="width:0.9rem; height:0.9rem;"></i>
                </button>
            </div>
            "#.to_string()
        } else {
            "".to_string()
        };

        let mut integration_widget = String::new();
        if is_admin && !app.integration_type.is_empty() && !api_metrics_hidden {
            if use_filled {
                integration_widget = build_filled_integration_widget(app.show_container_metrics);
            } else if app.integration_type == "rss" {
                integration_widget = r#"
                <div class="integration-widget integration-widget--always">
                    <div class="nested-metrics-grid app-card-metrics-fallback" x-show="!integrationData || !integrationData.type">
                        <div class="metric-block">
                            <span class="metric-value">—</span>
                            <span class="metric-label">Loading</span>
                        </div>
                    </div>
                    <div x-show="integrationData && integrationData.type">
                    <template x-if="integrationData.type === 'rss'">
                        <div class="rss-feed-list">
                            <template x-for="(entry, index) in integrationData.entries" :key="index">
                                <a x-show="entry.link" :href="entry.link" target="_blank" rel="noopener" class="rss-feed-item">
                                    <span class="rss-feed-title" x-text="entry.title"></span>
                                    <span class="rss-feed-date" x-text="entry.date"></span>
                                </a>
                                <div x-show="!entry.link" class="rss-feed-item rss-feed-item--text-only">
                                    <span class="rss-feed-title" x-text="entry.title"></span>
                                    <span class="rss-feed-date" x-text="entry.date"></span>
                                </div>
                            </template>
                            <div x-show="!integrationData.entries || integrationData.entries.length === 0" class="rss-feed-empty">
                                <span>No entries found</span>
                            </div>
                        </div>
                    </template>
                    </div>
                </div>"#
                .to_string();
            } else {
                let integration_class =
                    "integration-widget integration-widget--hover app-card-metrics-layer app-card-metrics-layer--integration";
                integration_widget = format!(
                    r#"
                <div class="{}">
                    <div class="nested-metrics-grid app-card-metrics-fallback" x-show="!integrationData || !integrationData.type">
                        <div class="metric-block">
                            <span class="metric-value">—</span>
                            <span class="metric-label">Loading</span>
                        </div>
                    </div>
                    <div x-show="integrationData && integrationData.type">
                    <template x-if="integrationData.type === 'pihole' || integrationData.type === 'adguard'">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block">
                                <span class="metric-value" x-text="integrationData.ads_blocked_today"></span>
                                <span class="metric-label">Ads Blocked</span>
                            </div>
                            <div class="metric-block" style="flex-direction: row; justify-content: center; gap: 0.5rem; align-items: center;">
                                <span class="metric-value" style="font-size: 0.8rem; text-transform: uppercase;" x-text="integrationData.status"></span>
                                <button type="button" class="btn btn-secondary" style="padding: 0.2rem 0.5rem; font-size: 0.7rem; height: auto;" @click="fetch('/api/apps/{}/integration/action', {{ method: 'POST', headers: {{'Content-Type': 'application/json', 'X-CSRF-Token': '{}'}}, body: JSON.stringify({{action: 'disable'}}) }}).then(() => fetch('/api/apps/{}/integration').then(r => r.ok ? r.json() : null)).then(d => {{ if (d && d.type) integrationData = d }}).catch(() => {{}})">Disable</button>
                            </div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'radarr' || integrationData.type === 'sonarr' || integrationData.type === 'lidarr' || integrationData.type === 'readarr' || integrationData.type === 'whisparr'">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block">
                                <span class="metric-value" x-text="integrationData.queue_size"></span>
                                <span class="metric-label">Queue</span>
                            </div>
                            <div class="metric-block">
                                <span class="metric-value" x-text="integrationData.missing"></span>
                                <span class="metric-label">Missing</span>
                            </div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'overseerr' || integrationData.type === 'jellyseerr'">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block">
                                <span class="metric-value" x-text="integrationData.pending_requests"></span>
                                <span class="metric-label">Pending</span>
                            </div>
                            <div class="metric-block">
                                <span class="metric-value" x-text="integrationData.total_requests"></span>
                                <span class="metric-label">Total</span>
                            </div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'prowlarr'">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block">
                                <span class="metric-value"><span x-text="integrationData.indexers_enabled"></span>/<span x-text="integrationData.indexers_total"></span></span>
                                <span class="metric-label">Indexers</span>
                            </div>
                            <div class="metric-block">
                                <span class="metric-value" x-text="integrationData.queue_size"></span>
                                <span class="metric-label">Queue</span>
                            </div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'uptime_kuma'">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block">
                                <span class="metric-value" x-text="integrationData.monitors_up"></span>
                                <span class="metric-label">Up</span>
                            </div>
                            <div class="metric-block">
                                <span class="metric-value" x-text="integrationData.monitors_down"></span>
                                <span class="metric-label">Down</span>
                            </div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'cloudflare_tunnel'">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block">
                                <span class="metric-value" style="font-size:0.8rem; text-transform:capitalize;" x-text="integrationData.tunnel_status"></span>
                                <span class="metric-label" x-text="integrationData.tunnel_name"></span>
                            </div>
                            <div class="metric-block">
                                <span class="metric-value" x-text="integrationData.connections"></span>
                                <span class="metric-label">Connections</span>
                            </div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'peanut'">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block">
                                <span class="metric-value"><span x-text="integrationData.battery_percent"></span>%</span>
                                <span class="metric-label">Battery</span>
                            </div>
                            <div class="metric-block">
                                <span class="metric-value" style="font-size:0.75rem;" x-text="integrationData.ups_status"></span>
                                <span class="metric-label">UPS</span>
                            </div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'qbittorrent'">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block">
                                <span class="metric-value" x-text="integrationData.download_speed"></span>
                                <span class="metric-label">Download</span>
                            </div>
                            <div class="metric-block">
                                <span class="metric-value"><span x-text="integrationData.active_downloads"></span>↓ <span x-text="integrationData.seeding"></span>↑</span>
                                <span class="metric-label">Active</span>
                            </div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'bazarr'">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block">
                                <span class="metric-value" x-text="integrationData.missing_episodes"></span>
                                <span class="metric-label">Ep. missing</span>
                            </div>
                            <div class="metric-block">
                                <span class="metric-value" x-text="integrationData.missing_movies"></span>
                                <span class="metric-label">Mov. missing</span>
                            </div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'fritz'">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block" x-show="metricVisible('status')"><span class="metric-value" x-text="integrationData.status"></span><span class="metric-label">Status</span></div>
                            <div class="metric-block" x-show="metricVisible('download_speed')"><span class="metric-value" x-text="integrationData.download_speed"></span><span class="metric-label">Down</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'portainer'">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block" x-show="metricVisible('containers_running')"><span class="metric-value" x-text="integrationData.containers_running"></span><span class="metric-label">Running</span></div>
                            <div class="metric-block" x-show="metricVisible('stacks')"><span class="metric-value" x-text="integrationData.stacks"></span><span class="metric-label">Stacks</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'opnsense' || integrationData.type === 'pfsense'">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block" x-show="metricVisible('cpu')"><span class="metric-value" x-text="integrationData.cpu"></span><span class="metric-label">CPU</span></div>
                            <div class="metric-block" x-show="metricVisible('states')"><span class="metric-value" x-text="integrationData.states"></span><span class="metric-label">States</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'truenas'">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block" x-show="metricVisible('pools_healthy')"><span class="metric-value" x-text="integrationData.pools_healthy"></span><span class="metric-label">Pools OK</span></div>
                            <div class="metric-block" x-show="metricVisible('storage_free')"><span class="metric-value" x-text="integrationData.storage_free"></span><span class="metric-label">Free</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'unifi'">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block" x-show="metricVisible('clients')"><span class="metric-value" x-text="integrationData.clients"></span><span class="metric-label">Clients</span></div>
                            <div class="metric-block" x-show="metricVisible('aps_online')"><span class="metric-value" x-text="integrationData.aps_online"></span><span class="metric-label">APs up</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'grafana'">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block" x-show="metricVisible('dashboards')"><span class="metric-value" x-text="integrationData.dashboards"></span><span class="metric-label">Dashboards</span></div>
                            <div class="metric-block" x-show="metricVisible('datasources')"><span class="metric-value" x-text="integrationData.datasources"></span><span class="metric-label">Sources</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'netdata' || integrationData.type === 'glances'">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block" x-show="metricVisible('cpu')"><span class="metric-value" x-text="integrationData.cpu"></span><span class="metric-label">CPU</span></div>
                            <div class="metric-block" x-show="metricVisible('alarms')"><span class="metric-value" x-text="integrationData.alarms ?? integrationData.memory"></span><span class="metric-label" x-text="integrationData.type === 'netdata' ? 'Alarms' : 'RAM'"></span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'beszel'">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block" x-show="metricVisible('systems_up')"><span class="metric-value" x-text="integrationData.systems_up"></span><span class="metric-label">Up</span></div>
                            <div class="metric-block" x-show="metricVisible('systems')"><span class="metric-value" x-text="integrationData.systems"></span><span class="metric-label">Systems</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'paperless'">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block" x-show="metricVisible('documents')"><span class="metric-value" x-text="integrationData.documents"></span><span class="metric-label">Documents</span></div>
                            <div class="metric-block" x-show="metricVisible('inbox')"><span class="metric-value" x-text="integrationData.inbox"></span><span class="metric-label">Inbox</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'mealie'">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block" x-show="metricVisible('recipes')"><span class="metric-value" x-text="integrationData.recipes"></span><span class="metric-label">Recipes</span></div>
                            <div class="metric-block" x-show="metricVisible('users')"><span class="metric-value" x-text="integrationData.users"></span><span class="metric-label">Users</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'nextcloud'">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block" x-show="metricVisible('users_active')"><span class="metric-value" x-text="integrationData.users_active"></span><span class="metric-label">Active</span></div>
                            <div class="metric-block" x-show="metricVisible('free_space')"><span class="metric-value" x-text="integrationData.free_space"></span><span class="metric-label">Free</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'vaultwarden'">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block" x-show="metricVisible('users')"><span class="metric-value" x-text="integrationData.users"></span><span class="metric-label">Users</span></div>
                            <div class="metric-block" x-show="metricVisible('organizations')"><span class="metric-value" x-text="integrationData.organizations"></span><span class="metric-label">Orgs</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'deluge'">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block" x-show="metricVisible('downloading')"><span class="metric-value" x-text="integrationData.downloading"></span><span class="metric-label">Downloading</span></div>
                            <div class="metric-block" x-show="metricVisible('seeding')"><span class="metric-value" x-text="integrationData.seeding"></span><span class="metric-label">Seeding</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'navidrome'">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block" x-show="metricVisible('artists')"><span class="metric-value" x-text="integrationData.artists"></span><span class="metric-label">Artists</span></div>
                            <div class="metric-block" x-show="metricVisible('status')"><span class="metric-value" x-text="integrationData.status"></span><span class="metric-label">Status</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'komga'">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block" x-show="metricVisible('series')"><span class="metric-value" x-text="integrationData.series"></span><span class="metric-label">Series</span></div>
                            <div class="metric-block" x-show="metricVisible('books')"><span class="metric-value" x-text="integrationData.books"></span><span class="metric-label">Books</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'photoprism'">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block" x-show="metricVisible('photos')"><span class="metric-value" x-text="integrationData.photos"></span><span class="metric-label">Photos</span></div>
                            <div class="metric-block" x-show="metricVisible('videos')"><span class="metric-value" x-text="integrationData.videos"></span><span class="metric-label">Videos</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'proxmox'">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block" x-show="metricVisible('vms')"><span class="metric-value" x-text="integrationData.vms"></span><span class="metric-label">VMs</span></div>
                            <div class="metric-block" x-show="metricVisible('lxcs')"><span class="metric-value" x-text="integrationData.lxcs"></span><span class="metric-label">LXCs</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'tailscale' || integrationData.type === 'netbird'">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block" x-show="metricVisible('devices_online')"><span class="metric-value" x-text="integrationData.devices_online ?? integrationData.peers_connected"></span><span class="metric-label">Online</span></div>
                            <div class="metric-block" x-show="metricVisible('devices')"><span class="metric-value" x-text="integrationData.devices ?? integrationData.peers"></span><span class="metric-label">Total</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.tier2 || integrationData.health_only">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block" x-show="metricVisible('status')"><span class="metric-value" x-text="integrationData.status"></span><span class="metric-label">Status</span></div>
                            <div class="metric-block" x-show="metricVisible('version')"><span class="metric-value" x-text="integrationData.version ?? integrationData.latency_ms ?? '—'"></span><span class="metric-label">Info</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'sabnzbd' || integrationData.type === 'nzbget'">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block" x-show="metricVisible('queue_size')"><span class="metric-value" x-text="integrationData.queue_size"></span><span class="metric-label">Queue</span></div>
                            <div class="metric-block" x-show="metricVisible('download_speed')"><span class="metric-value" x-text="integrationData.download_speed"></span><span class="metric-label">Download</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'transmission'">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block" x-show="metricVisible('download_speed')"><span class="metric-value" x-text="integrationData.download_speed"></span><span class="metric-label">Download</span></div>
                            <div class="metric-block"><span class="metric-value"><span x-text="integrationData.active_downloads"></span>↓ <span x-text="integrationData.seeding"></span>↑</span><span class="metric-label">Active</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'jackett'">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block"><span class="metric-value"><span x-text="integrationData.indexers_enabled"></span>/<span x-text="integrationData.indexers_total"></span></span><span class="metric-label">Indexers</span></div>
                            <div class="metric-block" x-show="metricVisible('failed_indexers')"><span class="metric-value" x-text="integrationData.failed_indexers"></span><span class="metric-label">Failed</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'tautulli'">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block" x-show="metricVisible('stream_count')"><span class="metric-value" x-text="integrationData.stream_count"></span><span class="metric-label">Streams</span></div>
                            <div class="metric-block" x-show="metricVisible('bandwidth')"><span class="metric-value" x-text="integrationData.bandwidth"></span><span class="metric-label">Bandwidth</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'audiobookshelf'">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block" x-show="metricVisible('library_count')"><span class="metric-value" x-text="integrationData.library_count"></span><span class="metric-label">Libraries</span></div>
                            <div class="metric-block" x-show="metricVisible('item_count')"><span class="metric-value" x-text="integrationData.item_count"></span><span class="metric-label">Items</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'immich'">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block" x-show="metricVisible('photos')"><span class="metric-value" x-text="integrationData.photos"></span><span class="metric-label">Photos</span></div>
                            <div class="metric-block" x-show="metricVisible('videos')"><span class="metric-value" x-text="integrationData.videos"></span><span class="metric-label">Videos</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'tdarr'">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block" x-show="metricVisible('queue_size')"><span class="metric-value" x-text="integrationData.queue_size"></span><span class="metric-label">Staged</span></div>
                            <div class="metric-block" x-show="metricVisible('workers')"><span class="metric-value" x-text="integrationData.workers"></span><span class="metric-label">Workers</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'maintainerr'">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block" x-show="metricVisible('issue_count')"><span class="metric-value" x-text="integrationData.issue_count"></span><span class="metric-label">Issues</span></div>
                            <div class="metric-block" x-show="metricVisible('rule_count')"><span class="metric-value" x-text="integrationData.rule_count"></span><span class="metric-label">Rules</span></div>
                        </div>
                    </template>
                    <template x-if="integrationData.type === 'frigate'">
                        <div class="nested-metrics-grid cols-2">
                            <div class="metric-block"><span class="metric-value"><span x-text="integrationData.cameras_up"></span>/<span x-text="integrationData.cameras_total"></span></span><span class="metric-label">Cameras</span></div>
                            <div class="metric-block" x-show="metricVisible('detection_fps')"><span class="metric-value" x-text="integrationData.detection_fps"></span><span class="metric-label">Det. FPS</span></div>
                        </div>
                    </template>
                    </div>
                </div>"#,
                    integration_class, app.id, csrf_token, app.id
                );
            }
        }

        let visible_metrics_attr = escape_html(&app.integration_visible_metrics);
        let visible_metrics_js = if app.integration_visible_metrics.trim().is_empty() {
            "null".to_string()
        } else {
            app.integration_visible_metrics.clone()
        };
        let alpine_init = if is_admin && !app.integration_type.is_empty() && !api_metrics_hidden {
            format!(
                r#"data-integration-visible-metrics="{}" x-data="{{ integrationData: null, visibleMetrics: {visible_metrics_js}, metricVisible(key) {{ const v = this.visibleMetrics; if (v === null || v === undefined) return true; if (Array.isArray(v) && v.length === 0) return false; return v.includes(key); }} }}" data-integration-refresh="{}" x-init="fetch('/api/apps/{}/integration').then(r => r.ok ? r.json() : null).then(d => {{ if (d && d.type) integrationData = d }}).catch(() => {{}})""#,
                visible_metrics_attr, app.id, app.id
            )
        } else {
            String::new()
        };

        let container_aliases = alias_tokens.join(" ");
        let is_host_agent_app = alias_tokens
            .iter()
            .any(|t| t == "proxmox" || t == "pve" || t == "beszel" || t == "filebrowser");
        let guest_compact_class = if !is_admin {
            " app-card--guest-compact"
        } else {
            ""
        };

        let span_class = if is_admin {
            match app.card_span.as_str() {
                "2x1" => " span-2",
                "1x2" => " span-tall",
                _ => "",
            }
        } else {
            ""
        };
        let (chrome_class, icon_class) = match theme_id {
            "glow-glass" => (" glow-glass-card", "app-card-icon gg-icon-frame"),
            "neumorphism" => (" neumorphism-card", "app-card-icon nm-icon-well"),
            _ => ("", "app-card-icon"),
        };

        let card_description = if is_admin {
            escape_html(&app.description)
        } else {
            String::new()
        };

        let metrics_slot = if use_filled {
            format!(
                r#"<div class="app-card-metrics-slot app-card-metrics-slot--filled">{}</div>"#,
                integration_widget
            )
        } else if app.integration_type == "rss" {
            if sub_metrics.is_empty() {
                integration_widget.clone()
            } else {
                format!("{}{}", sub_metrics, integration_widget)
            }
        } else if sub_metrics.is_empty() && integration_widget.is_empty() {
            String::new()
        } else if integration_widget.is_empty() {
            format!(
                r#"<div class="app-card-metrics-slot">{}</div>"#,
                sub_metrics
            )
        } else if sub_metrics.is_empty() {
            format!(
                r#"<div class="app-card-metrics-slot app-card-metrics-slot--solo">{}</div>"#,
                integration_widget
            )
        } else {
            format!(
                r#"<div class="app-card-metrics-slot app-card-metrics-slot--dual">{}{}</div>"#,
                sub_metrics, integration_widget
            )
        };

        let open_url =
            if iframe_embeds_enabled && (app.embed_mode == "iframe" || app.embed_mode == "tab") {
                format!("/embed/{}", app.id)
            } else {
                app.url.clone()
            };
        let link_target = if app.embed_mode == "tab" && iframe_embeds_enabled {
            ""
        } else {
            r#" target="_blank" rel="noopener noreferrer""#
        };
        let embed_mode_attr = if iframe_embeds_enabled && !app.embed_mode.is_empty() {
            format!(r#" data-embed-mode="{}""#, escape_html(&app.embed_mode))
        } else {
            String::new()
        };

        let card = format!(
            r#"
            <div class="glass-panel app-card{}{}{}" data-app-name="{}" data-app-id="{}" data-category="{}" data-node-tag="{}" data-container-aliases="{}" data-host-agent-app="{}" data-show-container-metrics="{}" data-integration-visible-metrics="{}" {}>
                <div class="app-card-header">
                    <a href="{}"{}{} class="app-card-identity app-card-open" style="text-decoration:none; color:inherit;">
                        <div class="{}">
                            <img src="{}" onerror="this.src='/static/fallback.svg'">
                        </div>
                        <div>
                            <h3 class="app-card-title">{}</h3>
                            <p class="app-card-desc">{}</p>
                        </div>
                    </a>
                    <div style="display: flex; align-items: center; gap: 0.5rem;" class="app-card-badges">
                        {}
                        {}
                        {}
                    </div>
                </div>
                {}
            </div>"#,
            guest_compact_class,
            span_class,
            chrome_class,
            escape_html(&name_lower),
            app.id,
            escape_html(&cat_slug),
            escape_html(&app.node_tag),
            escape_html(&container_aliases),
            if is_host_agent_app { "true" } else { "false" },
            if app.show_container_metrics {
                "true"
            } else {
                "false"
            },
            visible_metrics_attr,
            alpine_init,
            escape_html(&open_url),
            link_target,
            embed_mode_attr,
            icon_class,
            escape_html(&brand_logo),
            escape_html(&app.name),
            card_description,
            status_badge,
            ctrl_container,
            delete_btn,
            metrics_slot
        );
        cards_html.push_str(&card);
    }

    cards_html
}

fn feed_category_meta(name: &str, feed_categories: &[serde_json::Value]) -> (String, String) {
    for cat in feed_categories {
        if cat.get("name").and_then(|v| v.as_str()) == Some(name) {
            let color = cat
                .get("color")
                .and_then(|v| v.as_str())
                .unwrap_or("#64748b")
                .to_string();
            let icon = cat
                .get("icon")
                .and_then(|v| v.as_str())
                .unwrap_or("rss")
                .to_string();
            return (color, icon);
        }
    }
    ("#64748b".to_string(), "rss".to_string())
}

fn render_feeds_grid(
    apps: &[App],
    is_admin: bool,
    csrf_attr: &str,
    logo_manifest: &HashMap<String, String>,
    feed_categories: &[serde_json::Value],
) -> String {
    if apps.is_empty() {
        return r#"
        <div class="glass-panel feed-card feed-card--empty">
            <p style="font-weight: 600; color: var(--text-secondary);">No RSS feeds configured yet</p>
            <p style="font-size: 0.8rem; color: var(--text-muted); margin-top: 0.5rem;">Admins: add feeds under <a href="/admin/settings?tab=rss" style="color:var(--accent-color);">Settings → RSS Feeds</a>.</p>
        </div>"#
            .to_string();
    }

    let mut cards_html = String::new();
    for app in apps.iter() {
        let feed_url =
            crate::secrets::decrypt_value(&app.api_key).unwrap_or_else(|_| app.api_key.clone());
        let feed_logo = resolve_feed_logo(&app.icon, &app.name, &app.url, &feed_url, logo_manifest);
        let cat_slug = category_slug(&app.category);
        let (cat_color, _cat_icon) = feed_category_meta(&app.category, feed_categories);
        let host = host_from_url(if app.url.is_empty() {
            &app.name
        } else {
            &app.url
        });
        let site_href = if app.url.is_empty() {
            "#".to_string()
        } else {
            escape_html(&app.url)
        };

        let admin_actions = if is_admin {
            format!(
                r#"
                <div class="feed-card-actions">
                    <a href="/admin/settings?tab=rss" class="btn-edit-app" title="Edit in RSS Feeds settings">
                        <i data-lucide="edit-2"></i>
                    </a>
                    <form action="/apps/delete" method="POST" style="margin:0; display:inline-flex;">
                        <input type="hidden" name="id" value="{}">
                        <input type="hidden" name="csrf_token" value="{}">
                        <button type="submit" class="btn-delete-app" title="Delete feed">
                            <i data-lucide="trash-2"></i>
                        </button>
                    </form>
                </div>"#,
                app.id, csrf_attr
            )
        } else {
            String::new()
        };

        let category_pill = format!(
            r#"<span class="feed-category-pill" style="--pill-color:{};">{}</span>"#,
            escape_html(&cat_color),
            escape_html(&app.category)
        );

        let card = format!(
            r#"
            <article class="glass-panel feed-card" data-app-id="{}" data-category="{}" data-app-name="{}" x-data="{{ integrationData: null }}" x-init="fetch('/api/apps/{}/integration').then(r => r.ok ? r.json() : null).then(d => {{ if (d && d.type) integrationData = d }}).catch(() => {{}})">
                <header class="feed-card-header">
                    <a href="{}" target="_blank" rel="noopener noreferrer" class="feed-card-identity">
                        <div class="feed-card-icon">
                            <img src="{}" alt="" onerror="this.src='/static/feeds/icons/rss.svg'">
                        </div>
                        <div class="feed-card-meta">
                            <h3 class="feed-card-title">{}</h3>
                            <span class="feed-card-host">{}</span>
                        </div>
                    </a>
                    <div class="feed-card-badges">
                        {}
                        {}
                    </div>
                </header>
                <div class="integration-widget feed-card-body" x-show="integrationData && integrationData.type === 'rss'">
                    <div class="rss-feed-list">
                        <template x-for="(entry, index) in integrationData.entries" :key="index">
                            <a x-show="entry.link" :href="entry.link" target="_blank" rel="noopener" class="rss-feed-item">
                                <span class="rss-feed-title" x-text="entry.title"></span>
                                <span class="rss-feed-date" x-text="entry.date"></span>
                            </a>
                            <div x-show="!entry.link" class="rss-feed-item rss-feed-item--text-only">
                                <span class="rss-feed-title" x-text="entry.title"></span>
                                <span class="rss-feed-date" x-text="entry.date"></span>
                            </div>
                        </template>
                        <div x-show="!integrationData || !integrationData.entries || integrationData.entries.length === 0" class="rss-feed-empty">
                            <span>Loading headlines…</span>
                        </div>
                    </div>
                </div>
                <footer class="feed-card-footer">
                    <a href="{}" target="_blank" rel="noopener noreferrer" class="feed-card-visit">Visit site <i data-lucide="external-link"></i></a>
                </footer>
            </article>"#,
            app.id,
            escape_html(&cat_slug),
            escape_html(&app.name.to_lowercase()),
            app.id,
            site_href,
            escape_html(&feed_logo),
            escape_html(&app.name),
            escape_html(if host.is_empty() { "news feed" } else { &host }),
            category_pill,
            admin_actions,
            site_href,
        );
        cards_html.push_str(&card);
    }

    cards_html
}

fn render_feed_category_tabs(apps: &[App], feed_categories: &[serde_json::Value]) -> String {
    let mut category_tabs_html = format!(
        r#"<button type="button" class="filter-tab feed-filter-tab active" @click="filterCategory('all', $el)"><i data-lucide="layers"></i> All <span class="filter-count">{}</span></button>"#,
        apps.len()
    );

    for cat in feed_categories {
        let name = cat.get("name").and_then(|v| v.as_str()).unwrap_or("");
        if name.is_empty() {
            continue;
        }
        let count = apps.iter().filter(|a| a.category == name).count();
        if count == 0 {
            continue;
        }
        let icon = cat.get("icon").and_then(|v| v.as_str()).unwrap_or("rss");
        let color = cat
            .get("color")
            .and_then(|v| v.as_str())
            .unwrap_or("#64748b");
        let cat_slug = category_slug(name);
        category_tabs_html.push_str(&format!(
            r#"<button type="button" class="filter-tab feed-filter-tab" style="--tab-accent: {};" @click="filterCategory('{}', $el)"><i data-lucide="{}"></i> {} <span class="filter-count">{}</span></button>"#,
            escape_html(color),
            escape_html(&cat_slug),
            escape_html(icon),
            escape_html(name),
            count
        ));
    }

    category_tabs_html
}

fn render_auth_buttons(
    session: &Option<Session>,
    csrf_attr: &str,
    mode: PageMode,
    kiosk_mode: bool,
) -> String {
    let status_link = r#"<a href="/status" class="glass-panel topbar-action"><i data-lucide="activity"></i> Status</a>"#;
    if kiosk_mode && session.is_none() {
        return status_link.to_string();
    }
    if let Some(ref sess) = session {
        let admin_settings_btn = if sess.role == "Admin" {
            match mode {
                PageMode::Feeds => {
                    r#"
            <a href="/admin/settings?tab=rss" class="glass-panel topbar-action btn-admin">
                <i data-lucide="rss"></i> Add RSS Feed
            </a>
            <a href="/admin/settings" class="glass-panel topbar-action btn-admin">
                <i data-lucide="sliders-horizontal"></i> Settings
            </a>
            "#
                }
                PageMode::Dashboard => {
                    r#"
            <button type="button" class="glass-panel topbar-action btn-admin" @click="addAppModalOpen = true; appIconUrl = ''; newApp = { integration_type: '', api_key: '', card_span: '1x1', show_container_metrics: true, guest_visible: true, embed_mode: 'link', show_integration_metrics: true, integration_visible_metrics: [] };">
                <i data-lucide="plus"></i> Add App
            </button>
            <a href="/admin/settings" class="glass-panel topbar-action btn-admin">
                <i data-lucide="sliders-horizontal"></i> Settings
            </a>
            "#
                }
            }
        } else {
            ""
        };
        format!(
            r#"
            {}
            <form action="/logout" method="POST" style="margin:0; display:inline-flex;">
                <input type="hidden" name="csrf_token" value="{}">
                <button type="submit" class="glass-panel topbar-action">
                    <i data-lucide="log-out"></i> Sign Out ({})
                </button>
            </form>
            "#,
            admin_settings_btn,
            csrf_attr,
            escape_html(&sess.username)
        )
    } else {
        r#"
        <a href="/login" class="glass-panel topbar-action topbar-action--accent">
            <i data-lucide="key-round"></i> Sign In
        </a>
        "#
        .to_string()
    }
}

fn category_slug(category: &str) -> String {
    category
        .to_lowercase()
        .replace(' ', "-")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect()
}

fn render_streams(
    apps: &[App],
    session: &Option<Session>,
    is_admin: bool,
    _csrf_attr: &str,
    _logo_manifest: &HashMap<String, String>,
) -> String {
    if session.is_none() {
        return String::new();
    }

    let has_plex = apps
        .iter()
        .any(|app| app.integration_type == "plex" || is_plex_app(app));
    let has_jellyfin = apps.iter().any(|app| {
        matches!(app.integration_type.as_str(), "jellyfin" | "emby") || is_jellyfin_app(app)
    });

    if !has_plex && !has_jellyfin {
        return String::new();
    }

    let mut html = String::new();

    let mut media_cards = String::new();

    if has_plex {
        media_cards.push_str(r#"
            <!-- Plex stream card -->
            <div class="glass-panel stream-card">
                <div class="stream-main">
                    <div class="stream-meta">
                        <div class="stream-icon">
                            <i data-lucide="play" style="color: #ff9900;"></i>
                        </div>
                        <div>
                            <h2 class="stream-text-title">Plex</h2>
                            <p class="stream-text-desc">Watch movies and TV shows.</p>
                        </div>
                    </div>
                    <span class="status-badge status-checking stream-status-badge" data-stream-app="plex" data-stream-service="plex">CHECKING...</span>
                </div>
                
                <div class="stream-player">
                    <div class="stream-controls-row">
                        <span class="stream-track-title" id="plex-track">No Active Streams</span>
                        <div style="display: flex; gap: 0.5rem; align-items: center;">
                            <button type="button" class="stream-play-btn"><i data-lucide="play" style="width:0.85rem; height:0.85rem;"></i></button>
                            <span id="plex-timer">-</span>
                        </div>
                    </div>
                    <div class="stream-progress-track">
                        <div class="stream-progress-fill" id="plex-progress" style="width: 0%;"></div>
                    </div>
                </div>
            </div>
            "#);
    }
    if has_jellyfin {
        let playback_controls = if is_admin {
            r#"<span class="stream-playback-controls" id="jellyfin-controls" style="display: none;">
                                <button type="button" class="stream-ctrl-btn" data-jf-command="unpause" title="Resume" style="display: none;"><i data-lucide="play"></i></button>
                                <button type="button" class="stream-ctrl-btn" data-jf-command="pause" title="Pause"><i data-lucide="pause"></i></button>
                                <button type="button" class="stream-ctrl-btn" data-jf-command="stop" title="Stop"><i data-lucide="square"></i></button>
                            </span>"#
        } else {
            ""
        };
        media_cards.push_str(&format!(r#"
            <!-- Jellyfin/Emby stream card -->
            <div class="glass-panel stream-card">
                <div class="stream-main">
                    <div class="stream-meta">
                        <div class="stream-icon">
                            <i data-lucide="play-circle" style="color: #10b981;"></i>
                        </div>
                        <div>
                            <h2 class="stream-text-title">Jellyfin</h2>
                            <p class="stream-text-desc">Watch movies and TV shows.</p>
                        </div>
                    </div>
                    <span class="status-badge status-checking stream-status-badge" data-stream-app="jellyfin emby" data-stream-service="jellyfin">CHECKING...</span>
                </div>
                
                <div class="stream-player">
                    <img id="jellyfin-poster" class="stream-poster" alt="" draggable="false" style="display: none;">
                    <div class="stream-player-info">
                        <div class="stream-controls-row">
                            <span class="stream-track-title" id="jellyfin-track" style="color: var(--text-muted);">No Active Streams</span>
                            <span class="stream-actions">{playback_controls}<span id="jellyfin-timer">-</span></span>
                        </div>
                        <div class="stream-progress-track">
                            <div class="stream-progress-fill" id="jellyfin-progress" style="width: 0%;"></div>
                        </div>
                    </div>
                </div>
            </div>
            "#));
    }

    if has_plex || has_jellyfin {
        let media_count = (has_plex as u8) + (has_jellyfin as u8);
        let cols_class = if media_count > 1 {
            "streams-row"
        } else {
            "streams-row single-col"
        };
        html.push_str(&format!(
            r#"<section class="{}" data-filter-section="media">{}</section>"#,
            cols_class, media_cards
        ));
    }

    html
}

#[allow(dead_code)]
fn render_proxmox_stream_card(
    app: &App,
    is_admin: bool,
    csrf_attr: &str,
    logo_manifest: &HashMap<String, String>,
) -> String {
    let lowercase_icon = app.icon.to_lowercase();
    let resolved_logo = resolve_logo_from_manifest(&app.icon, logo_manifest);
    let brand_logo = if !resolved_logo.is_empty() {
        resolved_logo
    } else if !lowercase_icon.is_empty() {
        fallback_brand_logo(&lowercase_icon)
    } else {
        "/static/logos/proxmox.svg".to_string()
    };

    let desc = if app.description.trim().is_empty() {
        "Hypervisor node — containers, CPU, and memory at a glance.".to_string()
    } else {
        app.description.clone()
    };

    let admin_actions = if is_admin {
        let mut app_for_json = app.clone();
        if !app_for_json.api_key.is_empty() {
            app_for_json.api_key = "Configured — leave blank to keep unchanged".to_string();
        }
        let app_json = serde_json::to_string(&app_for_json).unwrap_or_default();
        let escaped_json = app_json
            .replace('&', "&amp;")
            .replace('"', "&quot;")
            .replace('\'', "&#39;");
        format!(
            r#"<div style="display: inline-flex; align-items: center; gap: 0.25rem; margin-left: 0.35rem;">
                <button type="button" class="btn-edit-app" title="Edit application" data-app="{}" @click="editApp = JSON.parse($el.getAttribute('data-app')); window.editingAppOriginalName = (editApp.name || '').toLowerCase(); if (window.amudHydrateAppMetrics) amudHydrateAppMetrics(editApp); editAppModalOpen = true; setTimeout(checkDuplicateAppName, 0); setTimeout(function(){{ if (window.amudRefreshIntegrationPicker) window.amudRefreshIntegrationPicker('edit-app-integration-picker', editApp.integration_type || ''); }}, 50);">
                    <i data-lucide="edit-2"></i>
                </button>
                <form action="/apps/delete" method="POST" style="margin: 0; display: inline-flex; align-items: center;">
                    <input type="hidden" name="id" value="{}">
                    <input type="hidden" name="csrf_token" value="{}">
                    <button type="submit" class="btn-delete-app" title="Delete application">
                        <i data-lucide="trash-2"></i>
                    </button>
                </form>
            </div>"#,
            escaped_json, app.id, csrf_attr
        )
    } else {
        String::new()
    };

    format!(
        r#"
            <!-- Proxmox host stream card -->
            <div class="glass-panel stream-card" id="proxmox-host-card" data-proxmox-host="true">
                <div class="stream-main">
                    <a href="{url}" target="_blank" rel="noopener noreferrer" class="stream-meta" style="text-decoration:none; color:inherit;">
                        <div class="stream-icon stream-icon--logo">
                            <img src="{logo}" alt="" onerror="this.src='/static/logos/proxmox.svg'">
                        </div>
                        <div>
                            <h2 class="stream-text-title">{name}</h2>
                            <p class="stream-text-desc">{desc}</p>
                        </div>
                    </a>
                    <div style="display:flex; align-items:center; gap:0.35rem; flex-shrink:0;">
                        <span class="status-badge status-checking stream-status-badge" data-proxmox-badge title="Proxmox agent telemetry">CHECKING...</span>
                        {admin_actions}
                    </div>
                </div>
                <div class="stream-player">
                    <div class="stream-controls-row">
                        <span class="stream-track-title" id="proxmox-host-summary" style="color: var(--text-muted);">Waiting for agent telemetry…</span>
                        <span id="proxmox-host-node">—</span>
                    </div>
                    <div class="nested-metrics-grid cols-3 stream-host-metrics" id="proxmox-host-metrics">
                        <div class="metric-block">
                            <span class="metric-value">—</span>
                            <span class="metric-label">Containers</span>
                        </div>
                        <div class="metric-block">
                            <span class="metric-value">—</span>
                            <span class="metric-label">CPU</span>
                        </div>
                        <div class="metric-block">
                            <span class="metric-value">—</span>
                            <span class="metric-label">Mem</span>
                        </div>
                    </div>
                </div>
            </div>
        "#,
        url = escape_html(&app.url),
        logo = escape_html(&brand_logo),
        name = escape_html(&app.name),
        desc = escape_html(&desc),
        admin_actions = admin_actions,
    )
}

fn render_category_tabs(apps: &[App]) -> String {
    let mut categories = Vec::<String>::new();
    for app in apps.iter() {
        if !app.category.is_empty() && !categories.contains(&app.category) {
            categories.push(app.category.clone());
        }
    }

    let mut category_tabs_html = format!(
        r#"<button type="button" class="filter-tab active" @click="filterCategory('all', $el)">All <span class="filter-count">{}</span></button>"#,
        apps.len()
    );
    for cat in categories.iter() {
        let count = apps.iter().filter(|a| &a.category == cat).count();
        let cat_slug: String = cat
            .to_lowercase()
            .replace(' ', "-")
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
        category_tabs_html.push_str(&format!(
            r#"<button type="button" class="filter-tab" @click="filterCategory('{}', $el)">{} <span class="filter-count">{}</span></button>"#,
            escape_html(&cat_slug), escape_html(cat), count
        ));
    }
    category_tabs_html
}

fn render_category_sections(apps: &[App]) -> String {
    let mut categories = Vec::<String>::new();
    for app in apps.iter() {
        if !app.category.is_empty() && !categories.contains(&app.category) {
            categories.push(app.category.clone());
        }
    }
    if categories.is_empty() {
        categories.push("General".to_string());
    }
    let mut html = String::from(r#"<div class="category-sections">"#);
    html.push_str(
        r#"<details class="category-section" open data-filter-section="all"><summary class="category-section-title">All apps</summary></details>"#,
    );
    for cat in &categories {
        let cat_slug = category_slug(cat);
        let count = apps.iter().filter(|a| &a.category == cat).count();
        html.push_str(&format!(
            r#"<details class="category-section" open data-filter-section="{}"><summary class="category-section-title">{} <span class="filter-count">{}</span></summary></details>"#,
            escape_html(&cat_slug),
            escape_html(cat),
            count
        ));
    }
    html.push_str("</div>");
    html
}

fn render_support_section(settings: &HashMap<String, String>) -> String {
    let donate_enabled = settings
        .get("donate_enabled")
        .map(|s| s.as_str())
        .unwrap_or("1");
    if donate_enabled == "1" {
        let mut links = String::new();
        for (url, label, icon) in DONATION_LINKS.iter() {
            links.push_str(&format!(
                r#"<a href="{}" target="_blank" rel="noopener noreferrer" class="support-link"><i data-lucide="{}" style="width:1rem; height:1rem;"></i> {}</a>"#,
                url, icon, label
            ));
        }
        format!(
            r#"<section class="support-section">
                <div class="glass-panel support-card">
                    <div class="support-head">
                        <i data-lucide="heart" style="color:var(--accent-color); width:1.2rem; height:1.2rem;"></i>
                        <h2>Support AMUD</h2>
                    </div>
                    <p class="support-msg">{}</p>
                    <div class="support-links">{}</div>
                </div>
            </section>"#,
            DONATION_MESSAGE, links
        )
    } else {
        String::new()
    }
}

fn render_wol_devices(
    devices: &[crate::models::WolDevice],
    is_admin: bool,
    _csrf_attr: &str,
) -> String {
    if devices.is_empty() {
        return String::new();
    }

    let mut cards = String::new();
    for dev in devices {
        let icon_name = if dev.icon.trim().is_empty() {
            "cpu"
        } else {
            &dev.icon
        };
        let ip_info = if dev.ip_address.trim().is_empty() {
            "".to_string()
        } else {
            format!(
                r#"<span class="wol-ip" style="font-size:0.75rem; color:var(--text-muted);">({})</span>"#,
                escape_html(&dev.ip_address)
            )
        };

        let action_btn = if is_admin {
            format!(
                r#"<button type="button" class="btn-wake-app" title="Wake Device" data-wake-id="{}" style="background:var(--accent-glow); border:1px solid rgba(255,255,255,0.06); color:#fff; cursor:pointer; padding:0.4rem 0.8rem; border-radius:6px; display:inline-flex; align-items:center; gap:0.35rem; font-size:0.78rem; font-weight:600;" @click="triggerWakeAction($el)">
                    <i data-lucide="power" style="width:0.9rem; height:0.9rem;"></i> Wake
                </button>"#,
                dev.id
            )
        } else {
            "".to_string()
        };

        cards.push_str(&format!(
            r#"
            <div class="glass-panel wol-card" style="display:flex; align-items:center; justify-content:space-between; padding:0.75rem 1rem; border-radius:var(--radius-xl); background:rgba(255,255,255,0.01); border:1px solid rgba(255,255,255,0.04); min-width:240px; flex:1;">
                <div style="display:flex; align-items:center; gap:0.75rem;">
                    <div class="wol-icon-wrapper" style="width:2.2rem; height:2.2rem; border-radius:8px; background:rgba(255,255,255,0.02); display:flex; align-items:center; justify-content:center; color:var(--accent-color);">
                        <i data-lucide="{}"></i>
                    </div>
                    <div style="display:flex; flex-direction:column;">
                        <span class="wol-name" style="font-weight:600; font-size:0.88rem; color:#fff;">{}</span>
                        <div style="display:flex; align-items:center; gap:0.3rem;">
                            <span class="wol-mac" style="font-size:0.72rem; color:var(--text-muted); font-family:monospace;">{}</span>
                            {}
                        </div>
                    </div>
                </div>
                {}
            </div>
            "#,
            escape_html(icon_name),
            escape_html(&dev.name),
            escape_html(&dev.mac_address),
            ip_info,
            action_btn
        ));
    }

    format!(
        r#"
        <section class="wol-section" style="margin-bottom:2rem;">
            <div style="display:flex; align-items:center; gap:0.5rem; margin-bottom:0.75rem;">
                <i data-lucide="zap" style="color:var(--accent-color); width:1.2rem; height:1.2rem;"></i>
                <h2 style="font-size:1.1rem; font-weight:700; color:#fff; margin:0;">Power Controls (Wake-on-LAN)</h2>
            </div>
            <div class="wol-grid" style="display:flex; flex-wrap:wrap; gap:0.75rem;">
                {}
            </div>
        </section>
        "#,
        cards
    )
}

fn safe_weather_coord(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.parse::<f64>().is_ok() {
        escape_html(trimmed)
    } else {
        String::new()
    }
}
