use super::imports::*;

pub async fn settings_handler(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Form(form): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    if let Some(resp) = check_api_rate_limit(&state, &headers, "settings", 20, 60) {
        return resp.into_response();
    }

    let session = get_session(&headers, &state.sessions);
    let admin_user = match session {
        Some(ref s) if s.role == "Admin" => s.username.clone(),
        _ => return Redirect::to("/admin/settings").into_response(),
    };
    if !validate_csrf(&headers, &state.sessions, Some(&form)) {
        return csrf_forbidden_response().into_response();
    }

    let settings_cache = state.settings_cache.clone();
    let headers = headers.clone();
    let agent_config_keys_changed = form
        .keys()
        .any(|k| AGENT_CONFIG_SETTING_KEYS.contains(&k.as_str()) || k == "pve_api_token");
    let purge_rss_cache = form
        .get("feeds_enabled")
        .map(|s| s.as_str() == "0")
        .unwrap_or(false);
    let new_token = with_db(state.db.clone(), move |db| {
        let mut new_token = None;
        let mut changed_keys = 0usize;
        let mut preset_to_apply: Option<String> = None;
        let mut pending_default_engine: Option<String> = None;
        let mut sanitized_custom_engines: Option<String> = None;
        for (key, val) in form {
            if key == "csrf_token"
                || key == "new_password"
                || key == "old_password"
                || key == "repeat_password"
                || key == "new_username"
            {
                continue;
            }
            if !setting_key_allowed(&key) {
                continue;
            }
            if key == "default_search_engine" {
                pending_default_engine = Some(val.trim().to_lowercase());
                continue;
            }
            let value = if key == "custom_bg_url" || key == "app_logo" {
                sanitize_setting_url(&val)
            } else if key == "custom_css" {
                sanitize_custom_css(&val)
            } else if key == "ha_url" {
                sanitize_integration_url(&val)
            } else if key == "theme_mode" {
                sanitize_theme_mode(&val)
            } else if key == "theme_scheduler" {
                sanitize_theme_scheduler(&val)
            } else if key == "active_theme_id" {
                sanitize_active_theme_id(&val)
            } else if key == "theme_light_at" {
                sanitize_time_hhmm(&val, "07:00")
            } else if key == "theme_dark_at" {
                sanitize_time_hhmm(&val, "19:00")
            } else if key == "guest_category_restrict" {
                sanitize_bool_setting(&val)
            } else if key == "guest_visible_categories" {
                sanitize_guest_visible_categories(&val)
            } else if key == "dashboard_layout" {
                sanitize_dashboard_layout(&val)
            } else if key == "embed_mode" {
                sanitize_embed_mode(&val)
            } else if key == "oidc_enabled"
                || key == "status_page_public"
                || key == "kiosk_mode"
                || key == "iframe_embeds_enabled"
                || key == "feeds_enabled"
                || key == "enable_proxmox"
                || key == "webgl_effects_enabled"
                || key == "greeting_animations_enabled"
                || key == "dashboard_reorder_enabled"
            {
                sanitize_bool_setting(&val)
            } else if key == "integration_cache_max_entries" {
                sanitize_cache_max_entries(&val)
            } else if key == "integration_cache_ttl_secs" {
                sanitize_cache_ttl_secs(&val)
            } else if key == "agent_telemetry_interval_secs" {
                sanitize_interval_setting(&val, 5, 3, 60)
            } else if key == "agent_lxc_poll_interval_secs"
                || key == "agent_docker_poll_interval_secs"
            {
                sanitize_interval_setting(&val, 10, 5, 120)
            } else if key == "status_poll_interval_secs" || key == "ha_poll_interval_secs" {
                sanitize_interval_setting(&val, 15, 10, 300)
            } else if key == "media_poll_interval_secs" {
                sanitize_interval_setting(&val, 5, 3, 120)
            } else if key == "telemetry_broadcast_interval_secs" {
                sanitize_interval_setting(&val, 5, 3, 60)
            } else if key == "integration_coordinator_interval_secs" {
                sanitize_interval_setting(&val, 45, 15, 600)
            } else if key == "performance_preset" {
                sanitize_performance_preset(&val)
            } else if key == "idle_grace_secs" {
                sanitize_interval_setting(&val, 45, 15, 300)
            } else if key == "backup_reminder_days" {
                sanitize_interval_setting(&val, 30, 7, 365)
            } else if key == "alert_cpu_threshold"
                || key == "alert_ram_threshold"
                || key == "alert_disk_threshold"
            {
                sanitize_interval_setting(&val, 90, 50, 100)
            } else if key == "telemetry_external_ifaces" || key == "telemetry_internal_ifaces" {
                sanitize_iface_list(&val)
            } else if key == "telemetry_disk_mounts" {
                sanitize_disk_mount_list(&val)
            } else if key == "accent_color" {
                crate::templates::safe_accent_hex(&val)
            } else if key == "wallpaper_overlay_strength" {
                sanitize_wallpaper_overlay_strength(&val)
            } else if key == "clock_timezone" {
                sanitize_clock_timezone(&val)
            } else if key == "clock_time_format" {
                sanitize_clock_time_format(&val)
            } else if key == "custom_search_engines" {
                let cleaned = sanitize_custom_search_engines(&val);
                sanitized_custom_engines = Some(cleaned.clone());
                cleaned
            } else if SECRET_SETTING_KEYS.contains(&key.as_str()) {
                match setting_value_or_existing(db, &key, &val) {
                    Some(v) => v,
                    None => continue,
                }
            } else {
                val
            };
            if key == "pve_api_token" {
                new_token = Some(value.clone());
            }
            if key == "performance_preset" && value != "custom" {
                preset_to_apply = Some(value.clone());
            }
            crate::db::upsert_setting(db, &key, &value);
            changed_keys += 1;
        }
        if let Some(raw_default) = pending_default_engine {
            let customs = sanitized_custom_engines.unwrap_or_else(|| {
                db.query_row(
                    "SELECT value FROM settings WHERE key = 'custom_search_engines'",
                    [],
                    |r| r.get::<_, String>(0),
                )
                .unwrap_or_else(|_| "[]".into())
            });
            let value = sanitize_default_search_engine(&raw_default, &customs);
            crate::db::upsert_setting(db, "default_search_engine", &value);
            changed_keys += 1;
        }
        if let Some(preset) = preset_to_apply {
            apply_performance_preset(db, &preset);
        }
        refresh_settings_cache(db, &settings_cache);
        if changed_keys > 0 {
            record_audit_blocking(
                db,
                &headers,
                &admin_user,
                "settings_update",
                "settings",
                &format!("{changed_keys} keys updated"),
            );
        }
        new_token
    })
    .await;

    {
        let settings = state.settings_cache.read().unwrap().clone();
        apply_integration_cache_limits(&state.integration_cache, &settings);
        if purge_rss_cache {
            let cache = state.integration_cache.clone();
            let ids = with_db(state.db.clone(), crate::db::load_rss_app_ids).await;
            cache.invalidate_many(&ids);
        }
    }

    if agent_config_keys_changed || new_token.is_some() {
        push_agent_config(&state, new_token.as_deref());
    }
    Redirect::to("/admin/settings").into_response()
}

