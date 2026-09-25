//! **Onglet « À propos » de la fenêtre Options** — ce qui concerne le programme lui-même, et non
//! ce qu'il affiche : ce qu'il est, ce qu'il fait de vos données, sa mise à jour, son redémarrage,
//! son arrêt.
//!
//! Créé le 2026-09-18, en **dernière** position du menu (demande utilisateur : « ajouter l'onglet
//! "À propos" en dernier et y déplacer la section Mise à jour, les boutons "Redémarrer" et "Fermer
//! l'overlay" »). Les trois vivaient jusque-là au pied de l'onglet « Paramètres », après « Compte »
//! — un onglet qui avait grossi jusqu'à huit sections défilantes, où les sorties du programme se
//! trouvaient sous les réglages de notifications, à un défilement de distance. Ni la mise à jour
//! ni les sorties ne sont des réglages : elles ne passent pas par « Valider », à l'exception de la
//! case d'installation automatique, brouillon comme les autres (voir
//! `panels::options_modal::OptionsModalState::auto_update`).
//!
//! ## Les trois sections d'information (2026-09-18, le soir)
//!
//! Demande utilisateur : « alimenter l'onglet "À propos" avec les informations nécessaires par
//! rapport au RGPD, aux CGU Wakfu et aux informations importantes du site/overlay, avant la
//! section "Mise à jour" ». Elles répondent à deux constats du jour même —
//! [`docs/analyse-rgpd.md`](../../../../docs/analyse-rgpd.md) C4 (aucune information des
//! personnes dans l'application) et [`docs/analyse-cgu.md`](../../../../docs/analyse-cgu.md) §3.5
//! (aucune mention de non-affiliation à Ankama dans l'interface) — et **décrivent
//! ce que le code fait, pas ce qu'on voudrait qu'il fasse** : la frappe de commandes dans le chat
//! et la lecture d'image de la notification de tour y sont dites, parce qu'elles existent
//! (analyse CGU §3.1 et §3.2). Un texte qui les tairait serait faux, et c'est exactement le
//! reproche fait au §10 du plan d'architecture.
//!
//! 1. **« Wakfu Companion »** — ce qu'est le programme : un projet de fan, non affilié à Ankama,
//!    dont les éléments du jeu restent la propriété d'Ankama, puis la mention de droits d'auteur
//!    que la licence d'utilisation des données Wakfu exige ([`copyright_notice`], seul bloc
//!    calculé à l'exécution : il porte l'année en cours). Liens : le site, le code source.
//! 2. **« Conditions d'utilisation de Wakfu »** — ce que l'overlay fait et ne fait pas vis-à-vis du
//!    client de jeu, ce que les CGU d'Ankama en disent, et à qui revient l'appréciation du risque
//!    (au joueur : c'est son compte). Lien : les CGU sur wakfu.com.
//! 3. **« Vos données »** — ce qui est lu, ce qui part au compte et à quels tiers, ce qui reste
//!    sur la machine et où, comment exercer ses droits. Liens : politique de confidentialité et
//!    conditions d'utilisation du service, sur le site.
//!
//! Les textes sont ceux de [`SECTIONS`] — des constantes, pour que le contenu se relise et se
//! teste sans peindre — et chaque bloc est un `design::info_text`, la façon dont le jeu commente
//! sans en faire un contrôle. Un seul bloc est en ton `Alert` : l'avertissement de responsabilité
//! sur les CGU, qui est la seule phrase de l'onglet dont l'ignorance a un prix.
//!
//! **Tout l'onglet défile** (`PanelZones::scroll_area`, comme « Paramètres ») : les trois sections
//! ne tiennent pas dans le cadre avec « Mise à jour » et les sorties, et agrandir la fenêtre pour
//! suivre le texte n'est pas une option — c'est une fenêtre posée par-dessus un jeu.
//!
//! La version courante n'est **pas** rappelée ici : la bannière de la fenêtre la porte déjà, à
//! gauche (`design::window`, `.version(true)`), et c'est le seul endroit où elle a à l'être.
//!
//! ## Ce que l'onglet ne fait pas lui-même
//!
//! Il ne confirme rien et n'agit sur rien : il **remonte une intention** ([`AProposTabAction`]) à
//! la fenêtre Options, qui ouvre la confirmation qui convient (installation, redémarrage, arrêt —
//! toutes en `design::confirm_dialog` sur la fenêtre entière) et, sur « Oui », passe l'action à
//! l'hôte. Un lien remonte de même (`OpenUrl`) : **aucun panneau n'ouvre le navigateur lui-même**,
//! les captures de non-régression cliquent réellement sur les boutons (voir
//! `render_content::RenderOutcome::open_url`). Les intentions s'excluent par construction chez
//! l'appelant ; l'onglet n'en produit qu'une par frame.

use overlay_sync::update::{self, UpdateStatus};

use crate::design::{self, ButtonSize, ButtonVariant, InfoTone, PanelZones};

/// Hauteur d'une ligne de contrôle — la même que dans `panels::options_modal` (36 px, relevé).
const ROW_HEIGHT: f32 = 36.0;
/// Écart entre la ligne d'information et la case qui la suit — voir
/// `panels::options_modal::INFO_GAP`. Sert aussi d'écart entre deux blocs d'information d'une
/// même section : la pastille de chacun suffit à les distinguer, un écart de section les
/// éloignerait comme deux sujets.
const INFO_GAP: f32 = 9.0;
/// Air entre la fin d'une section et ce qui suit — voir `panels::options_modal::SECTION_GAP`.
const SECTION_GAP: f32 = 17.0;

