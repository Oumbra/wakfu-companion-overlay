//! **Effacement des données locales** — le droit à l'effacement (RGPD art. 17) rendu exerçable
//! depuis l'overlay, constat **C5** de [`docs/analyse-rgpd.md`](../../../docs/analyse-rgpd.md)
//! : la déconnexion n'effaçait que le jeton, et rien dans l'interface ne purgeait les
//! combats en cours, la file d'envoi, les gabarits de tour, les journaux ni la configuration.
//!
//! ## Deux portées, jamais une seule
//!
//! | Portée | Déclencheur | Ce qui part |
//! | --- | --- | --- |
//! | [`Scope::OnDisconnect`] | toute déconnexion volontaire (fenêtre Options, Carte, zone de notification) — `background::spawn_auth_thread`. Un jeton refusé (401) n'efface que le jeton | les fichiers qui portent des **tiers** ou une **capture d'écran** : `data/` (combats en cours et récap de session), `watchlist-counts.json`, `turn-templates/`, plus le contenu des journaux (`logs/*` vidés, `focus.log` supprimé) |
//! | [`Scope::Everything`] | bouton « Supprimer les données locales » (fenêtre Options › À propos › Vos données, écran de connexion) | la racine de dossiers en entier (et l'ancienne, si elle subsiste), les jetons du trousseau système (tous les déploiements, `token_store::clear_all_tokens`), l'inscription au démarrage de l'ordinateur et les clés de registre de l'overlay (Windows) — l'état d'une installation neuve |
//!
//! Les deux gestes commencent par **effacer la session côté serveur**
//! ([`revoke_server_session`]) : un jeton effacé du disque restait valide en base, donc utilisable
//! par qui en aurait pris copie (2026-09-18, route `DELETE /api/v1/auth/native/session` ajoutée
//! côté `wakfu-companion` pour ce constat). C'est la seule partie de l'effacement qui ne dépend pas
//! de cette machine, et la seule qui puisse échouer sans conséquence : hors ligne, la purge locale
//! se fait quand même et la session expire d'elle-même côté serveur.
//!
//! La portée de déconnexion ne touche ni la configuration ni les caches : se déconnecter n'est pas
//! désinstaller, et reperdre son chemin de `wakfu.log` ou son catalogue à chaque déconnexion serait
//! une punition, pas une protection. C'est le bouton qui va jusque-là, et il le dit avant.
//!
//! La file de synchro (`sync-queue.sqlite3`) n'est pas dans la liste de [`Scope::OnDisconnect`] :
//! elle est déjà vidée par le thread de synchro lui-même (`SyncCommand::Deactivate`, constat C3,
//! 2026-09-18) — la purger ici en doublon fermerait la base sous les pieds de ce thread.
//!
//! ## Une racine, plus l'ancienne tant qu'elle existe
//!
//! Depuis le 2026-09-19 tout le dépôt construit ses dossiers par `overlay_engine::app_dirs`
//! (constat C13) : une seule racine, `wakfu-companion-overlay`. L'ancienne racine de `config.toml`
//! et des gabarits (`%APPDATA%\Oumbra\wakfu-companion-overlay\` sous Windows) est migrée puis
//! supprimée au démarrage (`config::migrate_legacy_root`) ; [`Scope::Everything`] la liste quand
//! même, au cas où elle aurait survécu à une migration en échec — un effacement complet ne doit
//! rien laisser derrière lui. Sous Linux les deux se confondent : les chemins sont **dédupliqués**
//! ([`targets`]).
//!
//! ## Ce que « best-effort » veut dire ici
//!
//! Aucune suppression n'est vitale et aucune n'interrompt quoi que ce soit : un fichier absent est
//! le résultat attendu, un fichier verrouillé (antivirus, journal du jour ouvert par un autre
//! process) est journalisé dans le [`PurgeReport`] et la purge continue. Le rapport est rendu à
//! l'appelant, qui le journalise — ce que l'interface en montre, elle, est un compte de fichiers,
//! jamais une bannière d'erreur par chemin.
//!
//! ## Les journaux du jour sont VIDÉS, pas supprimés
//!
//! Une déconnexion ne ferme pas l'overlay : il revient à son écran de connexion et continue de
//! journaliser. Or `tracing_appender` tient le fichier du jour **ouvert** et ne rouvre qu'à la
//! rotation (minuit) — supprimer ce fichier laisse le programme écrire dans un inode sans nom, et
//! le journal serait muet jusqu'au lendemain, précisément quand on cherche pourquoi la
//! reconnexion échoue. [`Scope::OnDisconnect`] **tronque** donc chaque fichier de `logs/`
//! ([`truncate_logs`]) : le contenu — noms de personnages, chemin du log, auteur de message
//! (constat C6) — part pour de bon, le fichier reste et l'appender continue d'écrire dedans.
//!
//! [`Scope::Everything`] n'a pas ce souci : l'overlay se ferme juste après, le dossier part en
//! entier.
//!
//! **L'appelant doit garantir que rien ne réécrit derrière.** Les threads qui écrivent ces
//! fichiers tiennent leur contenu en mémoire : une purge seule serait défaite par la prochaine
//! sauvegarde périodique. C'est pourquoi la déconnexion fait d'abord oublier sa session au moteur
//! (`Engine::forget_session`, appelé sur `EngineCommand::Disconnect`) et au récap de l'hôte
//! (`recap_session::RecapSession::purge`), et pourquoi le bouton **ferme l'overlay** juste après.

