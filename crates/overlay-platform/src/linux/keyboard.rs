//! Frappe clavier SYNTHÉTIQUE sous X11 (extension XTEST) — pendant Linux de `SendInput` sous
//! Windows (`overlay_ui::chat_command`, module `imp` Windows). Sert à taper une commande de chat
//! dans la fenêtre de jeu **qui a déjà le focus** : l'overlay n'active jamais aucune fenêtre (voir
//! `overlay_ui::chat_command`, doc de module), il se contente d'envoyer la frappe là où le clavier
//! pointe déjà.
//!
//! **Pourquoi XTEST et pas un `SendEvent` ciblé sur la fenêtre** : un `XSendEvent` porte
//! `send_event = true`, drapeau que la plupart des toolkits (AWT/Swing compris, donc le client
//! Wakfu) ignorent délibérément — la frappe n'arriverait jamais. XTEST injecte au niveau du
//! SERVEUR, exactement comme un clavier physique : aucune application ne peut la distinguer d'une
//! vraie frappe, et elle suit le focus clavier réel.
//!
//! **Ce que ce module ne fait PAS** : il ne déplace jamais le focus, n'ouvre aucune fenêtre et ne
//! lit aucun état du jeu. Il ne sait que « appuyer puis relâcher » une suite de touches.
//!
//! ## Du caractère à la touche
//!
//! XTEST raisonne en *keycodes* (numéro de touche physique), pas en caractères : il faut donc
//! traduire chaque caractère en keysym (`'a'` → `XK_a`), puis retrouver quelle touche du
//! **layout courant de l'utilisateur** produit ce keysym, et à quel niveau (nu ou Maj). C'est ce
//! qui rend la frappe correcte en AZERTY comme en QWERTY : `/` est une touche nue en QWERTY et un
//! `Maj+:` en AZERTY, et la table lue ici le dit — aucun keycode n'est codé en dur.
//!
//! Quand le layout ne produit PAS le caractère voulu (un nom de personnage accentué sur un clavier
//! qui n'a pas cette lettre), on emprunte la technique de `xdotool` : réaffecter temporairement un
//! keycode LIBRE au keysym manquant, frapper, puis rendre le keycode à son état d'origine
//! ([`Typist::with_scratch`]). Le layout de l'utilisateur est restauré dans tous les cas, y compris
//! en cas d'erreur en cours de frappe.

use std::thread::sleep;
use std::time::Duration;

use x11rb::connection::{Connection, RequestConnection as _};
use x11rb::errors::{ConnectError, ConnectionError, ReplyError};
use x11rb::protocol::xproto::{
    ConnectionExt as _, Keycode, Window, KEY_PRESS_EVENT, KEY_RELEASE_EVENT,
};
use x11rb::protocol::xtest::ConnectionExt as _;
use x11rb::rust_connection::RustConnection;

/// `XK_Return` — la touche qui ouvre puis valide le chat de Wakfu.
const KEYSYM_RETURN: u32 = 0xff0d;
/// `XK_Shift_L` — modificateur pressé pour atteindre le niveau 2 d'une touche (voir
/// [`Typist::locate`]).
const KEYSYM_SHIFT_L: u32 = 0xffe1;

/// Pause entre deux frappes. Le client Wakfu (Java) échantillonne le clavier par image : une rafale
/// envoyée d'un bloc lui ferait perdre des caractères. 12 ms est le défaut de `xdotool type`, assez
/// lent pour être vu de tous et assez rapide pour qu'une commande de chat parte en moins de 300 ms.
const KEY_DELAY: Duration = Duration::from_millis(12);

/// Délai laissé au serveur X et aux clients pour prendre en compte une réaffectation de keycode
/// (`MappingNotify` est traité de façon asynchrone par les toolkits) — même ordre de grandeur que
/// `xdotool`, qui attend lui aussi avant de frapper une touche qu'il vient de réaffecter.
const REMAP_SETTLE: Duration = Duration::from_millis(30);