/// Dépôt du code source de l'overlay — le dépôt est public, le lien l'assume.
pub const SOURCE_URL: &str = "https://github.com/Oumbra/wakfu-companion-overlay";
/// Les Conditions Générales d'Utilisation d'Ankama (voir `docs/analyse-cgu.md` §1). La page est
/// derrière une redirection SSO : un navigateur la suit sans peine, c'est bien là qu'elle vit.
pub const WAKFU_TERMS_URL: &str = "https://www.wakfu.com/fr/cgu";
/// Adresse de contact du service — celle des mentions légales du site.
pub const CONTACT_EMAIL: &str = "contact@wakfu-companion.com";

/// Le site — la même origine que l'API (`overlay_sync::client::base_url`), donc prod pour un
/// binaire de Release et le déploiement dev pour tout autre profil : un lien de l'overlay ouvre
/// le site auquel il parle, jamais un autre.
pub fn site_url() -> String {
    overlay_sync::client::base_url()
        .trim_end_matches('/')
        .to_string()
}

/// La politique de confidentialité du service. Le chemin sans préfixe de langue
/// (`/privacy-policy`) est celui que le site redirige vers la langue du visiteur
/// (`app.routes.ts`, dépôt `wakfu-companion`) ; il est servi en lien direct par le `_redirects`
/// de Cloudflare Pages.
pub fn privacy_policy_url() -> String {
    format!("{}/privacy-policy", site_url())
}

/// Les conditions générales d'utilisation du service — même mécanique que
/// [`privacy_policy_url`].
pub fn terms_of_service_url() -> String {
    format!("{}/terms-of-service", site_url())
}

/// Un bloc d'information d'une section : son texte et son ton.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InfoBlock {
    pub text: &'static str,
    pub tone: InfoTone,
}

const fn info(text: &'static str) -> InfoBlock {
    InfoBlock {
        text,
        tone: InfoTone::Info,
    }
}

const fn alert(text: &'static str) -> InfoBlock {
    InfoBlock {
        text,
        tone: InfoTone::Alert,
    }
}

/// Un lien d'une section : un bouton secondaire qui remonte son URL à l'hôte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Link {
    Site,
    Source,
    WakfuTerms,
    PrivacyPolicy,
    TermsOfService,
}

impl Link {
    pub fn label(self) -> &'static str {
        match self {
            Link::Site => "Site web",
            Link::Source => "Code source",
            Link::WakfuTerms => "CGU de Wakfu",
            Link::PrivacyPolicy => "Politique de confidentialité",
            Link::TermsOfService => "Conditions d'utilisation",
        }
    }

    pub fn tooltip(self) -> &'static str {
        match self {
            Link::Site => "Ouvrir wakfu-companion.com dans le navigateur",
            Link::Source => "Ouvrir le dépôt GitHub de l'overlay dans le navigateur",
            Link::WakfuTerms => {
                "Ouvrir les Conditions Générales d'Utilisation d'Ankama sur wakfu.com"
            }
            Link::PrivacyPolicy => {
                "Ouvrir la politique de confidentialité du service dans le navigateur"
            }
            Link::TermsOfService => {
                "Ouvrir les conditions générales d'utilisation du service dans le navigateur"
            }
        }
    }

    pub fn url(self) -> String {
        match self {
            Link::Site => site_url(),
            Link::Source => SOURCE_URL.to_string(),
            Link::WakfuTerms => WAKFU_TERMS_URL.to_string(),
            Link::PrivacyPolicy => privacy_policy_url(),
            Link::TermsOfService => terms_of_service_url(),
        }
    }

    fn log_name(self) -> &'static str {
        match self {
            Link::Site => "a-propos-lien-site",
            Link::Source => "a-propos-lien-source",
            Link::WakfuTerms => "a-propos-lien-cgu-wakfu",
            Link::PrivacyPolicy => "a-propos-lien-confidentialite",
            Link::TermsOfService => "a-propos-lien-conditions",
        }
    }
}

/// Une section d'information : un titre, ses blocs, ses liens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Section {
    pub title: &'static str,
    pub blocks: &'static [InfoBlock],
    pub links: &'static [Link],
}

