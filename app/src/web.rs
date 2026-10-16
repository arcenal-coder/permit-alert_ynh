use crate::{
    App, db,
    model::{Permit, date, due},
    pdf,
};
use axum::{
    Router,
    extract::{DefaultBodyLimit, Form, Multipart, Path, Query, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use chrono::{Local, Utc};
use rusqlite::{OptionalExtension, params};
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub type FormData = HashMap<String, String>;
pub struct Error(StatusCode, String);
pub type Result<T> = std::result::Result<T, Error>;
impl From<rusqlite::Error> for Error {
    fn from(error: rusqlite::Error) -> Self {
        eprintln!("Database operation failed: {error}");
        Self(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Opération impossible. Réessayez, puis consultez les journaux du service.".into(),
        )
    }
}
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (self.0, Html(format!("<!doctype html><html lang=fr><meta charset=utf-8><title>Erreur — Permit Alert</title><h1>Opération non effectuée</h1><p>{}</p><p>Revenez à la page précédente pour corriger ou réessayer.</p></html>", e(&self.1)))).into_response()
    }
}
fn invalid(message: impl Into<String>) -> Error {
    Error(StatusCode::BAD_REQUEST, message.into())
}
fn not_found() -> Error {
    Error(
        StatusCode::NOT_FOUND,
        "Élément introuvable ou import expiré.".into(),
    )
}
pub fn e(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn value<'a>(data: &'a FormData, key: &str) -> &'a str {
    data.get(key).map(|v| v.trim()).unwrap_or("")
}
fn csrf(app: &App, data: &FormData) -> Result<()> {
    if value(data, "csrf") != app.0.csrf {
        return Err(Error(
            StatusCode::FORBIDDEN,
            "Session du formulaire expirée. Rechargez la page avant de réessayer.".into(),
        ));
    }
    Ok(())
}
fn token(app: &App) -> String {
    format!("<input type=hidden name=csrf value=\"{}\">", app.0.csrf)
}
fn field(name: &str, label: &str, value: &str, kind: &str, required: bool) -> String {
    format!(
        "<label>{}<input type=\"{}\" name=\"{}\" value=\"{}\" {}></label>",
        e(label),
        kind,
        e(name),
        e(value),
        if required { "required" } else { "" }
    )
}
fn page(app: &App, title: &str, content: String) -> Html<String> {
    Html(format!(
        "<!doctype html><html lang=fr><head><meta charset=utf-8><meta name=viewport content=\"width=device-width,initial-scale=1\"><title>{} — Permit Alert</title><link rel=stylesheet href=\"{}\"></head><body><header><a class=brand href=\"{}\">Permit<span>Alert</span></a><nav><a href=\"{}\">Permis</a><a href=\"{}\">Imports à valider</a><a href=\"{}\">Réglages</a><a href=\"{}\">Journal</a></nav><small>v{}</small></header><main><div class=heading><div><p class=eyebrow>SUIVI DES ÉCHÉANCES · E-PERMIT</p><h1>{}</h1></div><a class=button href=\"{}\">Importer un PDF</a></div>{}</main><footer>Accès coordinateurs · Les PDF sont temporaires et exclus des sauvegardes.</footer></body></html>",
        e(title),
        app.url("/style.css"),
        app.url("/"),
        app.url("/"),
        app.url("/imports"),
        app.url("/settings"),
        app.url("/journal"),
        env!("CARGO_PKG_VERSION"),
        e(title),
        app.url("/imports"),
        content
    ))
}

