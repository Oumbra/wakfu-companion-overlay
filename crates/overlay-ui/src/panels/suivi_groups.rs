//! **Groupes d'éléments suivis** (2026-09-30) — la ligne « Groupe » de l'onglet « Suivi » et le
//! brouillon qu'elle édite.
//!
//! Un groupe est une liste d'éléments suivis avec un libellé. Plusieurs groupes peuvent exister,
//! **un seul est affiché à la fois** : c'est sa liste que le bandeau montre, que le compte reçoit
//! (clé `watchlist`, modèle inchangé) et dont les compteurs avancent. Changer de groupe gèle les
//! compteurs de celui qu'on quitte et reprend ceux du groupe rejoint où ils en étaient
//! (`overlay_engine::watchlist::WatchlistState::switch_group`).
//!
//! La fonctionnalité est **optionnelle** : case « Activer les groupes » de la section « Suivi » de
//! l'onglet « Paramètres », décochée par défaut. Décochée, cette ligne n'est pas peinte et l'onglet
//! Suivi est exactement celui d'avant les groupes — il n'édite que le groupe par défaut.
//!
//! ## Ce que la ligne montre (maquette validée le 2026-09-30)
//!
//! Une ligne de réglage de 48 px (le fond et le rayon du bloc « Type de compteur ») : le libellé
//! « Groupe », une liste déroulante, puis trois boutons icône — **Renommer**, **Supprimer** (socle
//! rouge, `IconButton::danger`) et **Nouveau**, dans cet ordre. À partir de cinq groupes, la liste
//! déroulante porte un champ de recherche (`Select::searchable_from`).
//!
//! - **Le groupe par défaut n'a pas de libellé** et ne se renomme ni ne se supprime : la liste
//!   l'écrit « Groupe par défaut », un texte de l'interface qui n'est stocké nulle part, et ses deux
//!   boutons sont grisés.
//! - **Le groupe choisi est à la fois celui qu'on édite et celui qui s'affichera en jeu** à la
//!   validation. Un seul concept : ce qu'on voit dans l'onglet est ce qu'on verra sur le bandeau.
//! - **Renommer** remplace la liste par un champ texte, avec Valider (coche) et Annuler (flèche) ;
//!   `Entrée` et `Échap` font de même. Un libellé vide, trop long ou déjà pris est refusé sur place.
//! - **Nouveau** crée un groupe vide nommé « Nouveau groupe » (numéroté si besoin), le choisit, et
//!   ouvre directement son renommage.
//! - **Supprimer** retire le groupe du brouillon sans confirmation — la fenêtre est
//!   transactionnelle, « Annuler » le rattrape, comme le retrait d'une tuile (règle 3 de l'onglet).
//!
//! ## Le brouillon
//!
//! [`SuiviGroupsDraft`] porte tous les groupes, **sauf la liste du groupe choisi**, qui vit dans le
//! brouillon habituel de l'onglet (`OptionsModalState::suivi_draft`) — avec ses entrées retirées
//! (`SuiviTabState::retirees`). Ainsi tout le reste de l'onglet (formulaire, recherche, grille,
//! sélection multiple, recette) travaille sans rien savoir des groupes. Changer de groupe range la
//! liste courante dans son groupe et en sort celle du nouveau ([`SuiviGroupsDraft::select`]).

use egui::{RichText, Vec2};
use overlay_engine::{WatchlistEntry, DEFAULT_GROUP_ID};

use crate::config::{WatchlistGroupConfig, WatchlistGroupEntry, WatchlistGroupsConfig};
use crate::design::{self, DsIcon, IconContext};
use crate::engine_thread::EngineCommand;

/// Libellé du groupe par défaut dans la liste — un texte de l'interface, pas un libellé : le groupe
/// par défaut n'en a pas.
pub const DEFAULT_GROUP_TEXT: &str = "Groupe par défaut";