/// **Les trois sections d'information, dans l'ordre d'affichage.** Chaque phrase y décrit un
/// comportement vérifié du code au 2026-09-25 (audit des textes RGPD/CGU contre le code ;
/// références dans `docs/analyse-cgu.md` §3-4 et, pour le RGPD, dans l'analyse complète retirée du
/// dépôt, `efcb787:docs/analyse-rgpd.md` §2) ; **une fonctionnalité qui change ce que l'overlay lit, envoie ou
/// conserve doit changer ce texte dans le même commit** — un texte d'information périmé est pire
/// que pas de texte.
pub const SECTIONS: &[Section] = &[
    Section {
        title: WAKFU_COMPANION_TITLE,
        blocks: &[
            info(
                "Wakfu Companion est un overlay non officiel pour Wakfu : un projet de fan, \
                 gratuit et sans but lucratif, qui n'est ni édité, ni hébergé, ni approuvé par \
                 Ankama et n'a aucun lien avec cette société.",
            ),
            info(
                "WAKFU est une marque d'Ankama. Les noms, images, icônes et éléments d'interface \
                 du jeu affichés ici restent la propriété d'Ankama et ne servent qu'à \
                 illustrer ; tout contenu lui appartenant est retiré sur simple demande de sa part.",
            ),
        ],
        links: &[Link::Site, Link::Source],
    },
    Section {
        title: "Conditions d'utilisation de Wakfu",
        blocks: &[
            info(
                "L'overlay lit le fichier wakfu.log que le jeu écrit lui-même et affiche ce qu'il \
                 en tire par-dessus la fenêtre du jeu. Il ne se connecte jamais aux serveurs \
                 d'Ankama, ne lit pas la mémoire du client, ne modifie aucun de ses fichiers et \
                 ne joue jamais à votre place.",
            ),
            info(
                "Trois fonctions vont plus loin. Les raccourcis Inviter et Suivre (F1 et F2 par \
                 défaut, désactivables dans l'onglet Raccourcis) et la réponse à une alerte de \
                 chat tapent une commande dans le chat du jeu à votre place. La notification de \
                 tour (Windows, désactivée par défaut) lit l'image de la fenêtre du jeu pendant \
                 un combat pour y reconnaître votre nom. Rien de tout cela ne quitte votre \
                 ordinateur.",
            ),
            alert(
                "Les CGU d'Ankama interdisent les bots, les logiciels d'automatisation et tout \
                 programme non autorisé s'exécutant avec le jeu, et n'autorisent aucun outil \
                 tiers par défaut. Utiliser cet overlay relève de votre appréciation et de votre \
                 seule responsabilité : c'est votre compte de jeu qui est en jeu.",
            ),
        ],
        links: &[Link::WakfuTerms],
    },
    Section {
        title: VOS_DONNEES_TITLE,
        blocks: &[
            info(
                "Seul wakfu.log est lu, sur cet ordinateur. Il contient aussi les noms d'autres \
                 joueurs et les messages des canaux publics : ces messages ne sont jamais envoyés \
                 ni enregistrés, et le fichier lui-même n'est jamais téléversé. L'overlay ne \
                 connaît ni votre e-mail, ni votre mot de passe.",
            ),
            info(
                "Une fois un compte appairé, l'overlay envoie à wakfu-companion.com ce que le \
                 site enregistrerait à votre place : vos combats (pour chaque participant : nom, \
                 classe, dégâts, soins, armure et sorts ; pour le combat : durée, butin, \
                 expérience, kamas et serveur), vos achats et vos ventes récupérées à l'Hôtel \
                 des ventes, vos échanges (avec le nom du partenaire), vos extractions de pacte, \
                 vos personnages, votre Suivi, vos alertes et vos recherches de chat. Ce que \
                 wakfu.log contient déjà au lancement part au moment de l'appairage. Avant \
                 l'appairage, seule la liste publique des serveurs de jeu est demandée, sans \
                 rien qui vous concerne.",
            ),
            info(
                "Un seul service tiers est contacté : GitHub, à chaque lancement, pour vérifier \
                 s'il existe une nouvelle version — il voit votre adresse IP, comme tout site \
                 consulté, sans identifiant ni numéro de version. Les icônes d'objets, de \
                 monstres et de sorts sont servies par wakfu-companion.com.",
            ),
            info(
                "Sur cet ordinateur, l'overlay conserve sa configuration, le jeton de session \
                 (dans le trousseau du système quand il en existe un), les combats en cours, la \
                 file d'envoi, un journal technique de vos 14 derniers jours d'utilisation, ses \
                 caches et, si la notification de tour est active, l'image du nom de vos \
                 personnages — sous %APPDATA% (Windows) ou ~/.config et ~/.local/share (Linux). \
                 Sous Windows, il inscrit aussi dans le registre ce qu'il faut à ses \
                 notifications. Se déconnecter efface le jeton — ici, et sur le serveur si la \
                 connexion le permet —, les combats en cours, l'historique pas encore envoyé, \
                 les compteurs de suivi, l'image des noms et le contenu du journal ; « Supprimer \
                 les données locales », ci-dessous, efface tout le reste puis ferme l'overlay.",
            ),
            info(
                "Vos droits d'accès, de rectification, d'effacement et de portabilité s'exercent \
                 depuis la page « Mon compte » du site (export et suppression du compte) ou par \
                 courriel à contact@wakfu-companion.com. La politique de confidentialité et les \
                 conditions d'utilisation du service détaillent le reste.",
            ),
        ],
        links: &[Link::PrivacyPolicy, Link::TermsOfService],
    },
];

/// Titre de la section sous laquelle se glissent [`api_override_notice`] et le bloc
/// « Supprimer les données locales » ([`PURGE_INFO`]).
pub const VOS_DONNEES_TITLE: &str = "Vos données";

/// Le bloc d'alerte qui précède le bouton « Supprimer les données locales » — ce que « tout »
/// recouvre, et ce qui reste (le compte, sur le site). Sorti en constante pour être vérifiable
/// par un test, comme les blocs de [`SECTIONS`].
///
/// **Sous « Vos données » depuis le 2026-09-21** (demande utilisateur : « déplace le bloc
/// d'information "Supprimer les données locales" et son bouton associé en dessous de la section
/// "Vos données" »). Il fermait jusque-là la section « Compte » de l'onglet « Paramètres », à
/// côté de « Se déconnecter » — mais ce n'est pas un réglage de compte : c'est le droit à
/// l'effacement (RGPD art. 17, constat C5 de `docs/analyse-rgpd.md`), et le bloc qui dit ce
/// que l'overlay conserve sur la machine est ici. Le geste et son explication se lisent au même
/// endroit.
pub const PURGE_INFO: &str = "« Supprimer les données locales » efface de cet ordinateur ce que \
                              l'overlay y a écrit : réglages, combats en cours, file d'envoi, \
                              gabarits de tour, journaux, caches, session enregistrée, \
                              inscription au démarrage de l'ordinateur et entrées du registre \
                              (Windows). L'overlay se ferme ensuite, et repart comme une \
                              installation neuve — votre compte et son historique, eux, restent \
                              sur le site.";

/// **Ce qu'efface « Se déconnecter »** (2026-09-25, audit des textes contre le code) — partagé par
/// la section « Compte » de la fenêtre Options, celle de la Carte et la confirmation de la Carte.
/// Ces textes ne parlaient que de la session, alors que la déconnexion vide aussi la file d'envoi
/// (`background.rs`, `SyncCommand::Deactivate`) : l'historique pas encore transmis est perdu, et
/// c'est la seule conséquence qu'on ne rattrape pas en se reconnectant. Même liste que le bloc
/// « Sur cet ordinateur » de [`SECTIONS`] (`local_data::purge`, `Scope::OnDisconnect`).
pub const DISCONNECT_INFO: &str = "Se déconnecter efface la session enregistrée — ici, et sur le \
                                   serveur si la connexion le permet —, les combats en cours, \
                                   l'historique pas encore envoyé, les compteurs de suivi, \
                                   l'image des noms et le contenu du journal.";