use std::path::{Path, PathBuf};

/// Nom de projet de la racine « sans qualifieur » — celle de `logging::log_dir`,
/// `overlay_engine::fight_store`/`watchlist` et de tout `overlay_sync` (jeton, file de synchro,
/// caches, mises à jour). Les crates concernés ont chacun la même constante, avec la même variante
/// de test : un test qui purge ne doit jamais viser le vrai dossier de l'utilisateur.
#[cfg(not(test))]
const SHARED_APP_NAME: &str = "wakfu-companion-overlay";
#[cfg(test)]
const SHARED_APP_NAME: &str = "wakfu-companion-overlay-test";

/// Ce que la purge emporte — voir le tableau de la doc de module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Les fichiers à tiers et à captures, plus les journaux. Configuration et caches conservés.
    OnDisconnect,
    /// Tout : la racine de dossiers (et l'ancienne), le jeton du trousseau, l'inscription au démarrage.
    Everything,
}

impl Scope {
    /// Comment la portée se nomme au journal — l'interface, elle, a ses propres libellés.
    fn log_name(self) -> &'static str {
        match self {
            Scope::OnDisconnect => "déconnexion",
            Scope::Everything => "effacement complet",
        }
    }
}

/// Ce qu'une purge a fait — rendu à l'appelant plutôt que journalisé ici : c'est lui qui sait
/// depuis quel geste elle part, et le journal fait partie de ce qui est effacé.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PurgeReport {
    /// Chemins effectivement supprimés.
    pub removed: Vec<PathBuf>,
    /// Fichiers vidés sur place plutôt que supprimés — les journaux, voir la doc de module.
    pub truncated: Vec<PathBuf>,
    /// Chemins déjà absents — le cas le plus courant, et un succès.
    pub absent: usize,
    /// Chemins que le système a refusé de supprimer, avec la cause.
    pub failed: Vec<(PathBuf, String)>,
}

impl PurgeReport {
    /// Une ligne pour le journal : ce qui est parti, ce qui a résisté.
    pub fn summary(&self) -> String {
        let mut line = format!(
            "{} chemin(s) supprimé(s), {} journal(aux) vidé(s), {} déjà absent(s)",
            self.removed.len(),
            self.truncated.len(),
            self.absent
        );
        if !self.failed.is_empty() {
            line.push_str(&format!(", {} en échec", self.failed.len()));
        }
        line
    }
}

/// La racine de données et de configuration de l'overlay — la même que `crate::config`, avec le
/// nom de projet de CE crate (suffixe `-test` sous `cfg(test)`).
fn shared_dirs() -> Option<directories::ProjectDirs> {
    overlay_engine::app_dirs::project_dirs(SHARED_APP_NAME)
}

