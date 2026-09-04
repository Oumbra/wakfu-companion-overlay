//! Reconstruction d'un horodatage complet (ms, époque Unix) depuis l'heure brute d'une ligne de
//! log (`HH:MM:SS,mmm`, seule information que Wakfu écrit) + un ancrage de date
//! (`LogEntry::LogDateAnchor`, émis une fois en tête de fichier) — miroir réduit de
//! `buildFullTimestampMs`/`primeLogDateAnchorFromBatch` (`stats-store.service.ts`), nécessaire à
//! L5 (§7) : `FightPayload::started_at`/`PurchasePayload::occurred_at`/`TradePayload::occurred_at`
//! doivent être des instants réels, jamais l'heure brute seule (qui redeviendrait un doublon à
//! chaque relecture du même fichier un autre jour — voir la doc de `history-event.model.ts`
//! côté web, qui pose exactement cette exigence pour la SIGNATURE ; ici c'est le PAYLOAD envoyé,
//! pas la signature, qui a besoin d'une date réelle : les fonctions de signature de `history.rs`
//! continuent, elles, à n'utiliser QUE l'heure brute, comme le web).
//!
//! **Simplification assumée par rapport au web, à lever si elle devient gênante en usage réel :**
//! pas de `primeLogDateAnchorFromBatch` (balayage en avant du lot pour dater correctement les
//! toute premières lignes d'un fichier qui PRÉCÈDENT son propre ancrage) : tant qu'aucun ancrage
//! n'a encore été vu, repli sur la date système de la machine — exactement le même repli que le
//! web utilise déjà pour le cas "aucun ancrage nulle part dans le fichier", qui ne peut affecter
//! qu'une poignée de lignes tout au début de l'ingestion.
//!
//! **Fuseau horaire (correctif du 2026-09-03, retour utilisateur en conditions réelles)** : le log
//! Wakfu n'écrit que l'heure LOCALE de la machine qui fait tourner le client de jeu (comme le
//! `new Date(year, month, day, ...)` du web, implicitement local au fuseau du NAVIGATEUR — même
//! principe, ici la machine qui héberge l'overlay). `full_timestamp_ms` traitait jusqu'ici cette
//! date civile comme un instant **UTC** au lieu de la convertir depuis le fuseau LOCAL — décalage
//! constant égal au fuseau de la machine sur `startedAt`/`occurredAt` (jamais sur la SIGNATURE, qui
//! n'utilise que l'heure brute). Repéré par l'utilisateur (23:54 heure locale CEST affiché comme le
//! lendemain sur le site : l'instant UTC réel, ~1h plus tôt en horloge murale, tombait après minuit
//! une fois réinterprété comme "23:54 UTC"). Corrigé via `Local` (feature `chrono` "clock") : PAS
//! `chrono-tz`, aucune base de fuseaux IANA embarquée — on délègue entièrement à l'OS (Windows API
//! côté Windows, `/etc/localtime` côté Linux), donc pas la simplification écartée ci-dessus par le
//! commentaire historique de ce module (qui visait la résolution d'un fuseau NOMMÉ arbitraire,
//! jamais nécessaire ici : seul le fuseau COURANT de la machine locale compte).

/// Seuil au-delà duquel un bond en arrière de l'heure du jour signale un passage de minuit plutôt
/// qu'un simple réordonnancement — miroir exact de `DAY_ROLLOVER_THRESHOLD_MS` côté web.
const DAY_ROLLOVER_THRESHOLD_MS: i64 = 12 * 60 * 60 * 1000;

/// Miroir de `timeToMs` — heure du jour en millisecondes, `None` si la ligne ne suit pas le format
/// `HH:MM:SS,mmm` (ne devrait jamais arriver, `LogEntry::time()` vient déjà du parser).
pub fn time_of_day_ms(time: &str) -> Option<i64> {
    let bytes = time.as_bytes();
    if bytes.len() != 12
        || bytes[2] != b':'
        || bytes[5] != b':'
        || bytes[8] != b','
        || !bytes
            .iter()
            .enumerate()
            .all(|(i, b)| matches!(i, 2 | 5 | 8) || b.is_ascii_digit())
    {
        return None;
    }
    let h: i64 = time[0..2].parse().ok()?;
    let m: i64 = time[3..5].parse().ok()?;
    let s: i64 = time[6..8].parse().ok()?;
    let ms: i64 = time[9..12].parse().ok()?;
    Some(((h * 60 + m) * 60 + s) * 1000 + ms)
}

