//! Lecture du widget « Fin du tour » dans une bande d'image — la partie **pure** de la
//! surveillance de tour (§9.1 decies du plan), sans OS ni capture : elle prend des pixels, rend une
//! géométrie, un glyphe ou un score. Testée sur de vraies captures du spike S4
//! (`tests/fixtures/turn-watch/`), compilée et testée sur toutes les plateformes.
//!
//! Ce que la vidéo et le spike ont établi, et que ce module encode :
//!
//! - le widget est ancré au **coin bas-droit** de la fenêtre de jeu ;
//! - au repos, sa moitié droite est un **panneau doré** uniforme (« Fin du tour » + chrono) ; au
//!   survol d'un combattant, une carte de stats le remplace — mais **le nom, lui, ne bouge pas** :
//!   texte blanc sur fond noir, juste au-dessus du cadre, aligné à droite ;
//! - en phase de placement le même emplacement porte un bouton « Prêt », doré lui aussi.
//!
//! D'où la stratégie : **localiser une fois** la bande du nom grâce au panneau doré (il n'est pas
//! toujours là, mais il l'est au repos, et une fenêtre en arrière-plan est au repos), puis **lire
//! la bande à chaque tick** quel que soit l'état. Le nom n'est pas reconnu par OCR mais comparé à
//! un **gabarit appris** — voir `watcher.rs` pour l'apprentissage, ici seulement la comparaison.

/// Une bande RGBA8 top-down, `width * height * 4` octets — le bas d'une fenêtre de jeu, telle que
/// capturée par `capture.rs`, ou une fixture entière en test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Band {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Band {
    #[inline]
    fn px(&self, x: u32, y: u32) -> (u8, u8, u8) {
        let i = ((y * self.width + x) * 4) as usize;
        (self.rgba[i], self.rgba[i + 1], self.rgba[i + 2])
    }

    #[inline]
    fn luma(&self, x: u32, y: u32) -> u32 {
        let (r, g, b) = self.px(x, y);
        (r as u32 + g as u32 + b as u32) / 3
    }
}

/// Rectangle en pixels de bande, `x1`/`y1` exclus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x0: u32,
    pub y0: u32,
    pub x1: u32,
    pub y1: u32,
}

impl Rect {
    pub fn width(&self) -> u32 {
        self.x1 - self.x0
    }
    pub fn height(&self) -> u32 {
        self.y1 - self.y0
    }
}

/// Un pixel du panneau doré — couleur mesurée sur les captures S4 : moyenne `(200, 184, 129)`,
/// avec le sablier en filigrane et le texte « Fin du tour » plus sombres, et le chrono blanc. Les
/// tolérances gardent le fond et le filigrane, écartent le texte et le chrono ; la contrainte
/// `r > g > b` écarte les gris et le bleu nuit du cadre.
#[inline]
fn is_gold(r: u8, g: u8, b: u8) -> bool {
    (r as i32 - 214).abs() <= 28
        && (g as i32 - 195).abs() <= 28
        && (b as i32 - 138).abs() <= 34
        && r > g
        && g > b
}

/// Plus long run de `true` dans `mask`, les trous d'au plus `max_gap` comblés au préalable —
/// `(début, longueur)`. Le texte sombre « Fin du tour », le sablier en filigrane et le chrono blanc
/// creusent le panneau doré de courtes entailles (mesuré : jusqu'à 41 % de doré sur quelques
/// colonnes) qu'il ne faut pas prendre pour ses bords.
fn longest_run(mask: &[bool], max_gap: usize) -> Option<(usize, usize)> {
    let mut closed = mask.to_vec();
    let mut last_true: Option<usize> = None;
    for i in 0..mask.len() {
        if mask[i] {
            if let Some(l) = last_true {
                if i - l - 1 <= max_gap {
                    closed[l + 1..i].fill(true);
                }
            }
            last_true = Some(i);
        }
    }
    let mut best: Option<(usize, usize)> = None;
    let mut start = None;
    for (i, &m) in closed.iter().chain(std::iter::once(&false)).enumerate() {
        match (m, start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                let len = i - s;
                if best.map_or(true, |(_, l)| len > l) {
                    best = Some((s, len));
                }
                start = None;
            }
            _ => {}
        }
    }
    best
}

