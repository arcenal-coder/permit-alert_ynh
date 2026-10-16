use super::*;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use chrono::{Duration, Local};
use model::Permit;
use rusqlite::params;
use std::{collections::BTreeMap, os::unix::fs::PermissionsExt};
use tower::ServiceExt;

fn app() -> App {
    let temporary = tempfile::tempdir().unwrap();
    let database = temporary.path().join("test.sqlite3");
    db::initialize(&database).unwrap();
    let sendmail = temporary.path().join("sendmail");
    std::fs::write(
        &sendmail,
        format!(
            "#!/bin/sh\ncat >> '{}'\n",
            temporary.path().join("sent.mail").display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&sendmail, std::fs::Permissions::from_mode(0o700)).unwrap();
    App(Arc::new(Config {
        database,
        temporary: temporary.path().to_path_buf(),
        _temporary_owner: Some(temporary),
        csrf: "test-csrf".into(),
        public_url: "https://example.org/permit-alert".into(),
        base_path: "/permit-alert".into(),
        sender: "permit-alert@example.org".into(),
        sendmail,
    }))
}
fn permit() -> Permit {
    Permit {
        number: "P-001".into(),
        status: "Valide".into(),
        unit: "U1".into(),
        equipment: "Pompe".into(),
        companies: "Entreprise A".into(),
        intervention: "Maintenance <test>".into(),
        start_date: Local::now().date_naive().to_string(),
        end_date: (Local::now().date_naive() + Duration::days(5)).to_string(),
        extra: BTreeMap::from([("Zone".into(), "Z2".into())]),
    }
}
fn encode(value: &str) -> String {
    value
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() {
                char::from(b).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}
fn form(values: &[(String, String)]) -> String {
    values
        .iter()
        .map(|(k, v)| format!("{}={}", encode(k), encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}
async fn request(app: &App, method: &str, path: &str, body: String) -> (StatusCode, String) {
    let response = web::router(app.clone())
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    (
        response.status(),
        String::from_utf8(
            to_bytes(response.into_body(), 16 * 1024 * 1024)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap(),
    )
}
fn add_draft(app: &App, p: &Permit) -> String {
    let path = app.0.temporary.join("source.pdf");
    std::fs::write(&path, b"%PDF-1.7\nsource").unwrap();
    app.db()
        .unwrap()
        .execute(
            "INSERT INTO drafts VALUES ('draft',?,?,?,?)",
            params![
                serde_json::to_string(&vec![p]).unwrap(),
                "web",
                chrono::Utc::now().to_rfc3339(),
                path.to_str().unwrap()
            ],
        )
        .unwrap();
    "draft".into()
}
fn validation(p: &Permit, versions: BTreeMap<String, Option<Permit>>) -> String {
    let mut values = vec![
        ("csrf".into(), "test-csrf".into()),
        ("confirmed".into(), "yes".into()),
        ("versions".into(), serde_json::to_string(&versions).unwrap()),
    ];
    for (k, v) in [
        ("number", &p.number),
        ("status", &p.status),
        ("unit", &p.unit),
        ("equipment", &p.equipment),
        ("companies", &p.companies),
        ("intervention", &p.intervention),
        ("start_date", &p.start_date),
        ("end_date", &p.end_date),
    ] {
        values.push((format!("0.{k}"), v.clone()));
    }
    values.push(("0.extra".into(), serde_json::to_string(&p.extra).unwrap()));
    form(&values)
}

#[tokio::test]
async fn review_validate_filter_delete_and_no_pdf_left() {
    let app = app();
    let p = permit();
    add_draft(&app, &p);
    let (status, html) = request(&app, "GET", "/imports/draft", String::new()).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("Maintenance &lt;test&gt;"));
    assert!(db::permits(&app.db().unwrap()).unwrap().is_empty());
    let (status, _) = request(
        &app,
        "POST",
        "/imports/draft/validate",
        validation(&p, BTreeMap::from([(p.number.clone(), None)])),
    )
    .await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert!(!app.0.temporary.join("source.pdf").exists());
    let saved = db::permits(&app.db().unwrap()).unwrap();
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].1.extra["Zone"], "Z2");
    let (_, html) = request(&app, "GET", "/?unit=U1&company=Entreprise", String::new()).await;
    assert!(html.contains("P-001"));
    let (_, html) = request(&app, "GET", "/?unit=AUTRE", String::new()).await;
    assert!(!html.contains("P-001"));
    let path = format!("/permits/{}/delete", saved[0].0);
    assert_eq!(
        request(&app, "POST", &path, "confirmed=yes".into()).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        request(&app, "POST", &path, "csrf=test-csrf&confirmed=yes".into())
            .await
            .0,
        StatusCode::SEE_OTHER
    );
    assert!(db::permits(&app.db().unwrap()).unwrap().is_empty());
}

#[tokio::test]
async fn duplicate_update_conflict_preserves_corrections_and_invalid_batch_is_atomic() {
    let app = app();
    let mut p = permit();
    add_draft(&app, &p);
    db::upsert(&app.db().unwrap(), &p).unwrap();
    p.unit = "U2".into();
    let data = validation(&p, BTreeMap::from([(p.number.clone(), None)]));
    assert_eq!(
        request(&app, "POST", "/imports/draft/validate", data)
            .await
            .0,
        StatusCode::CONFLICT
    );
    assert_eq!(db::permits(&app.db().unwrap()).unwrap()[0].1.unit, "U1");
    assert!(app.0.temporary.join("source.pdf").exists());
    let current = db::by_number(&app.db().unwrap(), &p.number).unwrap();
    let mut bad = p.clone();
    bad.end_date = "invalid".into();
    assert_eq!(
        request(
            &app,
            "POST",
            "/imports/draft/validate",
            validation(&bad, BTreeMap::from([(p.number.clone(), current.clone())]))
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(db::permits(&app.db().unwrap()).unwrap()[0].1.unit, "U1");
    assert_eq!(
        request(
            &app,
            "POST",
            "/imports/draft/validate",
            validation(&p, BTreeMap::from([(p.number.clone(), current)]))
        )
        .await
        .0,
        StatusCode::SEE_OTHER
    );
    let saved = db::permits(&app.db().unwrap()).unwrap();
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].1.unit, "U2");
}

#[tokio::test]
async fn confirmation_requires_post_and_expires_and_cannot_replay() {
    let app = app();
    assert_eq!(
        request(
            &app,
            "POST",
            "/recipients",
            "csrf=test-csrf&email=outside%40example.org".into()
        )
        .await
        .0,
        StatusCode::SEE_OTHER
    );
    let db = app.db().unwrap();
    let token: String = db
        .query_row("SELECT token FROM recipients", [], |r| r.get(0))
        .unwrap();
    let (_, html) = request(
        &app,
        "GET",
        &format!("/confirm?token={token}"),
        String::new(),
    )
    .await;
    assert!(!html.contains("test-csrf"));
    assert!(
        db.query_row("SELECT confirmed_at FROM recipients", [], |r| r
            .get::<_, Option<String>>(0))
            .unwrap()
            .is_none()
    );
    assert_eq!(
        request(&app, "POST", "/confirm", format!("token={token}"))
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        request(&app, "POST", "/confirm", format!("token={token}"))
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    let expired = uuid::Uuid::new_v4().to_string();
    db.execute(
        "INSERT INTO recipients(email,token,token_created_at) VALUES ('expired@example.org',?,?)",
        params![
            expired,
            (chrono::Utc::now() - Duration::days(8)).to_rfc3339()
        ],
    )
    .unwrap();
    assert_eq!(
        request(&app, "POST", "/confirm", format!("token={expired}"))
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn alerts_deduplicate_exclude_unconfirmed_and_stop_at_expiry() {
    let app = app();
    let p = permit();
    let db = app.db().unwrap();
    db::upsert(&db, &p).unwrap();
    db.execute("UPDATE settings SET value='1' WHERE key='configured'", [])
        .unwrap();
    db.execute(
        "INSERT INTO recipients(email,confirmed_at) VALUES ('yes@example.org','2020-01-01')",
        [],
    )
    .unwrap();
    db.execute(
        "INSERT INTO recipients(email) VALUES ('no@example.org')",
        [],
    )
    .unwrap();
    mail::alert_tick(&app).await.unwrap();
    mail::alert_tick(&app).await.unwrap();
    let count: i64 = db
        .query_row("SELECT COUNT(*) FROM deliveries", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);
    let sent = std::fs::read_to_string(app.0.temporary.join("sent.mail")).unwrap();
    assert!(sent.contains("yes@example.org"));
    assert!(!sent.contains("no@example.org"));
    assert_eq!(sent.matches("MIME-Version").count(), 1);
    db.execute(
        "UPDATE permits SET end_date=?",
        [Local::now().date_naive().to_string()],
    )
    .unwrap();
    db.execute("DELETE FROM deliveries", []).unwrap();
    cleanup(&app).unwrap();
    mail::alert_tick(&app).await.unwrap();
    assert_eq!(db::permits(&db).unwrap()[0].1.status, "expire");
    assert_eq!(
        std::fs::read_to_string(app.0.temporary.join("sent.mail")).unwrap(),
        sent
    );
}

#[tokio::test]
async fn sending_failure_logged_and_not_counted_as_delivered() {
    let app = app();
    let db = app.db().unwrap();
    db::upsert(&db, &permit()).unwrap();
    db.execute("UPDATE settings SET value='1' WHERE key='configured'", [])
        .unwrap();
    db.execute(
        "INSERT INTO recipients(email,confirmed_at) VALUES ('yes@example.org','2020-01-01')",
        [],
    )
    .unwrap();
    std::fs::write(&app.0.sendmail, "#!/bin/sh\nexit 1\n").unwrap();
    assert!(mail::alert_tick(&app).await.is_err());
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM deliveries", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM events WHERE kind='Échec d’envoi'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
}

#[tokio::test]
async fn older_journal_entries_and_expired_draft_are_accessible_and_cleaned() {
    let app = app();
    let db = app.db().unwrap();
    add_draft(&app, &permit());
    db.execute(
        "UPDATE drafts SET created_at=?",
        [(chrono::Utc::now() - Duration::hours(25)).to_rfc3339()],
    )
    .unwrap();
    cleanup(&app).unwrap();
    assert!(!app.0.temporary.join("source.pdf").exists());
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM rejected_emails", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    for index in 0..201 {
        db::event(&db, "test", &format!("entry-{index}")).unwrap();
    }
    let (_, first) = request(&app, "GET", "/journal", String::new()).await;
    assert!(first.contains("Plus anciens"));
    assert!(!first.contains("entry-0</td>"));
    let (_, older) = request(&app, "GET", "/journal?page=1", String::new()).await;
    assert!(older.contains("entry-0</td>"));
    assert_eq!(
        request(&app, "GET", "/journal?page=-1", String::new())
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
}

#[test]
fn migration_preserves_old_records_and_is_repeatable() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old.sqlite3");
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch("CREATE TABLE permits (id INTEGER PRIMARY KEY,permit_number TEXT UNIQUE,status TEXT,end_date TEXT,created_at TEXT,updated_at TEXT); INSERT INTO permits VALUES(1,'OLD','Valide','2027-01-01','2026','2026'); CREATE TABLE recipients(id INTEGER PRIMARY KEY,email TEXT UNIQUE,confirmed_at TEXT);").unwrap();
    db::initialize(&path).unwrap();
    db::initialize(&path).unwrap();
    assert_eq!(db::permits(&db).unwrap()[0].1.number, "OLD");
}

#[test]
fn date_boundaries_and_header_injection() {
    let today = Local::now().date_naive();
    assert!(!model::due(&today.to_string(), today, 14));
    assert!(model::due(
        &(today + Duration::days(14)).to_string(),
        today,
        14
    ));
    assert!(!model::due(
        &(today + Duration::days(15)).to_string(),
        today,
        14
    ));
    assert!(!web::valid_email("a@example.org\r\nBcc: other@example.org"));
    assert!(!web::valid_email("a@@example.org"));
}

fn sample_pdf() -> Vec<u8> {
    let text = "BT /F1 12 Tf 40 750 Td (E-Permit) Tj 0 -22 Td (Numero de permis : P-REAL) Tj 0 -22 Td (Etat : Valide) Tj 0 -22 Td (Date de fin : 31/12/2027) Tj ET";
    let objects=["<< /Type /Catalog /Pages 2 0 R >>".to_owned(),"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".into(),"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>".into(),"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".into(),format!("<< /Length {} >>\nstream\n{text}\nendstream",text.len())];
    let mut pdf = String::from("%PDF-1.4\n");
    let mut offsets = vec![0];
    for (i, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.push_str(&format!("{} 0 obj\n{}\nendobj\n", i + 1, object));
    }
    let xref = pdf.len();
    pdf.push_str("xref\n0 6\n0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        pdf.push_str(&format!("{offset:010} 00000 n \n"));
    }
    pdf.push_str(&format!(
        "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF"
    ));
    pdf.into_bytes()
}

#[tokio::test]
#[ignore = "requires Poppler pdftotext (Debian package poppler-utils)"]
async fn real_pdf_extraction_and_rejection_remove_source_files() {
    let app = app();
    let bytes = sample_pdf();
    let id = web::create_draft(&app, &bytes, "PDF fixture")
        .await
        .unwrap();
    let db = app.db().unwrap();
    let data: String = db
        .query_row("SELECT data FROM drafts WHERE id=?", [&id], |r| r.get(0))
        .unwrap();
    let permits: Vec<Permit> = serde_json::from_str(&data).unwrap();
    assert_eq!(permits[0].number, "P-REAL");
    assert!(
        pdf::extract(b"%PDF-1.4\ncorrupt", &app.0.temporary)
            .await
            .is_err()
    );
    request(
        &app,
        "POST",
        &format!("/imports/{id}/cancel"),
        "csrf=test-csrf".into(),
    )
    .await;
    assert_eq!(
        std::fs::read_dir(&app.0.temporary)
            .unwrap()
            .filter_map(|p| p.ok())
            .filter(|p| p.path().extension().is_some_and(|e| e == "pdf"))
            .count(),
        0
    );
}
