//! **La fenêtre « Objets de la recette »** — celle qui décompose un objet craftable en ce qu'il
//! faut réellement récolter, et l'ajoute au Suivi en un geste.
//!
//! Ouverte par le bouton « marteau » d'une suggestion du champ d'ajout
//! ([`design::AutocompleteEntry::action`]), elle reprend la modale du web
//! (`recipe-quantity-modal.component.html`) : l'objet source en tête, la quantité voulue, puis un
//! ingrédient par ligne avec sa quantité **multipliée par cette quantité**, et un bouton
//! d'imbrication sur les lignes qui ont elles-mêmes une recette.
//!
//! ## Pas un composant nouveau — une fenêtre du design system posée plus haut
//!
//! [`design::window`] peint dans tout le `max_rect` du `Ui` qu'on lui donne : il suffit de lui en
//! donner un plus petit, dans une couche au-dessus, derrière le voile de confirmation
//! ([`design::tokens::SCRIM_ALPHA`]). C'est le seul écran du lot qui n'a rien demandé au
//! design system.
//!
//! ## Ce que « Suivre » ajoute, et ce qu'il n'ajoute pas
//!
//! Une ligne **dépliée** est remplacée par ses propres ingrédients — « suivre les ingrédients de
//! cet objet plutôt que l'objet lui-même » (`tracker.recipeNestTooltip`) — et les quantités se
//! multiplient en descendant. C'est [`overlay_engine::flatten_for_tracking`] qui tranche, pas cette
//! fenêtre : elle ne fait que lui passer les chemins dépliés.

use egui::{Color32, Rect, RichText, Vec2};

use crate::design::{self, DsIcon, IconContext, SlotFrame};
use crate::panels::options_modal::OptionsModalContext;
use crate::panels::suivi_tab::RecipeDialogState;
use crate::rarity_bridge::to_slot_rarity;

/// Texte courant — blanc, comme tout texte de corps du jeu.
const TEXT: Color32 = Color32::WHITE;
/// Gris des quantités — `#b8b9ba`, le gris unique du jeu.
const SUBDUED: Color32 = Color32::from_rgb(0xB8, 0xB9, 0xBA);
/// Fond d'une ligne — l'idiome des lignes d'aptitude du jeu.
const ROW_FILL: Color32 = Color32::from_rgb(0x26, 0x28, 0x2B);
const ROW_RADIUS: u8 = 4;
const ROW_HEIGHT: f32 = 38.0;
const FORM_ROW_HEIGHT: f32 = 40.0;
const FORM_ROW_PAD_X: f32 = 12.0;
const BODY_FONT_SIZE: f32 = 15.0;

/// Taille de la fenêtre — assez haute pour montrer l'objet source, la quantité et une recette
/// dépliée sans que la liste ne se coupe.
const DIALOG_SIZE: Vec2 = Vec2::new(500.0, 580.0);

/// Largeur de la colonne du bouton d'imbrication — **réservée même quand la ligne n'en a pas**,
/// pour que les quantités restent alignées d'une ligne à l'autre (demande du 2026-09-13). Une
/// disposition de droite à gauche, où chaque élément pousse le suivant, les décalait : l'œil compare
/// des nombres en colonne, pas des nombres qui flottent.
const NEST_COL: f32 = 32.0;
/// Largeur de la colonne de la quantité — `×10800` y tient, le pire cas observé sur le bandeau.
const QTY_COL: f32 = 62.0;
/// Retrait d'une ligne imbriquée.
const NEST_INDENT: f32 = 22.0;

/// Ce que la fenêtre rend à la frame.
pub enum RecipeChoice {
    /// Elle reste ouverte.
    Pending,
    /// Fermée sans rien suivre.
    Cancel,
    /// « Suivre » : les lignes à ajouter au brouillon, `(nom, id, quantité)`.
    Track(Vec<(String, i64, i64)>),
}

/// Peint la fenêtre et rend ce que l'utilisateur vient d'en faire.
pub fn show(
    ui: &mut egui::Ui,
    window: Rect,
    state: &mut RecipeDialogState,
    ctx: &mut OptionsModalContext<'_>,
) -> RecipeChoice {
    // Le voile et la couche au-dessus de tout sont ceux de `design::scrim` (2026-09-17) — ce
    // fichier en portait une copie à la main, sel d'identifiants compris.
    let mut nested_toggle: Option<String> = None;
    let chrome = design::scrim(window)
        .centered(DIALOG_SIZE)
        .log_name("suivi.recette")
        .show(ui, |fenetre| {
            let chrome = design::window("Objets de la recette")
                .footer("Annuler", "Suivre")
                .log_name("suivi.recette")
                .show(fenetre);
            paint_body(fenetre, chrome.content, state, ctx, &mut nested_toggle);
            chrome
        })
        .inner;

    if let Some(chemin) = nested_toggle {
        if !state.nested.remove(&chemin) {
            state.nested.insert(chemin);
        }
    }

    match chrome.footer {
        design::FooterClick::Validate => {
            let ingredients = state.ingredients.clone().unwrap_or_default();
            RecipeChoice::Track(overlay_engine::flatten_for_tracking(
                &ingredients,
                state.quantity,
                &state.nested,
            ))
        }
        design::FooterClick::Cancel => RecipeChoice::Cancel,
        design::FooterClick::None => RecipeChoice::Pending,
    }
}