pub fn router(app: App) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/health", get(health))
        .route("/style.css", get(|| async { ([(header::CONTENT_TYPE,"text/css; charset=utf-8")], include_str!("style.css")) }))
        .route("/permits/{id}", get(detail))
        .route("/permits/{id}/delete", post(delete))
        .route("/imports", get(imports).post(upload))
        .route("/imports/{id}", get(review))
        .route("/imports/{id}/pdf", get(source_pdf))
        .route("/imports/{id}/validate", post(validate))
        .route("/imports/{id}/cancel", post(cancel))
        .route("/settings", get(settings).post(save_settings))
        .route("/recipients", post(add_recipient))
        .route("/recipients/{id}/delete", post(delete_recipient))
        .route("/recipients/{id}/resend", post(resend_recipient))
        .route("/confirm", get(confirm_page).post(confirm))
        .route("/journal", get(journal))
        .layer(DefaultBodyLimit::max(pdf::MAX_PDF + 1024 * 1024))
        .layer(axum::middleware::map_response(|mut response: Response| async {
            let headers = response.headers_mut();
            headers.insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
            headers.insert("x-content-type-options", "nosniff".parse().unwrap());
            headers.insert("referrer-policy", "no-referrer".parse().unwrap());
            if !headers.contains_key(header::CONTENT_SECURITY_POLICY) {
                headers.insert(header::CONTENT_SECURITY_POLICY, "default-src 'none'; style-src 'self'; frame-src 'self'; img-src 'self'; form-action 'self'; frame-ancestors 'self'; base-uri 'none'".parse().unwrap());
            }
            response
        }))
        .with_state(app)
}

async fn health(State(app): State<App>) -> StatusCode {
    match app
        .db()
        .and_then(|db| db.query_row("SELECT 1", [], |_| Ok(())))
    {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(_) => StatusCode::SERVICE_UNAVAILABLE,
    }
}

async fn index(State(app): State<App>, Query(filters): Query<FormData>) -> Result<Html<String>> {
    crate::cleanup(&app)?;
    for key in ["from", "until"] {
        if !value(&filters, key).is_empty() && date(value(&filters, key)).is_none() {
            return Err(invalid("Filtre de date invalide."));
        }
    }
    let db = app.db()?;
    let lead = db::setting(&db, "lead_days", "14").parse().unwrap_or(14);
    let configured = db::setting(&db, "configured", "0") == "1";
    let mut body = String::new();
    if !configured {
        body.push_str(&format!("<p class=notice>Pour activer les rappels, choisissez le délai et la fréquence dans les <a href=\"{}\">réglages</a>.</p>",app.url("/settings")));
    }
    body.push_str(&format!("<form class=filters method=get action=\"{}\">{}{}{}{}{}<button>Filtrer</button><a href=\"{}\">Réinitialiser</a></form>",app.url("/"),field("status","État",value(&filters,"status"),"text",false),field("unit","Unité",value(&filters,"unit"),"text",false),field("company","Entreprise",value(&filters,"company"),"text",false),field("from","Fin à partir du",value(&filters,"from"),"date",false),field("until","Fin jusqu’au",value(&filters,"until"),"date",false),app.url("/")));
    let permits: Vec<_> = db::permits(&db)?
        .into_iter()
        .filter(|(_, p)| {
            [
                ("status", &p.status),
                ("unit", &p.unit),
                ("company", &p.companies),
            ]
            .iter()
            .all(|(k, v)| {
                v.to_lowercase()
                    .contains(&value(&filters, k).to_lowercase())
            }) && (value(&filters, "from").is_empty()
                || p.end_date.as_str() >= value(&filters, "from"))
                && (value(&filters, "until").is_empty()
                    || p.end_date.as_str() <= value(&filters, "until"))
        })
        .collect();
    body.push_str(&format!("<section class=panel><div class=section-title><h2>Permis enregistrés</h2><span>{} résultat(s)</span></div><div class=table-wrap><table><thead><tr><th>Numéro</th><th>État</th><th>Unité / équipement</th><th>Entreprises</th><th>Date de fin</th><th>Échéance</th></tr></thead><tbody>",permits.len()));
    for (id, p) in &permits {
        let badge = if p.status == "expire" {
            "<span class=\"badge expired\">Expiré</span>"
        } else if due(&p.end_date, Local::now().date_naive(), lead) && configured {
            "<span class=\"badge warning\">À renouveler</span>"
        } else {
            "<span class=badge>En cours</span>"
        };
        body.push_str(&format!("<tr><td><a href=\"{}\"><strong>{}</strong></a></td><td>{}</td><td>{}<small>{}</small></td><td>{}</td><td>{}</td><td>{}</td></tr>",app.url(&format!("/permits/{id}")),e(&p.number),e(&p.status),e(&p.unit),e(&p.equipment),e(&p.companies),e(&p.end_date),badge));
    }
    if permits.is_empty() {
        body.push_str("<tr><td colspan=6 class=empty>Aucun permis pour ces critères. Importez un PDF E-Permit pour commencer.</td></tr>");
    }
    body.push_str("</tbody></table></div></section>");
    Ok(page(&app, "Vos permis", body))
}

