//! Integration wizard test + Custom API template manifest.

use super::api_tokens::api_token_authorized;
use super::imports::*;
use axum::Json;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct IntegrationTestBody {
    pub integration_type: String,
    pub url: String,
    #[serde(default)]
    pub api_key: String,
}

pub async fn integration_test_handler(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Json(body): Json<IntegrationTestBody>,
) -> impl IntoResponse {
    if require_admin_session(&headers, &state.sessions).is_err() {
        return Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(Body::from(r#"{"error":"Forbidden"}"#))
            .unwrap()
            .into_response();
    }
    if body.url.trim().is_empty() {
        return api_json(
            StatusCode::BAD_REQUEST,
            serde_json::json!({ "ok": false, "error": "url_required" }),
        )
        .into_response();
    }
    let accept_invalid = {
        let cache = state.settings_cache.read().unwrap();
        cache
            .get("accept_invalid_certs")
            .map(|s| s == "1")
            .unwrap_or(false)
    };
    let app = crate::models::App {
        id: 0,
        name: "wizard-test".into(),
        url: body.url,
        icon: String::new(),
        description: String::new(),
        category: "General".into(),
        node_tag: "Local".into(),
        mac_address: String::new(),
        integration_type: body.integration_type,
        api_key: body.api_key,
        sort_order: 0,
        card_span: "1x1".into(),
        show_container_metrics: false,
        guest_visible: false,
        embed_mode: "link".into(),
        integration_visible_metrics: String::new(),
    };
    let clients = state.http_clients.clone();
    match crate::integrations::fetch_integration_data_uncached(&app, accept_invalid, &clients).await
    {
        Some(data) => api_json(
            StatusCode::OK,
            serde_json::json!({ "ok": true, "data": data }),
        )
        .into_response(),
        None => api_json(
            StatusCode::BAD_GATEWAY,
            serde_json::json!({ "ok": false, "error": "connection_failed" }),
        )
        .into_response(),
    }
}

pub async fn custom_api_templates_handler(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let session = get_session(&headers, &state.sessions);
    if session.is_none() && !api_token_authorized(&headers, &state, "read:apps") {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Body::from(r#"{"error":"Unauthorized"}"#))
            .unwrap()
            .into_response();
    }
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(include_str!(
            "../../../ui/static/integrations/custom-api/manifest.json"
        )))
        .unwrap()
        .into_response()
}

pub async fn logos_list_handler(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    if require_admin_session(&headers, &state.sessions).is_err() {
        return Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(Body::from(r#"{"error":"Forbidden"}"#))
            .unwrap()
            .into_response();
    }
    let mut entries: Vec<_> = state
        .logo_manifest
        .iter()
        .map(|(stem, url)| serde_json::json!({ "id": stem, "url": url }))
        .collect();
    entries.sort_by(|a, b| {
        a.get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .cmp(b.get("id").and_then(|v| v.as_str()).unwrap_or(""))
    });
    api_json(StatusCode::OK, serde_json::json!({ "logos": entries })).into_response()
}

pub async fn api_telemetry_handler(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    if !api_token_authorized(&headers, &state, "read:telemetry") {
        let session = get_session(&headers, &state.sessions);
        if !session.as_ref().map(|s| s.role == "Admin").unwrap_or(false) {
            return Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .body(Body::from(r#"{"error":"Unauthorized"}"#))
                .unwrap()
                .into_response();
        }
    }
    let system = state.latest_telemetry.read().unwrap().clone();
    let nodes = state.telemetry_by_node.read().unwrap().clone();
    let nodes_meta = state.nodes_meta.read().unwrap().clone();
    api_json(
        StatusCode::OK,
        serde_json::json!({
            "system": system,
            "nodes": nodes,
            "nodes_meta": nodes_meta,
            "agent_connected": *state.agent_connected.read().unwrap(),
        }),
    )
    .into_response()
}
