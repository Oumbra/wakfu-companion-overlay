//! **Essai automatique de la vraie fenêtre**, sans toucher à l'écran de l'utilisateur
//! (2026-11-27) : `wakfu-companion-overlay --autotest=<dossier>`.
//!
//! Demande de l'utilisateur : les essais lancés sur son poste ne doivent **jamais** mettre une
//! fenêtre au premier plan ni se servir de sa souris (une première passe le faisait, par
//! `SetForegroundWindow` et des clics système). Même principe que l'essai automatique de son
//! projet `velmire` :
//!
//! - la fenêtre est créée **invisible** — ni montrée, ni activée, absente de la barre des tâches
//!   (une fenêtre montrée, même sans focus, Windows la met devant) ;
//! - un **robot** joue un scénario en injectant des événements egui (déplacement du pointeur,
//!   clic, texte) dans l'entrée de la fenêtre — le vrai code de la fenêtre les traite comme ceux
//!   de la souris, mais la souris du système ne bouge pas ;
//! - les **captures** sont relues sur le GPU (la frame est peinte une seconde fois dans une
//!   texture hors écran, copiée en mémoire) : rien n'est lu à l'écran ;
//! - un `journal.txt` résume chaque étape, puis le processus se termine.
//!
//! Le reste de l'overlay ne démarre pas dans ce mode (voir `main.rs`) : pas de verrou d'instance
//! unique (l'overlay de l'utilisateur peut tourner à côté), ni moteur, ni synchronisation, ni mise
//! à jour, ni raccourcis globaux, ni icône de notification, ni fenêtre sur le jeu. La base
//! d'historique ouverte est une **copie** de la vraie, faite dans le dossier de l'essai.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// L'option de ligne de commande : `--autotest=<dossier>`.
pub const FLAG: &str = "--autotest";
/// Le scénario joué : `--autotest-scenario=historique` (défaut) ou `memoire`.
pub const SCENARIO_FLAG: &str = "--autotest-scenario";

/// Délai entre deux étapes : le temps que les frames injectées soient peintes.
pub const STEP_INTERVAL: Duration = Duration::from_millis(120);

/// Attente maximale du référentiel (noms d'objets) avant la première capture.
const READY_TIMEOUT: Duration = Duration::from_secs(20);

/// Le dossier demandé par `--autotest=<dossier>`, s'il y en a un.
pub fn requested(args: impl IntoIterator<Item = String>) -> Option<PathBuf> {
    args.into_iter().find_map(|arg| {
        arg.strip_prefix(FLAG)
            .and_then(|rest| rest.strip_prefix('='))
            .filter(|dir| !dir.is_empty())
            .map(PathBuf::from)
    })
}

/// Le scénario demandé par `--autotest-scenario=<nom>` et l'onglet sur lequel la fenêtre
/// s'ouvre. `None` : nom inconnu.
pub fn scenario(args: impl IntoIterator<Item = String>) -> Option<(Vec<Step>, &'static str)> {
    let name = args
        .into_iter()
        .find_map(|arg| {
            arg.strip_prefix(SCENARIO_FLAG)
                .and_then(|rest| rest.strip_prefix('='))
                .map(str::to_string)
        })
        .unwrap_or_else(|| "historique".to_string());
    match name.as_str() {
        "historique" => Some((history_scenario(), "achats")),
        "memoire" => Some((memory_scenario(), "echanges")),
        _ => None,
    }
}

/// Mémoire du processus : working set et mémoire privée, en octets.
#[derive(Debug, Clone, Copy)]
pub struct ProcessMemory {
    pub working_set: u64,
    pub private: u64,
}

impl std::fmt::Display for ProcessMemory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const MB: f64 = 1024.0 * 1024.0;
        write!(
            f,
            "working set {:.0} Mo, privée {:.0} Mo",
            self.working_set as f64 / MB,
            self.private as f64 / MB
        )
    }
}

/// Mémoire du processus courant (`GetProcessMemoryInfo` ; `/proc/self/status` sous Linux).
#[cfg(windows)]
pub fn process_memory() -> Option<ProcessMemory> {
    use windows::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX,
    };
    use windows::Win32::System::Threading::GetCurrentProcess;
    let mut counters = PROCESS_MEMORY_COUNTERS_EX::default();
    let size = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
    // SAFETY: `counters` est un PROCESS_MEMORY_COUNTERS_EX valide, de la taille annoncée ;
    // `GetCurrentProcess` rend un pseudo-handle qui n'a pas à être fermé.
    unsafe {
        GetProcessMemoryInfo(
            GetCurrentProcess(),
            (&mut counters as *mut PROCESS_MEMORY_COUNTERS_EX).cast::<PROCESS_MEMORY_COUNTERS>(),
            size,
        )
        .ok()?;
    }
    Some(ProcessMemory {
        working_set: counters.WorkingSetSize as u64,
        private: counters.PrivateUsage as u64,
    })
}