pub async fn test_proxmox_handler(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Form(form): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    let session = match get_session(&headers, &state.sessions) {
        Some(s) if s.role == "Admin" => s,
        _ => {
            if let Err(resp) = require_admin_session(&headers, &state.sessions) {
                return *resp;
            }
            return Response::builder()
                .status(StatusCode::FORBIDDEN)
                .header("Content-Type", "application/json")
                .body(axum::body::Body::from(
                    r#"{"success":false,"error":"Admin required"}"#,
                ))
                .unwrap();
        }
    };
    let admin_user = session.username.clone();

    if let Err(resp) = require_admin_session(&headers, &state.sessions) {
        return *resp;
    }
    if !validate_csrf(&headers, &state.sessions, Some(&form)) {
        return csrf_forbidden_response();
    }

    let form_token = form.get("pve_api_token").cloned().unwrap_or_default();
    if !form_token.trim().is_empty() {
        let token_trim = form_token.trim().to_string();
        let settings_cache = state.settings_cache.clone();
        let headers_clone = headers.clone();
        let admin_user_clone = admin_user.clone();
        with_db(state.db.clone(), move |db| {
            crate::db::upsert_setting(db, "pve_api_token", &token_trim);
            refresh_settings_cache(db, &settings_cache);
            record_audit_blocking(
                db,
                &headers_clone,
                &admin_user_clone,
                "proxmox_token_update",
                "pve_api_token",
                "updated via Proxmox test panel",
            );
        })
        .await;
        let config_payload = crate::agent::agent_config_for_state(&state, Some(form_token.trim()));
        if let Ok(mut serialized) = serde_json::to_vec(&config_payload) {
            serialized.push(b'\n');
            if let Some(tx) = &*state.agent_command_tx.lock().unwrap() {
                let _ = tx
                    .tx
                    .send(String::from_utf8_lossy(&serialized).into_owned());
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    *state.pve_test_response.write().unwrap() = None;

    // Trigger agent self-test without exposing secrets
    let cmd = serde_json::json!({ "action": "test_pve" });

    let mut success = false;
    let mut error = None;

    if let Ok(mut serialized) = serde_json::to_vec(&cmd) {
        serialized.push(b'\n');

        let sent = {
            if let Some(tx) = &*state.agent_command_tx.lock().unwrap() {
                tx.tx
                    .send(String::from_utf8_lossy(&serialized).into_owned())
                    .is_ok()
            } else {
                false
            }
        };

        if sent {
            let start = std::time::Instant::now();
            while start.elapsed() < std::time::Duration::from_secs(5) {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                if let Some(res) = state.pve_test_response.read().unwrap().as_ref() {
                    success = res.success;
                    error = res.error.clone();
                    break;
                }
            }
            if !success && error.is_none() {
                error = Some("Connection test timed out waiting for agent response".to_string());
            }
        } else {
            error = Some("AMUD Agent is offline or not connected".to_string());
        }
    } else {
        error = Some("Failed to serialize test command".to_string());
    }

    let body = serde_json::json!({
        "success": success,
        "error": error
    });

    let headers_clone = headers.clone();
    let admin_user_clone = admin_user.clone();
    let action = if success {
        "proxmox_test_success"
    } else {
        "proxmox_test_failed"
    }
    .to_string();
    let details = if success {
        "connection test succeeded".to_string()
    } else {
        error
            .clone()
            .unwrap_or_else(|| "connection test failed".to_string())
    };
    with_db(state.db.clone(), move |db| {
        record_audit_blocking(
            db,
            &headers_clone,
            &admin_user_clone,
            &action,
            "proxmox",
            &details,
        );
    })
    .await;

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_string(&body).unwrap(),
        ))
        .unwrap()
}

pub async fn credentials_handler(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Form(form): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    if let Some(resp) = check_api_rate_limit(&state, &headers, "credentials", 5, 900) {
        return resp;
    }

    let session = get_session(&headers, &state.sessions);
    let sess = match session {
        Some(ref s) if s.role == "Admin" => s,
        _ => {
            return Response::builder()
                .status(StatusCode::FORBIDDEN)
                .header("Content-Type", "application/json")
                .body(axum::body::Body::from(r#"{"error":"Forbidden"}"#))
                .unwrap();
        }
    };

    if !validate_csrf(&headers, &state.sessions, Some(&form)) {
        return csrf_forbidden_response();
    }

    let old_password = form.get("old_password").cloned().unwrap_or_default();
    let new_password = form.get("new_password").cloned().unwrap_or_default();
    let new_username = form.get("new_username").cloned().unwrap_or_default();

    if old_password.is_empty() {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(
                r#"{"error":"Old password is required"}"#,
            ))
            .unwrap();
    }

    enum CredOutcome {
        Ok {
            actual_username: String,
            old_username: String,
        },
        WrongPassword,
        UsernameTaken,
    }

    let current_username = sess.username.clone();
    let settings_cache = state.settings_cache.clone();
    let headers = headers.clone();
    let outcome = with_db(state.db.clone(), move |db| {
        let stored_hash: Result<String, _> = db
            .prepare("SELECT password_hash FROM users WHERE username = ?")
            .unwrap()
            .query_row(params![current_username], |row| row.get(0));

        let old_needs_rehash = match stored_hash {
            Ok(ref h) => {
                let (verified, needs_rehash) = verify_password(h, &old_password);
                if verified {
                    needs_rehash
                } else {
                    return CredOutcome::WrongPassword;
                }
            }
            _ => return CredOutcome::WrongPassword,
        };

        let mut actual_username = current_username.clone();
        let old_username = current_username.clone();
        if !new_username.is_empty() && new_username != current_username {
            let count: i64 = db
                .prepare("SELECT COUNT(*) FROM users WHERE username = ?")
                .unwrap()
                .query_row(params![new_username], |row| row.get(0))
                .unwrap_or(0);

            if count > 0 {
                return CredOutcome::UsernameTaken;
            }

            if db
                .execute(
                    "UPDATE users SET username = ? WHERE username = ?",
                    params![new_username, current_username],
                )
                .is_ok()
            {
                actual_username = new_username.clone();
            }
        }

        if !new_password.is_empty() || old_needs_rehash {
            let new_hash = hash_password(if new_password.is_empty() {
                &old_password
            } else {
                &new_password
            });
            db.execute(
                "UPDATE users SET password_hash = ? WHERE username = ?",
                params![new_hash, actual_username],
            )
            .ok();
            db.execute(
                "INSERT INTO settings (key, value) VALUES ('admin_must_change_password', '0')
                 ON CONFLICT(key) DO UPDATE SET value = '0'",
                [],
            )
            .ok();
            refresh_settings_cache(db, &settings_cache);
            let details = if actual_username != old_username {
                "username and/or password updated"
            } else {
                "password updated"
            };
            record_audit_blocking(
                db,
                &headers,
                &actual_username,
                "credentials_change",
                &actual_username,
                details,
            );
        }

        CredOutcome::Ok {
            actual_username,
            old_username,
        }
    })
    .await;

    let (actual_username, old_username) = match outcome {
        CredOutcome::WrongPassword => {
            return Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .header("Content-Type", "application/json")
                .body(axum::body::Body::from(
                    r#"{"error":"Old password is incorrect"}"#,
                ))
                .unwrap();
        }
        CredOutcome::UsernameTaken => {
            return Response::builder()
                .status(StatusCode::CONFLICT)
                .header("Content-Type", "application/json")
                .body(axum::body::Body::from(
                    r#"{"error":"Username is already taken"}"#,
                ))
                .unwrap();
        }
        CredOutcome::Ok {
            actual_username,
            old_username,
        } => (actual_username, old_username),
    };

    revoke_sessions_for_user(&state.sessions, &old_username);
    if actual_username != old_username {
        revoke_sessions_for_user(&state.sessions, &actual_username);
    }

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(r#"{"success":true}"#))
        .unwrap()
}
