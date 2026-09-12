//! Icônes réelles d'objets/monstres (`wakassets`, voir `overlay_engine::IconRef::image_url`) —
//! remplace l'icône générique du panneau Suivi (`panels::watchlist`) dès qu'une entrée est résolue
//! par le catalogue (`CatalogIndex`, voir `catalog.rs`). Retour utilisateur 2026-09-02 : « comme
//! les images de ressources/monstres n'est pas présent c'est très compliqué pour l'utilisateur »
//! de distinguer les tuiles entre elles avec la même icône générique partout.
//!
//! Deux couches distinctes, pour la même raison que `portraits.rs`/`ui_icons.rs` ne sont PAS
//! partagées entre fenêtres (une `egui::TextureHandle` est liée à SON `egui::Context`, jamais
//! transférable à un autre) :
//! - [`RemoteIconStore`] : un thread dédié PARTAGÉ par toutes les fenêtres overlay — télécharge
//!   (best-effort, `overlay_sync::icon_cache` en repli disque) et décode chaque icône UNE SEULE
//!   fois même avec plusieurs comptes/fenêtres, réveille l'UI (`UserEvent::NewSnapshot`,
//!   réutilisé tel quel — toutes les fenêtres redessinent déjà dessus) dès qu'une icône est prête.
//! - [`RemoteIconTextures`] : cache PAR FENÊTRE des `egui::TextureHandle` déjà uploadées depuis
//!   les octets décodés partagés ci-dessus — un upload par fenêtre qui affiche cette icône, mais
//!   jamais un second téléchargement.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

use overlay_engine::{IconKind, IconRef};
use winit::event_loop::EventLoopProxy;

use crate::render_content::UserEvent;

type IconKey = (IconKind, String);

fn key_of(icon: &IconRef) -> IconKey {
    (icon.kind, icon.gfx_id.clone())
}

/// Image décodée en RGBA8, prête à être uploadée comme texture — voir `RemoteIconTextures`.
struct DecodedIcon {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

/// Nombre de téléchargements simultanés. Quatre : assez pour qu'une liste d'autocomplétion de
/// cent entrées s'illustre en une ou deux secondes plutôt qu'en quinze, assez peu pour ne pas
/// ressembler à une rafale vue du CDN. Le cache disque rend de toute façon les lancements suivants
/// quasi instantanés.
const WORKERS: usize = 4;

/// Une icône en attente de téléchargement, ordonnée pour le tas de `PendingRequests` : la frame
/// la plus récente d'abord, et **dans l'ordre de demande à l'intérieur d'une même frame** — voir
/// `RemoteIconStore`.
struct Pending {
    frame: u64,
    seq: std::cmp::Reverse<u64>,
    icon: IconRef,
}

impl PartialEq for Pending {
    fn eq(&self, other: &Self) -> bool {
        self.frame == other.frame && self.seq == other.seq
    }
}
impl Eq for Pending {}
impl PartialOrd for Pending {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Pending {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.frame, self.seq).cmp(&(other.frame, other.seq))
    }
}

/// File d'attente des icônes à télécharger — **un tas, pas une file** : voir `RemoteIconStore`.
#[derive(Default)]
struct PendingRequests {
    heap: std::collections::BinaryHeap<Pending>,
    /// Numéro d'ordre de la prochaine demande — départage deux demandes d'une même frame.
    seq: u64,
    /// Vrai une fois le dernier `RemoteIconStore` abandonné : les threads s'arrêtent au lieu
    /// d'attendre indéfiniment une requête qui ne viendra plus.
    closed: bool,
}