/// Nombre de jours écoulés depuis l'époque Unix pour une date civile `(year, month, day)` —
/// algorithme constant connu (Howard Hinnant, `days_from_civil`), correct pour tout calendrier
/// grégorien y compris `month`/`day` débordants (un `day` de 32 août est normalisé en 1er
/// septembre) — miroir de la normalisation native qu'offre `new Date(y, m, d)` côté web.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    // Ramène (month, day) dans [1,12]/civil normal en reportant l'excès sur l'année d'abord —
    // months peut déborder largement (day_offset ajouté au jour peut valoir plusieurs dizaines).
    let (y, m) = {
        let m0 = month - 1; // 0..11 normalement, peut déborder
        let y = year + m0.div_euclid(12);
        let m = m0.rem_euclid(12) + 1;
        (y, m)
    };
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = if y2 >= 0 { y2 } else { y2 - 399 } / 400;
    let yoe = y2 - era * 400; // [0, 399]
    let mp = (m + 9) % 12; // [0,11], mars=0 ... février=11
    let doy = (153 * mp + 2) / 5 + day - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146097 + doe - 719468
}

/// Suit l'ancrage de date courant et le décompte de passages de minuit — un par fichier ingéré
/// (voir `Engine`/`SessionState`, un seul tracker partagé par tous les combats/achats/échanges
/// puisque `wakfu.log` est un flux chronologique unique, cf. doc de module de `session.rs`).
#[derive(Debug, Default)]
pub struct LogDateTracker {
    anchor: Option<(i64, i64, i64)>,
    day_offset: i64,
    last_time_of_day_ms: Option<i64>,
}

impl LogDateTracker {
    /// `LogEntry::LogDateAnchor` — reset le décompte de passages de minuit, comme côté web.
    pub fn set_anchor(&mut self, year: i64, month: i64, day: i64) {
        self.anchor = Some((year, month, day));
        self.day_offset = 0;
        self.last_time_of_day_ms = None;
    }

    /// Miroir de `buildFullTimestampMs` — voir la doc de module pour la conversion de fuseau.
    pub fn full_timestamp_ms(&mut self, time: &str) -> i64 {
        let Some(time_of_day_ms) = time_of_day_ms(time) else {
            return system_now_ms();
        };
        let Some((year, month, day)) = self.anchor else {
            return system_now_ms();
        };

        if let Some(last) = self.last_time_of_day_ms {
            if time_of_day_ms < last - DAY_ROLLOVER_THRESHOLD_MS {
                self.day_offset += 1;
            }
        }
        self.last_time_of_day_ms = Some(time_of_day_ms);

        // `days_from_civil` normalise déjà un `day` débordant (`day + day_offset` peut dépasser le
        // nombre de jours du mois) — on repasse par `civil_from_days` pour récupérer une date civile
        // normalisée, seule forme que `chrono::NaiveDate` accepte (voir `local_wall_clock_to_utc_ms`).
        let days = days_from_civil(year, month, day + self.day_offset);
        let (norm_year, norm_month, norm_day) = civil_from_days(days);
        local_wall_clock_to_utc_ms(norm_year, norm_month, norm_day, time_of_day_ms)
    }
}

fn system_now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Convertit une date civile + heure du jour, **interprétées dans le fuseau LOCAL de la machine**,
/// en un instant UTC réel (ms, époque Unix) — voir la doc de module pour le bug que ça corrige.
/// Repli sur `system_now_ms()` si la date/heure elle-même est invalide (ne devrait jamais arriver,
/// `days_from_civil`/`time_of_day_ms` produisent déjà des valeurs normalisées) ou si `Local` ne
/// parvient à résoudre AUCUN instant (cas `LocalResult::None`, un bond DANS un saut horaire de
/// bascule heure d'hiver/été : décalé d'une heure et retenté avant d'abandonner).
fn local_wall_clock_to_utc_ms(year: i64, month: i64, day: i64, ms_of_day: i64) -> i64 {
    use chrono::{Duration, LocalResult, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};

    let Ok(year) = i32::try_from(year) else {
        return system_now_ms();
    };
    let (Ok(month), Ok(day)) = (u32::try_from(month), u32::try_from(day)) else {
        return system_now_ms();
    };
    let Some(date) = NaiveDate::from_ymd_opt(year, month, day) else {
        return system_now_ms();
    };
    let secs_of_day = (ms_of_day / 1000) as u32;
    let nanos = ((ms_of_day % 1000) * 1_000_000) as u32;
    let Some(time) = NaiveTime::from_num_seconds_from_midnight_opt(secs_of_day, nanos) else {
        return system_now_ms();
    };
    let naive = NaiveDateTime::new(date, time);

    let resolve = |naive: NaiveDateTime| match chrono::Local.from_local_datetime(&naive) {
        // Heure ambiguë (bascule heure d'été → hiver, l'heure locale existe deux fois) : la
        // première occurrence (avant le recul de l'horloge) est la meilleure approximation
        // possible sans information supplémentaire — écart maximal d'une heure, jamais pire.
        LocalResult::Single(dt) | LocalResult::Ambiguous(dt, _) => Some(dt),
        LocalResult::None => None,
    };
    let resolved = resolve(naive)
        // Heure inexistante (bascule heure d'hiver → été, ex. 02:30 qui n'existe jamais un jour de
        // printemps) : décale d'une heure, exactement le saut qu'une telle bascule introduit.
        .or_else(|| resolve(naive + Duration::hours(1)));
    match resolved {
        Some(dt) => dt.with_timezone(&Utc).timestamp_millis(),
        None => system_now_ms(),
    }
}

