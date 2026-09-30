use super::imports::*;

pub async fn discover_docker_handler(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Form(form): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    if let Err(resp) = require_admin_session(&headers, &state.sessions) {
        return *resp;
    }
    if !validate_csrf(&headers, &state.sessions, Some(&form)) {
        return csrf_forbidden_response();
    }

    *state.docker_discover_response.write().unwrap() = None;
    let node_tag = form.get("node_tag").map(|s| s.as_str());
    let cmd = serde_json::json!({ "action": "discover_docker" });
    if let Ok(serialized) = serde_json::to_string(&cmd) {
        if let Some(tx) = crate::agent::agent_session_for_or_any(&state, node_tag) {
            let _ = tx.tx.send(format!("{serialized}\n"));
        }
    }

    for _ in 0..20 {
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        if state.docker_discover_response.read().unwrap().is_some() {
            break;
        }
    }

    let result = state.docker_discover_response.read().unwrap().clone();
    match result {
        Some(val) => {
            let apps = val
                .get("discover_docker_result")
                .and_then(|r| r.get("apps"))
                .cloned()
                .unwrap_or_else(|| serde_json::json!([]));
            api_json(StatusCode::OK, serde_json::json!({ "apps": apps }))
        }
        None => api_json(
            StatusCode::SERVICE_UNAVAILABLE,
            serde_json::json!({ "error": "Agent not connected or Docker unavailable" }),
        ),
    }
}

pub async fn telemetry_discover_handler(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Form(form): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    if let Err(resp) = require_admin_session(&headers, &state.sessions) {
        return *resp;
    }
    if !validate_csrf(&headers, &state.sessions, Some(&form)) {
        return csrf_forbidden_response();
    }

    *state.telemetry_discover_response.write().unwrap() = None;
    let node_tag = form.get("node_tag").map(|s| s.as_str());
    let cmd = serde_json::json!({ "action": "telemetry_discover" });
    if let Ok(serialized) = serde_json::to_string(&cmd) {
        if let Some(tx) = crate::agent::agent_session_for_or_any(&state, node_tag) {
            let _ = tx.tx.send(format!("{serialized}\n"));
        }
    }

    for _ in 0..20 {
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        if state.telemetry_discover_response.read().unwrap().is_some() {
            break;
        }
    }

    let result = state.telemetry_discover_response.read().unwrap().clone();
    match result {
        Some(val) => {
            let payload = val
                .get("telemetry_discover_result")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));
            api_json(StatusCode::OK, payload)
        }
        None => api_json(
            StatusCode::SERVICE_UNAVAILABLE,
            serde_json::json!({ "error": "Agent not connected" }),
        ),
    }
}

pub async fn import_discovered_apps_handler(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Form(form): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    if let Err(resp) = require_admin_session(&headers, &state.sessions) {
        return *resp;
    }
    if !validate_csrf(&headers, &state.sessions, Some(&form)) {
        return csrf_forbidden_response();
    }

    let payload = form.get("apps_json").cloned().unwrap_or_default();
    let Ok(apps) = serde_json::from_str::<Vec<serde_json::Value>>(&payload) else {
        return Redirect::to("/admin/settings?tab=integrations").into_response();
    };

    with_db(state.db.clone(), move |db| {
        for app in apps {
            let name = app.get("name").and_then(|v| v.as_str()).unwrap_or("").trim();
            let url = app
                .get("url")
                .and_then(|v| v.as_str())
                .map(crate::templates::normalize_url)
                .unwrap_or_default();
            if name.is_empty() || url.is_empty() {
                continue;
            }
            let integration_type = app
                .get("integration_type")
                .and_then(|v| v.as_str())
                .map(|t| {
                    crate::integration_registry::map_homepage_widget_type(t)
                        .unwrap_or(t)
                        .to_string()
                })
                .unwrap_or_default();
            let api_key = app
                .get("api_key")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let exists: i64 = db
                .query_row(
                    "SELECT COUNT(*) FROM apps WHERE lower(name) = lower(?) OR url = ?",
                    params![name, url],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            if exists > 0 {
                continue;
            }
            let category = app
                .get("category")
                .and_then(|v| v.as_str())
                .unwrap_or("General");
            let category = crate::db::resolve_app_category(db, category);
            let icon = app
                .get("icon")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let sort_order = crate::db::next_app_sort_order(db);
            let _ = db.execute(
                "INSERT INTO apps (name, url, icon, description, category, node_tag, sort_order, guest_visible, embed_mode, integration_type, api_key) VALUES (?, ?, ?, '', ?, 'Local', ?, 1, 'link', ?, ?)",
                params![name, url, icon, category, sort_order, integration_type, api_key],
            );
        }
    })
    .await;

    Redirect::to("/").into_response()
}