fn permit_data(p: &Permit) -> String {
    let pairs = [
        ("Numéro", &p.number),
        ("État", &p.status),
        ("Unité", &p.unit),
        ("Équipement", &p.equipment),
        ("Entreprises", &p.companies),
        ("Intervention", &p.intervention),
        ("Date de début", &p.start_date),
        ("Date de fin", &p.end_date),
    ];
    let mut html = String::from("<dl class=data>");
    for (key, value) in pairs
        .into_iter()
        .chain(p.extra.iter().map(|(k, v)| (k.as_str(), v)))
    {
        html.push_str(&format!(
            "<div><dt>{}</dt><dd>{}</dd></div>",
            e(key),
            e(value)
        ));
    }
    html.push_str("</dl>");
    html
}
async fn detail(State(app): State<App>, Path(id): Path<i64>) -> Result<Html<String>> {
    crate::cleanup(&app)?;
    let p = db::permits(&app.db()?)?
        .into_iter()
        .find(|(key, _)| *key == id)
        .ok_or_else(not_found)?
        .1;
    Ok(page(
        &app,
        &format!("Permis {}", p.number),
        format!(
            "<section class=panel>{}</section><section class=panel><h2>Suppression définitive</h2><p>Le permis et ses rappels seront supprimés. Cette action ne dispose pas de corbeille.</p><form method=post action=\"{}\">{}<label class=check><input type=checkbox name=confirmed value=yes required>Je confirme la suppression définitive</label><button class=danger>Supprimer le permis</button></form></section>",
            permit_data(&p),
            app.url(&format!("/permits/{id}/delete")),
            token(&app)
        ),
    ))
}
async fn delete(
    State(app): State<App>,
    Path(id): Path<i64>,
    Form(data): Form<FormData>,
) -> Result<Redirect> {
    csrf(&app, &data)?;
    if value(&data, "confirmed") != "yes" {
        return Err(invalid("Confirmez la suppression."));
    }
    app.db()?.execute("DELETE FROM permits WHERE id=?", [id])?;
    Ok(Redirect::to(&app.url("/")))
}

pub async fn create_draft(
    app: &App,
    bytes: &[u8],
    source: &str,
) -> std::result::Result<String, String> {
    create_draft_with_key(app, bytes, source, None).await
}

pub async fn create_draft_with_key(
    app: &App,
    bytes: &[u8],
    source: &str,
    message_key: Option<&str>,
) -> std::result::Result<String, String> {
    let (path, permits) = pdf::extract(bytes, &app.0.temporary).await?;
    let id = uuid::Uuid::new_v4().to_string();
    let result = app.db().and_then(|mut db| {
        let tx = db.transaction()?;
        tx.execute(
            "INSERT INTO drafts(id,data,source,created_at,pdf_path) VALUES (?,?,?,?,?)",
            params![
                id,
                serde_json::to_string(&permits).unwrap(),
                source,
                Utc::now().to_rfc3339(),
                path.to_string_lossy()
            ],
        )?;
        if let Some(key) = message_key {
            tx.execute("INSERT INTO processed_mail VALUES (?)", [key])?;
        }
        tx.commit()
    });
    if result.is_err() {
        let _ = tokio::fs::remove_file(path).await;
        return Err("L’import n’a pas pu être sauvegardé.".into());
    }
    Ok(id)
}

