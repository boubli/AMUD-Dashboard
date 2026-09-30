pub mod activity;
pub mod agent;
pub mod apps;
pub mod audit;
pub mod auth;
pub mod boards;
pub mod calendar;
pub mod custom_api;
pub mod db;
pub mod feed_icons;
pub mod fritz;
pub mod handlers;
pub mod homarr_import;
pub mod homelab;
pub mod homepage_import;
pub mod http_client;
pub mod integration_cache;
pub mod integration_coordinator;
pub mod integration_registry;
pub mod integrations;
pub mod integrations_longtail;
pub mod ldap_auth;
pub mod logos;
pub mod media;
pub mod models;
pub mod rss_discover;
pub mod secrets;
pub mod security;
pub mod settings;
pub mod smart_home;
pub mod telemetry_broadcast;
pub mod templates;
pub mod webhooks;

#[cfg(feature = "integration-tests")]
pub mod integration_test_exports {
    use crate::activity;
    use crate::models::AppState;
    use crate::settings;
    use crate::telemetry_broadcast;
    use rusqlite::Connection;
    use std::sync::Arc;
    use tokio::sync::watch;

    pub use crate::activity::MODE_DEEP_IDLE;

    pub fn is_active(state: &AppState) -> bool {
        activity::is_active(state)
    }

    pub fn is_deep_idle(state: &AppState) -> bool {
        activity::is_deep_idle(state)
    }

    pub fn signal_ws_connected(state: &Arc<AppState>) {
        activity::signal_ws_connected(state);
    }

    pub fn apply_performance_preset(db: &Connection, preset: &str) {
        settings::apply_performance_preset(db, preset);
    }

    pub fn new_telemetry_broadcast(
    ) -> watch::Sender<Arc<crate::telemetry_broadcast::WsTelemetryBundle>> {
        telemetry_broadcast::new_telemetry_broadcast()
    }
}

use activity::{start_activity_supervisor, start_alert_evaluator};
use agent::start_agent_listener;
use auth::{
    generate_bootstrap_password, hash_password, resolve_agent_secret, security_headers,
    start_session_cleanup,
};
use db::refresh_settings_cache;
use handlers::*;
use http_client::build_shared_http_clients;
use integration_cache::IntegrationCache;
use integration_coordinator::start_integration_coordinator;
use logos::build_logo_manifest;
use media::start_media_poller;
use models::{ActionResult, AgentTelemetry, AppState, Session};
use settings::get_default_settings;
use smart_home::start_ha_polling;
use telemetry_broadcast::{new_telemetry_broadcast, start_telemetry_broadcaster};
use webhooks::start_status_poller;

use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, AtomicUsize};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use tokio::net::TcpListener as TokioTcpListener;

fn start_action_results_cleanup(action_results: Arc<RwLock<HashMap<String, ActionResult>>>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            action_results
                .write()
                .unwrap()
                .retain(|_, result| result.at.elapsed() < Duration::from_secs(120));
        }
    });
}