/// Titre de la première section — celle sous laquelle se peint [`copyright_notice`].
pub const WAKFU_COMPANION_TITLE: &str = "Wakfu Companion";

/// La mention de droits d'auteur que la **licence d'utilisation des données Wakfu** d'Ankama
/// (v1 du 2019-03-11, `docs/analyse-cgu.md` §1) impose à tout projet qui exploite ses données de
/// jeu : « WAKFU MMORPG : © 2012-[année en cours] Ankama Studio. Tous droits réservés. » Le
/// catalogue d'objets et de monstres (noms, raretés, catégories — `overlay_engine::catalog`) en
/// descend, donc la mention est due, au mot près, à la première place où l'overlay parle de
/// lui-même. L'année vient de [`copyright_year`].
pub fn copyright_notice(year: i32) -> String {
    format!("WAKFU MMORPG : © 2012-{year} Ankama Studio. Tous droits réservés.")
}

/// L'année de la mention : celle de l'horloge locale — sauf sous le gel des captures
/// (`build_info::freeze_for_snapshots`), où elle vaut [`FROZEN_COPYRIGHT_YEAR`] pour que les
/// références de l'onglet ne changent pas au 1er janvier sans qu'on l'ait demandé.
pub fn copyright_year() -> i32 {
    use chrono::Datelike;
    if crate::build_info::is_frozen() {
        FROZEN_COPYRIGHT_YEAR
    } else {
        chrono::Local::now().year()
    }
}

/// L'année peinte dans les captures de non-régression — voir [`copyright_year`].
pub const FROZEN_COPYRIGHT_YEAR: i32 = 2026;

/// Le bloc d'alerte « À propos » quand l'origine de l'API est surchargée par l'environnement
/// (`overlay_sync::client::base_url_override`) — `None` sinon, et rien n'est affiché : le cas
/// normal ne mérite pas une ligne.
pub fn api_override_notice(api_override: Option<&str>) -> Option<String> {
    api_override.map(|origin| {
        format!(
            "Cet overlay n'envoie pas ses données à wakfu-companion.com mais à {origin} : \
             l'origine du serveur est surchargée par la variable d'environnement \
             WAKFU_COMPANION_API_URL. Si vous ne l'avez pas posée vous-même, retirez-la."
        )
    })
}

/// Ce que l'onglet reçoit de la fenêtre Options.
pub struct AProposTabContext<'a> {
    /// Où en est la mise à jour automatique — copié par l'hôte avant chaque rendu, jamais figé à
    /// l'ouverture (voir `OptionsModalState::update`).
    pub update: &'a UpdateStatus,
    /// La case « Installer automatiquement les mises à jour au démarrage » — un brouillon, que
    /// « Valider » écrit et qu'« Annuler » abandonne.
    pub auto_update: &'a mut bool,
    /// L'origine d'API surchargée par l'environnement, s'il y en a une
    /// (`overlay_sync::client::base_url_override`, lue par l'hôte) — voir [`api_override_notice`].
    pub api_override: Option<String>,
}

/// L'intention que l'onglet remonte à la fenêtre Options, au plus une par frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AProposTabAction {
    None,
    /// Un lien d'une section d'information : l'hôte ouvre cette URL dans le navigateur.
    OpenUrl(String),
    /// « Recherche de mise à jour » / « Réessayer » : lire la dernière version publiée, sans rien
    /// installer.
    CheckUpdate,
    /// « Mettre à jour vers X » : demander l'installation de cette version — à confirmer, elle
    /// ferme l'overlay.
    Install(String),
    /// « Redémarrer l'overlay » — à confirmer.
    Restart,
    /// « Fermer l'overlay » — à confirmer.
    Quit,
    /// « Supprimer les données locales » — à confirmer : l'hôte efface tout ce que l'overlay a
    /// écrit sur cet ordinateur, puis ferme (voir `OptionsModalAction::PurgeLocalData`).
    PurgeLocalData,
}

