use crate::{App, web};
use serde::{Deserialize, Serialize};
use std::{env, fs, io::Write, os::unix::fs::PermissionsExt, process::Stdio, time::Duration};
use tokio::{io::AsyncWriteExt, process::Command};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Connectors {
    pub revision: String,
    pub outgoing: Outgoing,
    pub imap: Imap,
    pub ldap: Ldap,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Outgoing {
    pub enabled: bool,
    pub mode: String,
    pub sender: String,
    pub host: String,
    pub port: u16,
    pub tls: String,
    pub username: String,
    pub password: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Imap {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub folder: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ldap {
    pub url: String,
    pub users_base: String,
    pub groups_base: String,
    pub group: String,
    pub bind_dn: String,
    pub bind_password: String,
}

pub fn load(app: &App) -> Result<Connectors, String> {
    match fs::read(&app.0.connector_file) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|_| "Configuration privée des connecteurs illisible.".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Connectors {
            revision: "initial".into(),
            outgoing: Outgoing {
                enabled: true,
                mode: "sendmail".into(),
                sender: app.0.sender.clone(),
                host: String::new(),
                port: 587,
                tls: "starttls".into(),
                username: String::new(),
                password: String::new(),
            },
            imap: Imap {
                enabled: env::var("PERMIT_ALERT_IMAP_HOST").is_ok(),
                host: env::var("PERMIT_ALERT_IMAP_HOST").unwrap_or_default(),
                port: env::var("PERMIT_ALERT_IMAP_PORT")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(993),
                username: env::var("PERMIT_ALERT_IMAP_USER").unwrap_or_default(),
                password: env::var("PERMIT_ALERT_IMAP_PASSWORD").unwrap_or_default(),
                folder: env::var("PERMIT_ALERT_IMAP_FOLDER").unwrap_or_else(|_| "INBOX".into()),
            },
            ldap: Ldap {
                url: "ldap://127.0.0.1".into(),
                users_base: "ou=users,dc=yunohost,dc=org".into(),
                groups_base: "ou=groups,dc=yunohost,dc=org".into(),
                group: env::var("PERMIT_ALERT_LDAP_GROUP")
                    .unwrap_or_else(|_| "coordinateurs".into()),
                bind_dn: String::new(),
                bind_password: String::new(),
            },
        }),
        Err(_) => Err("Impossible de lire la configuration privée des connecteurs.".into()),
    }
}

fn host(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 253
        && (value.parse::<std::net::IpAddr>().is_ok()
            || value
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b".-".contains(&c)))
}
fn clean(value: &str) -> bool {
    value.len() <= 1024 && !value.chars().any(char::is_control)
}

impl Connectors {
    pub fn validate(&self) -> Result<(), String> {
        let smtp = &self.outgoing;
        if !["sendmail", "smtp"].contains(&smtp.mode.as_str())
            || !["starttls", "tls"].contains(&smtp.tls.as_str())
        {
            return Err("Mode d’envoi ou protection TLS invalide.".into());
        }
        if !web::valid_email(&smtp.sender) {
            return Err("Adresse expéditeur invalide.".into());
        }
        if !clean(&smtp.username)
            || !clean(&self.imap.username)
            || !clean(&self.imap.folder)
            || self.imap.folder.is_empty()
        {
            return Err("Identifiant ou dossier de messagerie invalide.".into());
        }
        for secret in [
            &smtp.password,
            &self.imap.password,
            &self.ldap.bind_password,
        ] {
            if secret.len() > 4096 || secret.contains(['\r', '\n', '\0']) {
                return Err(
                    "Un secret dépasse la taille autorisée ou contient un caractère interdit."
                        .into(),
                );
            }
        }
        if (!smtp.host.is_empty() && !host(&smtp.host))
            || (!self.imap.host.is_empty() && !host(&self.imap.host))
        {
            return Err(
                "Utilisez un nom de serveur ou une adresse IP, sans schéma URL ni chemin.".into(),
            );
        }
        if smtp.port == 0 || self.imap.port == 0 {
            return Err("Le port doit être compris entre 1 et 65535.".into());
        }
        if smtp.enabled
            && smtp.mode == "smtp"
            && (!host(&smtp.host) || (!smtp.username.is_empty() && smtp.password.is_empty()))
        {
            return Err("Le serveur SMTP et, si nécessaire, son mot de passe doivent être renseignés avant activation.".into());
        }
        if smtp.username.is_empty() && !smtp.password.is_empty() {
            return Err("Le mot de passe SMTP nécessite un identifiant.".into());
        }
        if self.imap.enabled
            && (!host(&self.imap.host)
                || self.imap.username.is_empty()
                || self.imap.password.is_empty())
        {
            return Err(
                "Renseignez le serveur, l’identifiant et le mot de passe IMAP avant activation."
                    .into(),
            );
        }
        let uri: axum::http::Uri = self.ldap.url.parse().map_err(|_| "URL LDAP invalide.")?;
        let scheme = uri.scheme_str().unwrap_or("");
        let ldap_host = uri.host().unwrap_or("");
        let local = ["127.0.0.1", "localhost", "[::1]", "::1"].contains(&ldap_host);
        if !["ldap", "ldaps"].contains(&scheme)
            || (!local && scheme != "ldaps")
            || !host(ldap_host.trim_matches(['[', ']']))
            || uri.authority().is_some_and(|a| a.as_str().contains('@'))
            || uri.path() != "/"
            || uri.query().is_some()
            || uri.port_u16() == Some(0)
        {
            return Err("LDAP local : ldap://127.0.0.1. Pour un serveur distant, utilisez ldaps:// avec certificat valide, sans chemin ni identifiant dans l’URL.".into());
        }
        for field in [
            &self.ldap.users_base,
            &self.ldap.groups_base,
            &self.ldap.group,
        ] {
            if field.is_empty() || !clean(field) {
                return Err(
                    "Les bases LDAP et le groupe sont obligatoires et doivent être valides.".into(),
                );
            }
        }
        if !clean(&self.ldap.bind_dn)
            || (self.ldap.bind_dn.is_empty() && !self.ldap.bind_password.is_empty())
            || (!self.ldap.bind_dn.is_empty() && self.ldap.bind_password.is_empty())
        {
            return Err("Renseignez ensemble le DN de connexion LDAP et son mot de passe, ou laissez les deux vides pour une recherche anonyme.".into());
        }
        Ok(())
    }
}