/// Longueur maximale d'un libellé de groupe, en caractères.
pub const LABEL_MAX_CHARS: usize = 32;

/// Nombre de groupes à partir duquel la liste déroulante porte un champ de recherche.
pub const SEARCH_FROM: usize = 5;

/// Hauteur de la ligne — la liste déroulante mesure 36 px, la ligne lui laisse 6 px de chaque côté.
const ROW_HEIGHT: f32 = 48.0;
const ROW_PAD_X: f32 = 12.0;
const ROW_FILL: egui::Color32 = egui::Color32::from_rgb(0x26, 0x28, 0x2B);
const ROW_RADIUS: u8 = 4;
const BODY_FONT_SIZE: f32 = 15.0;
/// Écart entre le libellé et la liste, puis entre la liste et les boutons.
const GAP: f32 = 12.0;
/// Écart entre deux boutons icône.
const BUTTON_GAP: f32 = 6.0;

/// Un groupe du brouillon.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupDraft {
    pub id: String,
    /// `None` pour le groupe par défaut, et pour lui seul.
    pub label: Option<String>,
    /// **Vide pour le groupe choisi** : sa liste vit dans le brouillon de l'onglet — voir la doc
    /// de module.
    pub entries: Vec<WatchlistEntry>,
    /// Les entrées retirées de ce groupe pendant l'édition — même rôle que
    /// `SuiviTabState::retirees`, qui les porte pour le groupe choisi.
    pub retirees: Vec<WatchlistEntry>,
}

impl GroupDraft {
    pub fn is_default(&self) -> bool {
        self.id == DEFAULT_GROUP_ID
    }

    /// Ce que la liste déroulante écrit pour ce groupe.
    pub fn display_label(&self) -> &str {
        self.label.as_deref().unwrap_or(DEFAULT_GROUP_TEXT)
    }
}

/// Tous les groupes, et celui qui est choisi — voir la doc de module.
#[derive(Debug, Clone, PartialEq)]
pub struct SuiviGroupsDraft {
    /// Le groupe par défaut en premier, toujours.
    pub groups: Vec<GroupDraft>,
    /// L'index du groupe choisi dans `groups`.
    pub selected: usize,
}

impl Default for SuiviGroupsDraft {
    fn default() -> Self {
        Self {
            groups: vec![GroupDraft {
                id: DEFAULT_GROUP_ID.to_string(),
                label: None,
                entries: Vec::new(),
                retirees: Vec::new(),
            }],
            selected: 0,
        }
    }
}

impl SuiviGroupsDraft {
    /// L'identifiant du groupe choisi.
    pub fn selected_id(&self) -> &str {
        &self.groups[self.selected].id
    }

    /// **Choisit un autre groupe** : la liste courante (et ses retraits) est rangée dans le groupe
    /// quitté, celle du groupe choisi en sort. Sans effet si l'index est hors bornes ou déjà choisi.
    pub fn select(
        &mut self,
        index: usize,
        entries: &mut Vec<WatchlistEntry>,
        retirees: &mut Vec<WatchlistEntry>,
    ) {
        if index == self.selected || index >= self.groups.len() {
            return;
        }
        let quitte = &mut self.groups[self.selected];
        quitte.entries = std::mem::take(entries);
        quitte.retirees = std::mem::take(retirees);
        self.selected = index;
        let choisi = &mut self.groups[index];
        *entries = std::mem::take(&mut choisi.entries);
        *retirees = std::mem::take(&mut choisi.retirees);
    }

    /// **Tous les groupes, liste du groupe choisi comprise** — ce que la validation enregistre et
    /// ce que la comparaison « la fenêtre a-t-elle changé ? » compare.
    pub fn flushed(
        &self,
        entries: &[WatchlistEntry],
        retirees: &[WatchlistEntry],
    ) -> Vec<GroupDraft> {
        let mut groups = self.groups.clone();
        groups[self.selected].entries = entries.to_vec();
        groups[self.selected].retirees = retirees.to_vec();
        groups
    }

