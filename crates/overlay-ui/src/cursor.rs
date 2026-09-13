//! Curseur de souris du jeu, affiché à la place du curseur système quand le pointeur survole un
//! overlay interactif — pour que l'overlay se fonde dans l'interface de Wakfu jusque dans le
//! curseur, comme il le fait déjà pour ses boutons et ses infobulles.
//!
//! **Source.** Les deux bitmaps d'`assets/cursor/` (voir son `README.md`) : `wakfu-cursor-idle.png`
//! (repos, remplissage crème) et `wakfu-cursor-flash.png` (éclair, remplissage cyan), isolés pixel
//! par pixel depuis un enregistrement d'écran du jeu (2026-09-13). Le point chaud (la pointe de la
//! flèche) est **déduit de l'image** — première ligne opaque, pixel opaque le plus à gauche — plutôt
//! que codé en dur : les fichiers peuvent être re-détourés (marge, taille) sans toucher à ce module.
//!
//! **Comportement, calqué sur le jeu** (mesuré sur la vidéo, 30 i/s) :
//!
//! - flèche système d'egui (`CursorIcon::Default`, rien de cliquable sous le pointeur) → bitmap de
//!   repos, fixe ;
//! - main (`CursorIcon::PointingHand`, tout ce qui se clique — voir `style::apply`,
//!   `Visuals::interact_cursor`) → **clignotement** : éclair pendant [`FLASH_DURATION`], repos
//!   pendant [`IDLE_DURATION`], et ainsi de suite. L'éclair vient **en premier** dès l'entrée en
//!   survol, comme dans le jeu (le cyan apparaît dans les deux images qui suivent l'arrivée du
//!   pointeur sur un bouton), et la bascule est franche, sans fondu (un seul pas d'image entre les
//!   deux couleurs sur l'enregistrement) ;
//! - croix fléchée (tout curseur de déplacement ou de saisie : `CursorIcon::Move`, posé par les
//!   tuiles de l'onglet Suivi — voir `panels::suivi_tab` —, mais aussi `Grab`/`Grabbing`, qu'egui
//!   pose LUI-MÊME au survol d'un glissable sans clic (`Response::dnd_set_drag_payload`) et tant
//!   qu'une charge est en vol (`DragAndDrop::on_end_pass`), et `AllScroll`, la même croix pour un
//!   panoramique) → bitmap `wakfu-cursor-move.png`, **fixe** : le jeu ne fait pas clignoter
//!   celui-là (couleur constante sur les 113 images où il est visible, voir
//!   `assets/cursor/README.md`). Une seule image pour les quatre : le jeu n'a qu'une croix, et une
//!   main système qui se fermerait au milieu d'un geste commencé sous la croix casserait
//!   l'illusion. Son point chaud est au CENTRE, pas à la pointe : c'est une croix symétrique, elle
//!   ne pointe nulle part ;
//! - tout autre curseur (`Text` d'un champ de saisie, `ResizeHorizontal` d'un curseur de réglage,
//!   `None`…) → curseur **système** correspondant, inchangé : le jeu lui-même n'a pas de variante
//!   de sa flèche pour ces cas, et un I-beam reste plus lisible qu'une flèche sur du texte.
//!
//! **Mécanique.** egui 0.36 sait porter un curseur bitmap ([`egui::Context::set_cursor_image`],
//! relayé par `PlatformOutput::cursor_image`) et `egui-winit` l'applique via
//! `State::handle_platform_output_with_event_loop` (`winit::window::CustomCursor`, dédoublonné sur
//! l'identité de l'`Arc` des pixels — d'où les deux images décodées **une seule fois** ici et
//! toujours réutilisées). Ce module ne fait que choisir laquelle des deux (ou aucune) publier pour
//! la frame en cours, à la fin de `render_content::paint_content` : la décision vit donc dans le
//! code PARTAGÉ (les deux binaires, ET le harnais `overlay-testkit`, qui peut la vérifier en lisant
//! `FullOutput::platform_output.cursor_image` sans fenêtre ni GPU).
//!
//! **Pas de boucle de rendu continue dans cette architecture** (§6.1 du plan) : chaque frame en
//! mode « main » demande un redessin pile pour la prochaine bascule
//! (`Context::request_repaint_after`), que les binaires honorent via `next_redraw_at`. Hors survol
//! d'un élément cliquable, rien n'est redemandé — le curseur de repos ne coûte aucune frame.
//!
//! **Limite connue.** `CustomCursor` est en pixels physiques : sur un écran à 150 %, le curseur
//! garde ses 24 × 31 px là où le système agrandirait le sien. Le jeu n'a pas été observé à cette
//! échelle ; à revoir si un retour utilisateur le signale.

