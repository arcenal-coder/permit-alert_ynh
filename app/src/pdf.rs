use crate::model::{Permit, normalized_date};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
};
use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};

pub const MAX_PDF: usize = 10 * 1024 * 1024;

fn normalize(value: &str) -> String {
    value
        .nfd()
        .filter(|c| !is_combining_mark(*c))
        .collect::<String>()
        .to_lowercase()
        .replace(['_', '-', '°', 'º', '.'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn key(label: &str) -> Option<&'static str> {
    match normalize(label).trim() {
        "numero" | "numero de permis" | "n de permis" | "n permis" | "permis" | "permit number"
        | "permit no" => Some("number"),
        "etat" | "statut" | "status" => Some("status"),
        "unite" | "unit" => Some("unit"),
        "equipement" | "equipment" => Some("equipment"),
        "entreprise" | "entreprises" | "company" | "companies" => Some("companies"),
        "intervention" | "travaux" | "description" | "nature des travaux" => Some("intervention"),
        "debut" | "date debut" | "date de debut" | "start date" => Some("start_date"),
        "fin" | "date fin" | "date de fin" | "end date" => Some("end_date"),
        _ => None,
    }
}

fn assign(permit: &mut Permit, label: &str, value: &str) {
    let value = value.trim();
    match key(label) {
        Some("number") => permit.number = value.into(),
        Some("status") => permit.status = value.into(),
        Some("unit") => permit.unit = value.into(),
        Some("equipment") => permit.equipment = value.into(),
        Some("companies") => permit.companies = value.into(),
        Some("intervention") => permit.intervention = value.into(),
        Some("start_date") => {
            permit.start_date = normalized_date(value);
            if value.contains(':') {
                permit
                    .extra
                    .insert(format!("{} (source)", label.trim()), value.into());
            }
        }
        Some("end_date") => {
            permit.end_date = normalized_date(value);
            if value.contains(':') {
                permit
                    .extra
                    .insert(format!("{} (source)", label.trim()), value.into());
            }
        }
        _ => {
            permit.extra.insert(label.trim().into(), value.into());
        }
    }
}

// Column starts follow pdftotext -layout spacing. Delimited exports (| or tabs)
// are also accepted. Unknown columns are kept, never silently discarded.
fn columns(line: &str) -> Vec<(usize, String)> {
    let bytes = line.as_bytes();
    let mut result = Vec::new();
    let mut start = None;
    let mut index = 0;
    while index < bytes.len() {
        let separator = bytes[index] == b'|'
            || bytes[index] == b'\t'
            || (bytes[index] == b' ' && bytes.get(index + 1) == Some(&b' '));
        if separator {
            if let Some(begin) = start.take() {
                result.push((begin, line[begin..index].trim().to_owned()));
            }
            while index < bytes.len()
                && (bytes[index] == b' ' || bytes[index] == b'|' || bytes[index] == b'\t')
            {
                index += 1;
            }
        } else {
            if start.is_none() && bytes[index] != b' ' {
                start = Some(index);
            }
            index += 1;
        }
    }
    if let Some(begin) = start {
        result.push((begin, line[begin..].trim().to_owned()));
    }
    result
}

pub fn parse(text: &str) -> Result<Vec<Permit>, String> {
    let marker = normalize(text);
    if !marker.contains("e permit") && !marker.contains("epermit") {
        return Err("Document non reconnu : la mention E-Permit est absente.".into());
    }
    if text.len() > 2_000_000 {
        return Err("Le texte extrait dépasse la limite autorisée.".into());
    }
    let mut permits = Vec::new();
    let mut current = Permit::default();
    let mut header: Vec<(usize, String)> = Vec::new();
    for line in text
        .lines()
        .map(|l| l.trim_end_matches('\u{c}'))
        .filter(|l| !l.trim().is_empty())
    {
        let cells = columns(line);
        if cells.iter().any(|(_, s)| key(s) == Some("number"))
            && cells.iter().any(|(_, s)| key(s) == Some("end_date"))
        {
            header = cells;
            continue;
        }
        if !header.is_empty() {
            let values = columns(line);
            if values.len() == header.len() {
                let mut permit = Permit::default();
                for ((_, label), (_, value)) in header.iter().zip(values) {
                    assign(&mut permit, label, &value);
                }
                if !permit.number.is_empty() && crate::model::date(&permit.end_date).is_some() {
                    permits.push(permit);
                    continue;
                }
            }
            // Reject ambiguous wrapped rows rather than guessing column boundaries.
            let footer = normalize(line);
            if footer.starts_with("page ") || footer == "e permit" || footer == "epermit" {
                continue;
            }
            return Err("Une ligne du tableau E-Permit n’a pas pu être analysée sans ambiguïté. L’import entier est rejeté pour éviter de perdre des permis.".into());
        }
        if let Some((label, value)) = line.split_once(':') {
            if key(label) == Some("number") && !current.number.is_empty() {
                permits.push(std::mem::take(&mut current));
            }
            assign(&mut current, label, value);
        }
    }
    if !current.number.is_empty() {
        permits.push(current);
    }
    if permits.is_empty() || permits.len() > 200 {
        return Err(
            "Structure E-Permit non reconnue ou plus de 200 permis. Aucun permis enregistré."
                .into(),
        );
    }
    let mut numbers = BTreeMap::new();
    for permit in &permits {
        if permit.number.is_empty() || crate::model::date(&permit.end_date).is_none() {
            return Err(
                "Structure incomplète : numéro et date de fin doivent être identifiables.".into(),
            );
        }
        if numbers.insert(&permit.number, ()).is_some() {
            return Err(
                "Le PDF contient plusieurs lignes du même numéro ; import ambigu rejeté.".into(),
            );
        }
    }
    Ok(permits)
}

pub async fn extract(bytes: &[u8], directory: &Path) -> Result<(PathBuf, Vec<Permit>), String> {
    if bytes.len() > MAX_PDF || !bytes.starts_with(b"%PDF-") {
        return Err("Un PDF valide de 10 Mo maximum est requis.".into());
    }
    let path = directory.join(format!("{}.pdf", uuid::Uuid::new_v4()));
    let result = async {
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .await
            .map_err(|_| "Stockage temporaire indisponible.")?;
        file.write_all(bytes)
            .await
            .map_err(|_| "Écriture temporaire impossible.")?;
        drop(file);
        let info = tokio::time::timeout(
            Duration::from_secs(10),
            Command::new("pdfinfo")
                .arg(&path)
                .stderr(std::process::Stdio::null())
                .kill_on_drop(true)
                .output(),
        )
        .await
        .map_err(|_| "Analyse PDF : délai dépassé.")?
        .map_err(|_| "Analyse PDF indisponible (pdfinfo).")?;
        if !info.status.success() {
            return Err("Document PDF illisible ou protégé.".into());
        }
        let pages = String::from_utf8_lossy(&info.stdout)
            .lines()
            .find_map(|line| {
                line.strip_prefix("Pages:")
                    .and_then(|s| s.trim().parse::<usize>().ok())
            });
        if !matches!(pages, Some(1..=50)) {
            return Err(
                "Le PDF doit contenir entre 1 et 50 pages. Aucun import partiel n’est effectué."
                    .into(),
            );
        }
        // Bound the pipe before allocating or writing extracted data. Cancelling
        // the timeout drops and kills the child process as well.
        tokio::time::timeout(Duration::from_secs(20), async {
            let mut child = Command::new("pdftotext")
                .args(["-layout", "-enc", "UTF-8", "-f", "1", "-l", "50"])
                .arg(&path)
                .arg("-")
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .kill_on_drop(true)
                .spawn()
                .map_err(|_| "Extraction PDF indisponible.")?;
            let mut text = Vec::new();
            child
                .stdout
                .take()
                .ok_or("Texte extrait indisponible.")?
                .take(2_000_001)
                .read_to_end(&mut text)
                .await
                .map_err(|_| "Texte PDF illisible.")?;
            if text.len() > 2_000_000 {
                return Err("PDF trop volumineux une fois extrait.".into());
            }
            if !child
                .wait()
                .await
                .map_err(|_| "Extraction interrompue.")?
                .success()
            {
                return Err("PDF illisible ou protégé.".into());
            }
            parse(std::str::from_utf8(&text).map_err(|_| "Texte PDF illisible.")?)
        })
        .await
        .map_err(|_| "Extraction PDF : délai dépassé.".to_owned())?
    }
    .await;
    match result {
        Ok(permits) => Ok((path, permits)),
        Err(error) => {
            let _ = tokio::fs::remove_file(path).await;
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn labelled_multiple_permits_and_unknown_fields() {
        let permits = parse("E-Permit\nNuméro de permis : P-1\nÉtat : Validé\nDate de fin : 31/12/2027\nZone : Z2\nNuméro de permis : P-2\nDate de fin : 2027-12-30").unwrap();
        assert_eq!(permits.len(), 2);
        assert_eq!(permits[0].extra["Zone"], "Z2");
        assert_eq!(permits[0].end_date, "2027-12-31");
    }
    #[test]
    fn table_and_rejections() {
        let permits =
            parse("E-Permit\nNuméro  État  Unité  Date de fin\nP-1     Actif  U2     31/12/2027")
                .unwrap();
        assert_eq!(permits[0].unit, "U2");
        assert!(parse("Facture\nNuméro : 123\nDate de fin : 2027-12-31").is_err());
        assert!(parse("E-Permit\nNuméro : 123\nDate de fin : hier").is_err());
        assert!(
            parse(
                "E-Permit\nNuméro  État  Date de fin\nP-1  Actif  31/12/2027\nP-2  Actif  inconnu"
            )
            .is_err()
        );
        let p = parse("E-Permit\nNuméro : P-1\nDate de fin : 31/12/2027 23:59").unwrap();
        assert_eq!(p[0].end_date, "2027-12-31");
        assert_eq!(p[0].extra["Date de fin (source)"], "31/12/2027 23:59");
    }
}
