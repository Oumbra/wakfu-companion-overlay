//! Raccourcis **multicompte** : taper une commande de chat du jeu (`/i "Nom"`, `/fol "Nom"`) dans
//! la fenêtre Wakfu qui a le focus, en visant automatiquement le personnage de **l'AUTRE** fenêtre.
//!
//! Demande utilisateur (2026-09-13) : « en multicompte, on souhaite généralement suivre son second
//! compte [...] j'aimerais rendre facile ces interactions [...] utiliser le nom du personnage de la
//! fenêtre qui n'est PAS la fenêtre en focus [...] `/i "<nom>"` pour inviter, `/fol "<nom>"` pour
//! suivre, raccourcis F1 et F2 ». Le nom du personnage ne vient d'aucune saisie ni d'aucun réglage :
//! il est déjà là, dans le titre de la fenêtre de jeu (`"<Nom> - WAKFU"`, voir `game_window`), que
//! l'overlay scrute déjà en continu pour s'ancrer.
//!
//! ## Qui reçoit la frappe
//!
//! La fenêtre de jeu **au premier plan**, celle sur laquelle l'utilisateur joue — la frappe
//! synthétique suit le focus clavier, l'overlay ne l'active ni ne la déplace jamais (ses propres
//! fenêtres portent `WS_EX_NOACTIVATE` et ne prennent pas le focus, voir `main.rs`). Le personnage
//! VISÉ par la commande est celui de l'autre fenêtre ([`partner_character`]).
//!
//! **Rien n'est envoyé si la fenêtre au premier plan n'est pas une fenêtre de jeu**
//! ([`PartnerError::NoGameFocused`]) : ces raccourcis sont globaux (voir `shortcuts`), F1 dans un
//! navigateur ne doit pas y écrire `/i "..."`. La touche reste néanmoins **confisquée** à
//! l'application au premier plan tant que l'overlay tourne — c'est le prix d'un raccourci global,
//! assumé et documenté ici comme au plan (§9.1 sexies).
//!
//! ## La séquence tapée
//!
//! `Entrée` (ouvre la saisie du chat de Wakfu), la ligne, `Entrée` (envoie). Trois précautions :
//!
//! - **Des délais, pas une rafale** : le client (Java) échantillonne le clavier par image. Sans
//!   [`CHAT_OPEN_DELAY`] après le premier `Entrée`, les premiers caractères partiraient avant que
//!   la saisie ait le focus et seraient perdus — ou pire, interprétés comme des raccourcis de jeu.
//! - **Sur un thread dédié** : la séquence dure ~300 ms (délais compris). La tenir dans la boucle
//!   d'événements figerait l'overlay le temps de la frappe, à chaque appui.
//! - **Un nom de personnage vérifié avant d'être cité** ([`PartnerError::UnsafeName`]) : le nom
//!   vient d'un titre de fenêtre, c'est-à-dire de l'extérieur. Un guillemet ou un retour à la ligne
//!   qui s'y glisserait sortirait de la citation et ferait taper une SECONDE commande de chat, pas
//!   celle que l'utilisateur a demandée.
//!
//! ## Ce que cette itération ne couvre pas
//!
//! - **Trois clients ou plus** : le « partenaire » est la première fenêtre de jeu qui n'a pas le
//!   focus dans l'ordre du scan — l'ordre de profondeur (la fenêtre la plus récemment au premier
//!   plan) sous Windows (`EnumWindows`), l'ordre de création (`_NET_CLIENT_LIST`) sous X11. Sans
//!   ambiguïté à deux clients, le cas visé par la demande ; au-delà, c'est un choix arbitraire mais
//!   stable, jamais une commande envoyée au hasard.
//! - **Une touche de chat autre qu'`Entrée`** : Wakfu ouvre sa saisie de chat sur `Entrée` par
//!   défaut. Un joueur qui l'aurait rebindée dans le jeu verrait la séquence échouer silencieusement
//!   (le jeu recevrait la frappe, mais hors de la saisie) — à rendre configurable le jour où le cas
//!   se présente.