/// Partagé (via `Clone`, tous les champs sont eux-mêmes des `Arc`) entre toutes les fenêtres
/// overlay — voir la doc de module. `spawn` démarre les threads dédiés une seule fois ; les clones
/// suivants partagent le même état.
///
/// ## La frame la plus récente d'abord, et dans son ordre (2026-09-12)
///
/// Le champ d'autocomplétion demande les icônes de tout ce qu'il affiche, à chaque frappe :
/// « bou », puis « bouf », puis « bouft »… Servies dans l'ordre d'arrivée, les icônes de la requête
/// courante passaient après celles de trois préfixes que l'utilisateur ne regarde déjà plus —
/// c'est ce que l'on voyait : des rangées qui restaient sans image plusieurs secondes après la
/// frappe, alors que le réseau travaillait.
///
/// D'où un tas ordonné par **numéro de frame egui décroissant** (`Context::cumulative_pass_nr`,
/// transmis par `RemoteIconTextures::resolve`) : ce que la dernière frame a demandé part en
/// premier, les préfixes abandonnés suivent, et finissent en cache pour la prochaine fois. Une
/// simple pile LIFO ne suffisait pas — essayée d'abord : elle renversait aussi l'ordre À
/// L'INTÉRIEUR d'une frame, donc servait la centième rangée (jamais visible sans défiler) avant la
/// première. À frame égale, l'ordre de demande est conservé — et l'appelant demande dans l'ordre
/// d'affichage.
#[derive(Clone)]
pub struct RemoteIconStore {
    decoded: Arc<Mutex<HashMap<IconKey, Arc<DecodedIcon>>>>,
    /// Icônes déjà demandées (avec ou sans succès) — jamais redemander une icône dont le
    /// téléchargement a échoué à CE lancement (un CDN externe qui répond une fois en erreur pour
    /// un `gfx_id` donné a peu de chances de mieux répondre à la tentative suivante ; un vrai
    /// mécanisme de retenter serait pertinent mais hors périmètre de cette itération).
    requested: Arc<Mutex<HashSet<IconKey>>>,
    pending: Arc<(Mutex<PendingRequests>, Condvar)>,
}

impl RemoteIconStore {
    pub fn spawn(proxy: EventLoopProxy<UserEvent>) -> Self {
        let decoded: Arc<Mutex<HashMap<IconKey, Arc<DecodedIcon>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let pending: Arc<(Mutex<PendingRequests>, Condvar)> = Arc::default();

        for n in 0..WORKERS {
            let decoded = Arc::clone(&decoded);
            let pending = Arc::clone(&pending);
            let proxy = proxy.clone();
            thread::Builder::new()
                .name(format!("overlay-icons-{n}"))
                .spawn(move || {
                    let (queue, ready) = &*pending;
                    loop {
                        let icon = {
                            let mut guard = queue.lock().unwrap();
                            loop {
                                if let Some(pending) = guard.heap.pop() {
                                    break pending.icon;
                                }
                                if guard.closed {
                                    return;
                                }
                                guard = ready.wait(guard).unwrap();
                            }
                        };
                        let key = key_of(&icon);
                        if let Some(decoded_icon) = fetch_and_decode(&icon) {
                            decoded.lock().unwrap().insert(key, Arc::new(decoded_icon));
                            let _ = proxy.send_event(UserEvent::NewSnapshot);
                        }
                    }
                })
                .expect("échec de création du thread des icônes distantes");
        }

        Self {
            decoded,
            requested: Arc::new(Mutex::new(HashSet::new())),
            pending,
        }
    }

    /// Store vide, SANS thread réseau — pour un harnais de test qui ne doit dépendre d'aucun accès
    /// réseau (§17.1 du plan : « `RemoteIconStore` construit à la main dans le harnais, jamais
    /// alimenté par le thread réseau réel »). `decoded_or_request` y renvoie toujours `None`
    /// (repli sur l'icône générique) : la file est marquée fermée, rien n'y est jamais déposé.
    pub fn empty() -> Self {
        Self {
            decoded: Arc::new(Mutex::new(HashMap::new())),
            requested: Arc::new(Mutex::new(HashSet::new())),
            pending: Arc::new((
                Mutex::new(PendingRequests {
                    closed: true,
                    ..Default::default()
                }),
                Condvar::new(),
            )),
        }
    }

    /// Injecte une icône déjà disponible sous forme de fichier PNG/WebP — pour un harnais de rendu
    /// (`overlay-testkit`) qui doit montrer de VRAIES icônes sans jamais toucher le réseau
    /// (§17.1 du plan : « construit à la main dans le harnais, jamais alimenté par le thread
    /// réseau réel »), à partir de fixtures versionnées. Même décodage que le thread réseau
    /// (`fetch_and_decode`), même table `decoded` : l'UI ne fait aucune différence. Renvoie
    /// `false` (et n'insère rien) si `bytes` n'est pas une image lisible. Jamais appelé par le code
    /// de production — mais pas `#[cfg(test)]` : les tests d'intégration d'un AUTRE crate ne
    /// compilent pas la lib en `cfg(test)`.
    pub fn preload(&self, icon: &IconRef, bytes: &[u8]) -> bool {
        let Some(decoded) = decode_icon(bytes) else {
            return false;
        };
        let key = key_of(icon);
        self.requested.lock().unwrap().insert(key.clone());
        self.decoded.lock().unwrap().insert(key, Arc::new(decoded));
        true
    }