/// Localise le **panneau doré** du widget au repos, dans le tiers droit de la bande.
///
/// Par projections : les lignes assez dorées forment un bloc, dans ce bloc les colonnes assez
/// dorées forment un bloc — chaque fois le **plus long run contigu**, pas la boîte englobante :
/// la bordure dorée du cadre, le liseré sous le médaillon d'initiative et l'étincelle animée sont
/// dorés eux aussi, mais fins ou épars. `None` si le bloc n'a pas la forme d'un panneau (carte de
/// stats au survol, écran noir de début de combat).
///
/// Le bouton « Prêt » de la phase de placement est doré de la même façon et passe ici : c'est
/// voulu, le nom au-dessus est tout aussi lisible (voir `watcher.rs` pour ce que ça implique).
pub fn find_gold_panel(band: &Band) -> Option<Rect> {
    if band.width < 60 || band.height < 40 {
        return None;
    }
    // Zone de recherche : au plus 600 px à droite — le widget en fait ~280 à l'échelle 100 %.
    let x_from = band.width.saturating_sub(600);
    let zone_w = band.width - x_from;

    // Lignes : au moins 40 px dorés sur la zone.
    let mut row_ok = vec![false; band.height as usize];
    for y in 0..band.height {
        let mut n = 0u32;
        for x in x_from..band.width {
            let (r, g, b) = band.px(x, y);
            if is_gold(r, g, b) {
                n += 1;
            }
        }
        row_ok[y as usize] = n >= 40.min(zone_w / 2);
    }
    let (ry, rh) = longest_run(&row_ok, 6)?;
    if rh < 30 {
        return None;
    }
    let (y0, y1) = (ry as u32, (ry + rh) as u32);

    // Colonnes du bloc : au moins 35 % de la hauteur dorée — le bloc de lignes inclut encore la
    // bordure du cadre, plus large que le panneau, et le texte creuse les colonnes du panneau.
    let mut col_ok = vec![false; zone_w as usize];
    for (i, x) in (x_from..band.width).enumerate() {
        let mut n = 0u32;
        for y in y0..y1 {
            let (r, g, b) = band.px(x, y);
            if is_gold(r, g, b) {
                n += 1;
            }
        }
        col_ok[i] = n * 100 >= rh as u32 * 35;
    }
    let (cx, cw) = longest_run(&col_ok, 8)?;
    if cw < 50 {
        return None;
    }
    let (x0, x1) = (x_from + cx as u32, x_from + (cx + cw) as u32);

    // Lignes, affinées sur la largeur du panneau : au moins 45 % de la largeur dorée — coupe la
    // bordure du cadre juste au-dessus et en dessous, plus large que le panneau mais fine.
    let mut row_ok2 = vec![false; rh];
    for (i, y) in (y0..y1).enumerate() {
        let mut n = 0u32;
        for x in x0..x1 {
            let (r, g, b) = band.px(x, y);
            if is_gold(r, g, b) {
                n += 1;
            }
        }
        row_ok2[i] = n * 100 >= cw as u32 * 45;
    }
    let (ry2, rh2) = longest_run(&row_ok2, 6)?;
    if rh2 < 30 {
        return None;
    }
    let rect = Rect {
        x0,
        y0: y0 + ry2 as u32,
        x1,
        y1: y0 + (ry2 + rh2) as u32,
    };
    // Un panneau, pas un trait : ratio largeur/hauteur borné.
    let ratio = rect.width() as f32 / rect.height() as f32;
    (0.6..=2.2).contains(&ratio).then_some(rect)
}

/// Bande du nom déduite du panneau doré : juste au-dessus du cadre, alignée sur le bord droit du
/// panneau. Marges mesurées sur les captures S4 à l'échelle 100 % (nom 12 px au-dessus du cadre,
/// cadre 3 px au-dessus du panneau) et prises larges — la bande est ensuite rognée au texte, une
/// marge ne coûte rien.
pub fn name_area_above(panel: Rect, band: &Band) -> Rect {
    Rect {
        x0: panel.x1.saturating_sub(320),
        y0: panel.y0.saturating_sub(48),
        x1: (panel.x1 + 4).min(band.width),
        y1: panel.y0.saturating_sub(6),
    }
}

/// Texte binarisé et rogné — ce qu'on compare. `bits` est row-major, `w * h`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Glyph {
    pub w: u32,
    pub h: u32,
    pub bits: Vec<bool>,
}

/// Luminance au-dessus de laquelle un pixel est « texte » : le nom est blanc pur anti-aliasé sur
/// un fond de luminance ~12 (mesuré). 170 garde le corps des lettres, écarte l'anti-aliasing.
const TEXT_LUMA: u32 = 170;