#[derive(Debug)]
pub enum TypeError {
    /// Pas de serveur X joignable (`$DISPLAY` absent ou refusé).
    Connect(ConnectError),
    /// Serveur X sans extension XTEST — refus explicite plutôt qu'une frappe silencieusement
    /// perdue. Rare (XTEST est compilé dans tous les serveurs X courants) mais possible sur un
    /// serveur durci.
    MissingXtest,
    /// Échec d'un échange avec le serveur X en cours de frappe.
    Protocol(String),
    /// Caractère introuvable dans le layout ET aucun keycode libre pour l'emprunter — la frappe
    /// s'arrête là plutôt que d'envoyer une commande tronquée au jeu.
    Unmappable(char),
    /// Aucun keycode libre à emprunter ([`Typist::free_keycode`]) — remonté tel quel par
    /// [`Typist::tap_return`], converti en [`Self::Unmappable`] par [`Typist::type_text`], qui
    /// seul sait de quel caractère il s'agissait.
    NoScratchKeycode,
}

impl std::fmt::Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connect(err) => write!(f, "connexion X11 impossible ({err})"),
            Self::MissingXtest => f.write_str("extension XTEST absente du serveur X"),
            Self::Protocol(err) => write!(f, "échange X11 en échec ({err})"),
            Self::Unmappable(c) => write!(
                f,
                "caractère « {c} » absent du layout et aucun keycode libre pour l'emprunter"
            ),
            Self::NoScratchKeycode => f.write_str("aucun keycode libre à emprunter dans le layout"),
        }
    }
}

impl std::error::Error for TypeError {}

impl From<ConnectionError> for TypeError {
    fn from(err: ConnectionError) -> Self {
        Self::Protocol(err.to_string())
    }
}

impl From<ReplyError> for TypeError {
    fn from(err: ReplyError) -> Self {
        Self::Protocol(err.to_string())
    }
}

/// Une connexion X11 dédiée à la frappe synthétique, avec la table de clavier de l'utilisateur
/// telle qu'elle est AU MOMENT de la connexion.
///
/// **Jamais gardée entre deux frappes** (voir [`type_chat_line`]) : l'utilisateur peut changer de
/// layout en cours de session (`setxkbmap`), et une table relue à chaque commande coûte un aller-
/// retour X11 de quelques centaines de microsecondes — sans commune mesure avec les ~300 ms que
/// dure la frappe elle-même.
pub struct Typist {
    conn: RustConnection,
    root: Window,
    min_keycode: Keycode,
    keysyms_per_keycode: usize,
    keysyms: Vec<u32>,
}

impl Typist {
    pub fn connect() -> Result<Self, TypeError> {
        let (conn, screen_num) = RustConnection::connect(None).map_err(TypeError::Connect)?;
        let root = conn.setup().roots[screen_num].root;
        if conn
            .extension_information(x11rb::protocol::xtest::X11_EXTENSION_NAME)?
            .is_none()
        {
            return Err(TypeError::MissingXtest);
        }
        let min_keycode = conn.setup().min_keycode;
        let max_keycode = conn.setup().max_keycode;
        let count = max_keycode.saturating_sub(min_keycode).saturating_add(1);
        let mapping = conn.get_keyboard_mapping(min_keycode, count)?.reply()?;
        Ok(Self {
            conn,
            root,
            min_keycode,
            keysyms_per_keycode: mapping.keysyms_per_keycode as usize,
            keysyms: mapping.keysyms,
        })
    }

    /// Appuie puis relâche Entrée.
    pub fn tap_return(&self) -> Result<(), TypeError> {
        self.tap_keysym(KEYSYM_RETURN)
    }

    /// Tape le texte caractère par caractère, dans l'ordre.
    pub fn type_text(&self, text: &str) -> Result<(), TypeError> {
        for c in text.chars() {
            self.tap_keysym(keysym_for(c)).map_err(|err| match err {
                // Un keycode manquant ne dit rien à l'appelant ; le CARACTÈRE en cause, si.
                TypeError::NoScratchKeycode => TypeError::Unmappable(c),
                other => other,
            })?;
        }
        Ok(())
    }