pub async fn run() {
    println!("AMUD Server starting up in Rust...");

    let db_path = std::env::var("DB_PATH").unwrap_or_else(|_| "data/amud.db".to_string());
    let db_parent = std::path::Path::new(&db_path)
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| std::path::PathBuf::from("data"));
    std::fs::create_dir_all(&db_parent).unwrap_or_else(|e| {
        panic!(
            "Failed to create data directory {}: {e}{}",
            db_parent.display(),
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                secrets::permission_denied_hint_for_path(&db_parent)
            } else {
                String::new()
            }
        );
    });
    secrets::init_secrets_key(&db_path).expect("Failed to initialize AMUD secrets encryption key");
    let conn = Connection::open(&db_path).expect("Failed to open SQLite database");
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA foreign_keys=ON;",
    )
    .expect("Failed to configure SQLite database pragmas");
    let migrated_secrets = secrets::migrate_plaintext_secrets(&conn);
    if migrated_secrets > 0 {
        println!("Encrypted {migrated_secrets} legacy plaintext secret(s) at rest.");
    }

    conn.execute(
        "CREATE TABLE IF NOT EXISTS apps (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL,
        url TEXT NOT NULL,
        icon TEXT,
        description TEXT,
        category TEXT DEFAULT 'General',
        node_tag TEXT DEFAULT 'Local'
    );",
        [],
    )
    .unwrap();

    let _ = conn.execute(
        "ALTER TABLE apps ADD COLUMN mac_address TEXT DEFAULT ''",
        [],
    );

    let _ = conn.execute(
        "ALTER TABLE apps ADD COLUMN integration_type TEXT DEFAULT ''",
        [],
    );

    let _ = conn.execute("ALTER TABLE apps ADD COLUMN api_key TEXT DEFAULT ''", []);

    let _ = conn.execute(
        "ALTER TABLE apps ADD COLUMN sort_order INTEGER DEFAULT 0",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE apps ADD COLUMN card_span TEXT DEFAULT '1x1'",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE apps ADD COLUMN show_container_metrics INTEGER DEFAULT 1",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE apps ADD COLUMN guest_visible INTEGER DEFAULT 1",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE apps ADD COLUMN embed_mode TEXT DEFAULT 'link'",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE apps ADD COLUMN integration_visible_metrics TEXT NOT NULL DEFAULT ''",
        [],
    );
    let _ = conn.execute(
        "UPDATE apps SET integration_visible_metrics = '[]' WHERE integration_type IN ('jellyfin','plex','emby') AND (integration_visible_metrics IS NULL OR TRIM(integration_visible_metrics) = '')",
        [],
    );

    conn.execute(
        "CREATE TABLE IF NOT EXISTS dashboard_widgets (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        widget_type TEXT NOT NULL DEFAULT 'note',
        title TEXT NOT NULL DEFAULT '',
        content TEXT NOT NULL DEFAULT '',
        sort_order INTEGER DEFAULT 0,
        guest_visible INTEGER NOT NULL DEFAULT 1,
        grid_span TEXT DEFAULT '1x1'
    );",
        [],
    )
    .unwrap();

    conn.execute(
        "CREATE TABLE IF NOT EXISTS api_tokens (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL,
        token_hash TEXT NOT NULL UNIQUE,
        scopes TEXT NOT NULL DEFAULT 'read:apps',
        created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
        expires_at DATETIME
    );",
        [],
    )
    .unwrap();

    conn.execute(
        "CREATE TABLE IF NOT EXISTS share_links (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        token TEXT NOT NULL UNIQUE,
        label TEXT NOT NULL DEFAULT '',
        allowed_paths TEXT NOT NULL DEFAULT '/',
        expires_at DATETIME,
        created_at DATETIME DEFAULT CURRENT_TIMESTAMP
    );",
        [],
    )
    .unwrap();

    conn.execute(
        "CREATE TABLE IF NOT EXISTS wol_devices (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL,
        mac_address TEXT NOT NULL,
        ip_address TEXT DEFAULT '',
        icon TEXT DEFAULT ''
    );",
        [],
    )
    .unwrap();

    crate::boards::ensure_dashboards_table(&conn);

    // Migrate existing apps with MAC address configured
    if let Ok(mut stmt) = conn.prepare("SELECT name, mac_address, icon FROM apps WHERE mac_address IS NOT NULL AND TRIM(mac_address) != ''") {
        if let Ok(mut rows) = stmt.query([]) {
            while let Ok(Some(row)) = rows.next() {
                if let (Ok(name), Ok(mac), Ok(icon)) = (row.get::<_, String>(0), row.get::<_, String>(1), row.get::<_, String>(2)) {
                    let mut check_stmt = conn.prepare("SELECT COUNT(*) FROM wol_devices WHERE mac_address = ?").unwrap();
                    let exists: i64 = check_stmt.query_row(params![mac], |r| r.get(0)).unwrap_or(0);
                    if exists == 0 {
                        let device_name = format!("WOL: {}", name);
                        conn.execute(
                            "INSERT INTO wol_devices (name, mac_address, ip_address, icon) VALUES (?, ?, '', ?)",
                            params![device_name, mac, icon],
                        ).ok();
                    }
                }
            }
        }
    }
    let _ = conn.execute("UPDATE apps SET mac_address = ''", []);

    conn.execute(
        "CREATE TABLE IF NOT EXISTS users (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        username TEXT UNIQUE NOT NULL,
        password_hash TEXT NOT NULL,
        role TEXT NOT NULL DEFAULT 'Guest'
    );",
        [],
    )
    .unwrap();

    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );",
        [],
    )
    .unwrap();

    conn.execute(
        "CREATE TABLE IF NOT EXISTS webhooks (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL,
        url TEXT NOT NULL,
        event_types TEXT NOT NULL,
        is_active INTEGER NOT NULL DEFAULT 1,
        created_at DATETIME DEFAULT CURRENT_TIMESTAMP
    );",
        [],
    )
    .unwrap();

    conn.execute(
        "CREATE TABLE IF NOT EXISTS categories (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT UNIQUE NOT NULL,
        color TEXT DEFAULT '#64748b',
        sort_order INTEGER DEFAULT 0
    );",
        [],
    )
    .unwrap();

    conn.execute(
        "CREATE TABLE IF NOT EXISTS feed_categories (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT UNIQUE NOT NULL,
        color TEXT DEFAULT '#64748b',
        icon TEXT DEFAULT 'rss',
        sort_order INTEGER DEFAULT 0
    );",
        [],
    )
    .unwrap();

    conn.execute(
        "CREATE TABLE IF NOT EXISTS audit_log (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
        username TEXT NOT NULL,
        action TEXT NOT NULL,
        target TEXT NOT NULL,
        details TEXT NOT NULL DEFAULT '',
        client_ip TEXT NOT NULL DEFAULT ''
    );",
        [],
    )
    .unwrap();

    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_apps_category ON apps(category);
         CREATE INDEX IF NOT EXISTS idx_categories_sort ON categories(sort_order, name);
         CREATE INDEX IF NOT EXISTS idx_feed_categories_sort ON feed_categories(sort_order, name);
         CREATE INDEX IF NOT EXISTS idx_audit_log_created ON audit_log(created_at);
         CREATE INDEX IF NOT EXISTS idx_webhooks_is_active ON webhooks(is_active);
         CREATE INDEX IF NOT EXISTS idx_wol_devices_mac ON wol_devices(mac_address);",
    )
    .unwrap();

    {
        let mut stmt_cats = conn.prepare("SELECT COUNT(*) FROM categories").unwrap();
        let cat_count: i64 = stmt_cats.query_row([], |r| r.get(0)).unwrap();
        if cat_count == 0 {
            println!("Seeding default categories...");
            let defaults = [
                ("General", "#64748b", 0),
                ("Media", "#f97316", 1),
                ("Infrastructure", "#3b82f6", 2),
                ("Monitoring", "#10b981", 3),
                ("Network", "#8b5cf6", 4),
                ("Storage", "#ec4899", 5),
            ];
            for (name, color, order) in defaults {
                conn.execute(
                    "INSERT INTO categories (name, color, sort_order) VALUES (?, ?, ?)",
                    params![name, color, order],
                )
                .ok();
            }
        }
    }

    {
        let mut stmt_fc = conn
            .prepare("SELECT COUNT(*) FROM feed_categories")
            .unwrap();
        let fc_count: i64 = stmt_fc.query_row([], |r| r.get(0)).unwrap();
        if fc_count == 0 {
            println!("Seeding default feed categories...");
            let defaults = [
                ("World News", "#3b82f6", "globe", 0),
                ("Tech", "#8b5cf6", "cpu", 1),
                ("Sports", "#10b981", "trophy", 2),
                ("Entertainment", "#ec4899", "clapperboard", 3),
                ("Science", "#06b6d4", "rocket", 4),
                ("Business", "#f59e0b", "trending-up", 5),
                ("General", "#64748b", "rss", 6),
            ];
            for (name, color, icon, order) in defaults {
                conn.execute(
                    "INSERT INTO feed_categories (name, color, icon, sort_order) VALUES (?, ?, ?, ?)",
                    params![name, color, icon, order],
                )
                .ok();
            }
        }
    }

    {
        println!("Ensuring default settings exist...");
        for (key, val) in get_default_settings() {
            conn.execute(
                "INSERT OR IGNORE INTO settings (key, value) VALUES (?, ?)",
                params![key, val],
            )
            .ok();
        }
        // Seed Proxmox toggle from env var on first boot only
        let pve_seed = if std::env::var("AMUD_ENABLE_PROXMOX").unwrap_or_default() == "true" {
            "1"
        } else {
            "0"
        };
        conn.execute(
            "INSERT OR IGNORE INTO settings (key, value) VALUES ('enable_proxmox', ?)",
            params![pve_seed],
        )
        .ok();
        db::migrate_media_settings_to_apps(&conn);
    }

    {
        let mut stmt_users = conn.prepare("SELECT COUNT(*) FROM users").unwrap();
        let user_count: i64 = stmt_users.query_row([], |r| r.get(0)).unwrap();
        if user_count == 0 {
            let bootstrap_password = generate_bootstrap_password();
            println!("Seeding initial admin account...");
            let admin_hash = hash_password(&bootstrap_password);
            conn.execute(
                "INSERT INTO users (username, password_hash, role) VALUES (?, ?, ?)",
                params!["admin", admin_hash, "Admin"],
            )
            .ok();
            conn.execute(
                "INSERT INTO settings (key, value) VALUES ('admin_must_change_password', '1')
             ON CONFLICT(key) DO UPDATE SET value = '1'",
                [],
            )
            .ok();
            eprintln!("================================================================");
            eprintln!(" AMUD INITIAL ADMIN CREDENTIALS (save this now — shown once)");
            eprintln!("   Username: admin");
            eprintln!("   Password: {}", bootstrap_password);
            eprintln!(" You will be prompted to change this password after first login.");
            eprintln!("================================================================");
        }
    }

    let agent_secret = resolve_agent_secret(&conn);
    if agent_secret.trim().is_empty() {
        eprintln!("FATAL: AMUD_AGENT_SECRET / agent_shared_secret is not configured.");
        std::process::exit(1);
    }

    match audit::ensure_audit_log_schema(&conn) {
        Ok(()) => match audit::audit_health_check(&conn) {
            Ok(count) => println!("Audit log ready ({count} existing entries)"),
            Err(e) => eprintln!("[AUDIT] health check failed: {e}"),
        },
        Err(e) => eprintln!("[AUDIT] schema setup failed: {e}"),
    }

    db::track_installed_version_on_boot(&conn);

    let shared_db = Arc::new(Mutex::new(conn));
    let sessions = Arc::new(RwLock::new(HashMap::<String, Session>::new()));
    let latest_telemetry = Arc::new(RwLock::new(AgentTelemetry::default()));
    let telemetry_by_node = Arc::new(RwLock::new(HashMap::new()));
    let agent_connected = Arc::new(RwLock::new(false));
    let media_streams = Arc::new(RwLock::new(HashMap::new()));
    let app_statuses = Arc::new(RwLock::new(HashMap::new()));
    let agent_sessions = Arc::new(Mutex::new(HashMap::new()));
    let nodes_meta = Arc::new(RwLock::new(HashMap::new()));
    let pve_test_response = Arc::new(RwLock::new(None));
    let docker_discover_response = Arc::new(RwLock::new(None));
    let telemetry_discover_response = Arc::new(RwLock::new(None));
    let share_sessions = Arc::new(RwLock::new(HashMap::new()));
    let action_results = Arc::new(RwLock::new(HashMap::new()));
    let settings_cache = Arc::new(RwLock::new(HashMap::new()));
    {
        let db = shared_db.lock().unwrap();
        refresh_settings_cache(&db, &settings_cache);
    }

    let logo_manifest = Arc::new(build_logo_manifest());
    let telemetry_broadcast = new_telemetry_broadcast();

    let cache_ttl = {
        let cache = settings_cache.read().unwrap();
        cache
            .get("integration_cache_ttl_secs")
            .and_then(|s| s.parse().ok())
            .unwrap_or(45u64)
    };
    let cache_max = {
        let cache = settings_cache.read().unwrap();
        cache
            .get("integration_cache_max_entries")
            .and_then(|s| s.parse().ok())
            .unwrap_or(256usize)
    };
    let integration_cache = Arc::new(IntegrationCache::new(cache_max, cache_ttl));
    let http_clients = Arc::new(build_shared_http_clients());

    let state = Arc::new(AppState {
        db: shared_db.clone(),
        sessions: sessions.clone(),
        latest_telemetry: latest_telemetry.clone(),
        telemetry_by_node: telemetry_by_node.clone(),
        agent_connected: agent_connected.clone(),
        media_streams: media_streams.clone(),
        app_statuses: app_statuses.clone(),
        agent_sessions: agent_sessions.clone(),
        nodes_meta: nodes_meta.clone(),
        next_agent_conn_id: Arc::new(std::sync::atomic::AtomicU64::new(1)),
        pve_test_response: pve_test_response.clone(),
        docker_discover_response: docker_discover_response.clone(),
        telemetry_discover_response: telemetry_discover_response.clone(),
        share_sessions: share_sessions.clone(),
        action_results: action_results.clone(),
        settings_cache: settings_cache.clone(),
        alert_cooldowns: Arc::new(Mutex::new(HashMap::new())),
        login_attempts: Arc::new(Mutex::new(HashMap::new())),
        api_rate_limits: Arc::new(Mutex::new(HashMap::new())),
        agent_secret: Arc::new(agent_secret),
        smart_home_telemetry: Arc::new(RwLock::new(Default::default())),
        logo_manifest: logo_manifest.clone(),
        telemetry_broadcast: telemetry_broadcast.clone(),
        integration_cache: integration_cache.clone(),
        http_clients: http_clients.clone(),
        ws_limited_clients: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        activity_mode: Arc::new(AtomicU8::new(activity::MODE_DEEP_IDLE)),
        active_ws_count: Arc::new(AtomicUsize::new(0)),
        active_gui_sessions: Arc::new(AtomicUsize::new(0)),
        visible_app_ids: Arc::new(RwLock::new(Vec::new())),
        last_activity_at: Arc::new(Mutex::new(Instant::now())),
        node_last_seen: Arc::new(RwLock::new(HashMap::new())),
    });

    start_activity_supervisor(state.clone());
    start_alert_evaluator(state.clone());
    start_telemetry_broadcaster(state.clone());
    start_agent_listener(state.clone());
    start_session_cleanup(sessions.clone());
    start_action_results_cleanup(action_results.clone());
    start_integration_coordinator(state.clone());
    tokio::spawn(start_ha_polling(state.clone()));

    start_media_poller(state.clone());
    start_status_poller(state.clone());

    let app = build_app_router(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8000".to_string());
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1".to_string());
    let addr = format!("{}:{}", bind_addr.trim(), port);
    println!("AMUD Web Server listening online on http://{}", addr);

    let listener = TokioTcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

pub fn build_app_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(dashboard_handler))
        .route("/feeds", get(feeds_page_handler))
        .route("/manifest.webmanifest", get(manifest_handler))
        .route("/status", get(status_page_handler))
        .route("/embed/:app_id", get(embed_app_handler))
        .route("/s/:token", get(share_link_handler))
        .route("/login", get(login_page).post(login_handler))
        .route("/auth/oidc/login", get(oidc_login_handler))
        .route("/auth/oidc/callback", get(oidc_callback_handler))
        .route("/logout", post(logout_handler))
        .route("/ws", get(ws_handler))
        .route("/health", get(health_handler))
        .route("/ready", get(ready_handler))
        .route(
            "/admin/settings",
            get(settings_page_handler).post(settings_handler),
        )
        .route("/admin/proxmox/test", post(test_proxmox_handler))
        .route(
            "/admin/upload",
            post(upload_handler).layer(axum::extract::DefaultBodyLimit::max(8 * 1024 * 1024)),
        )
        .route("/admin/credentials", post(credentials_handler))
        .route("/admin/backup/export", post(export_backup_handler))
        .route(
            "/admin/backup/validate",
            post(validate_backup_handler)
                .layer(axum::extract::DefaultBodyLimit::max(50 * 1024 * 1024)),
        )
        .route(
            "/admin/backup/import",
            post(import_backup_handler)
                .layer(axum::extract::DefaultBodyLimit::max(50 * 1024 * 1024)),
        )
        .route("/apps", post(add_app_handler))
        .route("/apps/delete", post(delete_app_handler))
        .route("/apps/edit", post(edit_app_handler))
        .route("/apps/reorder", post(reorder_apps_handler))
        .route("/apps/wake", post(wake_app_handler))
        .route("/apps/action", post(app_action_handler))
        .route("/api/wol", get(list_wol_devices_handler))
        .route("/api/wol/add", post(add_wol_device_handler))
        .route("/api/wol/delete", post(delete_wol_device_handler))
        .route("/api/apps", get(list_apps_api_handler))
        .route("/api/activity/viewport", post(activity_viewport_handler))
        .route("/api/activity/presence", post(activity_presence_handler))
        .route("/api/integrations/test", post(integration_test_handler))
        .route(
            "/api/integrations/custom-api/templates",
            get(custom_api_templates_handler),
        )
        .route("/api/telemetry", get(api_telemetry_handler))
        .route(
            "/api/apps/integrations/batch",
            post(batch_integration_handler),
        )
        .route(
            "/api/media/jellyfin/poster/:item_id",
            get(jellyfin_poster_handler),
        )
        .route(
            "/api/media/jellyfin/session/:session_id/:command",
            post(jellyfin_session_command_handler),
        )
        .route("/api/apps/:id/integration", get(integration_data_handler))
        .route(
            "/api/apps/:id/integration/action",
            post(integration_action_handler),
        )
        .route(
            "/api/categories",
            get(list_categories_handler).post(add_category_handler),
        )
        .route("/api/categories/delete", post(delete_category_handler))
        .route("/api/categories/edit", post(edit_category_handler))
        .route(
            "/api/feed-categories",
            get(list_feed_categories_handler).post(add_feed_category_handler),
        )
        .route(
            "/api/feed-categories/delete",
            post(delete_feed_category_handler),
        )
        .route(
            "/api/feed-categories/edit",
            post(edit_feed_category_handler),
        )
        .route(
            "/api/webhooks",
            get(list_webhooks_handler).post(add_webhook_handler),
        )
        .route("/api/webhooks/edit", post(edit_webhook_handler))
        .route("/api/webhooks/delete", post(delete_webhook_handler))
        .route("/api/webhooks/test", post(test_webhook_handler))
        .route("/api/rss-feeds", get(list_rss_feeds_handler))
        .route("/api/rss-feeds/add", post(add_rss_feed_handler))
        .route("/api/rss-feeds/edit", post(edit_rss_feed_handler))
        .route("/api/rss-feeds/delete", post(delete_rss_feed_handler))
        .route("/api/rss-feeds/reorder", post(reorder_rss_feeds_handler))
        .route("/api/rss/discover", post(rss_discover_handler))
        .route("/api/rss/favicon", get(rss_favicon_handler))
        .route("/api/audit", get(list_audit_handler))
        .route("/api/audit/clear", post(clear_audit_handler))
        .route("/api/audit/export", get(export_audit_handler))
        .route("/api/status", get(api_status_handler))
        .route("/api/widgets", get(list_widgets_handler))
        .route("/api/widgets/add", post(add_widget_handler))
        .route("/api/widgets/delete", post(delete_widget_handler))
        .route("/api/discover/docker", post(discover_docker_handler))
        .route("/api/telemetry/discover", post(telemetry_discover_handler))
        .route("/api/discover/import", post(import_discovered_apps_handler))
        .route(
            "/api/migration/homepage/preview",
            post(homepage_import_preview_handler),
        )
        .route(
            "/api/migration/homepage/import",
            post(homepage_import_apply_handler),
        )
        .route(
            "/api/migration/homarr/import",
            post(homarr_import_apply_handler),
        )
        .route(
            "/api/integrations/manifest",
            get(integration_manifest_handler),
        )
        .route("/api/logos", get(logos_list_handler))
        .route(
            "/api/boards",
            get(list_boards_handler).post(create_board_handler),
        )
        .route("/api/tokens", get(list_api_tokens_handler))
        .route("/api/tokens/create", post(create_api_token_handler))
        .route("/api/tokens/delete", post(delete_api_token_handler))
        .route("/api/share/create", post(create_share_link_handler))
        .route("/api/share/delete", post(delete_share_link_handler))
        .route("/api/system/version", get(system_version_handler))
        .route("/api/system/update", post(system_update_handler))
        .route("/api/users", get(list_users_handler).post(add_user_handler))
        .route("/api/users/edit", post(edit_user_handler))
        .route("/api/users/delete", post(delete_user_handler))
        .route("/uploads/:filename", get(serve_upload_handler))
        .nest_service("/static", tower_http::services::ServeDir::new("ui/static"))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            security_headers,
        ))
        .with_state(state)
}