pub fn show(
    ui: &mut egui::Ui,
    panel: &PanelZones,
    ctx: &mut AProposTabContext<'_>,
) -> AProposTabAction {
    let mut action = AProposTabAction::None;

    // **Tout l'onglet défile** : voir la doc de module. La largeur utile vient de la zone
    // défilable, réserve de barre déjà déduite (voir `design::PanelZones::scroll_area`).
    panel.scroll_area(ui, "options-a-propos", |ui, inner_width| {
        // **Les trois sections d'information** (voir la doc de module et [`SECTIONS`]), avant
        // « Mise à jour » (demande utilisateur) : ce qu'est le programme se lit avant ce qu'il
        // devient. Même grammaire que les sections de « Paramètres » — un titre, des blocs
        // d'information, puis les contrôles sur l'axe des contrôles.
        for (index, section) in SECTIONS.iter().enumerate() {
            if index > 0 {
                ui.add_space(SECTION_GAP);
            }
            ui.add(design::heading(section.title));
            for (block_index, block) in section.blocks.iter().enumerate() {
                if block_index > 0 {
                    ui.add_space(INFO_GAP);
                }
                ui.add(
                    design::info_text(block.text)
                        .tone(block.tone)
                        .width(inner_width)
                        .log_name(format!("a-propos-{}-{}", index + 1, block_index + 1)),
                );
            }
            // **Mention de droits d'auteur** (licence des données Wakfu, 2026-09-21) : sous les
            // blocs de « Wakfu Companion », en ton `Info` comme eux — c'est une information due,
            // pas un avertissement. Calculée ici et non dans `SECTIONS` parce qu'elle porte
            // l'année en cours (voir `copyright_year`).
            if section.title == WAKFU_COMPANION_TITLE {
                ui.add_space(INFO_GAP);
                ui.add(
                    design::info_text(copyright_notice(copyright_year()))
                        .tone(InfoTone::Info)
                        .width(inner_width)
                        .log_name("a-propos-droits-ankama"),
                );
            }
            // **Origine d'API surchargée** (constat C16 de `docs/analyse-rgpd.md`, 2026-09-19) :
            // quand `WAKFU_COMPANION_API_URL` est posée, « wakfu-companion.com » dans le bloc
            // ci-dessus n'est plus vrai — l'utilisateur doit voir où partent ses données, pas
            // seulement le journal. Bloc d'alerte sous la section « Vos données », jamais ailleurs.
            if section.title == VOS_DONNEES_TITLE {
                if let Some(text) = api_override_notice(ctx.api_override.as_deref()) {
                    ui.add_space(INFO_GAP);
                    ui.add(
                        design::info_text(text)
                            .tone(InfoTone::Alert)
                            .width(inner_width)
                            .log_name("a-propos-api-surchargee"),
                    );
                }
            }
            ui.add_space(INFO_GAP);
            if let Some(url) = link_row(ui, inner_width, section.links) {
                action = AProposTabAction::OpenUrl(url);
            }
            // **« Supprimer les données locales »**, sous les liens de « Vos données » — voir
            // [`PURGE_INFO`] pour le pourquoi de la place. Le bloc d'information est en ton
            // `Alert` comme l'avertissement CGU : c'est l'autre phrase de l'onglet dont l'ignorance
            // a un prix.
            //
            // **Actif même sans compte lié** : c'est exactement l'état où le droit à l'effacement
            // s'exerce — après s'être déconnecté ; et une fois la fenêtre de connexion seule à
            // l'écran, elle porte le même geste (`panels::login`).
            //
            // Rouge et centré comme « Fermer l'overlay », plus bas, et pour la même raison en plus
            // forte : c'est la seule action de toute la fenêtre qui ne laisse RIEN derrière elle.
            // La confirmation, chez l'appelant, nomme les deux conséquences dans l'ordre.
            if section.title == VOS_DONNEES_TITLE {
                ui.add_space(INFO_GAP);
                ui.add(
                    design::info_text(PURGE_INFO)
                        .tone(InfoTone::Alert)
                        .width(inner_width)
                        .log_name("options-compte-effacement"),
                );
                ui.add_space(INFO_GAP);
                let purge = design::button("Supprimer les données locales")
                    .variant(ButtonVariant::Danger)
                    .size(ButtonSize::Height(ROW_HEIGHT))
                    .tooltip(
                        "Effacer de cet ordinateur tout ce que l'overlay y a écrit, puis fermer",
                    )
                    .log_name("options-effacer-donnees");
                let purge_size = purge.desired_size(ui);
                let purge_row = ui.allocate_space(egui::vec2(inner_width, ROW_HEIGHT)).1;
                if ui
                    .put(
                        egui::Rect::from_center_size(purge_row.center(), purge_size),
                        purge,
                    )
                    .clicked()
                {
                    action = AProposTabAction::PurgeLocalData;
                }
            }
        }

        // **Section « Mise à jour »** (2026-09-15, `docs/plan-mise-a-jour.md` §8.2, décisions du
        // mainteneur) : une ligne d'information (dernière vérification, version disponible et son
        // poids), la case d'installation automatique, et UN bouton dont le libellé suit l'état :
        // « Recherche de mise à jour » → « Recherche… » → « Mettre à jour vers X » / « Réessayer ».
        // Pas de bouton « Notes de version » pour l'instant (aucune note n'est rédigée aujourd'hui).
        //
        // « Mettre à jour » n'est PAS un brouillon : il ferme l'overlay de jeu le temps de
        // l'installation — d'où sa confirmation, chez l'appelant. « Recherche », lui, ne touche à rien.
        ui.add_space(SECTION_GAP);
        ui.add(design::heading("Mise à jour"));
        let (info, tone) = update_info_line(ctx.update, std::time::Instant::now());
        ui.add(
            design::info_text(info)
                .tone(tone)
                .width(inner_width)
                .log_name("options-mise-a-jour-info"),
        );
        ui.add_space(INFO_GAP);
        ui.add(
            design::checkbox(
                ctx.auto_update,
                "Installer automatiquement les mises à jour au démarrage",
            )
            .tooltip(
                "Au lancement, une version plus récente est téléchargée et installée avant \
                 d'ouvrir l'overlay. Décochée, elle est seulement signalée ici.",
            )
            .log_name("options-mise-a-jour-auto"),
        );
        ui.add_space(design::tokens::CHECKBOX_ROW_GAP);
        let button_spec = update_button(ctx.update);
        let update_button = design::button(button_spec.label)
            .variant(button_spec.variant)
            .size(ButtonSize::Height(ROW_HEIGHT))
            .enabled(button_spec.enabled)
            .tooltip(button_spec.tooltip)
            .log_name("options-mise-a-jour-bouton");
        let update_size = update_button.desired_size(ui);
        let row = ui.allocate_space(egui::vec2(inner_width, ROW_HEIGHT)).1;
        if ui
            .put(
                egui::Rect::from_center_size(row.center(), update_size),
                update_button,
            )
            .clicked()
        {
            action = match ctx.update {
                UpdateStatus::Available { version, .. } => {
                    AProposTabAction::Install(version.clone())
                }
                _ => AProposTabAction::CheckUpdate,
            };
        }

        // **La section « Journal » a vécu ici du 2026-09-18 au 2026-09-21**, entre « Mise à
        // jour » et les sorties, au motif que « Vos données » parlait déjà de ce que l'overlay
        // écrit sur la machine. Elle est repartie dans « Paramètres », sous « Fichier » (demande
        // utilisateur) : sa case est un réglage, brouillon comme les autres, et l'onglet garde
        // ce qui n'en est pas — voir `panels::options_modal`.

        // **« Redémarrer l'overlay » et « Fermer l'overlay »** (2026-09-16 pour la sortie, 2026-09-17
        // pour le redémarrage, ici depuis le 2026-09-18), sans section : ce ne sont pas des réglages,
        // ce sont les sorties. Jusqu'au 2026-09-16, quitter demandait le raccourci « Quitter » ou
        // l'icône de la zone de notification — deux chemins qu'un joueur qui a la fenêtre Options sous
        // les yeux ne voit pas ; et relancer demandait de faire les deux à la suite, à la main.
        //
        // **Centrés** (demande utilisateur), comme « Se déconnecter » dans l'onglet « Paramètres »,
        // parce que ce sont des actions qui échappent à « Annuler ». Les confirmations, elles,
        // restent : un clic en plein combat coupe le détail des dégâts sans retour, et le brouillon de
        // la fenêtre part avec.
        //
        // **« Fermer l'overlay » est `Danger` depuis le 2026-09-18** (demande utilisateur). Les deux
        // boutons étaient secondaires à leur arrivée, au motif que ni l'une ni l'autre des deux sorties
        // ne détruit quoi que ce soit — le compte reste appairé, les réglages validés restent écrits.
        // C'est vrai des DONNÉES, pas de la SESSION : fermer est la seule action de toute la fenêtre
        // qui ne se rejoue pas depuis l'overlay (il faut le relancer depuis le bureau), et ce qui part
        // avec — combat en cours, compteurs de session — ne revient pas. Le rouge dit cette
        // irréversibilité-là, celle que la variante signale partout ailleurs dans l'interface.
        //
        // **« Redémarrer l'overlay » reste secondaire** : il ressort, lui, et de lui-même. Deux rouges
        // côte à côte n'auraient plus distingué la sortie de son voisin réversible — c'est justement
        // la distinction qui a motivé le changement.
        //
        // **« Redémarrer l'overlay », en toutes lettres** (2026-09-18, en même temps que le
        // déplacement) : il disait « Redémarrer » tant qu'il était collé à « Fermer l'overlay », qui
        // lui prêtait son complément. Les deux libellés se répondent maintenant mot pour mot.
        //
        // **La paire est centrée, pas chaque bouton** : les deux largeurs naturelles et la gouttière
        // du pied de page (`WINDOW_FOOTER_GUTTER`, la seule gouttière bouton-à-bouton relevée dans le
        // jeu) forment un bloc, centré d'un seul tenant sur la colonne — centrer chacun dans une
        // moitié les éloignerait l'un de l'autre au gré de la largeur de la fenêtre, et
        // « Redémarrer l'overlay » ne se lirait plus comme la variante de son voisin. Il est à GAUCHE :
        // on lit l'action la moins définitive en premier.
        ui.add_space(SECTION_GAP);
        let restart = design::button("Redémarrer l'overlay")
            .variant(ButtonVariant::Secondary)
            .size(ButtonSize::Height(ROW_HEIGHT))
            .tooltip(
                "Arrêter puis relancer l'overlay. Utile après avoir changé de fichier de \
                 journal ou quand l'affichage ne suit plus le jeu.",
            )
            .log_name("options-redemarrer-overlay");
        let quit = design::button("Fermer l'overlay")
            .variant(ButtonVariant::Danger)
            .size(ButtonSize::Height(ROW_HEIGHT))
            .tooltip(
                "Arrêter l'overlay. Le compte reste appairé et les réglages validés sont \
                 conservés pour la prochaine fois.",
            )
            .log_name("options-fermer-overlay");
        let restart_size = restart.desired_size(ui);
        let quit_size = quit.desired_size(ui);
        let gutter = design::tokens::WINDOW_FOOTER_GUTTER;
        let row = ui.allocate_space(egui::vec2(inner_width, ROW_HEIGHT)).1;
        let paire_gauche = row.center().x - (restart_size.x + gutter + quit_size.x) / 2.0;
        let restart_rect = egui::Rect::from_min_size(
            egui::pos2(paire_gauche, row.center().y - restart_size.y / 2.0),
            restart_size,
        );
        let quit_rect = egui::Rect::from_min_size(
            egui::pos2(
                restart_rect.right() + gutter,
                row.center().y - quit_size.y / 2.0,
            ),
            quit_size,
        );
        if ui.put(restart_rect, restart).clicked() {
            action = AProposTabAction::Restart;
        }
        if ui.put(quit_rect, quit).clicked() {
            action = AProposTabAction::Quit;
        }
    });

    action
}