    /// Frappe le keysym demandé — directement s'il existe dans le layout, via un keycode emprunté
    /// sinon (voir doc de module).
    fn tap_keysym(&self, keysym: u32) -> Result<(), TypeError> {
        match self.locate(keysym) {
            Some((keycode, shifted)) => self.tap_keycode(keycode, shifted),
            None => self.with_scratch(keysym, |keycode| self.tap_keycode(keycode, false)),
        }
    }

    /// `(keycode, faut-il Maj)` pour ce keysym dans le layout courant.
    ///
    /// Seuls les deux premiers niveaux de chaque touche sont considérés (nu, Maj) : les suivants
    /// demandent AltGr ou un changement de groupe, dont la combinaison de modificateurs exacte
    /// dépend de la configuration XKB de l'utilisateur. Plutôt que de la deviner — et de risquer
    /// d'envoyer au jeu un AltGr parasite — un keysym qui n'est atteignable qu'au-delà passe par
    /// le keycode emprunté, dont on maîtrise entièrement l'état.
    fn locate(&self, keysym: u32) -> Option<(Keycode, bool)> {
        if keysym == 0 || self.keysyms_per_keycode == 0 {
            return None;
        }
        let levels = self.keysyms_per_keycode.min(2);
        self.keysyms
            .chunks(self.keysyms_per_keycode)
            .enumerate()
            .find_map(|(index, chunk)| {
                let level = chunk.iter().take(levels).position(|&sym| sym == keysym)?;
                let keycode = self.min_keycode.checked_add(u8::try_from(index).ok()?)?;
                Some((keycode, level == 1))
            })
    }

    /// Premier keycode dont TOUS les niveaux sont `NoSymbol` : réaffecter celui-là ne prend aucune
    /// touche à l'utilisateur. `None` sur un layout qui remplit toute la plage `min_keycode..=max`
    /// (jamais vu en pratique : la plage compte 248 entrées pour ~110 touches réelles).
    fn free_keycode(&self) -> Option<Keycode> {
        if self.keysyms_per_keycode == 0 {
            return None;
        }
        self.keysyms
            .chunks(self.keysyms_per_keycode)
            .enumerate()
            .find(|(_, chunk)| chunk.iter().all(|&sym| sym == 0))
            .and_then(|(index, _)| self.min_keycode.checked_add(u8::try_from(index).ok()?))
    }