/// **L'overlay a-t-il déjà écrit quelque chose sur cette machine ?** — ce qui décide d'offrir ou
/// non « Supprimer les données locales » (2026-09-22, demande utilisateur : « ce lien devrait être
/// proposé uniquement s'il y a des données locales ; à la première connexion, l'utilisateur ne
/// comprendra pas qu'il s'affiche alors qu'il n'a rien, ça va peut-être l'inquiéter »).
///
/// **Ce n'est pas `targets(Everything).iter().any(exists)`**, et c'est tout le piège : le dossier
/// de configuration et le journal du jour sont créés par le lancement EN COURS, avant même que la
/// carte s'affiche. Ce test-là répondrait toujours « oui », y compris au tout premier démarrage —
/// exactement le cas qu'on veut écarter.
///
/// Sont donc sondés les seuls dépôts qu'un usage réel laisse derrière lui :
///
/// - un jeton de session dans le trousseau — il n'y en a qu'après un appairage ;
/// - un combat en cours ou un récap de session (`fight_store`) ;
/// - les compteurs de Suivi (`watchlist`) ;
/// - un gabarit de tour (`turn_watch::templates`) ;
/// - l'ancienne racine de configuration, si une migration l'a laissée ;
/// - un `config.toml` **antérieur au démarrage du processus** — le seul test qui distingue
///   « déjà utilisé » de « première ouverture », puisque le fichier existe dans les deux cas.
///
/// Sondé une fois, au premier affichage de l'écran, et retenu par l'appelant : pas d'accès disque
/// à chaque frame.
pub fn has_user_data(process_start: std::time::SystemTime) -> bool {
    if overlay_sync::token_store::load_token().is_some() {
        return true;
    }
    let mut paths = vec![
        overlay_engine::fight_store::default_store_dir(),
        overlay_engine::watchlist::default_store_path(),
    ];
    paths.extend(crate::turn_watch::templates::dir());
    paths.extend(crate::config::legacy_root());
    if paths.iter().any(|path| not_empty(path)) {
        return true;
    }
    crate::config::config_path().is_some_and(|path| older_than(&path, process_start))
}

/// Le chemin existe **et porte quelque chose** : un dossier créé mais vide — `data/` posé au
/// démarrage sans qu'aucun combat n'ait eu lieu — ne compte pas comme une donnée de l'utilisateur.
fn not_empty(path: &Path) -> bool {
    match std::fs::metadata(path) {
        Ok(meta) if meta.is_dir() => std::fs::read_dir(path)
            .map(|mut entries| entries.next().is_some())
            .unwrap_or(false),
        Ok(meta) => meta.len() > 0,
        Err(_) => false,
    }
}

/// Le fichier existe et sa dernière écriture précède `instant` — voir [`has_user_data`]. Une
/// horloge illisible (système de fichiers exotique, date dans le futur après un changement d'heure)
/// rend `false` : dans le doute, la Carte n'annonce pas des données qu'elle n'a pas vues.
fn older_than(path: &Path, instant: std::time::SystemTime) -> bool {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .is_ok_and(|modified| modified < instant)
}

/// **Les chemins que `scope` emporte**, dans l'ordre de suppression et sans doublon.
///
/// Les journaux viennent **en dernier** : le rapport de purge y est écrit par l'appelant, autant
/// qu'il ait une chance d'y rester jusqu'au bout.
pub fn targets(scope: Scope) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    match scope {
        Scope::OnDisconnect => {
            // `data/` : un fichier par combat en cours (`fight-*.json`, ses combattants nommés) et
            // le récap de session (`recap-session.json`), qui vit dans le même dossier.
            paths.push(overlay_engine::fight_store::default_store_dir());
            // Compteurs de Suivi : répliqués au compte (`client::patch_watchlist_counts`), donc
            // retrouvés à la reconnexion — le fichier local n'est qu'un cache de secours.
            paths.push(overlay_engine::watchlist::default_store_path());
            // Gabarits de tour : un PNG du nom du personnage RENDU À L'ÉCRAN et le nom en clair
            // à côté (constat C8).
            paths.extend(crate::turn_watch::templates::dir());
            // `focus.log`, lui, est écrit par un process éphémère qui n'existe plus quand on
            // arrive ici (`turn_watch::notify::focus_window`) : rien ne le tient ouvert, il part
            // franchement. Le dossier `logs/` est traité à part, voir [`truncate_logs`].
            paths.extend(
                crate::turn_watch::templates::data_dir()
                    .map(|dir| dir.join(crate::turn_watch::FOCUS_LOG)),
            );
        }
        Scope::Everything => {
            // La racine en entier — voir « Une racine, plus l'ancienne » dans la doc de module.
            // Rien n'est énuméré fichier par fichier ici : un cache ajouté demain dans cette
            // racine doit partir avec le reste sans que ce module l'apprenne.
            if let Some(dirs) = shared_dirs() {
                paths.push(dirs.data_dir().to_path_buf());
                paths.push(dirs.config_dir().to_path_buf());
            }
            // Sous Windows, leur parent commun (`%APPDATA%\wakfu-companion-overlay`) : il restait
            // vide derrière l'effacement (2026-09-25). `dedup` retire alors `data` et `config`,
            // contenus dedans.
            paths.extend(crate::config::own_root());
            // L'ancienne racine de `config.toml` et des gabarits, si la migration du démarrage
            // l'a laissée (échec, ou fichier apparu depuis) : son dossier propre, jamais un
            // dossier partagé avec autre chose. En chemin absolu — voir
            // `overlay_engine::app_dirs::own_root`, qui explique pourquoi pas `project_path()`.
            paths.extend(crate::config::legacy_root());
        }
    }
    dedup(paths)
}