use std::time::Duration;

/// Délai après le premier `Entrée`, avant de taper la ligne — voir doc de module.
const CHAT_OPEN_DELAY: Duration = Duration::from_millis(140);

/// Délai après la ligne, avant le `Entrée` qui l'envoie : laisse le jeu enregistrer le dernier
/// caractère avant la validation.
const SEND_DELAY: Duration = Duration::from_millis(60);

/// Une commande de chat du jeu adressée à un personnage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatCommand {
    /// Invitation en groupe — `/i "<nom>"`.
    Invite,
    /// Suivi de déplacement — `/fol "<nom>"`.
    Follow,
}

impl ChatCommand {
    /// Le raccourci textuel du jeu, tel que Wakfu l'attend.
    pub fn slash(self) -> &'static str {
        match self {
            Self::Invite => "/i",
            Self::Follow => "/fol",
        }
    }

    /// Libellé français, pour les traces et l'onglet « Raccourcis ».
    pub fn label(self) -> &'static str {
        match self {
            Self::Invite => "Inviter",
            Self::Follow => "Suivre",
        }
    }

    /// La ligne exacte à taper. Le nom est **cité** : les noms de personnage Wakfu peuvent contenir
    /// une espace (« Sagittarius Caecus »), que le jeu couperait sinon au premier mot.
    pub fn line(self, character: &str) -> String {
        format!("{} \"{}\"", self.slash(), character)
    }
}

/// La ligne d'une réponse en privé — `/w "<nom>" `, **laissée ouverte** : espace final, pas
/// d'envoi, le joueur n'a plus que son message à taper (décision du 2026-09-13). Pas une variante
/// de [`ChatCommand`] : celles-ci sont des commandes COMPLÈTES, envoyées, liées à un raccourci.
pub fn whisper_line(character: &str) -> String {
    format!("/w \"{character}\" ")
}

/// Prépare une réponse en privé à `character` dans la fenêtre de jeu `target` — le clic sur la
/// carte d'alerte de chat (`panels::watchlist::toast_card`, 2026-09-13).
///
/// Séquence, sur un thread dédié comme [`send`] : **rendre le focus au jeu** (`target`, la fenêtre
/// de jeu au premier plan ou la première trouvée — le clic sur l'overlay a pu le lui prendre, sous
/// X11 surtout), un délai pour que l'OS livre le focus, `Entrée` (ouvre la saisie du chat), puis
/// `/w "Nom" ` — **sans `Entrée` final** : c'est au joueur d'écrire son message.
///
/// `target` est la clé de fenêtre de la plateforme (`HWND` réduit à son entier sous Windows, XID
/// sous X11) ; `None` quand aucune fenêtre de jeu n'est connue — on tape alors là où est le focus,
/// comme les raccourcis multicompte.
pub fn send_whisper(character: &str, target: Option<usize>) {
    if !is_quotable(character) {
        tracing::warn!(
            "[chat] réponse en privé refusée : nom de personnage non citable ({character:?})"
        );
        return;
    }
    let line = whisper_line(character);
    let character = character.to_string();
    std::thread::Builder::new()
        .name("chat-whisper".into())
        .spawn(move || {
            if let Some(target) = target {
                match imp::activate(target) {
                    Ok(()) => std::thread::sleep(FOCUS_DELAY),
                    Err(err) => tracing::warn!("[chat] focus du jeu non rendu : {err}"),
                }
            }
            match imp::type_open_line(&line, CHAT_OPEN_DELAY) {
                Ok(()) => tracing::info!("[chat] réponse en privé préparée pour {character}"),
                Err(err) => {
                    tracing::warn!("[chat] réponse en privé vers {character} en échec : {err}")
                }
            }
        })
        .map(|_| ())
        .unwrap_or_else(|err| {
            tracing::warn!("[chat] thread de frappe non démarré ({err}).");
        });
}

/// Délai entre la demande de focus et la première frappe : le temps que l'OS ou le WM livre le
/// focus à la fenêtre du jeu. Estimation, du même ordre que [`CHAT_OPEN_DELAY`].
const FOCUS_DELAY: Duration = Duration::from_millis(120);

