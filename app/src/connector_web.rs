use crate::{
    App,
    connectors::{self, Connectors},
    db,
    web::{self, FormData, Result, e, field, page, token},
};
use axum::{
    extract::{Form, Path, Query, State},
    response::{Html, Redirect},
};

fn checkbox(name: &str, label: &str, checked: bool) -> String {
    format!(
        "<label class=check><input type=checkbox name=\"{}\" value=yes {}>{}</label>",
        e(name),
        if checked { "checked" } else { "" },
        e(label)
    )
}
fn secret(name: &str, label: &str, present: bool) -> String {
    format!(
        "<label>{}<input type=password name=\"{}\" autocomplete=new-password value=\"\" placeholder=\"{}\"></label>{}",
        e(label),
        e(name),
        if present {
            "Secret enregistré — vide pour le conserver"
        } else {
            "Non renseigné"
        },
        checkbox(
            &format!("clear_{name}"),
            "Effacer le secret enregistré",
            false
        )
    )
}
fn select(name: &str, label: &str, current: &str, choices: &[(&str, &str)]) -> String {
    let mut html = format!("<label>{}<select name=\"{}\">", e(label), e(name));
    for (key, label) in choices {
        html.push_str(&format!(
            "<option value=\"{}\" {}>{}</option>",
            e(key),
            if *key == current { "selected" } else { "" },
            e(label)
        ));
    }
    html.push_str("</select></label>");
    html
}
fn render(app: &App, config: &Connectors, notice: Option<(&str, bool)>) -> Html<String> {
    let smtp = &config.outgoing;
    let imap = &config.imap;
    let ldap = &config.ldap;
    let mut body = String::new();
    if let Some((notice, success)) = notice {
        body.push_str(&format!(
            "<p class=\"notice {}\" role=status>{}</p>",
            if success { "" } else { "diagnostic-error" },
            e(notice)
        ));
    }
    body.push_str(&format!("<p>Paramétrez les connexions sur ce serveur. Les diagnostics utilisent la dernière configuration enregistrée et ne transmettent aucun e-mail, n’importent aucun PDF et ne suppriment aucun message.</p><form method=post action=\"{}\">{}<input type=hidden name=revision value=\"{}\"><section class=panel><h2>Envoi des notifications</h2>{}<div class=fields>{}{}{}{}{}{}{}</div>{}</section>",app.url("/connectors"),token(app),e(&config.revision),checkbox("outgoing_enabled","Activer l’envoi des notifications",smtp.enabled),select("outgoing_mode","Connecteur d’envoi",&smtp.mode,&[("sendmail","Messagerie locale YunoHost"),("smtp","Serveur SMTP avec TLS")]),field("sender","Adresse expéditeur",&smtp.sender,"email",true),field("smtp_host","Serveur SMTP",&smtp.host,"text",false),field("smtp_port","Port SMTP",&smtp.port.to_string(),"number",true),select("smtp_tls","Sécurité SMTP",&smtp.tls,&[("starttls","STARTTLS obligatoire (généralement 587)"),("tls","TLS implicite (généralement 465)")]),field("smtp_username","Identifiant SMTP (vide si sans authentification)",&smtp.username,"text",false),"<p>Les paramètres SMTP ne sont utilisés qu’en mode Serveur SMTP. La vérification du certificat TLS est obligatoire.</p>",secret("smtp_password","Mot de passe SMTP",!smtp.password.is_empty())));
    body.push_str(&format!("<section class=panel><h2>Réception des PDF par IMAP TLS</h2><p>Utiliser une boîte dédiée : une fois la relève activée, les messages traités ou rejetés sont supprimés après journalisation. Le test de connexion reste sans traitement de message.</p>{}<div class=fields>{}{}{}{}</div>{}</section>",checkbox("imap_enabled","Activer la relève automatique de la boîte dédiée",imap.enabled),field("imap_host","Serveur IMAP",&imap.host,"text",false),field("imap_port","Port IMAP TLS",&imap.port.to_string(),"number",true),field("imap_username","Identifiant IMAP",&imap.username,"text",false),field("imap_folder","Dossier à relever",&imap.folder,"text",true),secret("imap_password","Mot de passe IMAP",!imap.password.is_empty())));
    body.push_str(&format!("<section class=panel><h2>Annuaire des expéditeurs autorisés</h2><p>Ce connecteur vérifie les expéditeurs des imports e-mail. Les accès web restent gérés par les permissions YunoHost.</p><div class=fields>{}{}{}{}{}</div>{}<p>LDAP non chiffré limité à la boucle locale. Pour un annuaire distant, LDAPS avec certificat valide. Sans DN ni mot de passe, la recherche est anonyme.</p></section><p>Un mot de passe vide conserve le secret existant uniquement si le serveur et le compte sont inchangés. Si vous les modifiez, renseignez de nouveau le secret.</p><button>Enregistrer les connecteurs</button></form>",field("ldap_url","URL LDAP",&ldap.url,"text",true),field("ldap_users_base","Base de recherche des utilisateurs",&ldap.users_base,"text",true),field("ldap_groups_base","Base de recherche des groupes",&ldap.groups_base,"text",true),field("ldap_group","Groupe des coordinateurs",&ldap.group,"text",true),field("ldap_bind_dn","DN de connexion (facultatif)",&ldap.bind_dn,"text",false),secret("ldap_password","Mot de passe de connexion LDAP",!ldap.bind_password.is_empty())));
    body.push_str("<section class=panel><h2>Diagnostics de la configuration enregistrée</h2><p>L’enregistrement ne lance pas de test. Vous pouvez tester les connexions même si leur traitement automatique est désactivé. Les résultats ne confirment pas la délivrabilité d’un e-mail ni la validité d’un PDF réel.</p><div class=inline>");
    for (kind, label) in [
        (
            if smtp.mode == "sendmail" {
                "sendmail"
            } else {
                "smtp"
            },
            "Tester l’envoi",
        ),
        ("imap", "Tester IMAP"),
        ("ldap", "Tester LDAP"),
    ] {
        body.push_str(&format!("<form method=post action=\"{}\">{}<input type=hidden name=revision value=\"{}\"><button class=secondary>{}</button></form>",app.url(&format!("/connectors/test/{kind}")),token(app),e(&config.revision),e(label)));
    }
    body.push_str("</div></section>");
    page(app, "Connecteurs", body)
}