    /// Octets décodés déjà disponibles pour cette icône ; sinon programme son téléchargement (une
    /// seule fois par icône, voir `requested`) et renvoie `None` pour ce rendu — l'appelant garde
    /// son repli générique jusqu'au prochain redessin (déclenché par un thread ci-dessus une fois
    /// l'icône prête).
    fn decoded_or_request(&self, icon: &IconRef, frame: u64) -> Option<Arc<DecodedIcon>> {
        let key = key_of(icon);
        if let Some(decoded) = self.decoded.lock().unwrap().get(&key) {
            return Some(Arc::clone(decoded));
        }
        if self.requested.lock().unwrap().insert(key) {
            let (queue, ready) = &*self.pending;
            let mut guard = queue.lock().unwrap();
            if !guard.closed {
                let seq = guard.seq;
                guard.seq += 1;
                guard.heap.push(Pending {
                    frame,
                    seq: std::cmp::Reverse(seq),
                    icon: icon.clone(),
                });
                ready.notify_one();
            }
        }
        None
    }
}

impl Drop for RemoteIconStore {
    fn drop(&mut self) {
        // Le dernier clone parti, plus personne ne peut déposer : les threads peuvent s'arrêter.
        if Arc::strong_count(&self.pending) == 1 {
            let (queue, ready) = &*self.pending;
            queue.lock().unwrap().closed = true;
            ready.notify_all();
        }
    }
}

fn fetch_and_decode(icon: &IconRef) -> Option<DecodedIcon> {
    let url = icon.image_url();
    let bytes = overlay_sync::icon_cache::load(icon.kind, &icon.gfx_id).or_else(|| {
        let fetched = match overlay_sync::client::fetch_bytes(&url) {
            Ok(bytes) => bytes,
            Err(err) => {
                tracing::warn!(%url, %err, "icône distante injoignable, repli sur l'icône générique");
                return None;
            }
        };
        overlay_sync::icon_cache::save(icon.kind, &icon.gfx_id, &fetched);
        Some(fetched)
    })?;

    let decoded = decode_icon(&bytes);
    if decoded.is_none() {
        tracing::warn!(%url, "icône distante récupérée mais illisible");
    }
    decoded
}

/// Décode un fichier image (PNG/WebP, voir les features de `image` dans `Cargo.toml`) en RGBA8 —
/// partagé entre le thread réseau (`fetch_and_decode`) et l'injection de fixtures
/// (`RemoteIconStore::preload`).
fn decode_icon(bytes: &[u8]) -> Option<DecodedIcon> {
    let image = image::load_from_memory(bytes).ok()?;
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    Some(DecodedIcon {
        width,
        height,
        rgba: rgba.into_raw(),
    })
}

/// Cache PAR FENÊTRE des textures déjà uploadées — voir la doc de module. Un champ de plus sur
/// `OverlayWindow`, comme `portraits`/`icons`.
#[derive(Default)]
pub struct RemoteIconTextures {
    uploaded: HashMap<IconKey, egui::TextureHandle>,
}

impl RemoteIconTextures {
    /// Texture prête à afficher pour cette icône — `None` tant qu'elle n'a pas fini de
    /// télécharger/décoder (l'appelant garde son repli générique dans ce cas, voir
    /// `panels::watchlist::entry_tile`). Ne fait JAMAIS d'appel réseau elle-même : uploade
    /// seulement des octets déjà décodés par `RemoteIconStore`, sur le thread appelant (thread de
    /// rendu) — l'upload GPU d'une petite icône est négligeable, contrairement au téléchargement.
    pub fn resolve(
        &mut self,
        ctx: &egui::Context,
        store: &RemoteIconStore,
        icon: &IconRef,
    ) -> Option<egui::TextureHandle> {
        let key = key_of(icon);
        if let Some(handle) = self.uploaded.get(&key) {
            return Some(handle.clone());
        }
        let decoded = store.decoded_or_request(icon, ctx.cumulative_pass_nr())?;
        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            [decoded.width as usize, decoded.height as usize],
            &decoded.rgba,
        );
        let handle = ctx.load_texture(
            format!("remote-icon-{:?}-{}", icon.kind, icon.gfx_id),
            color_image,
            egui::TextureOptions::LINEAR,
        );
        self.uploaded.insert(key, handle.clone());
        Some(handle)
    }
}