    /// Le libellé proposé pour un nouveau groupe — « Nouveau groupe », puis « Nouveau groupe 2 »…
    fn next_label(&self) -> String {
        let pris = |label: &str| {
            self.groups.iter().any(|g| {
                g.label
                    .as_deref()
                    .is_some_and(|l| l.eq_ignore_ascii_case(label))
            })
        };
        let base = "Nouveau groupe";
        if !pris(base) {
            return base.to_string();
        }
        (2..)
            .map(|n| format!("{base} {n}"))
            .find(|label| !pris(label))
            .unwrap_or_else(|| base.to_string())
    }

    /// Un identifiant neuf — horodaté, et distinct de tous ceux du brouillon.
    fn next_id(&self) -> String {
        let base = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or_default();
        (0u128..)
            .map(|n| format!("g{:x}", base + n))
            .find(|id| self.groups.iter().all(|g| &g.id != id))
            .unwrap_or_else(|| format!("g{base:x}"))
    }

    /// Pourquoi `label` ne peut pas devenir le libellé du groupe `index` — `None` s'il le peut.
    pub fn label_error(&self, index: usize, label: &str) -> Option<&'static str> {
        let label = label.trim();
        if label.is_empty() {
            return Some("Donnez un nom au groupe.");
        }
        if label.chars().count() > LABEL_MAX_CHARS {
            return Some("32 caractères au plus.");
        }
        let pris = self.groups.iter().enumerate().any(|(i, g)| {
            i != index
                && g.label
                    .as_deref()
                    .is_some_and(|l| l.trim().to_lowercase() == label.to_lowercase())
        });
        if pris || label.to_lowercase() == DEFAULT_GROUP_TEXT.to_lowercase() {
            return Some("Un autre groupe porte déjà ce nom.");
        }
        None
    }

    /// Crée un groupe vide, le choisit, et renvoie son libellé (pour ouvrir le renommage).
    fn create(
        &mut self,
        entries: &mut Vec<WatchlistEntry>,
        retirees: &mut Vec<WatchlistEntry>,
    ) -> String {
        let label = self.next_label();
        self.groups.push(GroupDraft {
            id: self.next_id(),
            label: Some(label.clone()),
            entries: Vec::new(),
            retirees: Vec::new(),
        });
        self.select(self.groups.len() - 1, entries, retirees);
        label
    }

    /// Supprime le groupe choisi (jamais le groupe par défaut) et revient au groupe par défaut.
    fn delete_selected(
        &mut self,
        entries: &mut Vec<WatchlistEntry>,
        retirees: &mut Vec<WatchlistEntry>,
    ) {
        if self.groups[self.selected].is_default() {
            return;
        }
        let supprime = self.selected;
        self.select(0, entries, retirees);
        self.groups.remove(supprime);
    }
}

/// Ce que la ligne garde d'une frame à l'autre.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GroupRowState {
    /// Le renommage en cours — la saisie, `None` le reste du temps.
    pub rename: Option<String>,
    /// Le champ de renommage doit prendre le focus à la prochaine frame.
    pub rename_focus: bool,
}