/// Retire les doublons **et les chemins contenus dans un autre** de la liste : sous Linux les deux
/// racines se confondent, et `logs/` est sous la racine que [`Scope::Everything`] efface déjà.
/// Supprimer un dossier deux fois n'est pas faux (le second passage compte un absent), mais le
/// rapport serait trompeur.
fn dedup(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut kept: Vec<PathBuf> = Vec::with_capacity(paths.len());
    for path in paths {
        if kept.iter().any(|k| path == *k || path.starts_with(k)) {
            continue;
        }
        kept.retain(|k| !k.starts_with(&path));
        kept.push(path);
    }
    kept
}

/// **Efface ce que `scope` emporte** et rend le rapport. Pour [`Scope::Everything`], efface aussi
/// ce qui ne vit pas dans ces dossiers : les jetons du trousseau système, un par déploiement
/// (`overlay_sync::token_store::clear_all_tokens`), l'inscription au démarrage de l'ordinateur (`crate::autostart` —
/// `HKCU\…\Run` ou un `.desktop`) et, sous Windows, les deux clés de registre de l'identité de
/// notification et du protocole d'activation (`turn_watch::notify::unregister_identity`).
///
/// **Ne ferme pas l'overlay et ne déconnecte pas** : c'est à l'appelant, et c'est indispensable
/// pour [`Scope::Everything`] (voir la doc de module).
pub fn purge(scope: Scope) -> PurgeReport {
    if scope == Scope::Everything {
        // Le jeton d'abord : il est le seul élément effacé dont la perte est irréversible côté
        // utilisateur (il faudra réappairer), et le seul qui ne soit pas un fichier à nous. TOUS
        // les déploiements (2026-09-21) : le trousseau garde un jeton par déploiement depuis
        // `token_store::slot`, et « installation neuve » veut dire aucun d'eux.
        overlay_sync::token_store::clear_all_tokens();
        crate::autostart::apply(false);
        #[cfg(windows)]
        crate::turn_watch::notify::unregister_identity();
    }
    let mut report = purge_paths(&targets(scope));
    if scope == Scope::OnDisconnect {
        truncate_logs(&mut report);
    }
    tracing::info!(
        "[données locales] {} : {}.",
        scope.log_name(),
        report.summary()
    );
    for (path, err) in &report.failed {
        tracing::warn!("[données locales] {} non supprimé : {err}", path.display());
    }
    report
}

/// **Le geste complet du bouton « Supprimer les données locales »**, pour les deux hôtes — qui
/// enchaînent ensuite `logging::log_session_end` et la sortie de leur boucle d'événements.
///
/// Le récap de session part **avant** le reste, et pas seulement parce qu'il est dans les dossiers
/// effacés : son `Drop` réécrit `recap-session.json` quand il a des compteurs non pliés
/// (`RecapSession::save`, qui recrée son dossier parent au passage). Purgé sans ce préalable, le
/// fichier reviendrait quelques millisecondes après la purge, à la chute de l'hôte.
///
/// Ce qu'un thread de fond pourrait encore recréer entre cet appel et la fin du process (une icône
/// mise en cache, un événement remis en file) est le prix de ne pas arrêter chaque thread d'abord :
/// des dossiers presque vides, sans rien qui nomme personne. La fermeture immédiate est ce qui
/// borne cette fenêtre — voir la doc de module.
pub fn purge_everything_before_shutdown(
    recap: &mut crate::recap_session::RecapSession,
    engine_totals: &overlay_engine::SessionTotals,
) -> PurgeReport {
    revoke_server_session_within(SHUTDOWN_REVOKE_BUDGET);
    recap.purge(engine_totals, std::time::SystemTime::now());
    purge(Scope::Everything)
}

/// Ce qu'on accorde à la révocation serveur quand l'overlay est en train de se fermer — voir
/// [`revoke_server_session_within`]. Le client HTTP, lui, tolère dix secondes
/// (`overlay_sync::client`) : c'est bon pour un thread de fond, pas pour une fenêtre que
/// l'utilisateur vient de condamner et qui resterait figée d'autant.
const SHUTDOWN_REVOKE_BUDGET: std::time::Duration = std::time::Duration::from_secs(3);

