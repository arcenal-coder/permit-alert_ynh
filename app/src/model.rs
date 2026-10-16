use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Permit {
    pub number: String,
    pub status: String,
    pub unit: String,
    pub equipment: String,
    pub companies: String,
    pub intervention: String,
    pub start_date: String,
    pub end_date: String,
    #[serde(default)]
    pub extra: BTreeMap<String, String>,
}

impl Permit {
    pub fn validate(&self) -> Result<(), String> {
        if self.number.trim().is_empty() || self.number.len() > 120 {
            return Err("Le numéro du permis est obligatoire (120 caractères maximum).".into());
        }
        if self.status.trim().is_empty() {
            return Err("L’état du permis est obligatoire.".into());
        }
        let end = date(&self.end_date).ok_or("La date de fin doit être au format AAAA-MM-JJ.")?;
        if !self.start_date.is_empty() {
            let start = date(&self.start_date).ok_or("Date de début invalide.")?;
            if start > end {
                return Err("La date de début dépasse la date de fin.".into());
            }
        }
        if serde_json::to_string(self)
            .map_err(|e| e.to_string())?
            .len()
            > 64_000
        {
            return Err("Les données d’un permis dépassent la limite de 64 Ko.".into());
        }
        Ok(())
    }
}

pub fn date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}

pub fn normalized_date(value: &str) -> String {
    let trimmed = value.trim().split(['T', ' ']).next().unwrap_or("");
    for format in ["%Y-%m-%d", "%d/%m/%Y", "%d-%m-%Y", "%d.%m.%Y"] {
        if let Ok(value) = NaiveDate::parse_from_str(trimmed, format) {
            return value.to_string();
        }
    }
    trimmed.into()
}

pub fn due(end: &str, today: NaiveDate, lead: i64) -> bool {
    date(end).is_some_and(|end| {
        let remaining = (end - today).num_days();
        remaining > 0 && remaining <= lead
    })
}