async fn imports(State(app): State<App>) -> Result<Html<String>> {
    crate::cleanup(&app)?;
    let mut body = format!(
        "<section class=panel><h2>Importer un PDF E-Permit</h2><p>PDF textuel, 10 Mo maximum, 50 pages maximum. Les scans sans texte et les structures inconnues sont rejetés. Les fichiers expirent après 24 heures.</p><form method=post enctype=multipart/form-data action=\"{}\">{}<label>Document PDF<input type=file name=pdf accept=application/pdf required></label><button>Analyser le PDF</button></form></section><section class=panel><h2>Imports en attente de validation</h2>",
        app.url("/imports"),
        token(&app)
    );
    let db = app.db()?;
    let drafts = db
        .prepare("SELECT id,source,created_at FROM drafts ORDER BY created_at DESC")?
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if drafts.is_empty() {
        body.push_str("<p class=empty>Aucun import à valider.</p>");
    }
    for (id, source, created) in drafts {
        body.push_str(&format!("<article class=import-row><div><strong>{}</strong><small>{}</small></div><a class=button href=\"{}\">Vérifier et valider</a></article>",e(&source),e(&created),app.url(&format!("/imports/{id}"))));
    }
    body.push_str("</section>");
    Ok(page(&app, "Imports à valider", body))
}

async fn upload(State(app): State<App>, mut multipart: Multipart) -> Result<Redirect> {
    let mut csrf_value = String::new();
    let mut bytes = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| invalid("Formulaire d’import invalide."))?
    {
        match field.name() {
            Some("csrf") => {
                csrf_value = field
                    .text()
                    .await
                    .map_err(|_| invalid("Formulaire invalide."))?
            }
            Some("pdf") => {
                if bytes.is_some() {
                    return Err(invalid("Un seul PDF par import."));
                }
                bytes = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|_| invalid("PDF trop volumineux ou transfert interrompu."))?,
                );
            }
            _ => {}
        }
    }
    if csrf_value != app.0.csrf {
        return Err(Error(
            StatusCode::FORBIDDEN,
            "Formulaire expiré. Rechargez la page.".into(),
        ));
    }
    let bytes = bytes.ok_or_else(|| invalid("Choisissez un PDF."))?;
    match create_draft(&app, &bytes, "Import web").await {
        Ok(id) => Ok(Redirect::to(&app.url(&format!("/imports/{id}")))),
        Err(error) => {
            db::reject(&app.db()?, "Import web", &error)?;
            Err(invalid(error))
        }
    }
}

fn draft(db: &rusqlite::Connection, id: &str) -> Result<(Vec<Permit>, String)> {
    let row = db
        .query_row("SELECT data,pdf_path FROM drafts WHERE id=?", [id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })
        .optional()?
        .ok_or_else(not_found)?;
    let permits = serde_json::from_str(&row.0)
        .map_err(|_| invalid("Import endommagé. Annulez puis réimportez le PDF."))?;
    Ok((permits, row.1))
}
async fn review(State(app): State<App>, Path(id): Path<String>) -> Result<Html<String>> {
    crate::cleanup(&app)?;
    let db = app.db()?;
    let (permits, _) = draft(&db, &id)?;
    let mut body = format!(
        "<p class=notice>Aucun permis n’est encore enregistré. Vérifiez chaque champ avant de valider. Une fiche existante de même numéro sera remplacée. <a href=\"{}\" target=_blank rel=noopener>Ouvrir le PDF source</a></p><form method=post action=\"{}\">{}",
        app.url(&format!("/imports/{id}/pdf")),
        app.url(&format!("/imports/{id}/validate")),
        token(&app)
    );
    let mut versions = BTreeMap::new();
    for (index, p) in permits.iter().enumerate() {
        let existing = db::by_number(&db, &p.number)?;
        versions.insert(p.number.clone(), existing.clone());
        body.push_str(&format!(
            "<section class=panel><h2>Permis {} — {}</h2>",
            index + 1,
            e(&p.number)
        ));
        if let Some(existing) = existing {
            body.push_str(&format!(
                "<details open><summary>Fiche existante — sera mise à jour</summary>{}</details>",
                permit_data(&existing)
            ));
        }
        body.push_str("<div class=fields>");
        for (key, label, value, kind, required) in [
            ("number", "Numéro", &p.number, "text", true),
            ("status", "État", &p.status, "text", true),
            ("unit", "Unité", &p.unit, "text", false),
            ("equipment", "Équipement", &p.equipment, "text", false),
            ("companies", "Entreprises", &p.companies, "text", false),
            (
                "intervention",
                "Intervention",
                &p.intervention,
                "text",
                false,
            ),
            ("start_date", "Date de début", &p.start_date, "date", false),
            ("end_date", "Date de fin", &p.end_date, "date", true),
        ] {
            body.push_str(&field(
                &format!("{index}.{key}"),
                label,
                value,
                kind,
                required,
            ));
        }
        for (extra_index, (label, value)) in p.extra.iter().enumerate() {
            body.push_str(&field(
                &format!("{index}.extra_value.{extra_index}"),
                label,
                value,
                "text",
                false,
            ));
        }
        body.push_str(&format!(
            "</div><input type=hidden name=\"{index}.extra\" value=\"{}\"></section>",
            e(&serde_json::to_string(&p.extra).unwrap())
        ));
    }
    body.push_str(&format!("<input type=hidden name=versions value=\"{}\"><section class=panel><label class=check><input type=checkbox name=confirmed value=yes required>J’ai vérifié les données et les mises à jour des fiches existantes.</label><button>Valider {} permis et supprimer le PDF</button></section></form><form method=post action=\"{}\">{}<button class=secondary>Annuler l’import et supprimer le PDF</button></form>",e(&serde_json::to_string(&versions).unwrap()),permits.len(),app.url(&format!("/imports/{id}/cancel")),token(&app)));
    Ok(page(&app, "Vérifier l’import", body))
}

