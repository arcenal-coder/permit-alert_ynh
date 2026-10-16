use crate::model::Permit;
use chrono::{Local, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use std::{path::Path, time::Duration};

pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let db = Connection::open(path)?;
    db.busy_timeout(Duration::from_secs(10))?;
    db.pragma_update(None, "foreign_keys", "ON")?;
    Ok(db)
}

pub fn initialize(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut db = open(path)?;
    db.pragma_update(None, "journal_mode", "WAL")?;
    let tx = db.transaction()?;
    tx.execute_batch(
        "CREATE TABLE IF NOT EXISTS permits (
            id INTEGER PRIMARY KEY, permit_number TEXT NOT NULL UNIQUE,
            status TEXT NOT NULL, end_date TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);
         CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS recipients (id INTEGER PRIMARY KEY,
            email TEXT NOT NULL UNIQUE, confirmed_at TEXT);
         CREATE TABLE IF NOT EXISTS rejected_emails (id INTEGER PRIMARY KEY,
            received_at TEXT NOT NULL, sender TEXT NOT NULL, reason TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS drafts (id TEXT PRIMARY KEY, data TEXT NOT NULL,
            source TEXT NOT NULL, created_at TEXT NOT NULL, pdf_path TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS events (id INTEGER PRIMARY KEY,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, kind TEXT NOT NULL,
            detail TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS deliveries (permit_id INTEGER NOT NULL
            REFERENCES permits(id) ON DELETE CASCADE,
            recipient_id INTEGER NOT NULL REFERENCES recipients(id) ON DELETE CASCADE,
            sent_at TEXT NOT NULL, PRIMARY KEY(permit_id, recipient_id));
         CREATE TABLE IF NOT EXISTS processed_mail (message_key TEXT PRIMARY KEY);
         INSERT OR IGNORE INTO settings VALUES ('lead_days', '14');
         INSERT OR IGNORE INTO settings VALUES ('frequency_days', '1');
         INSERT OR IGNORE INTO settings VALUES ('configured', '0');",
    )?;
    // Migrate the 0.1.x database in place, including existing recipients.
    for (table, column, declaration) in [
        ("permits", "data", "TEXT NOT NULL DEFAULT '{}'"),
        ("recipients", "token", "TEXT"),
        ("recipients", "token_created_at", "TEXT"),
    ] {
        let exists = tx
            .prepare(&format!("PRAGMA table_info({table})"))?
            .query_map([], |r| r.get::<_, String>(1))?
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .any(|name| name == column);
        if !exists {
            tx.execute_batch(&format!(
                "ALTER TABLE {table} ADD COLUMN {column} {declaration}"
            ))?;
        }
    }
    tx.pragma_update(None, "user_version", 2)?;
    tx.commit()?;
    Ok(())
}

pub fn permits(db: &Connection) -> rusqlite::Result<Vec<(i64, Permit)>> {
    db.prepare("SELECT id, permit_number, status, COALESCE(end_date,''), data FROM permits ORDER BY end_date, permit_number")?
        .query_map([], |row| {
            let mut permit: Permit = serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or_default();
            permit.number = row.get(1)?;
            permit.status = row.get(2)?;
            permit.end_date = row.get(3)?;
            Ok((row.get(0)?, permit))
        })?.collect()
}

pub fn by_number(db: &Connection, number: &str) -> rusqlite::Result<Option<Permit>> {
    Ok(permits(db)?
        .into_iter()
        .find(|(_, p)| p.number == number)
        .map(|(_, p)| p))
}

pub fn upsert(db: &Connection, permit: &Permit) -> rusqlite::Result<()> {
    let status =
        if crate::model::date(&permit.end_date).is_some_and(|d| d <= Local::now().date_naive()) {
            "expire"
        } else {
            &permit.status
        };
    db.execute(
        "INSERT INTO permits(permit_number,status,end_date,data) VALUES (?1,?2,?3,?4)
         ON CONFLICT(permit_number) DO UPDATE SET status=excluded.status,
         end_date=excluded.end_date,data=excluded.data,updated_at=CURRENT_TIMESTAMP",
        params![
            permit.number.trim(),
            status,
            permit.end_date,
            serde_json::to_string(permit).unwrap()
        ],
    )?;
    Ok(())
}

pub fn setting(db: &Connection, key: &str, default: &str) -> String {
    db.query_row("SELECT value FROM settings WHERE key=?", [key], |r| {
        r.get(0)
    })
    .optional()
    .ok()
    .flatten()
    .unwrap_or_else(|| default.into())
}

pub fn event(db: &Connection, kind: &str, detail: &str) -> rusqlite::Result<()> {
    db.execute(
        "INSERT INTO events(kind,detail) VALUES (?,?)",
        params![kind, detail],
    )?;
    Ok(())
}

pub fn reject(db: &Connection, sender: &str, reason: &str) -> rusqlite::Result<()> {
    db.execute(
        "INSERT INTO rejected_emails(received_at,sender,reason) VALUES (?,?,?)",
        params![Utc::now().to_rfc3339(), sender, reason],
    )?;
    Ok(())
}