/// Le contenu de la fenêtre — l'objet source, la quantité, puis les ingrédients.
fn paint_body(
    fenetre: &mut egui::Ui,
    content: Rect,
    state: &mut RecipeDialogState,
    ctx: &mut OptionsModalContext<'_>,
    nested_toggle: &mut Option<String>,
) {
    design::panel().show(fenetre, content, |ui, panel| {
        let width = panel.inner.width();

        // L'objet source — même emplacement que dans la grille, à une taille réduite : c'est un
        // rappel de ce qu'on décompose, pas le sujet de la fenêtre.
        let ligne = ui.allocate_space(Vec2::new(width, 44.0)).1;
        let mut cell = ui.new_child(egui::UiBuilder::new().max_rect(ligne));
        let rarete = ctx
            .catalog
            .find_item_rarity(&state.item_name, Some(state.item_id));
        let icone = ctx
            .catalog
            .find_item_icon(&state.item_name, Some(state.item_id))
            .and_then(|icon| {
                ctx.remote_icon_textures
                    .resolve(ui.ctx(), ctx.remote_icons, &icon)
                    .map(|handle| egui::load::SizedTexture::from_handle(&handle))
            })
            .unwrap_or_else(|| {
                egui::load::SizedTexture::from_handle(ctx.icons.unknown_entity_texture())
            });
        cell.horizontal_centered(|ui| {
            ui.add(
                design::item_slot()
                    .size(40.0)
                    .frame(SlotFrame::Rarity(to_slot_rarity(rarete)))
                    .icon(icone)
                    .log_name("suivi.recette.source"),
            );
            ui.add_space(10.0);
            ui.label(
                RichText::new(&state.item_name)
                    .color(design::tokens::TEXT_GOLD)
                    .size(BODY_FONT_SIZE),
            );
        });

        ui.add_space(10.0);
        let ligne = ui.allocate_space(Vec2::new(width, FORM_ROW_HEIGHT)).1;
        ui.painter().rect_filled(ligne, ROW_RADIUS, ROW_FILL);
        let mut cell = ui.new_child(
            egui::UiBuilder::new().max_rect(ligne.shrink2(Vec2::new(FORM_ROW_PAD_X, 0.0))),
        );
        cell.horizontal_centered(|ui| {
            ui.label(RichText::new("Quantité").color(TEXT).size(BODY_FONT_SIZE));
            let pas = design::stepper(&mut state.quantity)
                .range(1..=9999)
                .size(28.0)
                .field_width(62.0)
                .log_name("suivi.recette.quantite");
            let (largeur, _) = pas.desired_size();
            let reste = ui.available_width() - largeur.unwrap_or(0.0);
            ui.add_space(reste.max(8.0));
            ui.add(pas);
        });

        ui.add_space(14.0);
        ui.add(design::heading("Ingrédients"));

        // **Un rouage pendant la résolution.** Chaque niveau de recette demande un aller-retour
        // réseau : la fenêtre s'ouvre AVANT la réponse, comme le web, et dit qu'elle attend plutôt
        // que de montrer une liste vide qu'on prendrait pour une recette sans ingrédient.
        let Some(ingredients) = state.ingredients.clone() else {
            let reste = Rect::from_min_max(
                egui::pos2(panel.inner.left(), ui.cursor().top()),
                panel.inner.right_bottom(),
            );
            let mut zone = ui.new_child(egui::UiBuilder::new().max_rect(reste));
            zone.put(
                Rect::from_center_size(
                    reste.center(),
                    Vec2::splat(design::LoaderSize::Medium.px()),
                ),
                design::loader().size(design::LoaderSize::Medium),
            );
            return;
        };

        let quantite = state.quantity;
        let nested = state.nested.clone();
        panel.scroll_area(ui, "suivi.recette.liste", |ui, content_width| {
            paint_level(
                ui,
                ctx,
                &ingredients,
                quantite,
                &nested,
                "",
                0.0,
                content_width,
                nested_toggle,
            );
        });
    });
}