/// **Efface la session côté serveur** (`client::delete_native_session`) — le jeton cesse d'être
/// utilisable pour qui en aurait pris copie, ce que l'effacement local ne pouvait pas obtenir
/// (constat C5 de `docs/analyse-rgpd.md` §3.5, route ajoutée côté `wakfu-companion` le
/// 2026-09-18).
///
/// **Bloquante, et c'est voulu** : appelée par le thread d'authentification à la déconnexion
/// (`background::spawn_auth_thread`), juste avant `token_store::clear_token` — sans le jeton, plus
/// rien ne désigne la session à effacer. Aucune fenêtre n'attend ce thread.
///
/// Best-effort de bout en bout : sans jeton (déjà effacé) il n'y a rien à révoquer,
/// et un échec réseau est journalisé sans rien interrompre. Un effacement local doit aboutir hors
/// ligne ; la session, elle, finira par expirer côté serveur.
pub fn revoke_server_session() {
    let Some(token) = overlay_sync::token_store::load_token() else {
        return;
    };
    match overlay_sync::client::delete_native_session(&token) {
        Ok(true) => tracing::info!("[compte] session effacée côté serveur."),
        Ok(false) => {
            tracing::info!("[compte] aucune session à effacer côté serveur (déjà faite).")
        }
        Err(err) => tracing::warn!(
            "[compte] session non effacée côté serveur ({err}) — le jeton local part quand même."
        ),
    }
}

/// [`revoke_server_session`] avec un budget de temps — pour le chemin où l'overlay **se ferme**
/// derrière (le bouton « Supprimer les données locales »).
///
/// Un thread porte l'appel, et l'attente est bornée : passé le budget, on cesse d'attendre et
/// l'effacement local continue. La requête, elle, n'est pas annulée — elle part et vit sa vie le
/// peu de temps qu'il reste au processus, ce qui est mieux que rien et ne coûte rien à personne.
/// Le thread n'est pas joint (il détient son propre agent HTTP et ne touche à aucun état partagé).
fn revoke_server_session_within(budget: std::time::Duration) {
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    if std::thread::Builder::new()
        .name("overlay-revoke".into())
        .spawn(move || {
            revoke_server_session();
            let _ = done_tx.send(());
        })
        .is_err()
    {
        // Pas de thread disponible : tenter l'appel ici plutôt que de renoncer. L'interface est
        // déjà condamnée, elle peut bien attendre le temps du client HTTP.
        revoke_server_session();
        return;
    }
    if done_rx.recv_timeout(budget).is_err() {
        tracing::warn!(
            "[compte] effacement de la session côté serveur toujours en cours après {} s — \
             l'effacement local continue sans l'attendre.",
            budget.as_secs()
        );
    }
}

/// Supprime chaque chemin de `paths` — fichier ou dossier entier —, sans jamais s'arrêter au
/// premier échec. Séparé de [`purge`] pour être testable sur un dossier temporaire : appelé avec
/// [`targets`], il viserait les données réelles de la machine.
fn purge_paths(paths: &[PathBuf]) -> PurgeReport {
    let mut report = PurgeReport::default();
    for path in paths {
        match remove(path) {
            Ok(true) => report.removed.push(path.clone()),
            Ok(false) => report.absent += 1,
            Err(err) => report.failed.push((path.clone(), err.to_string())),
        }
    }
    report
}

/// **Vide chaque fichier de `logs/` sur place** — voir « Les journaux du jour sont VIDÉS » dans la
/// doc de module. Un dossier absent (aucune écriture encore, ou `ProjectDirs` muet) n'est pas une
/// erreur : il n'y a alors rien à vider.
fn truncate_logs(report: &mut PurgeReport) {
    let Some(dir) = crate::logging::log_dir() else {
        return;
    };
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            report.absent += 1;
            return;
        }
        Err(err) => {
            report.failed.push((dir, err.to_string()));
            return;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        match std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&path)
        {
            Ok(_) => report.truncated.push(path),
            Err(err) => report.failed.push((path, err.to_string())),
        }
    }
}