/// Extrait le texte de `area` : binarise, garde le **bloc de lignes le plus dense** (le nom, pas
/// un reflet du médaillon d'initiative quelques lignes plus haut), puis, dans ces lignes, le
/// **bloc de colonnes contigu le plus à droite** (le nom est aligné à droite ; un gap de 12
/// colonnes vides le sépare de tout ce qui traîne à gauche — l'étincelle animée, typiquement).
/// `None` si rien de blanc — pas de widget, ou le tout début d'un combat.
pub fn extract_glyph(band: &Band, area: Rect) -> Option<Glyph> {
    let area = Rect {
        x0: area.x0.min(band.width),
        y0: area.y0.min(band.height),
        x1: area.x1.min(band.width),
        y1: area.y1.min(band.height),
    };
    if area.x1 <= area.x0 || area.y1 <= area.y0 {
        return None;
    }
    let w = area.width() as usize;
    let h = area.height() as usize;
    let mut bits = vec![false; w * h];
    let mut row_count = vec![0u32; h];
    for y in 0..h {
        for x in 0..w {
            let on = band.luma(area.x0 + x as u32, area.y0 + y as u32) >= TEXT_LUMA;
            bits[y * w + x] = on;
            row_count[y] += on as u32;
        }
    }
    // Lignes : parmi les blocs de lignes portant du texte (trous de 2 lignes tolérés, le corps
    // d'une lettre n'en a pas), celui qui porte le plus de pixels.
    let mut best: Option<(usize, usize, u32)> = None; // (top, bottom, pixels)
    let mut cur: Option<(usize, usize, u32)> = None;
    let mut gap = 0usize;
    for y in 0..h {
        if row_count[y] > 0 {
            cur = match cur {
                Some((t, _, n)) => Some((t, y, n + row_count[y])),
                None => Some((y, y, row_count[y])),
            };
            gap = 0;
        } else if let Some(c) = cur {
            gap += 1;
            if gap > 2 {
                if best.is_none_or(|b| c.2 > b.2) {
                    best = Some(c);
                }
                cur = None;
            }
        }
    }
    if let Some(c) = cur {
        if best.is_none_or(|b| c.2 > b.2) {
            best = Some(c);
        }
    }
    let (top, bottom, _) = best?;
    // Colonnes, dans ces lignes : depuis la droite, jusqu'au premier trou de 12 colonnes.
    let col_has = |x: usize| (top..=bottom).any(|y| bits[y * w + x]);
    let last = (0..w).rev().find(|&x| col_has(x))?;
    let mut first = last;
    let mut gap = 0;
    for x in (0..=last).rev() {
        if col_has(x) {
            first = x;
            gap = 0;
        } else {
            gap += 1;
            if gap >= 12 {
                break;
            }
        }
    }
    let gw = last - first + 1;
    let gh = bottom - top + 1;
    let mut out = Vec::with_capacity(gw * gh);
    for y in top..=bottom {
        out.extend_from_slice(&bits[y * w + first..y * w + last + 1]);
    }
    Some(Glyph {
        w: gw as u32,
        h: gh as u32,
        bits: out,
    })
}

/// Tolérance de taille entre deux glyphes comparés : au-delà, ce ne sont pas les mêmes mots.
const SIZE_TOLERANCE: u32 = 3;

/// Ressemblance entre deux glyphes, **alignés sur leur bord droit et leur bas** (texte aligné à
/// droite, ligne de base fixe) : intersection sur union des pixels allumés, dans `[0, 1]`. `0`
/// dès que les tailles diffèrent de plus de [`SIZE_TOLERANCE`] : un nom plus long n'est pas un
/// nom proche.
pub fn similarity(a: &Glyph, b: &Glyph) -> f64 {
    if a.w.abs_diff(b.w) > SIZE_TOLERANCE || a.h.abs_diff(b.h) > SIZE_TOLERANCE {
        return 0.0;
    }
    let w = a.w.max(b.w);
    let h = a.h.max(b.h);
    let get = |g: &Glyph, x: u32, y: u32| -> bool {
        // Coordonnées dans la grille commune, ancrée en bas à droite.
        let dx = w - g.w;
        let dy = h - g.h;
        if x < dx || y < dy {
            return false;
        }
        g.bits[((y - dy) * g.w + (x - dx)) as usize]
    };
    let (mut inter, mut union) = (0u32, 0u32);
    for y in 0..h {
        for x in 0..w {
            let (pa, pb) = (get(a, x, y), get(b, x, y));
            inter += (pa && pb) as u32;
            union += (pa || pb) as u32;
        }
    }
    if union == 0 {
        0.0
    } else {
        inter as f64 / union as f64
    }
}

impl Glyph {
    /// Encodage PNG 8 bits (255 = texte) — persistance des gabarits appris.
    pub fn to_png(&self) -> Result<Vec<u8>, image::ImageError> {
        let pixels: Vec<u8> = self.bits.iter().map(|&b| if b { 255 } else { 0 }).collect();
        let img = image::GrayImage::from_raw(self.w, self.h, pixels)
            .expect("dimensions cohérentes avec les bits");
        let mut out = std::io::Cursor::new(Vec::new());
        img.write_to(&mut out, image::ImageFormat::Png)?;
        Ok(out.into_inner())
    }

