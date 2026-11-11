use axum::http::HeaderMap;
use rusqlite::{params, Connection, ErrorCode};

use crate::security::client_ip;

/// Idempotent schema guard for databases upgraded from releases before audit_log existed.
pub(crate) fn ensure_audit_log_table(db: &Connection) -> Result<(), rusqlite::Error> {
    db.execute(
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
    )?;
    let _ = db.execute(
        "CREATE INDEX IF NOT EXISTS idx_audit_log_created ON audit_log(created_at);",
        [],
    );
    Ok(())
}

fn audit_table_has_column(db: &Connection, column: &str) -> bool {
    let Ok(mut stmt) = db.prepare("PRAGMA table_info(audit_log)") else {
        return false;
    };
    let Ok(mut rows) = stmt.query([]) else {
        return false;
    };
    while let Ok(Some(row)) = rows.next() {
        if row.get::<_, String>(1).ok().as_deref() == Some(column) {
            return true;
        }
    }
    false
}

/// Ensures audit_log exists and has all expected columns (handles partial upgrades).
pub(crate) fn ensure_audit_log_schema(db: &Connection) -> Result<(), rusqlite::Error> {
    ensure_audit_log_table(db)?;
    if !audit_table_has_column(db, "username") {
        db.execute(
            "ALTER TABLE audit_log ADD COLUMN username TEXT NOT NULL DEFAULT ''",
            [],
        )?;
    }
    if !audit_table_has_column(db, "details") {
        let _ = db.execute(
            "ALTER TABLE audit_log ADD COLUMN details TEXT NOT NULL DEFAULT ''",
            [],
        );
    }
    if !audit_table_has_column(db, "client_ip") {
        db.execute(
            "ALTER TABLE audit_log ADD COLUMN client_ip TEXT NOT NULL DEFAULT ''",
            [],
        )?;
    }
    // Some early databases used `user` instead of `username`.
    if audit_table_has_column(db, "user") {
        let _ = db.execute(
            "UPDATE audit_log SET username = user WHERE username = '' AND user IS NOT NULL AND user != ''",
            [],
        );
    }
    Ok(())
}

pub(crate) fn audit_health_check(db: &Connection) -> Result<i64, String> {
    ensure_audit_log_schema(db).map_err(|e| format!("schema setup failed: {e}"))?;
    db.query_row("SELECT COUNT(*) FROM audit_log", [], |row| row.get(0))
        .map_err(|e| format!("count query failed: {e}"))
}

pub(crate) fn record_audit(
    db: &Connection,
    username: &str,
    action: &str,
    target: &str,
    details: &str,
    headers: &HeaderMap,
) {
    if let Err(e) = ensure_audit_log_schema(db) {
        eprintln!("[AUDIT] failed to ensure audit_log schema: {e}");
        return;
    }

    let ip = client_ip(headers);
    let sql = "INSERT INTO audit_log (created_at, username, action, target, details, client_ip)
         VALUES (datetime('now'), ?1, ?2, ?3, ?4, ?5)";

    let mut last_err = None;
    for attempt in 0..3 {
        match db.execute(sql, params![username, action, target, details, ip]) {
            Ok(_) => return,
            Err(e) if e.sqlite_error_code() == Some(ErrorCode::DatabaseBusy) && attempt < 2 => {
                std::thread::yield_now();
                last_err = Some(e);
            }
            Err(e) => {
                if attempt == 0 {
                    if let Err(rebuild_err) = rebuild_audit_log_table(db) {
                        eprintln!("[AUDIT] rebuild failed: {rebuild_err}");
                        eprintln!("[AUDIT] insert failed (action={action}, user={username}): {e}");
                        return;
                    }
                    continue;
                }
                eprintln!("[AUDIT] insert failed (action={action}, user={username}): {e}");
                return;
            }
        }
    }
    if let Some(e) = last_err {
        eprintln!("[AUDIT] insert failed after retries (action={action}): {e}");
    }
}

fn rebuild_audit_log_table(db: &Connection) -> Result<(), rusqlite::Error> {
    let has_user = audit_table_has_column(db, "user");
    let has_username = audit_table_has_column(db, "username");
    let copy_sql = if has_username && has_user {
        "INSERT INTO audit_log_new (id, created_at, username, action, target, details, client_ip)
            SELECT id, created_at,
                COALESCE(NULLIF(username, ''), COALESCE(user, '')),
                action, COALESCE(target, ''), COALESCE(details, ''), COALESCE(client_ip, '')
            FROM audit_log"
    } else if has_username {
        "INSERT INTO audit_log_new (id, created_at, username, action, target, details, client_ip)
            SELECT id, created_at, COALESCE(username, ''), action, COALESCE(target, ''),
                COALESCE(details, ''), COALESCE(client_ip, '')
            FROM audit_log"
    } else if has_user {
        "INSERT INTO audit_log_new (id, created_at, username, action, target, details, client_ip)
            SELECT id, created_at, COALESCE(user, ''), action, COALESCE(target, ''),
                COALESCE(details, ''), COALESCE(client_ip, '')
            FROM audit_log"
    } else {
        "INSERT INTO audit_log_new (id, created_at, username, action, target, details, client_ip)
            SELECT id, created_at, '', action, COALESCE(target, ''), COALESCE(details, ''), ''
            FROM audit_log"
    };

    db.execute_batch(&format!(
        "CREATE TABLE IF NOT EXISTS audit_log_new (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            username TEXT NOT NULL DEFAULT '',
            action TEXT NOT NULL,
            target TEXT NOT NULL DEFAULT '',
            details TEXT NOT NULL DEFAULT '',
            client_ip TEXT NOT NULL DEFAULT ''
        );
        {copy_sql};
        DROP TABLE audit_log;
        ALTER TABLE audit_log_new RENAME TO audit_log;
        CREATE INDEX IF NOT EXISTS idx_audit_log_created ON audit_log(created_at);"
    ))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AuditScope {
    All,
    Ops,
    Updates,
}

