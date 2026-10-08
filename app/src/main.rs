use std::{env, net::SocketAddr, path::Path};

use axum::{Router, http::StatusCode, response::Html, routing::get};
use rusqlite::Connection;

#[tokio::main]
async fn main() {
    let database_path = env::var("PERMIT_ALERT_DATABASE")
        .unwrap_or_else(|_| "/var/lib/permit-alert/permit-alert.sqlite3".to_owned());
    let listen_address =
        env::var("PERMIT_ALERT_LISTEN").unwrap_or_else(|_| "127.0.0.1:3000".to_owned());

    initialize_database(Path::new(&database_path)).expect("database initialization failed");

    let address: SocketAddr = listen_address.parse().expect("invalid listen address");
    let app = Router::new()
        .route("/", get(index))
        .route("/health", get(health));
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("failed to bind HTTP listener");

    axum::serve(listener, app)
        .await
        .expect("HTTP server failed");
}

fn initialize_database(path: &Path) -> rusqlite::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("failed to create database directory");
    }

    let connection = Connection::open(path)?;
    connection.execute_batch(
        "
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS permits (
            id INTEGER PRIMARY KEY,
            permit_number TEXT NOT NULL UNIQUE,
            status TEXT NOT NULL,
            end_date TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS recipients (
            id INTEGER PRIMARY KEY,
            email TEXT NOT NULL UNIQUE,
            confirmed_at TEXT
        );

        CREATE TABLE IF NOT EXISTS rejected_emails (
            id INTEGER PRIMARY KEY,
            received_at TEXT NOT NULL,
            sender TEXT NOT NULL,
            reason TEXT NOT NULL
        );
        ",
    )
}

async fn index() -> Html<&'static str> {
    Html(
        "<!doctype html><html lang=\"fr\"><head><meta charset=\"utf-8\"><title>Permit Alert</title></head><body><main><h1>Permit Alert</h1><p>Le socle applicatif est operationnel.</p></main></body></html>",
    )
}

async fn health() -> StatusCode {
    StatusCode::NO_CONTENT
}

#[cfg(test)]
mod tests {
    use super::initialize_database;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn creates_the_required_tables() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("permit-alert-{suffix}.sqlite3"));

        initialize_database(&path).unwrap();
        let connection = rusqlite::Connection::open(&path).unwrap();
        let table_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('permits', 'settings', 'recipients', 'rejected_emails')",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(table_count, 4);
        fs::remove_file(path).unwrap();
    }
}