pub fn save(app: &App, mut config: Connectors, revision: &str) -> Result<(), String> {
    let _guard = app
        .0
        .connector_lock
        .lock()
        .map_err(|_| "Configuration temporairement indisponible.")?;
    if load(app)?.revision != revision {
        return Err("Les connecteurs ont été modifiés par un autre coordinateur. Rechargez la page avant de réessayer.".into());
    }
    config.validate()?;
    config.revision = uuid::Uuid::new_v4().to_string();
    let directory = app
        .0
        .connector_file
        .parent()
        .ok_or("Répertoire privé indisponible.")?;
    fs::create_dir_all(directory).map_err(|_| "Création du répertoire privé impossible.")?;
    fs::set_permissions(directory, fs::Permissions::from_mode(0o700))
        .map_err(|_| "Protection du répertoire privé impossible.")?;
    let mut file =
        tempfile::NamedTempFile::new_in(directory).map_err(|_| "Écriture privée impossible.")?;
    file.as_file()
        .set_permissions(fs::Permissions::from_mode(0o600))
        .map_err(|_| "Protection du fichier privé impossible.")?;
    file.write_all(&serde_json::to_vec_pretty(&config).map_err(|_| "Configuration invalide.")?)
        .map_err(|_| "Écriture privée impossible.")?;
    file.as_file()
        .sync_all()
        .map_err(|_| "Enregistrement privé impossible.")?;
    file.persist(&app.0.connector_file)
        .map_err(|_| "Enregistrement atomique impossible.")?;
    Ok(())
}

#[derive(Deserialize)]
pub struct Diagnostic {
    pub ok: bool,
    pub message: String,
}

pub async fn helper(
    app: &App,
    action: &str,
    payload: serde_json::Value,
) -> Result<Diagnostic, String> {
    tokio::time::timeout(Duration::from_secs(45), async {
        let mut child = Command::new("python3")
            .arg(&app.0.connector_script)
            .arg(action)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| "Connecteur indisponible sur ce serveur.")?;
        let mut stdin = child.stdin.take().ok_or("Connecteur indisponible.")?;
        stdin
            .write_all(&serde_json::to_vec(&payload).map_err(|_| "Configuration invalide.")?)
            .await
            .map_err(|_| "Impossible de transmettre la configuration privée.")?;
        drop(stdin);
        let output = child
            .wait_with_output()
            .await
            .map_err(|_| "Diagnostic interrompu.")?;
        if !output.status.success() {
            return Err("Le connecteur a échoué. Vérifiez les dépendances installées.".into());
        }
        serde_json::from_slice::<Diagnostic>(&output.stdout)
            .map_err(|_| "Réponse du connecteur invalide.".into())
    })
    .await
    .map_err(|_| "Délai de connexion dépassé.".to_owned())?
}

pub async fn diagnose(app: &App, config: &Connectors, kind: &str) -> Result<Diagnostic, String> {
    if kind == "sendmail" && config.outgoing.mode == "sendmail" {
        let executable = fs::metadata(&app.0.sendmail)
            .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0);
        return Ok(Diagnostic {ok:executable,message:if executable {"Programme d’envoi local présent. Aucun e-mail envoyé ; l’acceptation par Postfix reste à vérifier par un envoi réel."} else {"Programme d’envoi local absent ou non exécutable."}.into()});
    }
    if !["smtp", "imap", "ldap"].contains(&kind) {
        return Err("Connecteur inconnu.".into());
    }
    helper(
        app,
        &format!("test-{kind}"),
        serde_json::json!({"config":config}),
    )
    .await
}