/// Peint un niveau d'ingrédients, et récursivement ceux des lignes dépliées.
#[allow(clippy::too_many_arguments)]
fn paint_level(
    ui: &mut egui::Ui,
    ctx: &mut OptionsModalContext<'_>,
    ingredients: &[overlay_engine::RecipeIngredient],
    multiplier: i64,
    nested: &std::collections::HashSet<String>,
    prefixe: &str,
    indent: f32,
    width: f32,
    toggle: &mut Option<String>,
) {
    for (rang, ingredient) in ingredients.iter().enumerate() {
        let chemin = if prefixe.is_empty() {
            rang.to_string()
        } else {
            format!("{prefixe}.{rang}")
        };
        let quantite = ingredient.quantity * multiplier;
        let ouverte = ingredient.has_recipe && nested.contains(&chemin);
        if ingredient_row(ui, ctx, width, ingredient, quantite, ouverte, indent) {
            *toggle = Some(chemin.clone());
        }
        if ouverte {
            paint_level(
                ui,
                ctx,
                &ingredient.children,
                quantite,
                nested,
                &chemin,
                indent + NEST_INDENT,
                width,
                toggle,
            );
        }
    }
}

/// Une ligne d'ingrédient : emplacement, nom, quantité, et le bouton qui déplie sa propre recette.
/// Rend `true` si ce bouton vient d'être cliqué.
#[allow(clippy::too_many_arguments)]
fn ingredient_row(
    ui: &mut egui::Ui,
    ctx: &mut OptionsModalContext<'_>,
    width: f32,
    ingredient: &overlay_engine::RecipeIngredient,
    quantity: i64,
    nested: bool,
    indent: f32,
) -> bool {
    let ligne = ui.allocate_space(Vec2::new(width, ROW_HEIGHT)).1;
    let ligne = Rect::from_min_max(egui::pos2(ligne.left() + indent, ligne.top()), ligne.max);
    ui.painter().rect_filled(ligne, ROW_RADIUS, ROW_FILL);
    let corps = ligne.shrink2(Vec2::new(8.0, 0.0));

    // Les deux colonnes de droite, posées depuis le bord : identiques d'une ligne à l'autre, que la
    // ligne porte un bouton ou non.
    let colonne_bouton =
        Rect::from_min_max(egui::pos2(corps.right() - NEST_COL, corps.top()), corps.max);
    let colonne_qte = Rect::from_min_max(
        egui::pos2(colonne_bouton.left() - QTY_COL, corps.top()),
        egui::pos2(colonne_bouton.left(), corps.bottom()),
    );

    let icone = ctx
        .catalog
        .find_item_icon(&ingredient.name, Some(ingredient.id))
        .and_then(|icon| {
            ctx.remote_icon_textures
                .resolve(ui.ctx(), ctx.remote_icons, &icon)
                .map(|handle| egui::load::SizedTexture::from_handle(&handle))
        })
        .unwrap_or_else(|| {
            egui::load::SizedTexture::from_handle(ctx.icons.unknown_entity_texture())
        });

    let mut gauche = ui.new_child(
        egui::UiBuilder::new().max_rect(Rect::from_min_max(corps.min, colonne_qte.left_bottom())),
    );
    gauche.horizontal_centered(|ui| {
        ui.add(
            design::item_slot()
                .size(28.0)
                .frame(SlotFrame::Rarity(to_slot_rarity(ingredient.rarity)))
                .icon(icone)
                .log_name(format!("suivi.recette.{}", ingredient.name)),
        );
        ui.add_space(8.0);
        ui.add(
            design::label(&ingredient.name)
                .width(ui.available_width())
                // `design::label` centre par défaut — ce qui convient au nom sous une tuile, pas à
                // une ligne de liste, où le fer à gauche est ce qui rend la colonne lisible.
                .align(egui::Align::LEFT)
                .color(TEXT)
                .log_name("suivi.recette.nom"),
        );
    });

    ui.painter().text(
        egui::pos2(colonne_qte.right() - 6.0, colonne_qte.center().y),
        egui::Align2::RIGHT_CENTER,
        format!("×{quantity}"),
        design::text::label_font(ui.ctx(), 13.0),
        SUBDUED,
    );

    let mut clique = false;
    if ingredient.has_recipe {
        let mut cell = ui.new_child(egui::UiBuilder::new().max_rect(colonne_bouton));
        let mut bouton = design::icon_button(DsIcon::Hammer)
            .context(IconContext::Panel)
            .size(24.0)
            .tooltip("Suivre les ingrédients de cet objet plutôt que l'objet lui-même")
            .log_name(format!("suivi.recette.imbriquer-{}", ingredient.name));
        if nested {
            // La ligne ouverte se lit sur son bouton, comme un onglet actif.
            bouton = bouton.preview_state(design::IconButtonState::Hovered);
        }
        clique = cell
            .put(
                Rect::from_center_size(colonne_bouton.center(), Vec2::splat(24.0)),
                bouton,
            )
            .clicked();
    }
    ui.add_space(4.0);
    clique
}