use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// Durée de la phase « éclair » (cyan) en mode main — 16 images à 30 i/s sur l'enregistrement.
pub const FLASH_DURATION: Duration = Duration::from_millis(533);
/// Durée de la phase « repos » (crème) entre deux éclairs — 16 images également (période mesurée
/// 32,6 images ≈ 1,09 s, rapport cyclique 50 %).
pub const IDLE_DURATION: Duration = Duration::from_millis(533);

const IDLE_BYTES: &[u8] = include_bytes!("../../../assets/cursor/wakfu-cursor-idle.png");
const FLASH_BYTES: &[u8] = include_bytes!("../../../assets/cursor/wakfu-cursor-flash.png");
const MOVE_BYTES: &[u8] = include_bytes!("../../../assets/cursor/wakfu-cursor-move.png");

/// Les deux bitmaps décodés, prêts à être publiés tels quels à egui (voir la doc de module pour
/// pourquoi une seule instance partagée par tout le processus).
pub struct CursorImages {
    /// Repos (crème) — aussi le curseur fixe hors de tout élément cliquable.
    pub idle: egui::CustomCursorImage,
    /// Éclair (cyan) — première phase du clignotement en mode main.
    pub flash: egui::CustomCursorImage,
    /// Croix fléchée — ce qui se déplace au glisser-déposer. Sans clignotement.
    pub moving: egui::CustomCursorImage,
}

/// Décodage paresseux, une fois par processus. Panique si un des deux PNG embarqués est illisible
/// ou vide : ce serait un asset cassé à la compilation, jamais une condition d'exécution.
pub fn images() -> &'static CursorImages {
    static IMAGES: OnceLock<CursorImages> = OnceLock::new();
    IMAGES.get_or_init(|| CursorImages {
        idle: decode(IDLE_BYTES, "wakfu-cursor-idle.png", Hotspot::ArrowTip),
        flash: decode(FLASH_BYTES, "wakfu-cursor-flash.png", Hotspot::ArrowTip),
        moving: decode(MOVE_BYTES, "wakfu-cursor-move.png", Hotspot::Center),
    })
}

/// D'où un bitmap de curseur « vise ».
///
/// Déduire la pointe de l'image (voir [`hotspot`]) ne vaut que pour une flèche : appliquée à la
/// croix fléchée, la règle donnerait le sommet de la flèche du haut, et tout ce qu'on déplacerait
/// serait décalé d'une demi-croix vers le bas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hotspot {
    /// Pointe de la flèche, déduite des pixels.
    ArrowTip,
    /// Centre du bitmap — une croix symétrique ne pointe nulle part.
    Center,
}

fn decode(bytes: &[u8], name: &str, ancrage: Hotspot) -> egui::CustomCursorImage {
    let img = image::load_from_memory(bytes)
        .unwrap_or_else(|err| panic!("assets/cursor/{name} : PNG illisible : {err}"))
        .to_rgba8();
    let (width, height) = img.dimensions();
    let rgba: std::sync::Arc<[u8]> = img.into_raw().into();
    let hotspot = match ancrage {
        Hotspot::ArrowTip => hotspot(&rgba, width)
            .unwrap_or_else(|| panic!("assets/cursor/{name} : aucun pixel opaque, pas de pointe")),
        Hotspot::Center => [(width / 2) as u16, (height / 2) as u16],
    };
    egui::CustomCursorImage {
        rgba,
        size: [
            u16::try_from(width).expect("largeur du curseur"),
            u16::try_from(height).expect("hauteur du curseur"),
        ],
        hotspot,
    }
}