/// **Les liens d'une section, sur une ligne**, alignés à gauche sur l'axe des contrôles — ce sont
/// des contrôles, pas des sorties : ils ne sont pas centrés comme la paire du pied de l'onglet.
/// Séparés par la gouttière du pied de page (`WINDOW_FOOTER_GUTTER`, la seule gouttière
/// bouton-à-bouton relevée dans le jeu), à leur largeur naturelle — jamais figée, une largeur en
/// dur écrête le libellé dès que la police ou le mot changent.
///
/// Renvoie l'URL du lien cliqué, au plus un par frame.
fn link_row(ui: &mut egui::Ui, width: f32, links: &[Link]) -> Option<String> {
    let mut clicked = None;
    let row = ui.allocate_space(egui::vec2(width, ROW_HEIGHT)).1;
    let mut left = row.left();
    for link in links {
        let button = design::button(link.label())
            .variant(ButtonVariant::Secondary)
            .size(ButtonSize::Height(ROW_HEIGHT))
            .tooltip(link.tooltip())
            .log_name(link.log_name());
        let size = button.desired_size(ui);
        let rect = egui::Rect::from_min_size(egui::pos2(left, row.center().y - size.y / 2.0), size);
        if ui.put(rect, button).clicked() {
            clicked = Some(link.url());
        }
        left = rect.right() + design::tokens::WINDOW_FOOTER_GUTTER;
    }
    clicked
}

