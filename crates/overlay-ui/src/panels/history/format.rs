//! Formats d'affichage communs aux vues de la fenêtre d'historique — ceux du site en français
//! (`I18nService` : `formatRelativeDay`, `formatDate` en `dateStyle: 'medium'`, `formatNumber`).

use chrono::{Datelike, Local, NaiveDate, TimeZone};

pub(crate) use crate::panels::combat::format_fr_thousands as thousands;

const MONTHS_SHORT: [&str; 12] = [
    "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.",
    "déc.",
];

/// Le jour **local** d'un instant (ms UTC) — les groupes par jour suivent le calendrier de
/// l'utilisateur, comme sur le site.
pub fn local_day(ms: i64) -> NaiveDate {
    Local
        .timestamp_millis_opt(ms)
        .earliest()
        .map(|moment| moment.date_naive())
        .unwrap_or_default()
}

/// `27 nov. 2026` — `Intl.DateTimeFormat('fr-FR', { dateStyle: 'medium' })`.
pub fn medium_date(day: NaiveDate) -> String {
    format!(
        "{} {} {}",
        day.day(),
        MONTHS_SHORT[day.month0() as usize],
        day.year()
    )
}

/// « Aujourd'hui », « Hier », « Avant-hier », sinon la date — `formatRelativeDay`.
pub fn relative_day(day: NaiveDate, today: NaiveDate) -> String {
    match (today - day).num_days() {
        0 => "Aujourd'hui".to_string(),
        1 => "Hier".to_string(),
        2 => "Avant-hier".to_string(),
        _ => medium_date(day),
    }
}

/// `12 345 ₭`.
pub fn kamas(amount: i64) -> String {
    format!("{} ₭", thousands(amount))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_jours_proches_sont_nommes_comme_sur_le_site() {
        let today = NaiveDate::from_ymd_opt(2026, 11, 27).unwrap();
        assert_eq!(relative_day(today, today), "Aujourd'hui");
        assert_eq!(relative_day(today.pred_opt().unwrap(), today), "Hier");
        let three = NaiveDate::from_ymd_opt(2026, 11, 24).unwrap();
        assert_eq!(relative_day(three, today), "24 nov. 2026");
        assert_eq!(
            medium_date(NaiveDate::from_ymd_opt(2026, 8, 3).unwrap()),
            "3 août 2026"
        );
        assert_eq!(kamas(1_234_567), "1 234 567 ₭");
    }
}
