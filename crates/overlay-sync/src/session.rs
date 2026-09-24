//! **Jeton de session courant, à l'échelle du processus** (2026-09-20) — ce que les routes
//! « référentiel » de l'API exigent désormais de l'overlay.
//!
//! Côté `wakfu-companion`, `catalog/*`, `items/{id}`, `monsters/{id}`, `monster-loot`,
//! `monster-families`, `dungeons` et le relais d'icônes `icons/*` ne répondent plus qu'à deux
//! appelants (`functions/api/_caller.ts`, `docs/analyse-cgu.md` recommandation 4 — la licence de
//! données WAKFU n'autorise ni sous-licence ni cession, une API ouverte en était une de fait) : le
//! site, reconnu par `Sec-Fetch-Site: same-origin`, et l'overlay, reconnu par sa session
//! (`Authorization: Bearer <jeton>`, le même que pour l'historique). Tout le reste reçoit 403.
//!
//! Jusqu'ici ces routes étaient publiques et `client::fetch_catalog_*`, `fetch_dungeons`,
//! `fetch_item_detail`, `fetch_bytes` partaient sans en-tête — depuis n'importe quel thread, sans
//! connaître le jeton, que seul le thread Auth manipule (`background::attempt_connect`). Plutôt
//! que de faire circuler le jeton dans chaque signature et chaque fermeture (icônes du panneau
//! Suivi, détail de recette, thread Catalogue…), le thread Auth le **publie** ici dès qu'une
//! session est validée ([`publish`]) et le **retire** à la déconnexion ou quand le serveur le
//! refuse ([`clear`]) ; les fonctions de `client` le lisent ([`current`]) et posent l'en-tête.
//!
//! **Attente.** Les threads Catalogue et Donjons démarrent en même temps que le thread Auth : ils
//! publient leur cache disque tout de suite, puis attendent que le compte soit **résolu**
//! ([`wait_resolved`]) — jeton validé, ou aucun jeton (pas de connexion possible sans « Se
//! connecter », plus de mode invité depuis le 2026-09-14) — avant de tenter le réseau. Une
//! connexion qui arrive plus tard (premier lancement, « Se connecter » dans la fenêtre de
//! connexion, reconnexion après « Déconnecter ») est signalée par une nouvelle **génération**
//! ([`wait_token_after`]) : c'est ce qui permet à ces threads de rafraîchir le référentiel sans
//! relancer l'overlay.
//!
//! Le jeton n'est jamais tracé (§10 du plan) : ce module ne l'écrit dans aucun journal.
use std::sync::{Condvar, Mutex, OnceLock};

#[derive(Default)]
struct State {
    token: Option<String>,
    /// `true` dès que le thread Auth a rendu son premier verdict (connecté ou non) — voir
    /// [`wait_resolved`].
    resolved: bool,
    /// Incrémentée à chaque [`publish`]/[`clear`] : un observateur qui a vu la génération `n`
    /// sait qu'il y a du nouveau dès que `generation != n`.
    generation: u64,
}

fn cell() -> &'static (Mutex<State>, Condvar) {
    static CELL: OnceLock<(Mutex<State>, Condvar)> = OnceLock::new();
    CELL.get_or_init(|| (Mutex::new(State::default()), Condvar::new()))
}

/// Une session vient d'être validée : ce jeton sert désormais à toutes les routes authentifiées.
pub fn publish(token: &str) {
    let (state, ready) = cell();
    let mut guard = state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    guard.token = Some(token.to_string());
    guard.resolved = true;
    guard.generation += 1;
    ready.notify_all();
}

/// Plus de session (déconnexion, jeton refusé, ou aucun jeton stocké au démarrage) : les routes
/// authentifiées répondront [`crate::SyncError::NoSession`] jusqu'au prochain [`publish`].
pub fn clear() {
    let (state, ready) = cell();
    let mut guard = state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    guard.token = None;
    guard.resolved = true;
    guard.generation += 1;
    ready.notify_all();
}

/// Le jeton courant, s'il y en a un — ce que `client` pose en `Authorization: Bearer`.
pub fn current() -> Option<String> {
    let (state, _) = cell();
    let guard = state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    guard.token.clone()
}

/// Bloque jusqu'au premier verdict du thread Auth, puis rend le jeton (session validée) ou `None`
/// (aucune session — l'appelant garde son cache/repli) avec la génération observée, à repasser à
/// [`wait_token_after`] pour attendre une connexion ultérieure.
pub fn wait_resolved() -> (Option<String>, u64) {
    let (state, ready) = cell();
    let mut guard = state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    while !guard.resolved {
        guard = ready
            .wait(guard)
            .unwrap_or_else(|poisoned| poisoned.into_inner());
    }
    (guard.token.clone(), guard.generation)
}

/// Bloque jusqu'à ce qu'une session soit publiée **après** la génération `seen`, et la rend avec
/// sa génération. Un [`clear`] intermédiaire ne réveille pas l'appelant pour rien : seule une
/// session utilisable le fait.
pub fn wait_token_after(seen: u64) -> (String, u64) {
    let (state, ready) = cell();
    let mut guard = state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    loop {
        if guard.generation != seen {
            if let Some(token) = guard.token.clone() {
                return (token, guard.generation);
            }
        }
        guard = ready
            .wait(guard)
            .unwrap_or_else(|poisoned| poisoned.into_inner());
    }
}

#[cfg(test)]
mod tests {
    // Un seul état global pour tout le processus de test : ces tests le partagent, donc ils
    // s'exécutent un par un (`SERIAL`) et chacun repart d'un état qu'il pose lui-même.
    use super::*;
    use std::thread;
    use std::time::Duration;

    static SERIAL: Mutex<()> = Mutex::new(());

    #[test]
    fn publie_puis_retire_le_jeton_courant() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        publish("jeton-a");
        assert_eq!(current().as_deref(), Some("jeton-a"));
        clear();
        assert_eq!(current(), None);
    }

    #[test]
    fn wait_resolved_rend_la_main_des_le_premier_verdict() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        clear();
        let (token, generation) = wait_resolved();
        assert!(token.is_none());
        assert!(generation > 0);
    }

    #[test]
    fn wait_token_after_ne_se_reveille_que_sur_une_nouvelle_session() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        clear();
        let (_, seen) = wait_resolved();
        let waiter = thread::spawn(move || wait_token_after(seen));
        thread::sleep(Duration::from_millis(50));
        assert!(
            !waiter.is_finished(),
            "un clear ne doit pas réveiller l'attente"
        );
        clear();
        thread::sleep(Duration::from_millis(50));
        assert!(!waiter.is_finished(), "un second clear non plus");
        publish("jeton-b");
        let (token, generation) = waiter.join().expect("thread d'attente");
        assert_eq!(token, "jeton-b");
        assert!(generation > seen);
    }
}