/// Peint la ligne « Groupe » et applique ses gestes au brouillon.
pub fn row(
    ui: &mut egui::Ui,
    width: f32,
    draft: &mut SuiviGroupsDraft,
    row_state: &mut GroupRowState,
    entries: &mut Vec<WatchlistEntry>,
    retirees: &mut Vec<WatchlistEntry>,
) {
    let block = ui.allocate_space(Vec2::new(width, ROW_HEIGHT)).1;
    ui.painter().rect_filled(block, ROW_RADIUS, ROW_FILL);
    let inner = block.shrink2(Vec2::new(ROW_PAD_X, 0.0));
    let mut cell = ui.new_child(egui::UiBuilder::new().max_rect(inner));

    cell.horizontal_centered(|ui| {
        ui.label(
            RichText::new("Groupe")
                .color(egui::Color32::WHITE)
                .size(BODY_FONT_SIZE),
        );
        ui.add_space(GAP);
        // Les trois boutons finissent au bord droit de la ligne : chaque widget ajouté après le
        // libellé (la liste, puis les trois boutons) prend un `item_spacing` devant lui, les
        // `add_space` n'en prennent pas.
        let spacing = ui.spacing().item_spacing.x;
        let buttons = 3.0 * design::tokens::ICON_BUTTON_SIZE + 2.0 * BUTTON_GAP;
        let field_width =
            (inner.right() - ui.cursor().left() - 4.0 * spacing - GAP - buttons).max(120.0);

        if row_state.rename.is_some() {
            let focus = row_state_focus(row_state);
            let saisie = row_state.rename.as_mut().expect("renommage en cours");
            let issue = rename_line(ui, draft, focus, saisie, field_width);
            issue.apply(draft, row_state);
            return;
        }

        let mut choisi = draft.selected;
        let mut select = design::select(&mut choisi)
            .width(field_width)
            .searchable_from(SEARCH_FROM)
            .search_placeholder("Rechercher un groupe…")
            .empty_text("Aucun groupe ne correspond")
            .log_name("suivi.groupe");
        for (index, group) in draft.groups.iter().enumerate() {
            select = select.option(index, group.display_label());
        }
        ui.add(select);
        if choisi != draft.selected {
            draft.select(choisi, entries, retirees);
        }

        ui.add_space(GAP);
        let defaut = draft.groups[draft.selected].is_default();
        if ui
            .add(
                design::icon_button(DsIcon::Edit)
                    .context(IconContext::Panel)
                    .enabled(!defaut)
                    .tooltip(if defaut {
                        "Le groupe par défaut n'a pas de nom"
                    } else {
                        "Renommer le groupe"
                    })
                    .log_name("suivi.groupe-renommer"),
            )
            .clicked()
        {
            row_state.rename = Some(draft.groups[draft.selected].display_label().to_string());
            row_state.rename_focus = true;
        }
        ui.add_space(BUTTON_GAP);
        if ui
            .add(
                design::icon_button(DsIcon::Delete)
                    .context(IconContext::Panel)
                    .danger(true)
                    .enabled(!defaut)
                    .tooltip(if defaut {
                        "Le groupe par défaut ne se supprime pas"
                    } else {
                        "Supprimer le groupe — annulable tant que la fenêtre n'est pas validée"
                    })
                    .log_name("suivi.groupe-supprimer"),
            )
            .clicked()
        {
            draft.delete_selected(entries, retirees);
        }
        ui.add_space(BUTTON_GAP);
        if ui
            .add(
                design::icon_button(DsIcon::Plus)
                    .context(IconContext::Panel)
                    .tooltip("Nouveau groupe")
                    .log_name("suivi.groupe-nouveau"),
            )
            .clicked()
        {
            let label = draft.create(entries, retirees);
            row_state.rename = Some(label);
            row_state.rename_focus = true;
        }
    });
}

/// Consomme le drapeau « focus à la prochaine frame ».
fn row_state_focus(row_state: &mut GroupRowState) -> bool {
    std::mem::take(&mut row_state.rename_focus)
}

/// Ce que la ligne de renommage a décidé cette frame.
enum RenameOutcome {
    Continue,
    Commit(String),
    Cancel,
}

impl RenameOutcome {
    fn apply(self, draft: &mut SuiviGroupsDraft, row_state: &mut GroupRowState) {
        match self {
            RenameOutcome::Continue => {}
            RenameOutcome::Commit(label) => {
                let index = draft.selected;
                draft.groups[index].label = Some(label.trim().to_string());
                row_state.rename = None;
            }
            RenameOutcome::Cancel => row_state.rename = None,
        }
    }
}