/// Pourquoi aucune commande n'a pu être préparée — chaque cas mérite une trace différente, aucun ne
/// mérite d'interrompre l'overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartnerError {
    /// La fenêtre au premier plan n'est pas une fenêtre de jeu (navigateur, explorateur…) — voir
    /// doc de module.
    NoGameFocused,
    /// Une seule fenêtre de jeu ouverte : personne à inviter ou à suivre.
    Alone,
    /// Le nom lu dans le titre de la fenêtre ne peut pas être cité sans risque — voir doc de module.
    UnsafeName,
}

impl PartnerError {
    /// Message de trace, du point de vue de l'utilisateur (pourquoi « il ne s'est rien passé »).
    pub fn message(self) -> &'static str {
        match self {
            Self::NoGameFocused => {
                "aucune fenêtre de jeu au premier plan — raccourci multicompte ignoré"
            }
            Self::Alone => "une seule fenêtre de jeu ouverte — aucun autre personnage à viser",
            Self::UnsafeName => {
                "nom de personnage inattendu dans le titre de la fenêtre — commande non envoyée"
            }
        }
    }
}

/// Nom du personnage de la fenêtre de jeu qui **n'a pas** le focus.
///
/// `windows` est le scan courant (`game_window::GameWindowTracker::scan`, dans son ordre), `focused`
/// la clé (`HWND` sous Windows, XID sous X11) de la fenêtre au premier plan. Fonction PURE, testée
/// ici : c'est toute la logique de sélection, et elle ne doit dépendre d'aucun OS.
pub fn partner_character<'a, K: PartialEq>(
    windows: &'a [(String, K)],
    focused: &K,
) -> Result<&'a str, PartnerError> {
    if !windows.iter().any(|(_, key)| key == focused) {
        return Err(PartnerError::NoGameFocused);
    }
    let partner = windows
        .iter()
        .find(|(_, key)| key != focused)
        .map(|(name, _)| name.as_str())
        .ok_or(PartnerError::Alone)?;
    is_quotable(partner)
        .then_some(partner)
        .ok_or(PartnerError::UnsafeName)
}

/// Un nom citable tel quel dans la ligne de chat : non vide, sans guillemet (qui refermerait la
/// citation) ni caractère de contrôle (dont `\n`, qui enverrait la ligne au milieu du nom et
/// laisserait taper la suite comme une commande à part).
fn is_quotable(name: &str) -> bool {
    !name.is_empty() && !name.contains('"') && !name.chars().any(char::is_control)
}

/// Tape la commande dans le chat de la fenêtre au premier plan, sur un thread dédié (voir doc de
/// module) — rend la main immédiatement.
///
/// Aucun retour : un échec de frappe (serveur X sans XTEST, `SendInput` refusé par une élévation de
/// privilèges de la fenêtre visée…) est journalisé, jamais remonté à la boucle d'événements — il n'y
/// a rien que l'overlay puisse faire de mieux, et surtout rien qui justifie de l'interrompre.
pub fn send(command: ChatCommand, character: &str) {
    let line = command.line(character);
    let character = character.to_string();
    std::thread::Builder::new()
        .name("chat-command".into())
        .spawn(move || {
            if let Err(err) = imp::type_chat_line(&line, CHAT_OPEN_DELAY, SEND_DELAY) {
                tracing::warn!(
                    "[multicompte] « {} » vers {} en échec : {err}",
                    command.label(),
                    character
                );
            }
        })
        .map(|_| ())
        .unwrap_or_else(|err| {
            tracing::warn!("[multicompte] thread de frappe non démarré ({err}).");
        });
}