/// `Ok(true)` supprimé, `Ok(false)` déjà absent, `Err` refusé par le système.
fn remove(path: &Path) -> std::io::Result<bool> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(err),
    };
    if metadata.is_dir() {
        std::fs::remove_dir_all(path)?;
    } else {
        std::fs::remove_file(path)?;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir(nom: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("overlay-local-data-{nom}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn purge_efface_fichiers_et_dossiers_et_compte_les_absents() {
        let racine = tmp_dir("purge");
        let fichier = racine.join("fight-1.json");
        std::fs::write(&fichier, "{}").unwrap();
        let dossier = racine.join("turn-templates");
        std::fs::create_dir_all(dossier.join("sous-dossier")).unwrap();
        std::fs::write(dossier.join("Heros.png"), "png").unwrap();
        let absent = racine.join("jamais-ecrit.json");

        let rapport = purge_paths(&[fichier.clone(), dossier.clone(), absent]);

        assert!(!fichier.exists(), "le fichier devait partir");
        assert!(!dossier.exists(), "le dossier devait partir en entier");
        assert_eq!(rapport.removed.len(), 2);
        assert_eq!(rapport.absent, 1);
        assert!(rapport.failed.is_empty(), "{:?}", rapport.failed);
        assert!(rapport.summary().contains("2 chemin(s) supprimé(s)"));

        let _ = std::fs::remove_dir_all(racine);
    }

    /// Le cas Linux de la doc de module : les deux racines `ProjectDirs` se confondent, et `logs/`
    /// est sous la racine que l'effacement complet emporte déjà. Un doublon dans la liste ne
    /// casserait rien, mais le rapport compterait des absents inventés.
    #[test]
    fn les_chemins_contenus_dans_un_autre_sont_retires() {
        let racine = PathBuf::from("/tmp/overlay-dedup");
        let liste = dedup(vec![
            racine.join("logs"),
            racine.clone(),
            racine.clone(),
            racine.join("data/fight-1.json"),
            PathBuf::from("/tmp/overlay-dedup-bis"),
        ]);
        assert_eq!(
            liste,
            vec![racine, PathBuf::from("/tmp/overlay-dedup-bis")],
            "la racine doit absorber ses descendants, et un voisin de même préfixe rester"
        );
    }

    /// Les deux portées ne visent pas les mêmes chemins, et aucune ne doit rendre une liste vide
    /// sur une machine où `ProjectDirs` répond (sinon le bouton ne ferait rien du tout).
    ///
    /// **Ce test LIT des chemins, il n'efface rien** — et aucun test de ce module n'appelle
    /// [`purge`] ni [`truncate_logs`] : ils viseraient les données réelles de la machine qui lance
    /// la suite. Les variantes `#[cfg(test)]` des noms de projet ne protègent que le crate en
    /// cours de test (`crate::config` ici), pas celles d'`overlay-engine` ni d'`overlay-sync`, qui
    /// sont alors compilés en mode normal. Seul [`purge_paths`], qui prend ses chemins en
    /// argument, se teste sur un dossier temporaire.
    #[test]
    fn les_deux_portees_visent_ce_qu_elles_doivent() {
        let deconnexion = targets(Scope::OnDisconnect);
        let tout = targets(Scope::Everything);
        assert!(!deconnexion.is_empty(), "portée de déconnexion vide");
        assert!(!tout.is_empty(), "portée d'effacement complet vide");

        let config = crate::config::project_dirs()
            .map(|dirs| dirs.config_dir().join("config.toml"))
            .expect("racine de configuration");
        assert!(
            !deconnexion.iter().any(|p| config.starts_with(p)),
            "la déconnexion ne doit pas emporter config.toml : se déconnecter n'est pas désinstaller"
        );
        assert!(
            tout.iter().any(|p| config.starts_with(p)),
            "l'effacement complet doit emporter config.toml"
        );

        // Les combats en cours et les gabarits partent à la déconnexion (tiers, captures)...
        for attendu in [
            overlay_engine::fight_store::default_store_dir(),
            crate::turn_watch::templates::dir().expect("dossier des gabarits"),
        ] {
            assert!(
                deconnexion.iter().any(|p| attendu.starts_with(p)),
                "{} devait être visé par la déconnexion",
                attendu.display()
            );
        }
        // ...mais le dossier des journaux n'est PAS dans la liste : il est vidé sur place, jamais
        // supprimé, tant que l'overlay tourne (voir la doc de module).
        let logs = crate::logging::log_dir().expect("dossier des journaux");
        assert!(
            !deconnexion.iter().any(|p| logs.starts_with(p)),
            "le dossier des journaux ne doit pas être supprimé à la déconnexion"
        );
    }
}