    pub fn from_png(bytes: &[u8]) -> Result<Self, image::ImageError> {
        let img = image::load_from_memory(bytes)?.into_luma8();
        Ok(Self {
            w: img.width(),
            h: img.height(),
            bits: img.pixels().map(|p| p.0[0] >= 128).collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Band {
        let path = format!(
            "{}/tests/fixtures/turn-watch/{name}.png",
            env!("CARGO_MANIFEST_DIR")
        );
        let img = image::open(&path)
            .unwrap_or_else(|e| panic!("fixture {path} : {e}"))
            .into_rgba8();
        Band {
            width: img.width(),
            height: img.height(),
            rgba: img.into_raw(),
        }
    }

    #[test]
    fn le_panneau_dore_est_trouve_au_repos() {
        let band = fixture("repos-oumbra");
        let panel = find_gold_panel(&band).expect("panneau doré");
        // Mesuré sur la fixture (480 × 220) : moitié droite du cadre, ~110 × 95.
        assert!((100..=130).contains(&panel.width()), "{panel:?}");
        assert!((85..=105).contains(&panel.height()), "{panel:?}");
        assert!(panel.x1 >= 465 && panel.x1 <= 480, "{panel:?}");
        assert!(panel.y1 >= 185, "{panel:?}");
    }

    #[test]
    fn le_bouton_pret_du_placement_passe_pour_un_panneau() {
        // Voulu : le nom au-dessus se lit pareil — voir doc de `find_gold_panel`.
        let band = fixture("repos-pugio-t18");
        assert!(find_gold_panel(&band).is_some());
    }

    #[test]
    fn pas_de_panneau_sur_la_carte_de_survol_ni_au_debut() {
        assert!(find_gold_panel(&fixture("carte-pugio")).is_none());
        assert!(find_gold_panel(&fixture("placement-pugio")).is_none());
    }

    #[test]
    fn le_nom_est_extrait_au_dessus_du_panneau() {
        let band = fixture("repos-oumbra");
        let panel = find_gold_panel(&band).unwrap();
        let glyph = extract_glyph(&band, name_area_above(panel, &band)).expect("un nom");
        // « Oumbra » à l'échelle 100 % : ~70 × 13 px de corps de lettres.
        assert!((55..=85).contains(&glyph.w), "{}x{}", glyph.w, glyph.h);
        assert!((10..=18).contains(&glyph.h), "{}x{}", glyph.w, glyph.h);
    }

    #[test]
    fn le_meme_nom_se_lit_pareil_au_survol_et_au_repos() {
        // Deux fenêtres, même instant, même combattant actif (« Oumbra ») : l'une au repos,
        // l'autre sur la carte de stats. La géométrie vient du repos ; la lecture doit marcher
        // sur les deux.
        let repos = fixture("repos-oumbra");
        let carte = fixture("carte-pugio");
        let area = name_area_above(find_gold_panel(&repos).unwrap(), &repos);
        let a = extract_glyph(&repos, area).unwrap();
        let b = extract_glyph(&carte, area).unwrap();
        let s = similarity(&a, &b);
        assert!(s >= 0.9, "similarité {s}");
    }

    #[test]
    fn deux_noms_differents_ne_se_ressemblent_pas() {
        let repos = fixture("repos-oumbra");
        let pret = fixture("repos-pugio-t18");
        let area = name_area_above(find_gold_panel(&repos).unwrap(), &repos);
        let oumbra = extract_glyph(&repos, area).unwrap();
        let pugio = extract_glyph(
            &pret,
            name_area_above(find_gold_panel(&pret).unwrap(), &pret),
        )
        .unwrap();
        let s = similarity(&oumbra, &pugio);
        assert!(s < 0.2, "similarité {s}");
    }

    #[test]
    fn rien_a_lire_sur_l_ecran_noir_du_debut() {
        let band = fixture("placement-pugio");
        let area = Rect {
            x0: 160,
            y0: 50,
            x1: 480,
            y1: 95,
        };
        assert!(extract_glyph(&band, area).is_none());
    }

    #[test]
    fn un_glyphe_survit_a_l_aller_retour_png() {
        let band = fixture("repos-oumbra");
        let area = name_area_above(find_gold_panel(&band).unwrap(), &band);
        let g = extract_glyph(&band, area).unwrap();
        let back = Glyph::from_png(&g.to_png().unwrap()).unwrap();
        assert_eq!(g, back);
    }

    #[test]
    fn la_similarite_est_bornee_par_la_taille() {
        let a = Glyph {
            w: 10,
            h: 10,
            bits: vec![true; 100],
        };
        let b = Glyph {
            w: 20,
            h: 10,
            bits: vec![true; 200],
        };
        assert_eq!(similarity(&a, &b), 0.0);
        assert_eq!(similarity(&a, &a), 1.0);
    }
}