#[cfg(target_os = "windows")]
mod imp {
    use std::mem::size_of;
    use std::thread::sleep;
    use std::time::Duration;

    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
        KEYEVENTF_UNICODE, VIRTUAL_KEY, VK_RETURN,
    };

    /// Même raison et même ordre de grandeur que la pause entre deux frappes côté X11
    /// (`overlay_platform::linux::keyboard`) : le client Java perd des caractères envoyés d'un bloc.
    const KEY_DELAY: Duration = Duration::from_millis(12);

    /// `Entrée`, la ligne, `Entrée` — voir la doc de module pour la séquence et ses délais.
    pub fn type_chat_line(
        line: &str,
        open_delay: Duration,
        send_delay: Duration,
    ) -> Result<(), String> {
        tap_return()?;
        sleep(open_delay);
        for unit in line.encode_utf16() {
            // `KEYEVENTF_UNICODE` : la frappe porte le CARACTÈRE, pas une touche physique — aucune
            // dépendance au layout de l'utilisateur (le `/` d'un clavier AZERTY est ailleurs qu'en
            // QWERTY, et la commande doit partir identique dans les deux cas).
            send(&[unicode_input(unit, false), unicode_input(unit, true)])?;
            sleep(KEY_DELAY);
        }
        sleep(send_delay);
        tap_return()
    }

    /// `Entrée`, la ligne, et c'est tout — voir `send_whisper`.
    pub fn type_open_line(line: &str, open_delay: Duration) -> Result<(), String> {
        tap_return()?;
        sleep(open_delay);
        for unit in line.encode_utf16() {
            send(&[unicode_input(unit, false), unicode_input(unit, true)])?;
            sleep(KEY_DELAY);
        }
        Ok(())
    }

    /// Rend le premier plan à la fenêtre de jeu. `SetForegroundWindow` n'est honoré que par un
    /// processus qui vient de recevoir l'entrée utilisateur — c'est notre cas, la demande suit un
    /// clic sur l'overlay ; sinon Windows se contente de faire clignoter la barre des tâches, et
    /// la frappe part vers la fenêtre qui a réellement le focus.
    pub fn activate(target: usize) -> Result<(), String> {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow;
        let hwnd = HWND(target as *mut core::ffi::c_void);
        let ok = unsafe { SetForegroundWindow(hwnd) }.as_bool();
        ok.then_some(())
            .ok_or_else(|| "SetForegroundWindow a refusé".to_string())
    }

    fn tap_return() -> Result<(), String> {
        send(&[key_input(VK_RETURN, false), key_input(VK_RETURN, true)])
    }

    fn send(inputs: &[INPUT]) -> Result<(), String> {
        // `SendInput` renvoie le nombre d'événements RÉELLEMENT insérés : 0 quand un autre
        // processus a bloqué l'injection (UIPI — une fenêtre élevée au premier plan, par exemple).
        // Silencieux sans cette vérification.
        let sent = unsafe { SendInput(inputs, size_of::<INPUT>() as i32) } as usize;
        (sent == inputs.len()).then_some(()).ok_or_else(|| {
            format!(
                "SendInput n'a inséré que {sent}/{} événements",
                inputs.len()
            )
        })
    }

    fn key_input(key: VIRTUAL_KEY, up: bool) -> INPUT {
        keyboard_input(
            key,
            0,
            if up {
                KEYEVENTF_KEYUP
            } else {
                KEYBD_EVENT_FLAGS(0)
            },
        )
    }

    fn unicode_input(unit: u16, up: bool) -> INPUT {
        let flags = if up {
            KEYEVENTF_UNICODE | KEYEVENTF_KEYUP
        } else {
            KEYEVENTF_UNICODE
        };
        keyboard_input(VIRTUAL_KEY(0), unit, flags)
    }

    fn keyboard_input(key: VIRTUAL_KEY, scan: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: key,
                    wScan: scan,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    use std::time::Duration;

    /// Délègue à `overlay_platform::linux::keyboard` (XTEST) — même découpage que `game_window` :
    /// le code X11 réutilisable vit dans `overlay-platform`, le code Windows reste ici.
    pub fn type_chat_line(
        line: &str,
        open_delay: Duration,
        send_delay: Duration,
    ) -> Result<(), overlay_platform::linux::keyboard::TypeError> {
        overlay_platform::linux::keyboard::type_chat_line(line, open_delay, send_delay)
    }

    /// `Entrée`, la ligne, et c'est tout — voir `send_whisper`.
    pub fn type_open_line(
        line: &str,
        open_delay: Duration,
    ) -> Result<(), overlay_platform::linux::keyboard::TypeError> {
        overlay_platform::linux::keyboard::type_open_line(line, open_delay)
    }

    /// Rend le focus à la fenêtre de jeu par `_NET_ACTIVE_WINDOW` — voir
    /// `overlay_platform::linux::x11::activate_window`.
    pub fn activate(target: usize) -> Result<(), String> {
        let window = u32::try_from(target).map_err(|_| "XID hors plage".to_string())?;
        overlay_platform::linux::x11::activate_window(window)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fenetres(noms: &[(&str, u32)]) -> Vec<(String, u32)> {
        noms.iter()
            .map(|(nom, cle)| ((*nom).to_string(), *cle))
            .collect()
    }

    #[test]
    fn lignes_citees_comme_le_jeu_les_attend() {
        assert_eq!(ChatCommand::Invite.line("Oumbra"), "/i \"Oumbra\"");
        // La réponse en privé reste ouverte : espace final, pas d'envoi.
        assert_eq!(whisper_line("Huppermage-Bleu"), "/w \"Huppermage-Bleu\" ");
        assert_eq!(
            ChatCommand::Follow.line("Sagittarius Caecus"),
            "/fol \"Sagittarius Caecus\""
        );
    }

    #[test]
    fn le_partenaire_est_la_fenetre_qui_n_a_pas_le_focus() {
        let windows = fenetres(&[("Oumbra", 1), ("Sagittarius Caecus", 2)]);
        assert_eq!(partner_character(&windows, &1), Ok("Sagittarius Caecus"));
        assert_eq!(partner_character(&windows, &2), Ok("Oumbra"));
    }

    /// Le cas qui protège les autres applications : F1 dans un navigateur ne doit RIEN envoyer.
    #[test]
    fn rien_quand_le_premier_plan_n_est_pas_le_jeu() {
        let windows = fenetres(&[("Oumbra", 1), ("Sagittarius Caecus", 2)]);
        assert_eq!(
            partner_character(&windows, &99),
            Err(PartnerError::NoGameFocused)
        );
    }

    #[test]
    fn rien_avec_un_seul_client_ouvert() {
        let windows = fenetres(&[("Oumbra", 1)]);
        assert_eq!(partner_character(&windows, &1), Err(PartnerError::Alone));
    }

    #[test]
    fn rien_sans_aucune_fenetre_de_jeu() {
        assert_eq!(
            partner_character(&fenetres(&[]), &1),
            Err(PartnerError::NoGameFocused)
        );
    }

    /// À trois clients, le premier du scan qui n'a pas le focus — choix arbitraire mais STABLE
    /// (voir doc de module), jamais une commande envoyée à un personnage au hasard.
    #[test]
    fn a_trois_clients_le_premier_du_scan_sans_focus() {
        let windows = fenetres(&[("Alpha", 1), ("Beta", 2), ("Gamma", 3)]);
        assert_eq!(partner_character(&windows, &2), Ok("Alpha"));
        assert_eq!(partner_character(&windows, &1), Ok("Beta"));
    }

    /// Un titre de fenêtre est une donnée EXTERNE : un guillemet ou un retour à la ligne y
    /// transformerait la commande citée en deux commandes.
    #[test]
    fn nom_non_citable_refuse() {
        for nom in ["Ou\"mbra", "Oum\nbra", ""] {
            let windows = fenetres(&[("Focus", 1), (nom, 2)]);
            assert_eq!(
                partner_character(&windows, &1),
                Err(PartnerError::UnsafeName),
                "nom « {nom} »"
            );
        }
    }

    #[test]
    fn nom_avec_espace_ou_tiret_accepte() {
        let windows = fenetres(&[("Focus", 1), ("Sagittarius-Caecus Le Grand", 2)]);
        assert_eq!(
            partner_character(&windows, &1),
            Ok("Sagittarius-Caecus Le Grand")
        );
    }
}