/// La ligne d'information de la section « Mise à jour » et son ton — une fonction libre, pour
/// que ses formulations soient testées sans peindre.
pub fn update_info_line(
    status: &UpdateStatus,
    now: std::time::Instant,
) -> (String, design::InfoTone) {
    let since = |at: std::time::Instant| {
        let secs = now.saturating_duration_since(at).as_secs();
        if secs < 60 {
            "à l'instant".to_string()
        } else if secs < 3600 {
            format!("il y a {} min", secs / 60)
        } else {
            format!("il y a {} h", secs / 3600)
        }
    };
    match status {
        UpdateStatus::Idle => (
            "Aucune vérification depuis le lancement.".to_string(),
            design::InfoTone::Info,
        ),
        UpdateStatus::Checking => ("Recherche en cours…".to_string(), design::InfoTone::Info),
        UpdateStatus::UpToDate { checked_at } => (
            format!(
                "Dernière vérification {} · vous êtes à jour.",
                since(*checked_at)
            ),
            design::InfoTone::Info,
        ),
        UpdateStatus::Available {
            version,
            download_size,
            mandatory,
            checked_at,
            ..
        } => (
            format!(
                "Dernière vérification {} · version {version} disponible · {}{}",
                since(*checked_at),
                update::human_size(*download_size),
                if *mandatory { " · obligatoire" } else { "" }
            ),
            design::InfoTone::Info,
        ),
        UpdateStatus::Downloading {
            version,
            received,
            total,
        } => (
            format!(
                "Téléchargement de la version {version} : {} / {}",
                update::human_size(*received),
                update::human_size(*total)
            ),
            design::InfoTone::Info,
        ),
        UpdateStatus::Verifying { version } => (
            format!("Vérification de la version {version}…"),
            design::InfoTone::Info,
        ),
        UpdateStatus::ReadyToInstall { version, .. } | UpdateStatus::Installing { version } => (
            format!("Installation de la version {version}…"),
            design::InfoTone::Info,
        ),
        UpdateStatus::Unavailable { reason, checked_at } => (
            format!(
                "Dernière vérification {} · impossible ({reason}).",
                since(*checked_at)
            ),
            design::InfoTone::Alert,
        ),
        UpdateStatus::Failed {
            headline, detail, ..
        } => (
            format!("Mise à jour impossible : {headline} ({detail})"),
            design::InfoTone::Alert,
        ),
    }
}

/// Le bouton unique de la section « Mise à jour », selon l'état.
pub struct UpdateButtonSpec {
    pub label: String,
    pub variant: ButtonVariant,
    pub enabled: bool,
    pub tooltip: &'static str,
}