/// Le champ de renommage, sa coche et sa flèche d'annulation.
fn rename_line(
    ui: &mut egui::Ui,
    draft: &SuiviGroupsDraft,
    focus: bool,
    saisie: &mut String,
    field_width: f32,
) -> RenameOutcome {
    let erreur = draft.label_error(draft.selected, saisie);
    // Le champ reprend la largeur de la liste ET du premier bouton : la ligne garde deux boutons,
    // alignés à droite comme les deux derniers de la ligne ordinaire.
    let largeur = field_width + design::tokens::ICON_BUTTON_SIZE + BUTTON_GAP;
    let champ = ui.add(
        design::input(saisie)
            .size(design::InputSize::Height(
                design::tokens::SELECT_HEIGHT - 8.0,
            ))
            .width(largeur)
            .error(erreur.is_some())
            .tooltip(erreur.unwrap_or("Entrée pour valider, Échap pour annuler"))
            .request_focus(focus)
            .log_name("suivi.groupe-nom"),
    );
    let entree = champ.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
    let echap = ui.input(|i| i.key_pressed(egui::Key::Escape));

    ui.add_space(GAP);
    let valider = ui
        .add(
            design::icon_button(DsIcon::Tick)
                .context(IconContext::Panel)
                .enabled(erreur.is_none())
                .tooltip(erreur.unwrap_or("Enregistrer le nom"))
                .log_name("suivi.groupe-nom-valider"),
        )
        .clicked();
    ui.add_space(BUTTON_GAP);
    let annuler = ui
        .add(
            design::icon_button(DsIcon::Undo)
                .context(IconContext::Panel)
                .tooltip("Annuler le renommage")
                .log_name("suivi.groupe-nom-annuler"),
        )
        .clicked();

    if echap || annuler {
        return RenameOutcome::Cancel;
    }
    if (valider || entree) && erreur.is_none() {
        return RenameOutcome::Commit(saisie.clone());
    }
    RenameOutcome::Continue
}

/// **Le brouillon de groupes à l'ouverture de la fenêtre Options** — tous les groupes de la
/// config, le groupe affiché choisi, et l'état initial qui sert à « la fenêtre a-t-elle changé ? »
/// (`OptionsInitial::suivi_groups`).
///
/// La liste du groupe choisi est `live` — celle du moteur, compteurs vivants compris, qui devient
/// `OptionsModalState::suivi_draft` ; celle des autres vient de la config (définitions seules). Le
/// groupe par défaut existe toujours, même avant la première recopie de la liste du compte.
pub fn open_draft(
    config: &WatchlistGroupsConfig,
    enabled: bool,
    live: Option<&[WatchlistEntry]>,
) -> (SuiviGroupsDraft, (Vec<GroupDraft>, usize)) {
    let mut draft = SuiviGroupsDraft::default();
    for groupe in &config.groups {
        let entries = groupe
            .entries
            .iter()
            .map(WatchlistGroupEntry::to_entry)
            .collect();
        if groupe.id == DEFAULT_GROUP_ID {
            draft.groups[0].entries = entries;
            continue;
        }
        draft.groups.push(GroupDraft {
            id: groupe.id.clone(),
            label: Some(groupe.label.clone().unwrap_or_else(|| groupe.id.clone())),
            entries,
            retirees: Vec::new(),
        });
    }
    let affiche = config.effective_active(enabled);
    if let Some(index) = draft.groups.iter().position(|g| g.id == affiche) {
        draft.selected = index;
        draft.groups[index].entries.clear();
    }
    let initial = live
        .map(|live| draft.flushed(live, &[]))
        .unwrap_or_default();
    let selected = draft.selected;
    (draft, (initial, selected))
}