pub async fn index(State(app): State<App>, Query(query): Query<FormData>) -> Result<Html<String>> {
    let config = connectors::load(&app).map_err(web::invalid)?;
    let notice=query.get("saved").map(|_|("Connecteurs enregistrés. Ils seront utilisés dès la prochaine opération, sans redémarrage du service.",true));
    Ok(render(&app, &config, notice))
}

fn text(data: &FormData, key: &str) -> String {
    data.get(key)
        .map(|s| s.trim().to_owned())
        .unwrap_or_default()
}
fn password(data: &FormData, key: &str, old: &str, same_account: bool) -> String {
    if data
        .get(&format!("clear_{key}"))
        .is_some_and(|s| s == "yes")
    {
        return String::new();
    }
    match data.get(key) {
        Some(value) if !value.is_empty() => value.clone(),
        _ if same_account => old.into(),
        _ => String::new(),
    }
}
pub async fn save(State(app): State<App>, Form(data): Form<FormData>) -> Result<Redirect> {
    web::csrf(&app, &data)?;
    let mut config = connectors::load(&app).map_err(web::invalid)?;
    let smtp_host = text(&data, "smtp_host");
    let smtp_username = text(&data, "smtp_username");
    let imap_host = text(&data, "imap_host");
    let imap_username = text(&data, "imap_username");
    let ldap_url = text(&data, "ldap_url");
    let ldap_bind_dn = text(&data, "ldap_bind_dn");
    let port = |key: &str| {
        text(&data, key).parse::<u16>().map_err(|_| {
            web::invalid("Port invalide : utilisez un entier compris entre 1 et 65535.")
        })
    };
    let smtp_port = port("smtp_port")?;
    let imap_port = port("imap_port")?;
    config.outgoing.password = password(
        &data,
        "smtp_password",
        &config.outgoing.password,
        smtp_host == config.outgoing.host
            && smtp_username == config.outgoing.username
            && smtp_port == config.outgoing.port,
    );
    config.imap.password = password(
        &data,
        "imap_password",
        &config.imap.password,
        imap_host == config.imap.host
            && imap_username == config.imap.username
            && imap_port == config.imap.port,
    );
    config.ldap.bind_password = password(
        &data,
        "ldap_password",
        &config.ldap.bind_password,
        ldap_url == config.ldap.url && ldap_bind_dn == config.ldap.bind_dn,
    );
    config.outgoing.enabled = text(&data, "outgoing_enabled") == "yes";
    config.outgoing.mode = text(&data, "outgoing_mode");
    config.outgoing.sender = text(&data, "sender");
    config.outgoing.host = smtp_host;
    config.outgoing.port = smtp_port;
    config.outgoing.tls = text(&data, "smtp_tls");
    config.outgoing.username = smtp_username;
    config.imap.enabled = text(&data, "imap_enabled") == "yes";
    config.imap.host = imap_host;
    config.imap.port = imap_port;
    config.imap.username = imap_username;
    config.imap.folder = text(&data, "imap_folder");
    config.ldap.url = ldap_url;
    config.ldap.bind_dn = ldap_bind_dn;
    config.ldap.group = text(&data, "ldap_group");
    config.ldap.users_base = text(&data, "ldap_users_base");
    config.ldap.groups_base = text(&data, "ldap_groups_base");
    connectors::save(&app, config, &text(&data, "revision")).map_err(web::invalid)?;
    db::event(
        &app.db()?,
        "Connecteurs",
        "Configuration modifiée (secrets masqués).",
    )?;
    Ok(Redirect::to(&app.url("/connectors?saved=1")))
}

pub async fn test(
    State(app): State<App>,
    Path(kind): Path<String>,
    Form(data): Form<FormData>,
) -> Result<Html<String>> {
    web::csrf(&app, &data)?;
    let config = connectors::load(&app).map_err(web::invalid)?;
    if text(&data, "revision") != config.revision {
        return Err(web::invalid(
            "La configuration a changé. Rechargez la page avant le diagnostic.",
        ));
    }
    let result = connectors::diagnose(&app, &config, &kind)
        .await
        .map_err(web::invalid)?;
    let message = format!(
        "{} : {}",
        if result.ok {
            "Diagnostic réussi"
        } else {
            "Diagnostic en échec"
        },
        result.message
    );
    // Only predefined, sanitized helper messages reach the journal/UI.
    db::event(&app.db()?, "Diagnostic connecteur", &message)?;
    Ok(render(&app, &config, Some((&message, result.ok))))
}