    /// Prête un keycode libre au keysym demandé le temps de `frappe`, puis lui **rend son état
    /// d'origine quoi qu'il arrive** — y compris si `frappe` échoue : laisser une touche
    /// réaffectée derrière soi abîmerait le clavier de l'utilisateur jusqu'à sa prochaine
    /// reconfiguration XKB.
    fn with_scratch<T>(
        &self,
        keysym: u32,
        frappe: impl FnOnce(Keycode) -> Result<T, TypeError>,
    ) -> Result<T, TypeError> {
        let keycode = self.free_keycode().ok_or(TypeError::NoScratchKeycode)?;
        // Tous les niveaux sur le même keysym (comme `xdotool`) : la touche produit alors le
        // caractère voulu que Maj soit enfoncée ou non, ce qui évite de dépendre de l'état
        // physique du clavier au moment de la frappe.
        let assigned = vec![keysym; self.keysyms_per_keycode.max(1)];
        self.remap(keycode, &assigned)?;
        let outcome = frappe(keycode);
        let restored = self.remap(keycode, &vec![0; self.keysyms_per_keycode.max(1)]);
        // L'échec de la FRAPPE prime sur celui de la restauration : c'est lui qui explique
        // pourquoi la commande n'est pas partie. Une restauration en échec toute seule reste une
        // erreur — le clavier de l'utilisateur porte alors une touche de trop, il doit le savoir.
        match (outcome, restored) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(err), _) | (Ok(_), Err(err)) => Err(err),
        }
    }

    fn remap(&self, keycode: Keycode, keysyms: &[u32]) -> Result<(), TypeError> {
        let per = u8::try_from(keysyms.len()).unwrap_or(1);
        self.conn
            .change_keyboard_mapping(1, keycode, per, keysyms)?
            // `.check()` fait l'aller-retour qui garantit que le serveur a bien pris la nouvelle
            // table AVANT qu'on frappe la touche (une frappe envoyée d'abord produirait l'ancien
            // keysym, c'est-à-dire rien du tout sur un keycode libre).
            .check()
            .map_err(|err| TypeError::Protocol(err.to_string()))?;
        sleep(REMAP_SETTLE);
        Ok(())
    }

    fn tap_keycode(&self, keycode: Keycode, shifted: bool) -> Result<(), TypeError> {
        let shift = shifted
            .then(|| self.locate(KEYSYM_SHIFT_L).map(|(keycode, _)| keycode))
            .flatten();
        if let Some(shift) = shift {
            self.fake_key(shift, true)?;
        }
        self.fake_key(keycode, true)?;
        self.fake_key(keycode, false)?;
        if let Some(shift) = shift {
            self.fake_key(shift, false)?;
        }
        self.conn.flush()?;
        sleep(KEY_DELAY);
        Ok(())
    }

    fn fake_key(&self, keycode: Keycode, press: bool) -> Result<(), TypeError> {
        let event = if press {
            KEY_PRESS_EVENT
        } else {
            KEY_RELEASE_EVENT
        };
        self.conn
            .xtest_fake_input(event, keycode, 0, self.root, 0, 0, 0)?
            .ignore_error();
        Ok(())
    }
}

/// Keysym correspondant à un caractère Unicode : identité pour Latin-1 (les keysyms X11 y sont
/// alignés sur Unicode par construction), plage `0x01000000 + codepoint` au-delà — la convention
/// universelle d'X11 pour les caractères sans keysym historique.
fn keysym_for(c: char) -> u32 {
    let codepoint = c as u32;
    if codepoint < 0x100 {
        codepoint
    } else {
        0x0100_0000 + codepoint
    }
}

/// Ouvre une connexion, tape `ligne` dans le chat de la fenêtre qui a le focus, referme.
///
/// Séquence : `Entrée` (ouvre la saisie du chat), la ligne, `Entrée` (envoie) — voir
/// `overlay_ui::chat_command` pour le POURQUOI de cette séquence et de ses délais.
pub fn type_chat_line(
    line: &str,
    open_delay: Duration,
    send_delay: Duration,
) -> Result<(), TypeError> {
    let typist = Typist::connect()?;
    typist.tap_return()?;
    sleep(open_delay);
    typist.type_text(line)?;
    sleep(send_delay);
    typist.tap_return()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les caractères d'une commande de chat (`/`, espace, `"`, lettres, chiffres) restent dans la
    /// plage Latin-1, où keysym et codepoint coïncident — c'est ce qui permet à [`locate`] de les
    /// retrouver dans n'importe quel layout sans table de correspondance codée en dur.
    #[test]
    fn keysyms_latin1_identiques_au_codepoint() {
        for c in ['/', ' ', '"', 'a', 'Z', '0', '-', 'é'] {
            assert_eq!(keysym_for(c), c as u32, "keysym de « {c} »");
        }
    }

    /// Au-delà de Latin-1, la convention X11 `0x01000000 + codepoint` s'applique — un nom de
    /// personnage exotique reste frappable via le keycode emprunté.
    #[test]
    fn keysyms_hors_latin1_dans_la_plage_unicode() {
        assert_eq!(keysym_for('œ'), 0x0100_0000 + 'œ' as u32);
        assert_eq!(keysym_for('中'), 0x0100_0000 + '中' as u32);
    }
}