async fn source_pdf(State(app): State<App>, Path(id): Path<String>) -> Result<Response> {
    let (_, path) = draft(&app.db()?, &id)?;
    if !std::path::Path::new(&path).starts_with(&app.0.temporary) {
        return Err(not_found());
    }
    let bytes = tokio::fs::read(path).await.map_err(|_| not_found())?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/pdf"),
            (header::CONTENT_DISPOSITION, "inline; filename=epermit.pdf"),
            (
                header::CONTENT_SECURITY_POLICY,
                "sandbox; default-src 'none'; frame-ancestors 'self'",
            ),
        ],
        bytes,
    )
        .into_response())
}

async fn validate(
    State(app): State<App>,
    Path(id): Path<String>,
    Form(data): Form<FormData>,
) -> Result<Redirect> {
    csrf(&app, &data)?;
    if value(&data, "confirmed") != "yes" {
        return Err(invalid("Vérifiez et confirmez les données."));
    }
    let mut db = app.db()?;
    let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let (original, path) = draft(&tx, &id)?;
    let versions: BTreeMap<String, Option<Permit>> = serde_json::from_str(value(&data, "versions"))
        .map_err(|_| invalid("Rechargez la page de validation."))?;
    let mut permits = Vec::new();
    let mut numbers = BTreeSet::new();
    for index in 0..original.len() {
        let f = |key: &str| value(&data, &format!("{index}.{key}")).to_owned();
        let mut permit = Permit {
            number: f("number"),
            status: f("status"),
            unit: f("unit"),
            equipment: f("equipment"),
            companies: f("companies"),
            intervention: f("intervention"),
            start_date: f("start_date"),
            end_date: f("end_date"),
            extra: serde_json::from_str(&f("extra")).map_err(|_| {
                invalid("Les autres colonnes doivent être un objet JSON de textes.")
            })?,
        };
        for (extra_index, (_, extra_value)) in permit.extra.iter_mut().enumerate() {
            if let Some(corrected) = data.get(&format!("{index}.extra_value.{extra_index}")) {
                *extra_value = corrected.trim().to_owned();
            }
        }
        permit.validate().map_err(invalid)?;
        if !numbers.insert(permit.number.clone()) {
            return Err(invalid("Deux permis ont le même numéro dans cet import."));
        }
        permits.push(permit);
    }
    // If another coordinator changed a target, retain corrections and require a
    // fresh review. This also covers editing a number to target an existing file.
    for permit in &permits {
        let current = db::by_number(&tx, &permit.number)?;
        if versions.get(&permit.number).cloned().unwrap_or(None) != current {
            tx.execute(
                "UPDATE drafts SET data=? WHERE id=?",
                params![serde_json::to_string(&permits).unwrap(), id],
            )?;
            tx.commit()?;
            return Err(Error(StatusCode::CONFLICT,"Une fiche cible existe ou a changé. Vos corrections sont conservées : revenez aux imports et ouvrez à nouveau cet import pour comparer avant de valider.".into()));
        }
    }
    for permit in &permits {
        db::upsert(&tx, permit)?;
    }
    // Deletion precedes commit: an I/O error must never silently leave a source
    // behind after reporting a successful validation.
    std::fs::remove_file(&path)
        .map_err(|_| invalid("Suppression du PDF impossible. Aucun permis enregistré."))?;
    tx.execute("DELETE FROM drafts WHERE id=?", [id])?;
    tx.commit()?;
    Ok(Redirect::to(&app.url("/")))
}

