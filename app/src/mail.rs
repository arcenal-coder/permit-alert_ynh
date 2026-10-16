use crate::{App, db, model::due, web};
use base64::{Engine, engine::general_purpose::STANDARD};
use chrono::{Local, Utc};
use rusqlite::{OptionalExtension, params};
use serde::Deserialize;
use std::{env, process::Stdio, time::Duration};
use tokio::{io::AsyncWriteExt, process::Command};

pub async fn send(app: &App, recipient: &str, subject: &str, body: &str) -> Result<(), String> {
    if !web::valid_email(recipient)
        || !web::valid_email(&app.0.sender)
        || subject.contains(['\r', '\n'])
    {
        return Err("Adresse ou en-tête de notification invalide.".into());
    }
    let encoded = STANDARD.encode(body.as_bytes());
    let wrapped = encoded
        .as_bytes()
        .chunks(76)
        .map(|line| std::str::from_utf8(line).unwrap())
        .collect::<Vec<_>>()
        .join("\r\n");
    let message = format!(
        "From: {}\r\nTo: {}\r\nSubject: =?UTF-8?B?{}?=\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=UTF-8\r\nContent-Transfer-Encoding: base64\r\n\r\n{}\r\n",
        app.0.sender,
        recipient,
        STANDARD.encode(subject),
        wrapped
    );
    let result = tokio::time::timeout(Duration::from_secs(20), async {
        let mut child = Command::new(&app.0.sendmail)
            .args(["-i", "-t", "-f", &app.0.sender])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| "Service d’envoi indisponible.")?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or("Entrée du service d’envoi indisponible.")?;
        stdin
            .write_all(message.as_bytes())
            .await
            .map_err(|_| "Écriture du message impossible.")?;
        drop(stdin);
        if !child
            .wait()
            .await
            .map_err(|_| "Échec du service d’envoi.")?
            .success()
        {
            return Err("La messagerie a refusé le message.");
        }
        Ok(())
    })
    .await;
    match result {
        Ok(value) => value.map_err(String::from),
        Err(_) => Err("Délai d’envoi dépassé.".into()),
    }
}

pub async fn alert_tick(app: &App) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let db = app.db()?;
    if db::setting(&db, "configured", "0") != "1" {
        return Ok(());
    }
    let lead: i64 = db::setting(&db, "lead_days", "14").parse()?;
    let frequency: i64 = db::setting(&db, "frequency_days", "1").parse()?;
    let recipients = db
        .prepare("SELECT id,email FROM recipients WHERE confirmed_at IS NOT NULL ORDER BY id")?
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    let mut had_errors = false;
    for (id, permit) in db::permits(&db)? {
        if !due(&permit.end_date, Local::now().date_naive(), lead) {
            continue;
        }
        for (recipient_id, email) in &recipients {
            let previous: Option<String> = db
                .query_row(
                    "SELECT sent_at FROM deliveries WHERE permit_id=? AND recipient_id=?",
                    params![id, recipient_id],
                    |r| r.get(0),
                )
                .optional()?;
            if previous
                .as_ref()
                .and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok())
                .is_some_and(|sent| {
                    Utc::now().signed_duration_since(sent).num_seconds() < frequency * 86400
                })
            {
                continue;
            }
            // Re-check before sending: a recipient or permit may have been deleted
            // while another notification was being delivered.
            let exists:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM permits p,recipients r WHERE p.id=? AND r.id=? AND r.confirmed_at IS NOT NULL)",params![id,recipient_id],|r|r.get(0))?;
            if !exists {
                continue;
            }
            let body = format!(
                "Le permis {} arrive à échéance le {}.\n\nÉtat : {}\nUnité : {}\nÉquipement : {}\nEntreprises : {}\nIntervention : {}\n\nConsulter Permit Alert : {}/permits/{}\n\nContactez votre coordinateur pour retirer votre adresse de la liste des destinataires.",
                permit.number,
                permit.end_date,
                permit.status,
                permit.unit,
                permit.equipment,
                permit.companies,
                permit.intervention,
                app.0.public_url,
                id
            );
            match send(
                app,
                email,
                &format!("Permis {} : échéance le {}", permit.number, permit.end_date),
                &body,
            )
            .await
            {
                Ok(()) => {
                    // The user can delete during the SMTP call; don't recreate data.
                    db.execute("INSERT INTO deliveries(permit_id,recipient_id,sent_at) SELECT ?,?,? WHERE EXISTS(SELECT 1 FROM permits WHERE id=?) AND EXISTS(SELECT 1 FROM recipients WHERE id=?) ON CONFLICT(permit_id,recipient_id) DO UPDATE SET sent_at=excluded.sent_at",params![id,recipient_id,Utc::now().to_rfc3339(),id,recipient_id])?;
                    db::event(
                        &db,
                        "Alerte envoyée",
                        &format!("Permis {} → {}", permit.number, email),
                    )?;
                }
                Err(error) => {
                    db::event(
                        &db,
                        "Échec d’envoi",
                        &format!("Permis {} → {} : {}", permit.number, email, error),
                    )?;
                    // Don't retry every minute after a persistent mail failure.
                    // A global backoff still allows retries later in the hour.
                    had_errors = true;
                }
            }
        }
    }
    if had_errors {
        return Err(
            "Certaines notifications ont échoué ; les autres destinataires ont été traités.".into(),
        );
    }
    Ok(())
}

