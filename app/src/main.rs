mod db;
mod mail;
mod model;
mod pdf;
#[cfg(test)]
mod tests;
mod web;

use std::{
    env,
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Clone)]
pub struct App(Arc<Config>);

pub struct Config {
    database: PathBuf,
    temporary: PathBuf,
    _temporary_owner: Option<tempfile::TempDir>,
    csrf: String,
    public_url: String,
    base_path: String,
    sender: String,
    sendmail: PathBuf,
}

impl App {
    fn db(&self) -> rusqlite::Result<rusqlite::Connection> {
        db::open(&self.0.database)
    }
    fn url(&self, path: &str) -> String {
        format!("{}{}", self.0.base_path, path)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database = PathBuf::from(
        env::var("PERMIT_ALERT_DATABASE")
            .unwrap_or_else(|_| "/var/lib/permit-alert/permit-alert.sqlite3".into()),
    );
    db::initialize(&database)?;
    let public_url = env::var("PERMIT_ALERT_PUBLIC_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:3000".into())
        .trim_end_matches('/')
        .to_owned();
    let base_path = env::var("PERMIT_ALERT_BASE_PATH")
        .unwrap_or_default()
        .trim_end_matches('/')
        .to_owned();
    let temporary = env::var("PERMIT_ALERT_TEMP")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            tempfile::tempdir()
                .expect("temporary directory unavailable")
                .keep()
        })
        .join("imports");
    std::fs::create_dir_all(&temporary)?;
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&temporary, std::fs::Permissions::from_mode(0o700))?;
    let app = App(Arc::new(Config {
        database,
        temporary,
        _temporary_owner: None,
        public_url,
        base_path,
        csrf: format!("{}{}", uuid::Uuid::new_v4(), uuid::Uuid::new_v4()),
        sender: env::var("PERMIT_ALERT_SENDER").unwrap_or_else(|_| "permit-alert@localhost".into()),
        sendmail: PathBuf::from(
            env::var("PERMIT_ALERT_SENDMAIL").unwrap_or_else(|_| "/usr/sbin/sendmail".into()),
        ),
    }));
    cleanup(&app)?;
    tokio::spawn(mail::worker(app.clone()));
    let listener = tokio::net::TcpListener::bind(
        env::var("PERMIT_ALERT_LISTEN").unwrap_or_else(|_| "127.0.0.1:3000".into()),
    )
    .await?;
    eprintln!("Permit Alert {} ready", env!("CARGO_PKG_VERSION"));
    axum::serve(listener, web::router(app))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}

fn cleanup(app: &App) -> rusqlite::Result<()> {
    let db = app.db()?;
    let drafts = db
        .prepare("SELECT id,pdf_path,created_at,source FROM drafts")?
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (id, path, created, source) in drafts {
        let stale = chrono::DateTime::parse_from_rfc3339(&created)
            .map(|t| chrono::Utc::now().signed_duration_since(t).num_hours() >= 24)
            .unwrap_or(true);
        if stale || !Path::new(&path).is_file() {
            if Path::new(&path).starts_with(&app.0.temporary) && Path::new(&path).exists() {
                std::fs::remove_file(&path)
                    .map_err(|_| rusqlite::Error::InvalidPath(PathBuf::from(&path)))?;
            }
            db.execute("DELETE FROM drafts WHERE id=?", [id])?;
            db::reject(
                &db,
                &source,
                "Import expiré après 24 heures ou PDF temporaire indisponible. Réimportez le document.",
            )?;
        }
    }
    let referenced = db
        .prepare("SELECT pdf_path FROM drafts")?
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<Result<std::collections::BTreeSet<_>, _>>()?;
    if let Ok(entries) = std::fs::read_dir(&app.0.temporary) {
        for entry in entries.flatten() {
            let path = entry.path();
            let ours = path.extension().is_some_and(|v| v == "pdf")
                && path
                    .file_stem()
                    .and_then(|v| v.to_str())
                    .is_some_and(|v| uuid::Uuid::parse_str(v).is_ok());
            let old = entry
                .metadata()
                .and_then(|v| v.modified())
                .ok()
                .and_then(|v| v.elapsed().ok())
                .is_some_and(|elapsed| elapsed.as_secs() > 3600);
            if ours
                && old
                && !referenced.contains(path.to_string_lossy().as_ref())
                && entry.file_type().is_ok_and(|v| v.is_file())
            {
                let _ = std::fs::remove_file(path);
            }
        }
    }
    db.execute(
        "UPDATE permits SET status='expire' WHERE end_date<=? AND status<>'expire'",
        [chrono::Local::now().date_naive().to_string()],
    )?;
    Ok(())
}