/// Formate un instant (ms époque Unix) en ISO-8601 UTC (`AAAA-MM-JJTHH:MM:SS.mmmZ`) — format
/// accepté par `parseDate` côté serveur (`new Date(raw)`, `server/history/parse.ts`). Pas de
/// dépendance à `chrono` pour un simple formatage : conversion inverse de `days_from_civil`
/// (algorithme `civil_from_days`, même famille).
pub fn format_iso_utc(epoch_ms: i64) -> String {
    let days = epoch_ms.div_euclid(86_400_000);
    let ms_of_day = epoch_ms.rem_euclid(86_400_000);
    let (year, month, day) = civil_from_days(days);
    let h = ms_of_day / 3_600_000;
    let m = (ms_of_day / 60_000) % 60;
    let s = (ms_of_day / 1000) % 60;
    let ms = ms_of_day % 1000;
    format!("{year:04}-{month:02}-{day:02}T{h:02}:{m:02}:{s:02}.{ms:03}Z")
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let day = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let month = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let year = if month <= 2 { y + 1 } else { y };
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_time_of_day() {
        assert_eq!(time_of_day_ms("00:00:00,000"), Some(0));
        assert_eq!(time_of_day_ms("14:13:32,174"), Some(51_212_174));
        assert_eq!(time_of_day_ms("garbage"), None);
    }

    #[test]
    fn roundtrips_civil_date() {
        // 2026-09-02 vérifié contre un calendrier grégorien réel.
        let days = days_from_civil(2026, 9, 2);
        assert_eq!(civil_from_days(days), (2026, 9, 2));
        // Débordement de jour normalisé (32 août => 1er septembre), comme `new Date(y,m,32)`.
        assert_eq!(civil_from_days(days_from_civil(2026, 8, 32)), (2026, 9, 1));
    }

    #[test]
    fn full_timestamp_uses_anchor_and_detects_midnight_rollover() {
        let mut tracker = LogDateTracker::default();
        tracker.set_anchor(2026, 9, 1);
        let first = tracker.full_timestamp_ms("23:59:00,000");
        // Repasse à une heure du jour bien plus basse : passage de minuit détecté (>12h de recul).
        let second = tracker.full_timestamp_ms("00:05:00,000");
        assert!(second > first);

        // Attendu : 2026-09-02T00:05:00 LOCAL converti en UTC — comparé à une conversion
        // indépendante via `chrono`, pour que ce test reste correct quel que soit le fuseau de la
        // machine qui l'exécute (CI comprise), plutôt que de figer un décalage particulier (voir le
        // bug réel que ce correctif règle, doc de module).
        use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
        let naive = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2026, 9, 2).unwrap(),
            NaiveTime::from_hms_opt(0, 5, 0).unwrap(),
        );
        let expected = Local
            .from_local_datetime(&naive)
            .single()
            .expect("2026-09-02T00:05:00 local ne tombe pas dans un saut horaire")
            .with_timezone(&Utc);
        assert_eq!(second, expected.timestamp_millis());
    }

    /// Régression réelle (retour utilisateur, 2026-09-03) : l'heure du log est LOCALE, jamais UTC —
    /// `local_wall_clock_to_utc_ms` doit produire le même instant qu'une conversion `chrono`
    /// indépendante, pas la date civile traitée telle quelle comme un instant UTC (l'ancien
    /// comportement, qui décalait `startedAt`/`occurredAt` de tout le fuseau de la machine — un
    /// combat joué à 23h54 heure locale d'été (CEST, UTC+2) apparaissait daté du LENDEMAIN une fois
    /// réinterprété comme 23h54 UTC).
    #[test]
    fn local_wall_clock_to_utc_ms_matches_chrono_local_conversion() {
        use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
        let naive = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2026, 3, 15).unwrap(),
            NaiveTime::from_hms_milli_opt(14, 30, 0, 0).unwrap(),
        );
        let expected = Local
            .from_local_datetime(&naive)
            .single()
            .expect("15 mars 14:30 ne tombe pas dans un saut horaire")
            .with_timezone(&Utc)
            .timestamp_millis();
        assert_eq!(
            local_wall_clock_to_utc_ms(2026, 3, 15, 14 * 3_600_000 + 30 * 60_000),
            expected
        );
    }

    #[test]
    fn no_anchor_falls_back_to_system_time() {
        let mut tracker = LogDateTracker::default();
        let before = system_now_ms();
        let ts = tracker.full_timestamp_ms("10:00:00,000");
        assert!(ts >= before);
    }
}