/// Ce que la validation de l'onglet Suivi produit pour les groupes — voir [`plan_commit`].
#[derive(Default)]
pub struct GroupsCommit {
    /// Les commandes à envoyer au thread Engine, **dans cet ordre**.
    pub commands: Vec<EngineCommand>,
    /// La nouvelle config des groupes et la case « Activer les groupes », `None` si rien n'a
    /// changé (le fichier n'est alors pas réécrit).
    pub config: Option<(WatchlistGroupsConfig, bool)>,
}

/// **Ce que « Valider » fait des groupes** (2026-09-30) — partagé par les deux hôtes (`main.rs`,
/// `bin/wakfu-companion-overlay-x11.rs`), qui n'ont plus qu'à envoyer les commandes et à
/// enregistrer la config.
///
/// 1. **Le groupe vivant** (celui qui était affiché) passe par `SetWatchlistDefinitions`, comme la
///    liste unique d'avant les groupes — seulement s'il a changé : ne pas réécrire la clé du
///    compte pour rien, le « dernier écrivain gagne » du serveur ferait perdre une modification
///    faite depuis le site entre-temps.
/// 2. **Un groupe gelé édité** passe par `RedefineWatchlistGroup` : ses compteurs sont recalés sur
///    ses nouvelles définitions.
/// 3. **Un autre groupe affiché** (ou la case décochée, qui ramène au groupe par défaut) :
///    `SwitchWatchlistGroup`.
/// 4. **Les groupes supprimés** : `ForgetWatchlistGroup`, APRÈS la bascule — le moteur ne touche
///    jamais aux compteurs de son groupe vivant.
///
/// La config garde, case décochée, le dernier groupe choisi : on le retrouve en la recochant.
#[allow(clippy::too_many_arguments)]
pub fn plan_commit(
    current: &WatchlistGroupsConfig,
    current_enabled: bool,
    draft: &SuiviGroupsDraft,
    entries: &[WatchlistEntry],
    retirees: &[WatchlistEntry],
    enabled: bool,
    initial: &[GroupDraft],
) -> GroupsCommit {
    let mut commit = GroupsCommit::default();
    let groupes = draft.flushed(entries, retirees);
    let choisi = draft.selected_id().to_string();
    let affiche_avant = current.effective_active(current_enabled).to_string();
    let affiche_apres = if enabled {
        choisi.clone()
    } else {
        DEFAULT_GROUP_ID.to_string()
    };

    for groupe in &groupes {
        let avant = initial
            .iter()
            .find(|g| g.id == groupe.id)
            .map(|g| g.entries.as_slice());
        if avant == Some(groupe.entries.as_slice()) && groupe.retirees.is_empty() {
            continue;
        }
        if groupe.id == affiche_avant {
            commit
                .commands
                .push(EngineCommand::SetWatchlistDefinitions {
                    definitions: groupe.entries.clone(),
                    retirees: groupe.retirees.clone(),
                });
        } else if let Some(avant) = avant {
            commit.commands.push(EngineCommand::RedefineWatchlistGroup {
                group: groupe.id.clone(),
                before: avant.to_vec(),
                after: groupe.entries.clone(),
                retirees: groupe.retirees.clone(),
            });
        }
    }
    if affiche_apres != affiche_avant {
        let definitions = groupes
            .iter()
            .find(|g| g.id == affiche_apres)
            .map(|g| g.entries.clone())
            .unwrap_or_default();
        commit.commands.push(EngineCommand::SwitchWatchlistGroup {
            group: affiche_apres,
            definitions,
        });
    }
    for supprime in initial
        .iter()
        .filter(|initial| groupes.iter().all(|g| g.id != initial.id))
    {
        commit
            .commands
            .push(EngineCommand::ForgetWatchlistGroup(supprime.id.clone()));
    }

    let actif = if enabled {
        choisi
    } else if groupes.iter().any(|g| g.id == current.active) {
        current.active.clone()
    } else {
        DEFAULT_GROUP_ID.to_string()
    };
    let nouveaux = WatchlistGroupsConfig {
        active: actif,
        groups: groupes
            .iter()
            .map(|g| WatchlistGroupConfig {
                id: g.id.clone(),
                label: g.label.clone(),
                entries: g
                    .entries
                    .iter()
                    .map(WatchlistGroupEntry::from_entry)
                    .collect(),
            })
            .collect(),
    };
    if nouveaux != *current || enabled != current_enabled {
        commit.config = Some((nouveaux, enabled));
    }
    commit
}