impl AuditScope {
    pub(crate) fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "ops" => Self::Ops,
            "updates" => Self::Updates,
            _ => Self::All,
        }
    }

    fn where_sql(self) -> &'static str {
        match self {
            Self::All => "",
            Self::Ops => " WHERE action NOT LIKE 'system_update_%'",
            Self::Updates => " WHERE action LIKE 'system_update_%'",
        }
    }
}

pub(crate) fn list_audit_page(
    db: &Connection,
    limit: i64,
    offset: i64,
    scope: AuditScope,
) -> Result<(Vec<serde_json::Value>, i64), String> {
    ensure_audit_log_schema(db).map_err(|e| format!("audit log unavailable: {e}"))?;

    let limit = limit.clamp(1, 200);
    let offset = offset.max(0);
    let where_sql = scope.where_sql();

    let total: i64 = db
        .query_row(
            &format!("SELECT COUNT(*) FROM audit_log{where_sql}"),
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("audit log count failed: {e}"))?;

    let mut out = Vec::new();
    let mut stmt = db
        .prepare(&format!(
            "SELECT id, created_at, username, action, target, details, client_ip
         FROM audit_log{where_sql} ORDER BY id DESC LIMIT ?1 OFFSET ?2"
        ))
        .map_err(|e| format!("audit log query prepare failed: {e}"))?;
    let mut rows = stmt
        .query(params![limit, offset])
        .map_err(|e| format!("audit log query failed: {e}"))?;
    while let Some(row) = rows
        .next()
        .map_err(|e| format!("audit log read failed: {e}"))?
    {
        out.push(serde_json::json!({
            "id": row.get::<_, i64>(0).unwrap_or(0),
            "created_at": row.get::<_, String>(1).unwrap_or_default(),
            "username": row.get::<_, String>(2).unwrap_or_default(),
            "action": row.get::<_, String>(3).unwrap_or_default(),
            "target": row.get::<_, String>(4).unwrap_or_default(),
            "details": row.get::<_, String>(5).unwrap_or_default(),
            "client_ip": row.get::<_, String>(6).unwrap_or_default(),
        }));
    }
    Ok((out, total))
}

pub(crate) fn clear_audit(db: &Connection, scope: AuditScope) -> Result<i64, String> {
    ensure_audit_log_schema(db).map_err(|e| format!("audit log unavailable: {e}"))?;
    let where_sql = scope.where_sql();
    let deleted = db
        .execute(&format!("DELETE FROM audit_log{where_sql}"), [])
        .map_err(|e| format!("audit log clear failed: {e}"))?;
    Ok(deleted as i64)
}

pub(crate) fn audit_entries_to_csv(entries: &[serde_json::Value]) -> String {
    let mut out = String::from("id,created_at,username,action,target,details,client_ip\n");
    for e in entries {
        let cell = |key: &str| {
            let raw = e
                .get(key)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .replace('"', "\"\"");
            format!("\"{raw}\"")
        };
        let id = e.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
        out.push_str(&format!(
            "{},{},{},{},{},{},{}\n",
            id,
            cell("created_at"),
            cell("username"),
            cell("action"),
            cell("target"),
            cell("details"),
            cell("client_ip")
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    #[test]
    fn migrates_legacy_audit_log_without_username() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE audit_log (
                id INTEGER PRIMARY KEY,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                action TEXT NOT NULL,
                target TEXT NOT NULL,
                details TEXT DEFAULT ''
            )",
            [],
        )
        .unwrap();

        ensure_audit_log_schema(&conn).unwrap();

        record_audit(&conn, "admin", "login", "admin", "", &HeaderMap::new());

        let username: String = conn
            .query_row(
                "SELECT username FROM audit_log WHERE action = 'login'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(username, "admin");
    }

    #[test]
    fn backfills_username_from_legacy_user_column() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE audit_log (
                id INTEGER PRIMARY KEY,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                user TEXT NOT NULL,
                action TEXT NOT NULL,
                target TEXT NOT NULL
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO audit_log (user, action, target) VALUES ('TRADMSS', 'login', 'TRADMSS')",
            [],
        )
        .unwrap();

        ensure_audit_log_schema(&conn).unwrap();

        let username: String = conn
            .query_row("SELECT username FROM audit_log WHERE id = 1", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(username, "TRADMSS");
    }
}