#[derive(Deserialize)]
struct Incoming {
    mailbox: String,
    uid: String,
    validity: String,
    sender: String,
    reason: Option<String>,
    pdf: Option<String>,
}

async fn mailbox_command(action: &str, args: &[&str]) -> Result<Vec<u8>, String> {
    let script =
        env::var("PERMIT_ALERT_MAILBOX_SCRIPT").map_err(|_| "Script de relève non configuré.")?;
    let output = tokio::time::timeout(
        Duration::from_secs(60),
        Command::new("python3")
            .arg(script)
            .arg(action)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .output(),
    )
    .await
    .map_err(|_| "Relève IMAP : délai dépassé.")?
    .map_err(|_| "Impossible de lancer la relève IMAP.")?;
    if !output.status.success() {
        return Err("Relève IMAP/LDAP/DKIM impossible : vérifiez le fichier privé de messagerie et les services réseau.".into());
    }
    Ok(output.stdout)
}

async fn mailbox_tick(app: &App) -> Result<(), String> {
    if env::var("PERMIT_ALERT_IMAP_HOST").is_err() {
        return Ok(());
    }
    let output = mailbox_command("fetch", &[]).await?;
    let incoming: Option<Incoming> =
        serde_json::from_slice(&output).map_err(|_| "Réponse de la relève invalide.")?;
    let Some(incoming) = incoming else {
        return Ok(());
    };
    let key = format!(
        "{}:{}:{}",
        incoming.mailbox, incoming.validity, incoming.uid
    );
    let mut db = app.db().map_err(|_| "Base indisponible.")?;
    let processed: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM processed_mail WHERE message_key=?)",
            [&key],
            |r| r.get(0),
        )
        .map_err(|_| "Base indisponible.")?;
    if !processed {
        if let Some(reason) = incoming.reason {
            let tx = db.transaction().map_err(|_| "Base indisponible.")?;
            db::reject(&tx, &incoming.sender, &reason).map_err(|_| "Journal indisponible.")?;
            tx.execute("INSERT INTO processed_mail VALUES (?)", [&key])
                .map_err(|_| "Base indisponible.")?;
            tx.commit().map_err(|_| "Base indisponible.")?;
        } else {
            let bytes = STANDARD
                .decode(incoming.pdf.unwrap_or_default())
                .map_err(|_| "Pièce jointe invalide.")?;
            match web::create_draft_with_key(
                app,
                &bytes,
                &format!("E-mail : {}", incoming.sender),
                Some(&key),
            )
            .await
            {
                Ok(_) => {
                    db::event(
                        &db,
                        "Import e-mail",
                        &format!("Un PDF de {} attend une validation.", incoming.sender),
                    )
                    .map_err(|_| "Journal indisponible.")?;
                }
                Err(error) => {
                    if error == "L’import n’a pas pu être sauvegardé." {
                        return Err(error);
                    }
                    let tx = db.transaction().map_err(|_| "Base indisponible.")?;
                    db::reject(&tx, &incoming.sender, &error)
                        .map_err(|_| "Journal indisponible.")?;
                    tx.execute("INSERT INTO processed_mail VALUES (?)", [&key])
                        .map_err(|_| "Base indisponible.")?;
                    tx.commit().map_err(|_| "Base indisponible.")?;
                }
            }
        }
    }
    // Only acknowledge after the result has been persisted; retry on restart.
    mailbox_command("ack", &[&incoming.validity, &incoming.uid]).await?;
    Ok(())
}

pub async fn worker(app: App) {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut retry_alert = Utc::now();
    let mut last_mail_error = String::new();
    loop {
        interval.tick().await;
        if let Err(error) = crate::cleanup(&app) {
            eprintln!("Maintenance failed: {error}");
        }
        if Utc::now() >= retry_alert
            && let Err(error) = alert_tick(&app).await
        {
            eprintln!("Alert task failed: {error}");
            retry_alert = Utc::now() + chrono::Duration::minutes(30);
        }
        match mailbox_tick(&app).await {
            Ok(()) => last_mail_error.clear(),
            Err(error) => {
                if error != last_mail_error {
                    if let Ok(db) = app.db() {
                        let _ = db::event(&db, "Erreur de relève", &error);
                    }
                    last_mail_error = error;
                }
            }
        }
    }
}