#[cfg(test)]
mod tests {
    use super::*;
    use overlay_engine::{WatchlistKind, WatchlistMode};

    fn entree(name: &str) -> WatchlistEntry {
        WatchlistEntry {
            name: name.to_string(),
            kind: WatchlistKind::Item,
            mode: WatchlistMode::Up,
            count: 0,
            countdown_target: 0,
            catalog_id: None,
        }
    }

    fn brouillon() -> SuiviGroupsDraft {
        let mut draft = SuiviGroupsDraft::default();
        draft.groups.push(GroupDraft {
            id: "g1".into(),
            label: Some("Métier Paysan".into()),
            entries: vec![entree("Blé")],
            retirees: Vec::new(),
        });
        draft
    }

    #[test]
    fn choisir_un_groupe_range_la_liste_courante_et_sort_la_sienne() {
        let mut draft = brouillon();
        let mut entries = vec![entree("Bois de Frêne")];
        let mut retirees = vec![entree("Ortie")];
        draft.select(1, &mut entries, &mut retirees);
        assert_eq!(entries, vec![entree("Blé")]);
        assert!(retirees.is_empty());
        assert_eq!(draft.groups[0].entries, vec![entree("Bois de Frêne")]);
        assert_eq!(draft.groups[0].retirees, vec![entree("Ortie")]);
        let tous = draft.flushed(&entries, &retirees);
        assert_eq!(tous[1].entries, vec![entree("Blé")]);
        assert_eq!(tous[0].entries, vec![entree("Bois de Frêne")]);
    }

    #[test]
    fn creer_puis_supprimer_revient_au_groupe_par_defaut() {
        let mut draft = brouillon();
        let mut entries = vec![entree("Bois de Frêne")];
        let mut retirees = Vec::new();
        assert_eq!(draft.create(&mut entries, &mut retirees), "Nouveau groupe");
        assert_eq!(draft.selected, 2);
        assert!(entries.is_empty());
        assert_eq!(draft.next_label(), "Nouveau groupe 2");
        draft.delete_selected(&mut entries, &mut retirees);
        assert_eq!(draft.selected, 0);
        assert_eq!(draft.groups.len(), 2);
        assert_eq!(entries, vec![entree("Bois de Frêne")]);
        // Le groupe par défaut ne se supprime pas.
        draft.delete_selected(&mut entries, &mut retirees);
        assert_eq!(draft.groups.len(), 2);
    }

    #[test]
    fn un_libelle_vide_trop_long_ou_deja_pris_est_refuse() {
        let draft = brouillon();
        assert!(draft.label_error(1, "  ").is_some());
        assert!(draft.label_error(1, &"x".repeat(33)).is_some());
        assert!(draft.label_error(2, "métier paysan").is_some());
        assert!(draft.label_error(1, "Groupe par défaut").is_some());
        // Garder son propre nom est permis.
        assert!(draft.label_error(1, "Métier Paysan").is_none());
        assert!(draft.label_error(1, "Donjon Bworks").is_none());
    }

    fn config_deux_groupes() -> WatchlistGroupsConfig {
        let mut config = WatchlistGroupsConfig::default();
        config.set_definitions(DEFAULT_GROUP_ID, &[entree("Bois de Frêne")]);
        config.groups.push(WatchlistGroupConfig {
            id: "g1".into(),
            label: Some("Métier Paysan".into()),
            entries: vec![WatchlistGroupEntry::from_entry(&entree("Blé"))],
        });
        config
    }