async fn cancel(
    State(app): State<App>,
    Path(id): Path<String>,
    Form(data): Form<FormData>,
) -> Result<Redirect> {
    csrf(&app, &data)?;
    let db = app.db()?;
    let (_, path) = draft(&db, &id)?;
    if std::path::Path::new(&path).is_file() {
        std::fs::remove_file(path)
            .map_err(|_| invalid("Impossible de supprimer le fichier temporaire."))?;
    }
    db.execute("DELETE FROM drafts WHERE id=?", [id])?;
    Ok(Redirect::to(&app.url("/imports")))
}

async fn settings(State(app): State<App>) -> Result<Html<String>> {
    let db = app.db()?;
    let mut body = format!(
        "<section class=panel><h2>Rappels avant échéance</h2><p>Les rappels s’arrêtent à la date de fin. Heure du serveur : {}.</p><form method=post action=\"{}\">{}<div class=fields>{}{}</div><p>Valeurs comprises entre 1 et 365 jours. Le premier enregistrement active les rappels.</p><button>Enregistrer et activer</button></form></section><section class=panel><h2>Destinataires</h2><p>Chaque adresse doit confirmer son inscription (également pour les adresses internes). Une confirmation expire après 7 jours.</p><form class=inline method=post action=\"{}\">{}{}<button>Inviter le destinataire</button></form>",
        Local::now().format("%Y-%m-%d %H:%M %Z"),
        app.url("/settings"),
        token(&app),
        field(
            "lead_days",
            "Délai avant la fin (jours)",
            &db::setting(&db, "lead_days", "14"),
            "number",
            true
        ),
        field(
            "frequency_days",
            "Fréquence des rappels (jours)",
            &db::setting(&db, "frequency_days", "1"),
            "number",
            true
        ),
        app.url("/recipients"),
        token(&app),
        field("email", "Adresse e-mail", "", "email", true)
    );
    let recipients = db
        .prepare("SELECT id,email,confirmed_at FROM recipients ORDER BY email")?
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    for (id, email, confirmed) in recipients {
        body.push_str(&format!("<article class=import-row><div><strong>{}</strong><small>{}</small></div><div class=inline>",e(&email),if confirmed.is_some(){"Confirmée"}else{"Confirmation en attente"}));
        if confirmed.is_none() {
            body.push_str(&format!("<form method=post action=\"{}\">{}<button class=secondary>Renvoyer l’invitation</button></form>",app.url(&format!("/recipients/{id}/resend")),token(&app)));
        }
        body.push_str(&format!("<form method=post action=\"{}\">{}<button class=danger>Retirer</button></form></div></article>",app.url(&format!("/recipients/{id}/delete")),token(&app)));
    }
    body.push_str("</section><section class=panel><h2>Messagerie et intégration</h2>");
    body.push_str(&format!("<p>Expéditeur des notifications : <strong>{}</strong></p><p>URL des confirmations : {}</p><p>Réception IMAP : <strong>{}</strong>. Les paramètres secrets sont définis dans le fichier privé du service, jamais affichés ici.</p><p>Les échecs d’envoi et de relève sont visibles dans le journal.</p></section>",e(&app.0.sender),e(&app.0.public_url),if std::env::var("PERMIT_ALERT_IMAP_HOST").is_ok(){"configurée"}else{"non configurée"}));
    Ok(page(&app, "Réglages et destinataires", body))
}