/// Pointe de la flèche : pixel opaque le plus à gauche de la PREMIÈRE ligne contenant un pixel
/// opaque (`alpha > 0`), en balayant de haut en bas. Vaut pour toute flèche orientée vers le haut
/// à gauche, quelle que soit la marge transparente autour.
fn hotspot(rgba: &[u8], width: u32) -> Option<[u16; 2]> {
    let width = width as usize;
    rgba.chunks_exact(width * 4)
        .enumerate()
        .find_map(|(y, row)| {
            row.as_chunks::<4>()
                .0
                .iter()
                .position(|px| px[3] > 0)
                .map(|x| [x as u16, y as u16])
        })
}

/// Identifiant du temporaire egui portant l'instant d'ENTRÉE en mode main (voir [`apply`]) —
/// dans `Context::data`, donc par fenêtre (un `Context` par fenêtre overlay), sans état à
/// transporter dans `RenderContent`.
fn pointer_since_id() -> egui::Id {
    egui::Id::new("wakfu-cursor::pointer-since")
}

/// Choisit le curseur de CETTE frame à partir de ce que les widgets ont demandé
/// (`PlatformOutput::cursor_icon`, déjà renseigné puisque `paint_content` a fini de peindre), et
/// le publie via [`egui::Context::set_cursor_image`] — voir la doc de module pour la table de
/// correspondance. `now` est l'horloge unique de la frame (`RenderContent::now`), jamais relue ici.
///
/// En mode main, demande aussi le redessin qui fera la prochaine bascule ; hors mode main, oublie
/// l'instant d'entrée pour que le prochain survol reparte sur un éclair.
pub fn apply(ctx: &egui::Context, now: Instant) {
    let images = images();
    match ctx.output(|o| o.cursor_icon) {
        egui::CursorIcon::PointingHand => {
            let since =
                ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(pointer_since_id(), || now));
            let (image, until_next) = blink_phase(now.saturating_duration_since(since));
            ctx.set_cursor_image(Some(match image {
                BlinkImage::Flash => images.flash.clone(),
                BlinkImage::Idle => images.idle.clone(),
            }));
            ctx.request_repaint_after(until_next);
        }
        egui::CursorIcon::Default => {
            ctx.data_mut(|d| d.remove::<Instant>(pointer_since_id()));
            ctx.set_cursor_image(Some(images.idle.clone()));
        }
        // Fixe : aucun redessin réclamé, et l'instant d'entrée en mode main est oublié pour que le
        // prochain survol d'un cliquable reparte sur un éclair. Les quatre curseurs de déplacement
        // ou de saisie partagent la croix — voir la doc de module.
        egui::CursorIcon::Move
        | egui::CursorIcon::Grab
        | egui::CursorIcon::Grabbing
        | egui::CursorIcon::AllScroll => {
            ctx.data_mut(|d| d.remove::<Instant>(pointer_since_id()));
            ctx.set_cursor_image(Some(images.moving.clone()));
        }
        _ => {
            ctx.data_mut(|d| d.remove::<Instant>(pointer_since_id()));
            ctx.set_cursor_image(None);
        }
    }
}

/// Laquelle des deux images montrer en mode main.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlinkImage {
    Flash,
    Idle,
}