    fn noms(commit: &GroupsCommit) -> Vec<&'static str> {
        commit
            .commands
            .iter()
            .map(|c| match c {
                EngineCommand::SetWatchlistDefinitions { .. } => "set",
                EngineCommand::RedefineWatchlistGroup { .. } => "redefine",
                EngineCommand::SwitchWatchlistGroup { .. } => "switch",
                EngineCommand::ForgetWatchlistGroup(_) => "forget",
                _ => "autre",
            })
            .collect()
    }

    #[test]
    fn valider_sans_rien_changer_n_envoie_rien() {
        let config = config_deux_groupes();
        let live = vec![entree("Bois de Frêne")];
        let (draft, (initial, _)) = open_draft(&config, true, Some(&live));
        let commit = plan_commit(&config, true, &draft, &live, &[], true, &initial);
        assert!(commit.commands.is_empty());
        assert!(commit.config.is_none());
    }

    #[test]
    fn choisir_un_autre_groupe_bascule_le_moteur_et_la_config() {
        let config = config_deux_groupes();
        let mut live = vec![entree("Bois de Frêne")];
        let mut retirees = Vec::new();
        let (mut draft, (initial, _)) = open_draft(&config, true, Some(&live));
        draft.select(1, &mut live, &mut retirees);
        let commit = plan_commit(&config, true, &draft, &live, &retirees, true, &initial);
        assert_eq!(noms(&commit), vec!["switch"]);
        let (nouvelle, active) = commit.config.expect("config changée");
        assert!(active);
        assert_eq!(nouvelle.active, "g1");
    }

    #[test]
    fn supprimer_le_groupe_affiche_bascule_avant_d_oublier() {
        let mut config = config_deux_groupes();
        config.active = "g1".into();
        let mut live = vec![entree("Blé")];
        let mut retirees = Vec::new();
        let (mut draft, (initial, _)) = open_draft(&config, true, Some(&live));
        assert_eq!(draft.selected, 1);
        draft.delete_selected(&mut live, &mut retirees);
        let commit = plan_commit(&config, true, &draft, &live, &retirees, true, &initial);
        assert_eq!(noms(&commit), vec!["switch", "forget"]);
        let (nouvelle, _) = commit.config.unwrap();
        assert_eq!(nouvelle.groups.len(), 1);
        assert_eq!(nouvelle.active, DEFAULT_GROUP_ID);
    }

    #[test]
    fn decocher_les_groupes_ramene_au_defaut_et_garde_le_dernier_choix() {
        let mut config = config_deux_groupes();
        config.active = "g1".into();
        let mut live = vec![entree("Blé")];
        let mut retirees = Vec::new();
        let (mut draft, (initial, _)) = open_draft(&config, true, Some(&live));
        // Ce que fait l'onglet quand la case est décochée : il revient au groupe par défaut.
        draft.select(0, &mut live, &mut retirees);
        let commit = plan_commit(&config, true, &draft, &live, &retirees, false, &initial);
        assert_eq!(noms(&commit), vec!["switch"]);
        let (nouvelle, active) = commit.config.unwrap();
        assert!(!active);
        assert_eq!(nouvelle.active, "g1");
        assert_eq!(nouvelle.effective_active(false), DEFAULT_GROUP_ID);
    }

    #[test]
    fn editer_un_groupe_gele_le_redefinit_sans_toucher_au_groupe_vivant() {
        let config = config_deux_groupes();
        let mut live = vec![entree("Bois de Frêne")];
        let mut retirees = Vec::new();
        let (mut draft, (initial, _)) = open_draft(&config, true, Some(&live));
        draft.select(1, &mut live, &mut retirees);
        live.push(entree("Ortie"));
        draft.select(0, &mut live, &mut retirees);
        let commit = plan_commit(&config, true, &draft, &live, &retirees, true, &initial);
        assert_eq!(noms(&commit), vec!["redefine"]);
    }
}