async fn save_settings(State(app): State<App>, Form(data): Form<FormData>) -> Result<Redirect> {
    csrf(&app, &data)?;
    let lead: i64 = value(&data, "lead_days")
        .parse()
        .map_err(|_| invalid("Délai invalide."))?;
    let frequency: i64 = value(&data, "frequency_days")
        .parse()
        .map_err(|_| invalid("Fréquence invalide."))?;
    if !(1..=365).contains(&lead) || !(1..=365).contains(&frequency) {
        return Err(invalid("Choisissez entre 1 et 365 jours."));
    }
    let mut db = app.db()?;
    let tx = db.transaction()?;
    for (key, value) in [
        ("lead_days", lead.to_string()),
        ("frequency_days", frequency.to_string()),
        ("configured", "1".into()),
    ] {
        tx.execute("INSERT INTO settings(key,value) VALUES (?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value",params![key,value])?;
    }
    tx.commit()?;
    Ok(Redirect::to(&app.url("/settings")))
}

pub fn valid_email(email: &str) -> bool {
    email.len() <= 254
        && !email
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || "<>,;\"\\".contains(c))
        && email.split_once('@').is_some_and(|(local, domain)| {
            !local.is_empty()
                && !domain.starts_with('.')
                && !domain.ends_with('.')
                && domain.contains('.')
                && !domain.contains('@')
        })
}
async fn invite(app: &App, id: i64, email: &str) -> Result<()> {
    let token = uuid::Uuid::new_v4().to_string();
    let db = app.db()?;
    db.execute(
        "UPDATE recipients SET token=?,token_created_at=? WHERE id=? AND confirmed_at IS NULL",
        params![token, Utc::now().to_rfc3339(), id],
    )?;
    let link = format!("{}/confirm?token={token}", app.0.public_url);
    let result=crate::mail::send(app,email,"Confirmez votre inscription à Permit Alert",&format!("Un coordinateur vous propose de recevoir les alertes d’échéance des permis.\n\nOuvrez ce lien et confirmez explicitement votre inscription (valable 7 jours) :\n{link}\n\nSi vous n’êtes pas concerné, ignorez ce message.")).await;
    if result.is_err() {
        db::event(
            &db,
            "Échec d’envoi",
            &format!(
                "Invitation non envoyée à {email}. Vérifiez la messagerie puis renvoyez l’invitation."
            ),
        )?;
    }
    Ok(())
}
async fn add_recipient(State(app): State<App>, Form(data): Form<FormData>) -> Result<Redirect> {
    csrf(&app, &data)?;
    let email = value(&data, "email").to_lowercase();
    if !valid_email(&email) {
        return Err(invalid("Adresse e-mail invalide."));
    }
    let db = app.db()?;
    let exists: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM recipients WHERE email=?)",
        [&email],
        |r| r.get(0),
    )?;
    if exists {
        return Err(invalid("Cette adresse est déjà enregistrée."));
    }
    db.execute("INSERT INTO recipients(email) VALUES (?)", [&email])?;
    let id = db.last_insert_rowid();
    drop(db);
    invite(&app, id, &email).await?;
    Ok(Redirect::to(&app.url("/settings")))
}
async fn resend_recipient(
    State(app): State<App>,
    Path(id): Path<i64>,
    Form(data): Form<FormData>,
) -> Result<Redirect> {
    csrf(&app, &data)?;
    let email = app
        .db()?
        .query_row(
            "SELECT email FROM recipients WHERE id=? AND confirmed_at IS NULL",
            [id],
            |r| r.get::<_, String>(0),
        )
        .optional()?
        .ok_or_else(not_found)?;
    invite(&app, id, &email).await?;
    Ok(Redirect::to(&app.url("/settings")))
}
async fn delete_recipient(
    State(app): State<App>,
    Path(id): Path<i64>,
    Form(data): Form<FormData>,
) -> Result<Redirect> {
    csrf(&app, &data)?;
    app.db()?
        .execute("DELETE FROM recipients WHERE id=?", [id])?;
    Ok(Redirect::to(&app.url("/settings")))
}