pub fn update_button(status: &UpdateStatus) -> UpdateButtonSpec {
    match status {
        UpdateStatus::Available { version, .. } => UpdateButtonSpec {
            label: format!("Mettre à jour vers {version}"),
            variant: ButtonVariant::Primary,
            enabled: true,
            tooltip: "Ferme l'overlay, installe la nouvelle version et le relance",
        },
        UpdateStatus::Checking => UpdateButtonSpec {
            label: "Recherche…".to_string(),
            variant: ButtonVariant::Secondary,
            enabled: false,
            tooltip: "Lecture de la dernière version publiée",
        },
        UpdateStatus::Downloading { .. }
        | UpdateStatus::Verifying { .. }
        | UpdateStatus::ReadyToInstall { .. }
        | UpdateStatus::Installing { .. } => UpdateButtonSpec {
            label: "Mise à jour en cours…".to_string(),
            variant: ButtonVariant::Secondary,
            enabled: false,
            tooltip: "L'overlay se relancera une fois la version installée",
        },
        UpdateStatus::Failed { .. } => UpdateButtonSpec {
            label: "Réessayer".to_string(),
            variant: ButtonVariant::Secondary,
            enabled: true,
            tooltip: "Rechercher à nouveau la dernière version publiée",
        },
        UpdateStatus::Idle | UpdateStatus::UpToDate { .. } | UpdateStatus::Unavailable { .. } => {
            UpdateButtonSpec {
                label: "Recherche de mise à jour".to_string(),
                variant: ButtonVariant::Secondary,
                enabled: true,
                tooltip: "Lire la dernière version publiée, sans rien installer",
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ligne_et_bouton_de_la_section_mise_a_jour_suivent_l_etat() {
        use overlay_sync::update::UpdateStatus;
        let now = std::time::Instant::now();
        let (ligne, _) = update_info_line(
            &UpdateStatus::UpToDate { checked_at: now },
            now + std::time::Duration::from_secs(185),
        );
        assert_eq!(
            ligne,
            "Dernière vérification il y a 3 min · vous êtes à jour."
        );
        assert_eq!(
            update_button(&UpdateStatus::Idle).label,
            "Recherche de mise à jour"
        );
        let disponible = UpdateStatus::Available {
            version: "0.21.0".into(),
            download_size: 3_100_000,
            mandatory: false,
            notes_url: None,
            checked_at: now,
        };
        let (ligne, tone) = update_info_line(&disponible, now);
        assert_eq!(
            ligne,
            "Dernière vérification à l'instant · version 0.21.0 disponible · 3,1 Mo"
        );
        assert_eq!(tone, design::InfoTone::Info);
        let bouton = update_button(&disponible);
        assert_eq!(bouton.label, "Mettre à jour vers 0.21.0");
        assert!(bouton.enabled);
        assert_eq!(bouton.variant, ButtonVariant::Primary);
        assert!(!update_button(&UpdateStatus::Checking).enabled);
        let (_, tone) = update_info_line(
            &UpdateStatus::Unavailable {
                reason: "hors ligne".into(),
                checked_at: now,
            },
            now,
        );
        assert_eq!(tone, design::InfoTone::Alert);
    }

    /// Les liens du service partent de la même origine que l'API — un binaire de Release ouvre la
    /// prod, un binaire de dev le déploiement dev, jamais l'inverse — et visent les chemins sans
    /// préfixe de langue que le site redirige (voir `privacy_policy_url`).
    #[test]
    fn les_liens_du_service_suivent_l_origine_de_l_api() {
        let origine = overlay_sync::client::base_url();
        let origine = origine.trim_end_matches('/');
        assert_eq!(Link::Site.url(), origine);
        assert_eq!(
            Link::PrivacyPolicy.url(),
            format!("{origine}/privacy-policy")
        );
        assert_eq!(
            Link::TermsOfService.url(),
            format!("{origine}/terms-of-service")
        );
        assert_eq!(Link::Source.url(), SOURCE_URL);
        assert_eq!(Link::WakfuTerms.url(), WAKFU_TERMS_URL);
        for link in [
            Link::Site,
            Link::Source,
            Link::WakfuTerms,
            Link::PrivacyPolicy,
            Link::TermsOfService,
        ] {
            assert!(
                link.url().starts_with("https://"),
                "{link:?} : {}",
                link.url()
            );
        }
    }

    /// L'origine d'API surchargée est nommée ; sans surcharge, rien ne s'affiche.
    #[test]
    fn l_origine_surchargee_est_dite_et_le_cas_normal_reste_muet() {
        assert_eq!(api_override_notice(None), None);
        let texte = api_override_notice(Some("http://127.0.0.1:8788")).unwrap();
        assert!(texte.contains("http://127.0.0.1:8788"));
        assert!(texte.contains("WAKFU_COMPANION_API_URL"));
    }

    /// La mention exigée par la licence des données Wakfu, au mot près, et l'année en cours —
    /// hors gel des captures, où c'est l'année figée qui est peinte.
    #[test]
    fn la_mention_de_droits_est_celle_de_la_licence_ankama() {
        assert_eq!(
            copyright_notice(2026),
            "WAKFU MMORPG : © 2012-2026 Ankama Studio. Tous droits réservés."
        );
        // Ce test ne gèle rien : l'année est celle de l'horloge, jamais antérieure à celle de la
        // licence ni à celle des captures.
        assert!(copyright_year() >= FROZEN_COPYRIGHT_YEAR);
    }

    /// Les trois sections, dans l'ordre demandé, chacune avec au moins un lien — et la mention de
    /// non-affiliation à Ankama (analyse CGU §3.5) ainsi que l'adresse de contact (RGPD, art. 13)
    /// y figurent en toutes lettres.
    #[test]
    fn les_sections_d_information_portent_ce_qui_est_attendu() {
        let titres: Vec<&str> = SECTIONS.iter().map(|s| s.title).collect();
        assert_eq!(
            titres,
            [
                "Wakfu Companion",
                "Conditions d'utilisation de Wakfu",
                "Vos données"
            ]
        );
        for section in SECTIONS {
            assert!(!section.blocks.is_empty(), "{} : aucun bloc", section.title);
            assert!(!section.links.is_empty(), "{} : aucun lien", section.title);
        }
        let texte: String = SECTIONS
            .iter()
            .flat_map(|s| s.blocks.iter().map(|b| b.text))
            .collect::<Vec<_>>()
            .join(" ");
        assert!(texte.contains("non officiel"));
        assert!(texte.contains("aucun lien avec cette société"));
        assert!(texte.contains("WAKFU est une marque d'Ankama"));
        assert!(texte.contains(CONTACT_EMAIL));
        assert!(texte.contains("jamais téléversé"));
        // Les quatre types d'historique envoyés au compte (`overlay_engine::HistoryEventKind`),
        // tous nommés : la liste se lit comme exhaustive, elle doit l'être (2026-09-21).
        // Le Suivi et les ventes récupérées à l'Hôtel des ventes partent aussi (2026-09-25).
        for flux in [
            "vos combats",
            "vos achats",
            "vos ventes",
            "vos échanges",
            "vos extractions de pacte",
            "votre Suivi",
        ] {
            assert!(texte.contains(flux), "« {flux} » absent de « Vos données »");
        }
        // Un seul avertissement dans tout l'onglet : celui de la responsabilité vis-à-vis des CGU.
        let alertes: Vec<&InfoBlock> = SECTIONS
            .iter()
            .flat_map(|s| s.blocks.iter())
            .filter(|b| b.tone == InfoTone::Alert)
            .collect();
        assert_eq!(alertes.len(), 1);
        assert!(alertes[0].text.contains("votre seule responsabilité"));
    }

    /// La déconnexion vide la file d'envoi : le texte partagé doit le dire, et le bloc « Sur cet
    /// ordinateur » aussi (2026-09-25).
    #[test]
    fn la_deconnexion_annonce_la_perte_de_l_historique_non_envoye() {
        assert!(DISCONNECT_INFO.contains("l'historique pas encore envoyé"));
        let texte: String = SECTIONS
            .iter()
            .flat_map(|s| s.blocks.iter().map(|b| b.text))
            .collect::<Vec<_>>()
            .join(" ");
        assert!(texte.contains("l'historique pas encore envoyé"));
        assert!(PURGE_INFO.contains("inscription au démarrage"));
    }
}