/// Mémoire du processus courant (`GetProcessMemoryInfo` ; `/proc/self/status` sous Linux).
#[cfg(not(windows))]
pub fn process_memory() -> Option<ProcessMemory> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let field = |name: &str| {
        status
            .lines()
            .find_map(|line| line.strip_prefix(name))
            .and_then(|rest| {
                rest.trim()
                    .trim_end_matches("kB")
                    .trim()
                    .parse::<u64>()
                    .ok()
            })
            .map(|kb| kb * 1024)
    };
    Some(ProcessMemory {
        working_set: field("VmRSS:")?,
        private: field("RssAnon:").unwrap_or(0),
    })
}

/// Une étape du scénario. Les coordonnées sont en points egui, relatives à la fenêtre.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    /// Attendre que le référentiel soit chargé (les noms d'objets en dépendent).
    WaitReady,
    /// Clic gauche : survol et appui sur une frame, relâchement sur la suivante.
    Click { x: f32, y: f32, label: &'static str },
    /// Texte tapé dans le champ qui a le focus.
    Type(&'static str),
    /// Molette au-dessus de `(x, y)`, en lignes (positif : vers le bas).
    Scroll { x: f32, y: f32, lines: f32 },
    /// Capture de la fenêtre, `<nom>.png` dans le dossier de l'essai.
    Capture(&'static str),
    /// La fenêtre d'historique doit être fermée.
    ExpectClosed,
    /// Note la mémoire du processus dans le journal.
    Memory(&'static str),
    /// Attendre, sans rien peindre.
    Pause(Duration),
}

/// Taille de la fenêtre pendant l'essai — celle d'ouverture, sur laquelle sont relevées les
/// coordonnées du scénario.
pub const WINDOW_SIZE: (f64, f64) = (1100.0, 820.0);

/// Centres des onglets de la fenêtre d'historique à 1100 points de large, dans l'ordre de
/// `HistoryTab::ALL` (relevés sur la vraie fenêtre).
const TAB_CENTERS: [(f32, &str); 6] = [
    (111.0, "combats"),
    (286.0, "recap"),
    (452.0, "achats"),
    (632.0, "echanges"),
    (811.0, "pactes"),
    (987.0, "a-propos"),
];
const TAB_Y: f32 = 92.0;

/// Le scénario de la fenêtre d'historique : chaque onglet, une recherche dans les achats, puis
/// la croix.
pub fn history_scenario() -> Vec<Step> {
    // La frame de `WaitReady` demande les icônes d'objets : la pause leur laisse le temps
    // d'arriver (cache disque, sinon réseau) avant la première capture.
    let mut steps = vec![
        Step::WaitReady,
        Step::Pause(Duration::from_secs(3)),
        Step::Capture("00-ouverture"),
    ];
    for (x, name) in TAB_CENTERS {
        steps.push(Step::Click {
            x,
            y: TAB_Y,
            label: name,
        });
        // Une liste d'objets demande ses icônes à sa première frame : même pause qu'à
        // l'ouverture.
        if matches!(name, "combats" | "echanges" | "pactes") {
            steps.push(Step::Pause(Duration::from_secs(2)));
        }
        steps.push(Step::Capture(name));
        if name == "combats" {
            // La première entrée du premier jour, dépliée (un combat, ou un run de donjon), puis
            // la ligne suivante (le premier combat du run, ou le combat suivant).
            steps.extend([
                Step::Click {
                    x: 550.0,
                    y: 242.0,
                    label: "premier combat",
                },
                Step::Pause(Duration::from_secs(1)),
                Step::Capture("combats-deplie"),
                Step::Click {
                    x: 550.0,
                    y: 282.0,
                    label: "ligne suivante",
                },
                Step::Pause(Duration::from_secs(1)),
                Step::Capture("combats-deplie-2"),
                // Le premier allié du combat déplié : ses sorts.
                Step::Click {
                    x: 200.0,
                    y: 387.0,
                    label: "premier allié",
                },
                Step::Pause(Duration::from_secs(1)),
                Step::Capture("combats-sorts"),
                Step::Click {
                    x: 830.0,
                    y: 165.0,
                    label: "donjons et familles",
                },
                Step::Pause(Duration::from_secs(1)),
                Step::Capture("combats-types"),
                Step::Click {
                    x: 975.0,
                    y: 165.0,
                    label: "tout replier",
                },
                Step::Pause(Duration::from_secs(1)),
                Step::Capture("combats-types-replies"),
            ]);
        }
        if name == "recap" {
            // Aujourd'hui peut être vide : le mois et l'année en cours.
            for (x, label, capture) in [
                (174.0, "mois", "recap-mois"),
                (264.0, "annee", "recap-annee"),
            ] {
                steps.extend([
                    Step::Click { x, y: 165.0, label },
                    Step::Pause(Duration::from_secs(2)),
                    Step::Capture(capture),
                ]);
            }
        }
        if name == "echanges" {
            // Plus bas dans la liste : kamas et objets cédés.
            steps.extend([
                Step::Scroll {
                    x: 550.0,
                    y: 500.0,
                    lines: 12.0,
                },
                Step::Pause(Duration::from_secs(1)),
                Step::Capture("echanges-suite"),
            ]);
        }
    }
    steps.extend([
        Step::Click {
            x: TAB_CENTERS[2].0,
            y: TAB_Y,
            label: "achats",
        },
        Step::Click {
            x: 450.0,
            y: 160.0,
            label: "champ de recherche",
        },
        Step::Scroll {
            x: 550.0,
            y: 500.0,
            lines: 400.0,
        },
        Step::Capture("achats-fin-de-liste"),
        Step::Scroll {
            x: 550.0,
            y: 500.0,
            lines: -400.0,
        },
        Step::Type("Larme"),
        Step::Capture("achats-recherche"),
        Step::Click {
            x: 1071.0,
            y: 28.0,
            label: "croix",
        },
        Step::ExpectClosed,
    ]);
    steps
}

/// Le scénario de la mémoire : la fenêtre ouverte sur un onglet vide, puis sur les achats, puis
/// refermée — ce que coûte la fenêtre elle-même, et ce que coûte une liste.
pub fn memory_scenario() -> Vec<Step> {
    let settle = Step::Pause(Duration::from_secs(2));
    vec![
        Step::WaitReady,
        Step::Memory("fenêtre ouverte, onglet vide"),
        settle.clone(),
        Step::Memory("onglet vide, 2 s plus tard"),
        Step::Click {
            x: TAB_CENTERS[2].0,
            y: TAB_Y,
            label: "achats",
        },
        Step::Memory("achats peints"),
        settle.clone(),
        Step::Memory("achats, 2 s plus tard"),
        Step::Capture("memoire-achats"),
        Step::Click {
            x: TAB_CENTERS[3].0,
            y: TAB_Y,
            label: "echanges",
        },
        settle.clone(),
        Step::Memory("retour sur l'onglet vide, 2 s plus tard"),
        Step::Click {
            x: 1071.0,
            y: 28.0,
            label: "croix",
        },
        Step::ExpectClosed,
        settle,
        Step::Memory("fenêtre fermée, 2 s plus tard"),
    ]
}

/// L'état du robot, tenu par l'hôte.
pub struct AutoTest {
    pub dir: PathBuf,
    steps: VecDeque<Step>,
    next_at: Instant,
    started: Instant,
    journal: Vec<String>,
    failed: bool,
    shots: u32,
}

impl AutoTest {
    pub fn new(dir: PathBuf, steps: Vec<Step>) -> Self {
        let now = Instant::now();
        Self {
            dir,
            steps: steps.into(),
            next_at: now,
            started: now,
            journal: Vec::new(),
            failed: false,
            shots: 0,
        }
    }

    /// Repousse la prochaine étape (`Step::Pause`).
    pub fn delay(&mut self, by: Duration) {
        self.next_at += by;
    }

    /// Note la mémoire du processus.
    pub fn log_memory(&mut self, label: &str) {
        match process_memory() {
            Some(memory) => self.log(format!("mémoire ({label}) : {memory}")),
            None => self.log(format!("mémoire ({label}) : illisible")),
        }
    }

    /// L'instant de la prochaine étape.
    pub fn next_at(&self) -> Instant {
        self.next_at
    }

    /// La prochaine étape si elle est due, `None` sinon. Une fois le scénario épuisé,
    /// [`Self::done`] le dit.
    pub fn due(&mut self, now: Instant) -> Option<Step> {
        if now < self.next_at {
            return None;
        }
        let step = self.steps.pop_front()?;
        self.next_at = now + STEP_INTERVAL;
        Some(step)
    }

    pub fn done(&self) -> bool {
        self.steps.is_empty() && Instant::now() >= self.next_at
    }

    /// `WaitReady` pas encore satisfait : la remet en tête, ou échoue passé le délai.
    pub fn not_ready(&mut self) {
        if self.started.elapsed() > READY_TIMEOUT {
            self.fail("référentiel toujours vide après 20 s");
        } else {
            self.steps.push_front(Step::WaitReady);
        }
    }

    pub fn capture_path(&mut self, name: &str) -> PathBuf {
        self.shots += 1;
        self.dir.join(format!("{name}.png"))
    }

    pub fn log(&mut self, line: impl Into<String>) {
        let line = format!(
            "[{:>6} ms] {}",
            self.started.elapsed().as_millis(),
            line.into()
        );
        tracing::info!("[autotest] {line}");
        self.journal.push(line);
    }

    pub fn fail(&mut self, reason: impl Into<String>) {
        self.failed = true;
        self.log(format!("ÉCHEC : {}", reason.into()));
    }

    pub fn failed(&self) -> bool {
        self.failed
    }

    /// Écrit `journal.txt` et rend le verdict.
    pub fn finish(&mut self) -> bool {
        let verdict = if self.failed { "ÉCHEC" } else { "OK" };
        let summary = format!(
            "{verdict} — {} captures en {} ms",
            self.shots,
            self.started.elapsed().as_millis()
        );
        self.log(summary);
        let path = self.dir.join("journal.txt");
        if let Err(err) = std::fs::write(&path, self.journal.join("\n") + "\n") {
            tracing::warn!("[autotest] journal non écrit ({}) : {err}", path.display());
        }
        !self.failed
    }
}

/// Copie la base d'historique dans le dossier de l'essai (avec ses fichiers WAL), pour que
/// l'essai n'écrive jamais dans la vraie — même pas une migration.
pub fn copy_history_db(source: &Path, dir: &Path) -> std::io::Result<PathBuf> {
    let target = dir.join("history.sqlite3");
    std::fs::copy(source, &target)?;
    for suffix in ["-wal", "-shm"] {
        let mut from = source.as_os_str().to_owned();
        from.push(suffix);
        let from = PathBuf::from(from);
        if from.exists() {
            let mut to = target.as_os_str().to_owned();
            to.push(suffix);
            std::fs::copy(&from, PathBuf::from(to))?;
        }
    }
    Ok(target)
}

/// Les frames d'un coup de molette : le survol et la molette, puis de quoi laisser le
/// défilement animé d'egui aller au bout.
pub fn scroll_events(pos: egui::Pos2, lines: f32) -> Vec<Vec<egui::Event>> {
    let mut frames = vec![vec![
        egui::Event::PointerMoved(pos),
        egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, -lines),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        },
    ]];
    frames.extend(std::iter::repeat_with(Vec::new).take(20));
    frames
}

/// Les événements egui d'un clic gauche à `pos` : la première frame survole et appuie, la
/// seconde relâche — un clic egui se termine au relâchement, sur une frame distincte.
pub fn click_events(pos: egui::Pos2) -> [Vec<egui::Event>; 2] {
    let button = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    [
        vec![egui::Event::PointerMoved(pos), button(true)],
        vec![button(false)],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_de_ligne_de_commande() {
        let args = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            requested(args(&["--historique", "--autotest=C:\\essai"])),
            Some(PathBuf::from("C:\\essai"))
        );
        assert_eq!(requested(args(&["--autotest"])), None);
        assert_eq!(requested(args(&["--autotest="])), None);
        assert_eq!(requested(args(&["wakfu.log"])), None);
    }

    #[test]
    fn le_scenario_finit_par_la_croix() {
        let steps = history_scenario();
        assert_eq!(steps.first(), Some(&Step::WaitReady));
        assert_eq!(steps.last(), Some(&Step::ExpectClosed));
        assert_eq!(
            steps
                .iter()
                .filter(|s| matches!(s, Step::Capture(_)))
                .count(),
            17
        );
    }

    #[test]
    fn les_etapes_sont_espacees() {
        let mut test = AutoTest::new(PathBuf::new(), vec![Step::Type("a"), Step::Type("b")]);
        let now = Instant::now();
        assert_eq!(test.due(now), Some(Step::Type("a")));
        assert_eq!(test.due(now), None);
        assert_eq!(test.due(now + STEP_INTERVAL), Some(Step::Type("b")));
    }
}