fn confirmation_id(db: &rusqlite::Connection, token: &str) -> Result<i64> {
    if uuid::Uuid::parse_str(token).is_err() {
        return Err(invalid("Lien invalide ou expiré."));
    }
    let row = db
        .query_row(
            "SELECT id,token_created_at FROM recipients WHERE token=? AND confirmed_at IS NULL",
            [token],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)),
        )
        .optional()?
        .ok_or_else(|| invalid("Lien invalide, déjà utilisé ou expiré."))?;
    let created =
        chrono::DateTime::parse_from_rfc3339(&row.1).map_err(|_| invalid("Lien invalide."))?;
    if Utc::now().signed_duration_since(created).num_days() >= 7 {
        return Err(invalid(
            "Ce lien a expiré. Demandez une nouvelle invitation au coordinateur.",
        ));
    }
    Ok(row.0)
}
async fn confirm_page(
    State(app): State<App>,
    Query(data): Query<FormData>,
) -> Result<Html<String>> {
    confirmation_id(&app.db()?, value(&data, "token"))?;
    // No coordinator CSRF token or private data is disclosed on this public route.
    Ok(Html(format!(
        "<!doctype html><html lang=fr><meta charset=utf-8><meta name=viewport content=\"width=device-width,initial-scale=1\"><title>Confirmation — Permit Alert</title><h1>Confirmer votre inscription</h1><p>Vous recevrez les alertes d’échéance des permis.</p><form method=post action=\"{}\"><input type=hidden name=token value=\"{}\"><button>Je confirme mon inscription</button></form></html>",
        app.url("/confirm"),
        e(value(&data, "token"))
    )))
}
async fn confirm(State(app): State<App>, Form(data): Form<FormData>) -> Result<Html<String>> {
    let mut db = app.db()?;
    let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let id = confirmation_id(&tx, value(&data, "token"))?;
    tx.execute(
        "UPDATE recipients SET confirmed_at=?,token=NULL,token_created_at=NULL WHERE id=?",
        params![Utc::now().to_rfc3339(), id],
    )?;
    tx.commit()?;
    Ok(Html("<!doctype html><html lang=fr><meta charset=utf-8><title>Inscription confirmée</title><h1>Inscription confirmée</h1><p>Vous recevrez les prochaines alertes Permit Alert. Vous pouvez fermer cette page.</p></html>".into()))
}

async fn journal(State(app): State<App>, Query(query): Query<FormData>) -> Result<Html<String>> {
    let db = app.db()?;
    let current_page = query
        .get("page")
        .map(|v| v.parse::<i64>())
        .transpose()
        .map_err(|_| invalid("Page invalide."))?
        .unwrap_or(0);
    let offset = current_page
        .checked_mul(200)
        .filter(|v| *v >= 0)
        .ok_or_else(|| invalid("Page invalide."))?;
    let mut body = format!(
        "<section class=panel><h2>Événements et échecs d’envoi</h2><p>Page {} · 200 événements maximum par liste. Les journaux restent conservés en base.</p><div class=table-wrap><table><tr><th>Date</th><th>Type</th><th>Détail</th></tr>",
        current_page + 1
    );
    let events = db
        .prepare("SELECT created_at,kind,detail FROM events ORDER BY id DESC LIMIT 200 OFFSET ?")?
        .query_map([offset], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut more = events.len() == 200;
    for (at, kind, detail) in events {
        body.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
            e(&at),
            e(&kind),
            e(&detail)
        ));
    }
    body.push_str("</table></div></section><section class=panel><h2>Imports rejetés</h2><div class=table-wrap><table><tr><th>Date</th><th>Source</th><th>Motif</th></tr>");
    let rejects = db
        .prepare(
            "SELECT received_at,sender,reason FROM rejected_emails ORDER BY id DESC LIMIT 200 OFFSET ?",
        )?
        .query_map([offset], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    more |= rejects.len() == 200;
    for (at, sender, reason) in rejects {
        body.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
            e(&at),
            e(&sender),
            e(&reason)
        ));
    }
    body.push_str("</table></div></section>");
    if current_page > 0 {
        body.push_str(&format!(
            "<a class=button href=\"{}?page={}\">Plus récents</a> ",
            app.url("/journal"),
            current_page - 1
        ));
    }
    if more {
        body.push_str(&format!(
            "<a class=button href=\"{}?page={}\">Plus anciens</a>",
            app.url("/journal"),
            current_page + 1
        ));
    }
    Ok(page(&app, "Journal", body))
}