/// Phase du clignotement `elapsed` après l'entrée en mode main : l'image à montrer et le temps
/// restant avant la prochaine bascule (strictement positif). Pure, pour être testable sans
/// `egui::Context`.
pub fn blink_phase(elapsed: Duration) -> (BlinkImage, Duration) {
    let period = FLASH_DURATION + IDLE_DURATION;
    let phase = Duration::from_nanos((elapsed.as_nanos() % period.as_nanos()) as u64);
    if phase < FLASH_DURATION {
        (BlinkImage::Flash, FLASH_DURATION - phase)
    } else {
        (BlinkImage::Idle, period - phase)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_deux_bitmaps_se_decodent_avec_la_pointe_en_haut_a_gauche() {
        let images = images();
        for (name, image) in [("idle", &images.idle), ("flash", &images.flash)] {
            assert!(
                image.size[0] > 0 && image.size[1] > 0,
                "{name} : taille nulle"
            );
            assert_eq!(
                image.rgba.len(),
                usize::from(image.size[0]) * usize::from(image.size[1]) * 4,
                "{name} : tampon RGBA incohérent avec la taille"
            );
            // La pointe est dans le quart haut-gauche : une flèche qui pointerait ailleurs
            // trahirait un asset mal détouré (ou tourné), pas une nouvelle marge.
            assert!(
                image.hotspot[0] < image.size[0] / 2 && image.hotspot[1] < image.size[1] / 2,
                "{name} : point chaud {:?} hors du quart haut-gauche de {:?}",
                image.hotspot,
                image.size
            );
        }
        // Même dessin recoloré : les deux phases doivent se superposer exactement.
        assert_eq!(images.idle.size, images.flash.size);
        assert_eq!(images.idle.hotspot, images.flash.hotspot);
    }

    /// **La croix vise son centre**, et non la pointe d'une de ses quatre flèches : ce qu'on
    /// déplace suit le pixel qu'on a saisi, pas un point décalé d'une demi-croix.
    #[test]
    fn la_croix_flechee_est_ancree_en_son_centre() {
        let croix = &images().moving;
        assert_eq!(
            croix.hotspot,
            [croix.size[0] / 2, croix.size[1] / 2],
            "point chaud {:?} hors du centre de {:?}",
            croix.hotspot,
            croix.size
        );
        assert_eq!(
            croix.rgba.len(),
            usize::from(croix.size[0]) * usize::from(croix.size[1]) * 4,
            "tampon RGBA incohérent avec la taille"
        );
    }

    #[test]
    fn le_point_chaud_est_le_premier_pixel_opaque_en_lisant_de_haut_en_bas() {
        // 4 × 3, une seule ligne opaque (la deuxième), dont le premier pixel opaque est en x = 2.
        let mut rgba = vec![0u8; 4 * 3 * 4];
        rgba[(4 + 2) * 4 + 3] = 255;
        rgba[(4 + 3) * 4 + 3] = 255;
        rgba[(2 * 4) * 4 + 3] = 255; // plus à gauche, mais une ligne plus bas : ignoré
        assert_eq!(hotspot(&rgba, 4), Some([2, 1]));
        assert_eq!(hotspot(&vec![0u8; 4 * 3 * 4], 4), None);
    }

    #[test]
    fn le_clignotement_commence_par_l_eclair_puis_alterne_a_periode_fixe() {
        let ms = Duration::from_millis;
        assert_eq!(blink_phase(ms(0)), (BlinkImage::Flash, FLASH_DURATION));
        assert_eq!(blink_phase(ms(100)), (BlinkImage::Flash, ms(433)));
        assert_eq!(blink_phase(ms(533)), (BlinkImage::Idle, IDLE_DURATION));
        assert_eq!(blink_phase(ms(1000)), (BlinkImage::Idle, ms(66)));
        assert_eq!(blink_phase(ms(1066)), (BlinkImage::Flash, FLASH_DURATION));
        assert_eq!(blink_phase(ms(1066 * 5 + 10)), (BlinkImage::Flash, ms(523)));
    }

    /// Joue une frame egui où les widgets ont demandé `icon`, `elapsed` après l'entrée en mode
    /// main de la même fenêtre (état conservé dans `ctx.data` entre deux frames, comme en
    /// production), et renvoie l'image publiée et le délai de redessin demandé.
    fn frame(
        ctx: &egui::Context,
        t0: Instant,
        elapsed: Duration,
        icon: egui::CursorIcon,
    ) -> (Option<egui::CustomCursorImage>, Duration) {
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.ctx().set_cursor_icon(icon);
            apply(ui.ctx(), t0 + elapsed);
        });
        // Pas de GPU ici : les deltas de texture (police) sont abandonnés explicitement, sans
        // quoi `TexturesDelta` panique à sa destruction (voir `frame::render`).
        output.textures_delta.clear();
        let delay = output.viewport_output[&egui::ViewportId::ROOT].repaint_delay;
        (output.platform_output.cursor_image, delay)
    }

    /// `ViewportOutput::repaint_delay` n'est pas le délai demandé tel quel : egui en retranche
    /// sa durée de frame prédite (une image à 60 Hz, ≈ 16,7 ms) pour que le redessin ARRIVE à
    /// l'heure plutôt que de partir à l'heure — on tolère cette marge, jamais plus.
    fn assert_about(delay: Duration, requested: Duration) {
        assert!(
            delay <= requested && delay + Duration::from_millis(20) >= requested,
            "délai {delay:?} trop loin de {requested:?}"
        );
    }

    fn same(a: &Option<egui::CustomCursorImage>, b: &egui::CustomCursorImage) -> bool {
        a.as_ref()
            .is_some_and(|a| std::sync::Arc::ptr_eq(&a.rgba, &b.rgba))
    }

    #[test]
    fn la_main_fait_clignoter_et_la_fleche_reste_au_repos() {
        let ctx = egui::Context::default();
        let t0 = Instant::now();
        let ms = Duration::from_millis;
        let images = images();

        // Premières frames d'un `Context` neuf : egui réclame de lui-même des redessins
        // immédiats (polices, mise en place) — jouées à blanc jusqu'à ce qu'il n'en demande plus,
        // pour ne mesurer ensuite que ce que le curseur réclame.
        let mut delay = Duration::ZERO;
        for _ in 0..8 {
            delay = frame(&ctx, t0, ms(0), egui::CursorIcon::Default).1;
            if delay > Duration::ZERO {
                break;
            }
        }
        assert!(
            delay > Duration::ZERO,
            "egui redemande sans fin un redessin immédiat : {:?}",
            ctx.repaint_causes()
        );

        // Flèche : repos, fixe — aucun redessin réclamé pour le curseur.
        let (image, delay) = frame(&ctx, t0, ms(0), egui::CursorIcon::Default);
        assert!(same(&image, &images.idle));
        assert!(
            delay > Duration::from_secs(60),
            "délai inattendu : {delay:?}"
        );

        // Entrée en mode main : éclair d'abord, prochaine bascule dans FLASH_DURATION.
        let (image, delay) = frame(&ctx, t0, ms(100), egui::CursorIcon::PointingHand);
        assert!(same(&image, &images.flash));
        assert_about(delay, FLASH_DURATION);

        // Toujours en mode main : la phase se compte depuis l'ENTRÉE (t0 + 100 ms), pas depuis t0.
        let (image, delay) = frame(&ctx, t0, ms(100 + 533), egui::CursorIcon::PointingHand);
        assert!(same(&image, &images.idle));
        assert_about(delay, IDLE_DURATION);
        let (image, _) = frame(&ctx, t0, ms(100 + 1066), egui::CursorIcon::PointingHand);
        assert!(same(&image, &images.flash));

        // Glisser-déposer : la croix fléchée, fixe — aucun redessin réclamé, et le clignotement
        // est oublié comme pour un curseur système. Même croix pour les curseurs de saisie
        // qu'egui pose de lui-même (`Grab` au survol, `Grabbing` en vol) et pour le panoramique.
        for icon in [
            egui::CursorIcon::Move,
            egui::CursorIcon::Grab,
            egui::CursorIcon::Grabbing,
            egui::CursorIcon::AllScroll,
        ] {
            let (image, delay) = frame(&ctx, t0, ms(1500), icon);
            assert!(
                same(&image, &images.moving),
                "{icon:?} : attendu la croix fléchée"
            );
            assert!(
                delay > Duration::from_secs(60),
                "{icon:?} : la croix ne doit réclamer aucun redessin : {delay:?}"
            );
        }

        // Champ de saisie : curseur système, et l'entrée en mode main est oubliée…
        let (image, _) = frame(&ctx, t0, ms(2000), egui::CursorIcon::Text);
        assert!(image.is_none());
        // …donc un nouveau survol repart sur un éclair, quel que soit le temps écoulé.
        let (image, delay) = frame(&ctx, t0, ms(2600), egui::CursorIcon::PointingHand);
        assert!(same(&image, &images.flash));
        assert_about(delay, FLASH_DURATION);
    }
}
