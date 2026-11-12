//! Interface: soft glass cards, rounded pill buttons, colour sidebar —
//! rebuilt from scratch (no original assets or branding).

use std::path::PathBuf;
use std::time::SystemTime;

use eframe::egui::{self, Align, Color32, Layout, Pos2, Rect, RichText, Rounding, Stroke, Vec2};

use crate::engine::*;
use crate::mechanics::*;
use crate::store::{self, Loaded, Settings, State};
use crate::watcher::{self, WatchConfig, Watcher};

// ------------------------------------------------------------------- theme

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Dashboard,
    Folders,
    Rules,
    History,
    Settings,
}

/// Icon glyphs drawn with painter primitives — coherent set, no initials.
#[derive(Clone, Copy, PartialEq)]
enum Icon {
    Chart,
    Folder,
    List,
    Clock,
    Sliders,
    Funnel,
    Bolt,
    Grip,
    Bell,
    Gear,
    Search,
    Cross,
    Add,
    Trash,
    FolderOpen,
    Refresh,
    Tick,
    Undo,
}

const TABS: &[(Tab, &str, Icon, &str)] = &[
    (Tab::Dashboard, "Aperçu", Icon::Chart, "Tout en un coup d'œil"),
    (Tab::Folders, "Dossiers suivis", Icon::Folder, "Les lieux à organiser"),
    (Tab::Rules, "Règles", Icon::List, "Si ceci, alors cela"),
    (Tab::History, "Historique", Icon::Clock, "Passé, annulable"),
    (Tab::Settings, "Réglages", Icon::Sliders, "Ajuster l'application"),
];

// Accent swatches (Nocturne Studio): orange ambré en premier = accent par défaut.
const ACCENTS: &[(&str, Color32)] = &[
    ("orange", Color32::from_rgb(0xff, 0x78, 0x28)),
    ("blue", Color32::from_rgb(0x38, 0xbd, 0xf8)),
    ("purple", Color32::from_rgb(0xa7, 0x8b, 0xfa)),
    ("pink", Color32::from_rgb(0xf4, 0x72, 0xb6)),
    ("yellow", Color32::from_rgb(0xed, 0xc1, 0x3f)),
    ("green", Color32::from_rgb(0x27, 0xc9, 0x3f)),
    ("graphite", Color32::from_rgb(0x8d, 0x99, 0xaa)),
];

// Palette « Nocturne Studio Amber & Deep Blue » (DESIGN.md de la maquette).
const CANVAS: Color32 = Color32::from_rgb(0x0c, 0x0e, 0x15);
/// Barres du haut et latérale : sombres et translucides, les halos du fond transparaissent.
const PANEL_GLASS: Color32 = Color32::from_rgba_premultiplied(0x0b, 0x0c, 0x11, 170);
const RED: Color32 = Color32::from_rgb(0xf8, 0x71, 0x71);
const SKY: Color32 = Color32::from_rgb(0x38, 0xbd, 0xf8);
const SKY_DEEP: Color32 = Color32::from_rgb(0x02, 0x84, 0xc7);
const GREEN: Color32 = Color32::from_rgb(0x27, 0xc9, 0x3f);

/// Voile de verre : blanc translucide posé sur le fond à halos.
/// Le mélange se fait en espace linéaire (framebuffer sRGB) : un blanc à 5 % y paraît ~25 % plus clair,
/// d'où le facteur 0,16 pour que `alpha` corresponde à l'opacité perçue (sur 255).
fn glass(alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(0xff, 0xff, 0xff, (alpha as f32 * 0.16).round() as u8)
}
fn text_primary() -> Color32 {
    Color32::from_rgb(0xf1, 0xf5, 0xf9)
}
fn text_dim() -> Color32 {
    Color32::from_rgb(0xff, 0xea, 0xd9)
}
/// Petits libellés en majuscules (orangé clair, un cran plus soutenu que le texte secondaire).
fn text_label() -> Color32 {
    Color32::from_rgb(0xff, 0xd8, 0xb8)
}
fn with_alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

fn semibold() -> egui::FontFamily {
    egui::FontFamily::Name("semibold".into())
}
fn bold() -> egui::FontFamily {
    egui::FontFamily::Name("bold".into())
}
fn icons_family() -> egui::FontFamily {
    egui::FontFamily::Name("icons".into())
}

/// Polices embarquées : Outfit (3 graisses) + Material Symbols Outlined (sous-ensemble).
fn load_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let faces: [(&str, &'static [u8]); 4] = [
        ("outfit", include_bytes!("../assets/fonts/Outfit-Regular.ttf")),
        ("outfit-semibold", include_bytes!("../assets/fonts/Outfit-SemiBold.ttf")),
        ("outfit-bold", include_bytes!("../assets/fonts/Outfit-Bold.ttf")),
        ("icons", include_bytes!("../assets/fonts/MaterialSymbolsOutlined-subset.ttf")),
    ];
    for (name, bytes) in faces {
        fonts.font_data.insert(name.into(), egui::FontData::from_static(bytes));
    }
    fonts.families.entry(egui::FontFamily::Proportional).or_default().insert(0, "outfit".into());
    fonts.families.insert(semibold(), vec!["outfit-semibold".into(), "outfit".into()]);
    fonts.families.insert(bold(), vec!["outfit-bold".into(), "outfit".into()]);
    fonts.families.insert(icons_family(), vec!["icons".into()]);
    ctx.set_fonts(fonts);
}

/// Applique le style sombre global : champs `#0c0e15`, focus orange, coins doux.
fn apply_theme(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = Color32::TRANSPARENT;
    v.window_fill = Color32::from_rgb(0x1a, 0x1d, 0x27);
    v.extreme_bg_color = with_alpha(CANVAS, 150); // fond des champs de saisie
    v.faint_bg_color = glass(10);
    v.override_text_color = Some(text_primary());
    v.selection.bg_fill = with_alpha(ACCENTS[0].1, 90);
    v.selection.stroke = Stroke::new(1.0, ACCENTS[0].1);
    let r = Rounding::same(8.0);
    for w in [&mut v.widgets.noninteractive, &mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
        w.rounding = r;
    }
    // Aucun contour décoratif : seuls le survol et le focus gardent un liseré orange.
    v.widgets.noninteractive.bg_stroke = Stroke::NONE;
    v.widgets.inactive.bg_stroke = Stroke::NONE;
    v.widgets.inactive.bg_fill = glass(20);
    v.widgets.inactive.weak_bg_fill = glass(16);
    v.widgets.hovered.bg_fill = glass(30);
    v.widgets.hovered.weak_bg_fill = glass(28);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, with_alpha(ACCENTS[0].1, 120));
    v.widgets.active.bg_stroke = Stroke::new(1.0, ACCENTS[0].1);
    ctx.set_visuals(v);
    load_fonts(ctx);
    ctx.style_mut(|s| {
        s.spacing.scroll = egui::style::ScrollStyle::thin();
        s.spacing.item_spacing = Vec2::new(8.0, 6.0);
        for (style, size) in [(egui::TextStyle::Body, 15.0), (egui::TextStyle::Button, 15.0), (egui::TextStyle::Small, 12.0)] {
            if let Some(font) = s.text_styles.get_mut(&style) {
                font.size = size;
            }
        }
        s.spacing.interact_size.y = 30.0;
        s.spacing.button_padding = Vec2::new(10.0, 5.0);
    });
}

// --------------------------------------------------------------------- app

/// Dossier retiré (avec ses règles), rétablissable pendant quelques secondes.
struct UndoFolder {
    folder: Folder,
    rules: Vec<Rule>,
    index: usize,
    at: std::time::Instant,
}

const UNDO_SECONDS: u64 = 8;

const APP_ICON_PNG: &[u8] = include_bytes!("../../assets/icons/app-256.png");

fn app_icon_image() -> egui::ColorImage {
    let img = image::load_from_memory(APP_ICON_PNG).expect("icône embarquée valide").to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw())
}

/// Icône de la fenêtre (Dock, barre des tâches).
pub fn app_icon() -> egui::IconData {
    let img = image::load_from_memory(APP_ICON_PNG).expect("icône embarquée valide").to_rgba8();
    egui::IconData { width: img.width(), height: img.height(), rgba: img.into_raw() }
}

/// Options de lancement (ligne de commande).
pub struct Launch {
    pub bg_time: f64,
    pub screenshot: Option<String>,
    pub tab: Option<String>,
    pub empty: bool,
    /// Démarrage à l'ouverture de session : fenêtre réduite dans le Dock.
    pub background: bool,
}

pub struct App {
    tab: Tab,
    folders: Vec<Folder>,
    rules: Vec<Rule>,
    history: Vec<HistoryEntry>,
    next_id: u64,
    next_batch: u64,
    monthly_used: u32,
    accent: usize,
    menu_bar: bool,
    notify: bool,
    animate_bg: bool,
    bg_offset: f64,
    retention_days: u32,
    add_folder_text: String,
    selected_rule: usize,
    app_enabled: bool,
    launch_at_login: bool,
    /// Surveillance automatique ; `None` en mode capture d'écran et dans les tests.
    watcher: Option<Watcher>,
    watch_key: String,
    /// Logo de la barre latérale (chargé au premier affichage).
    logo: Option<egui::TextureHandle>,
    search: String,
    toast: Option<String>,
    undo_folder: Option<UndoFolder>,
    /// Fichier de sauvegarde ; `None` en mode capture d'écran (on n'écrit jamais dans les vraies données).
    store_path: Option<std::path::PathBuf>,
    last_saved: String,
    next_save_check: f64,
    show_path_input: bool,
    folder_counts: std::collections::HashMap<u64, Option<usize>>,
    last_run: Option<(usize, usize, usize)>, // matched, actions, scanned
    last_run_dry: bool,
    last_tab: Tab,
    tab_since: f64,
    frame: u32,
    screenshot: Option<String>,
    shot_done: bool,
    start_minimized: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, launch: Launch) -> Self {
        apply_theme(&cc.egui_ctx);
        let empty = launch.empty;
        let tab = launch.tab.clone();
        let mut app = Self::blank(launch);
        if app.screenshot.is_some() {
            // Capture d'écran : jamais d'écriture. Avec ORGANIZEMYFILES_STATE_DIR, on charge ce dossier (contrôle de la restauration).
            if std::env::var_os("ORGANIZEMYFILES_STATE_DIR").is_some() {
                app.load_saved_state();
                app.store_path = None;
            } else if !empty {
                app.seed_demo();
            }
        } else {
            app.load_saved_state();
            let ctx = cc.egui_ctx.clone();
            app.watcher = Some(Watcher::start(watcher::SETTLE, move || ctx.request_repaint()));
        }
        app.tab = match tab.as_deref() {
            Some("folders") => Tab::Folders,
            Some("rules") => Tab::Rules,
            Some("history") => Tab::History,
            Some("settings") => Tab::Settings,
            _ => Tab::Dashboard,
        };
        app
    }

    /// État initial sans aucun contexte graphique (utilisable dans les tests).
    fn blank(launch: Launch) -> Self {
        let Launch { screenshot, tab: _, empty: _, bg_time, background } = launch;
        App {
            tab: Tab::Dashboard,
            folders: Vec::new(),
            rules: Vec::new(),
            history: Vec::new(),
            next_id: 1,
            next_batch: 1,
            monthly_used: 0,
            accent: 0,
            menu_bar: true,
            notify: true,
            animate_bg: true,
            bg_offset: bg_time,
            retention_days: HISTORY_RETENTION_DEFAULT_DAYS,
            add_folder_text: String::new(),
            selected_rule: 0,
            app_enabled: true,
            launch_at_login: false,
            watcher: None,
            watch_key: String::new(),
            logo: None,
            search: String::new(),
            toast: None,
            undo_folder: None,
            store_path: None,
            last_saved: String::new(),
            next_save_check: 0.0,
            show_path_input: false,
            folder_counts: std::collections::HashMap::new(),
            last_run: None,
            last_run_dry: true,
            last_tab: Tab::Dashboard,
            tab_since: 0.0,
            frame: 0,
            screenshot,
            shot_done: false,
            start_minimized: background,
        }
    }

    /// Demo content so the saved picture shows a living app.
    fn seed_demo(&mut self) {
        let home = home_dir();
        for (p, prio) in [("Downloads", 0i64), ("Desktop", 1), ("Documents", 2)] {
            self.folders.push(Folder {
                id: self.next_id,
                path: PathBuf::from(format!("{home}/{p}")),
                enabled: true,
                priority: prio,
            });
            self.next_id += 1;
        }
        self.folders.push(Folder {
            id: self.next_id,
            path: PathBuf::from(format!("{home}/Projets-Clients")),
            enabled: false,
            priority: 3,
        });
        self.next_id += 1;
        let f0 = self.folders[0].id;
        self.rules.push(sample_rule(self.next_id, f0, "Tri Bureau & Téléchargements", 0));
        self.next_id += 1;
        self.rules.push(sample_rule(self.next_id, f0, "Compression Médias", 1));
        self.next_id += 1;
        self.rules.push(sample_rule(self.next_id, self.folders[1].id, "Import Musique", 2));
        self.next_id += 1;
        self.monthly_used = 42;
        let now = SystemTime::now();
        for i in 0..6 {
            self.history.push(HistoryEntry {
                batch_id: 1,
                rule_name: ["Tri Bureau & Téléchargements", "Compression Médias"][i % 2].into(),
                action_label: ["Move to folder", "Rename with pattern", "Compress (ZIP)"][i % 3].into(),
                original_path: PathBuf::from(format!("{home}/Downloads/file-{i}.pdf")),
                result_path: PathBuf::from(format!("{home}/Documents/Archive/2026-11/file-{i}.pdf")),
                undo: if i % 3 == 0 {
                    Some(Undo {
                        kind: ActionKind::Move,
                        from: PathBuf::from(format!("{home}/Documents/Archive/2026-11/file-{i}.pdf")),
                        to: PathBuf::from(format!("{home}/Downloads/file-{i}.pdf")),
                    })
                } else {
                    None
                },
                reversible: i % 3 == 0,
                status: if i == 5 { Status::BlockedByConflict } else { Status::Applied },
                message: if i == 5 {
                    "un fichier existe déjà à la destination".into()
                } else {
                    "déplacé → Archive/2026-11".into()
                },
                created: now - std::time::Duration::from_secs(i as u64 * 1500),
            });
        }
        self.last_run = Some((12, 18, 30));
        self.last_run_dry = false;
        self.next_batch = 2;
    }

    fn accent(&self) -> Color32 {
        ACCENTS[self.accent].1
    }

    fn toast(&mut self, msg: impl Into<String>) {
        self.toast = Some(msg.into());
    }

    /// Photographie de tout ce qui doit survivre à la fermeture.
    fn snapshot(&self) -> State {
        State {
            schema_version: store::SCHEMA_VERSION,
            folders: self.folders.clone(),
            rules: self.rules.clone(),
            history: self.history.clone(),
            settings: Settings {
                accent: self.accent,
                menu_bar: self.menu_bar,
                notify: self.notify,
                animate_bg: self.animate_bg,
                retention_days: self.retention_days,
                app_enabled: self.app_enabled,
                launch_at_login: self.launch_at_login,
            },
            next_id: self.next_id,
            next_batch: self.next_batch,
            monthly_used: self.monthly_used,
            last_run: self.last_run,
            last_run_dry: self.last_run_dry,
        }
    }

    fn restore(&mut self, state: State) {
        let max_id = state
            .folders
            .iter()
            .map(|f| f.id)
            .chain(state.rules.iter().map(|r| r.id))
            .max()
            .unwrap_or(0);
        self.folders = state.folders;
        self.rules = state.rules;
        self.history = state.history;
        self.accent = state.settings.accent.min(ACCENTS.len() - 1);
        self.menu_bar = state.settings.menu_bar;
        self.notify = state.settings.notify;
        self.animate_bg = state.settings.animate_bg;
        self.retention_days = state.settings.retention_days.clamp(1, 365);
        self.app_enabled = state.settings.app_enabled;
        self.launch_at_login = state.settings.launch_at_login;
        self.next_id = state.next_id.max(max_id + 1);
        self.next_batch = state.next_batch.max(1);
        self.monthly_used = state.monthly_used;
        self.last_run = state.last_run;
        self.last_run_dry = state.last_run_dry;
        // La rétention s'applique dès le chargement.
        purge_history(&mut self.history, self.retention_days, SystemTime::now());
    }

    /// Premier lancement : rien à faire. Sinon restaure les données ; un fichier illisible est mis de côté.
    fn load_saved_state(&mut self) {
        let path = store::state_path();
        match store::load(&path) {
            Loaded::Fresh => {}
            Loaded::Restored(state) => self.restore(*state),
            Loaded::Unreadable { backup, reason } => {
                self.toast(format!("Données illisibles ({reason}) : copie gardée dans {}", backup.display()));
            }
        }
        self.last_saved = store::to_json(&self.snapshot());
        self.store_path = Some(path);
    }

    /// Écrit l'état s'il a changé depuis la dernière sauvegarde.
    fn persist(&mut self) {
        let Some(path) = self.store_path.clone() else { return };
        let json = store::to_json(&self.snapshot());
        if json == self.last_saved {
            return;
        }
        match store::save(&path, &json) {
            Ok(()) => self.last_saved = json,
            Err(e) => self.toast(format!("Sauvegarde impossible : {e}")),
        }
    }

    fn run_batch(&mut self, dry_run: bool) {
        if self.folders.is_empty() {
            self.toast("Ajoutez d'abord un dossier");
            return;
        }
        let mut runner = Runner {
            monthly_used: self.monthly_used,
            batch_id: self.next_batch,
            dry_run,
            commands: Box::new(crate::system::SystemCommands),
            only_paths: None,
        };
        let outcome = runner.run(&self.folders, &self.rules, SystemTime::now());
        self.next_batch += 1;
        if !dry_run {
            self.monthly_used = runner.monthly_used;
            self.ignore_in_watcher(&outcome.history);
        }
        let actions = outcome.history.len();
        self.last_run = Some((outcome.matched, actions, outcome.scanned));
        self.last_run_dry = dry_run;
        for e in outcome.history {
            self.history.push(e);
        }
        purge_history(&mut self.history, self.retention_days, SystemTime::now());
        if dry_run {
            self.toast(format!("Aperçu : {} fichier(s) concerné(s)", outcome.matched));
        } else {
            self.toast(format!("{} fichier(s) organisé(s)", outcome.matched));
        }
    }

    fn undo(&mut self, idx: usize) {
        if let Some(e) = self.history.get_mut(idx) {
            match undo_entry(e) {
                Ok(()) => {
                    e.status = Status::Undone;
                    e.message = "restored".into();
                    let paths = vec![e.original_path.clone(), e.result_path.clone()];
                    if let Some(w) = &self.watcher {
                        w.ignore(paths);
                    }
                    self.toast("Annulé");
                }
                Err(msg) => self.toast(format!("Annulation impossible : {msg}")),
            }
        }
    }

    /// Les fichiers que l'appli vient de déplacer ne doivent pas être retraités par la surveillance.
    fn ignore_in_watcher(&self, history: &[HistoryEntry]) {
        if let Some(w) = &self.watcher {
            w.ignore(history.iter().flat_map(|e| [e.original_path.clone(), e.result_path.clone()]).collect());
        }
    }

    /// Envoie à la surveillance les dossiers et règles actuels, seulement s'ils ont changé.
    fn sync_watcher(&mut self) {
        let Some(w) = &self.watcher else { return };
        let key = serde_json::to_string(&(&self.folders, &self.rules, self.app_enabled)).unwrap_or_default();
        if key != self.watch_key {
            self.watch_key = key;
            w.set_config(WatchConfig { folders: self.folders.clone(), rules: self.rules.clone(), enabled: self.app_enabled });
        }
    }

    /// Reprend les résultats des traitements automatiques et les ajoute à l'historique.
    fn poll_watcher(&mut self) {
        let Some(w) = &self.watcher else { return };
        let mut results = Vec::new();
        while let Some(r) = w.try_recv() {
            results.push(r);
        }
        for mut r in results {
            for e in &mut r.history {
                e.batch_id = self.next_batch;
            }
            self.next_batch += 1;
            self.monthly_used += r.monthly_delta;
            self.last_run = Some((r.matched, r.history.len(), r.scanned));
            self.last_run_dry = false;
            self.toast(format!("{} fichier(s) organisé(s) automatiquement", r.matched));
            self.history.extend(r.history);
        }
        purge_history(&mut self.history, self.retention_days, SystemTime::now());
    }
}

fn sample_rule(id: u64, folder_id: u64, name: &str, prio: i64) -> Rule {
    let mut r = Rule {
        id,
        folder_id,
        name: name.into(),
        enabled: true,
        match_all: true,
        recursion_depth: 0,
        priority: prio,
        conditions: Vec::new(),
        actions: Vec::new(),
    };
    match prio {
        0 => {
            r.conditions.push(Condition { field: Field::Extension, op: Op::Equals, value: ".png, .jpg".into() });
            r.conditions.push(Condition { field: Field::Size, op: Op::Greater, value: "50 MB".into() });
            let mut a = Action::new(ActionKind::Move);
            a.params.insert("destination".into(), "~/Pictures/Screenshots-Classés".into());
            a.params.insert(ON_CONFLICT_PARAM_KEY.into(), CONFLICT_NUMBER.into());
            r.actions.push(a);
            let mut b = Action::new(ActionKind::Rename);
            b.params.insert("pattern".into(), "{current_date}-{name}.{extension}".into());
            r.actions.push(b);
        }
        1 => {
            r.conditions.push(Condition { field: Field::Kind, op: Op::Is, value: "video".into() });
            let mut c = Action::new(ActionKind::Compress);
            c.params.insert("pattern".into(), "{name}-{timestamp}".into());
            r.actions.push(c);
        }
        _ => {
            r.conditions.push(Condition { field: Field::Kind, op: Op::Is, value: "audio".into() });
            let mut a = Action::new(ActionKind::Move);
            a.params.insert("destination".into(), "~/Music/Audio-Classés".into());
            a.params.insert(ON_CONFLICT_PARAM_KEY.into(), CONFLICT_NUMBER.into());
            r.actions.push(a);
        }
    }
    r
}

// ------------------------------------------------------------ egui helpers

// ------------------------------------------------------------ fond animé (esprit NEAT)
//
// Principe repris de NEAT (https://neat.rebase.pro) : un plan 3D ondulé par du bruit, dont la couleur
// vient de plusieurs champs de bruit qui se disputent le terrain (seuils + `smoothstep`),
// éclairé comme un tissu (reflets et ombres selon la pente des vagues). Code réécrit ici, sur le CPU.

/// Table de permutation du bruit (mélange déterministe de 0..255, doublée).
fn perm_table() -> &'static [u8; 512] {
    static TABLE: std::sync::OnceLock<[u8; 512]> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        let mut p: Vec<u8> = (0..=255u8).collect();
        let mut state = 0x2545_F491_4F6C_DD1Du64;
        for i in (1..256).rev() {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            p.swap(i, (state % (i as u64 + 1)) as usize);
        }
        let mut out = [0u8; 512];
        for i in 0..512 {
            out[i] = p[i & 255];
        }
        out
    })
}

/// Bruit simplexe 3D (algorithme de Gustavson), valeurs dans ≈ [-1, 1].
fn snoise(x: f32, y: f32, z: f32) -> f32 {
    const GRAD: [[f32; 3]; 12] = [
        [1.0, 1.0, 0.0], [-1.0, 1.0, 0.0], [1.0, -1.0, 0.0], [-1.0, -1.0, 0.0],
        [1.0, 0.0, 1.0], [-1.0, 0.0, 1.0], [1.0, 0.0, -1.0], [-1.0, 0.0, -1.0],
        [0.0, 1.0, 1.0], [0.0, -1.0, 1.0], [0.0, 1.0, -1.0], [0.0, -1.0, -1.0],
    ];
    const F3: f32 = 1.0 / 3.0;
    const G3: f32 = 1.0 / 6.0;
    let perm = perm_table();
    let s = (x + y + z) * F3;
    let (i, j, k) = ((x + s).floor(), (y + s).floor(), (z + s).floor());
    let t = (i + j + k) * G3;
    let (x0, y0, z0) = (x - (i - t), y - (j - t), z - (k - t));
    let (i1, j1, k1, i2, j2, k2) = if x0 >= y0 {
        if y0 >= z0 {
            (1, 0, 0, 1, 1, 0)
        } else if x0 >= z0 {
            (1, 0, 0, 1, 0, 1)
        } else {
            (0, 0, 1, 1, 0, 1)
        }
    } else if y0 < z0 {
        (0, 0, 1, 0, 1, 1)
    } else if x0 < z0 {
        (0, 1, 0, 0, 1, 1)
    } else {
        (0, 1, 0, 1, 1, 0)
    };
    let (x1, y1, z1) = (x0 - i1 as f32 + G3, y0 - j1 as f32 + G3, z0 - k1 as f32 + G3);
    let (x2, y2, z2) = (x0 - i2 as f32 + 2.0 * G3, y0 - j2 as f32 + 2.0 * G3, z0 - k2 as f32 + 2.0 * G3);
    let (x3, y3, z3) = (x0 - 1.0 + 3.0 * G3, y0 - 1.0 + 3.0 * G3, z0 - 1.0 + 3.0 * G3);
    let (ii, jj, kk) = ((i as i32 & 255) as usize, (j as i32 & 255) as usize, (k as i32 & 255) as usize);
    let gi = |a: usize, b: usize, c: usize| (perm[a + perm[b + perm[c] as usize] as usize] % 12) as usize;
    let contrib = |g: usize, x: f32, y: f32, z: f32| {
        let t = 0.6 - x * x - y * y - z * z;
        if t < 0.0 {
            0.0
        } else {
            let t2 = t * t;
            t2 * t2 * (GRAD[g][0] * x + GRAD[g][1] * y + GRAD[g][2] * z)
        }
    };
    let n0 = contrib(gi(ii, jj, kk), x0, y0, z0);
    let n1 = contrib(gi(ii + i1, jj + j1, kk + k1), x1, y1, z1);
    let n2 = contrib(gi(ii + i2, jj + j2, kk + k2), x2, y2, z2);
    let n3 = contrib(gi(ii + 1, jj + 1, kk + 1), x3, y3, z3);
    32.0 * (n0 + n1 + n2 + n3)
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Vitesse du fond (secondes de « temps de bruit » par seconde réelle).
const BG_SPEED: f64 = 0.055;
/// Largeur du plan en unités de bruit (comme NEAT : 50), fréquence des vagues, amplitude, pression des couleurs.
const BG_PLANE_W: f32 = 50.0;
const BG_WAVE_FREQ: f32 = 0.055;
const BG_WAVE_AMP: f32 = 2.4;
const BG_COLOR_PRESSURE: f32 = 0.95;
const BG_BLENDING: f32 = 0.5;
/// Reflets et ombres du tissu.
const BG_HIGHLIGHTS: f32 = 0.07;
const BG_SHADOWS: f32 = 0.06;

/// Couleur du fond au point (u, v) ∈ [0,1]² à l'instant `t` (secondes).
/// Mêmes couleurs que le thème : nuit, orange accent, bleu nuit, et leurs variantes sombres.
fn bg_color(u: f64, v: f64, t: f64, aspect: f64, accent: Color32) -> Color32 {
    let tt = ((t * BG_SPEED) % 7200.0) as f32;
    let x = (u as f32 - 0.5) * BG_PLANE_W;
    let y = (v as f32 - 0.5) * BG_PLANE_W / aspect as f32;

    // 1. Vagues : élévation du plan et pente (différences finies) → normale.
    let h = 0.35;
    let disp = |px: f32, py: f32| snoise(BG_WAVE_FREQ * px + tt, BG_WAVE_FREQ * py + tt, tt);
    let d = disp(x, y);
    let sx = BG_WAVE_AMP * (disp(x + h, y) - d) / h;
    let sy = BG_WAVE_AMP * (disp(x, y + h) - d) / h;
    let inv = 1.0 / (sx * sx + sy * sy + 1.0).sqrt();
    let n = [-sx * inv, -sy * inv, inv];

    // 2. Couleurs : chaque champ de bruit pousse sa teinte plus ou moins loin (seuils + smoothstep).
    let mix = |a: Color32, b: Color32, k: f32| {
        [
            a.r() as f32 + (b.r() as f32 - a.r() as f32) * k,
            a.g() as f32 + (b.g() as f32 - a.g() as f32) * k,
            a.b() as f32 + (b.b() as f32 - a.b() as f32) * k,
        ]
    };
    let lerp_col = |a: Color32, b: Color32, k: f32| {
        let m = mix(a, b, k);
        Color32::from_rgb(m[0] as u8, m[1] as u8, m[2] as u8)
    };
    let palette = [
        CANVAS,
        lerp_col(CANVAS, accent, 0.58),
        lerp_col(CANVAS, SKY_DEEP, 0.50),
        lerp_col(CANVAS, accent, 0.28),
        lerp_col(CANVAS, SKY_DEEP, 0.26),
    ];
    let (cx, cy) = (x / 50.0 * BG_COLOR_PRESSURE * BG_COLOR_PRESSURE, y / 50.0 * BG_COLOR_PRESSURE * BG_COLOR_PRESSURE);
    let mut c = [CANVAS.r() as f32, CANVAS.g() as f32, CANVAS.b() as f32];
    for (i, col) in palette.iter().enumerate().skip(1) {
        let fi = i as f32;
        let seed = 13.0 + fi * 7.0;
        let noise = snoise(cx + tt * (1.0 + fi) / 30.0 * 2.0 + seed, cy + seed, tt * (1.0 + fi) * 0.11 + seed) - 0.1 * fi
            + 0.5 * BG_BLENDING;
        let noise = noise.clamp(0.0, 0.9 + fi * 0.02);
        let k = smoothstep(0.0, BG_BLENDING, noise);
        c = {
            let m = [
                c[0] + (col.r() as f32 - c[0]) * k,
                c[1] + (col.g() as f32 - c[1]) * k,
                c[2] + (col.b() as f32 - c[2]) * k,
            ];
            m
        };
    }

    // 3. Éclairage du tissu : lumière rasante venue d'en haut à droite, reflet spéculaire.
    let light = [0.577_35, 0.577_35, 0.577_35];
    let diffuse = (n[0] * light[0] + n[1] * light[1] + n[2] * light[2]).max(0.0);
    let half = [light[0], light[1], light[2] + 1.0];
    let hl = (half[0] * half[0] + half[1] * half[1] + half[2] * half[2]).sqrt();
    let spec = ((n[0] * half[0] + n[1] * half[1] + n[2] * half[2]) / hl).max(0.0).powi(32);
    let height_shadow = (1.0 - d) * (1.0 - d);
    let shade = spec * BG_HIGHLIGHTS + d * BG_HIGHLIGHTS * 0.5 - height_shadow * BG_SHADOWS * 0.5 - (1.0 - diffuse) * BG_SHADOWS * 0.5;
    let out = |v: f32| (v + shade * 255.0).clamp(0.0, 255.0) as u8;
    Color32::from_rgb(out(c[0]), out(c[1]), out(c[2]))
}

/// Fond « verre » animé : maillage coloré par sommet (interpolé par le GPU), recalculé à chaque image.
fn backdrop(ui: &mut egui::Ui, rect: Rect, accent: Color32, t: f64) {
    const COLS: u32 = 72;
    const ROWS: u32 = 46;
    let aspect = (rect.width() / rect.height().max(1.0)) as f64;
    let mut mesh = egui::epaint::Mesh::default();
    for j in 0..=ROWS {
        for i in 0..=COLS {
            let (u, v) = (i as f32 / COLS as f32, j as f32 / ROWS as f32);
            let pos = Pos2::new(rect.left() + rect.width() * u, rect.top() + rect.height() * v);
            mesh.colored_vertex(pos, bg_color(u as f64, v as f64, t, aspect, accent));
        }
    }
    let stride = COLS + 1;
    for j in 0..ROWS {
        for i in 0..COLS {
            let a = j * stride + i;
            mesh.add_triangle(a, a + 1, a + stride);
            mesh.add_triangle(a + 1, a + stride + 1, a + stride);
        }
    }
    ui.painter().add(egui::Shape::mesh(mesh));
}

/// Fondu du bas : les sommets reprennent exactement la couleur du fond animé, avec une opacité
/// qui monte de 0 à ~95 % ; les cartes s'effacent dans le fond au lieu d'être coupées net.
fn bottom_fade(ui: &mut egui::Ui, area: Rect, screen: Rect, t: f64, accent: Color32) {
    const COLS: u32 = 24;
    const ROWS: u32 = 8;
    let h = BOTTOM_FADE_H.min(area.height());
    let top = area.bottom() - h;
    let aspect = (screen.width() / screen.height().max(1.0)) as f64;
    let mut mesh = egui::epaint::Mesh::default();
    for j in 0..=ROWS {
        let f = j as f32 / ROWS as f32;
        let y = top + h * f;
        let alpha = (f * f * (3.0 - 2.0 * f) * 0.95 * 255.0) as u8;
        for i in 0..=COLS {
            let x = area.left() + area.width() * i as f32 / COLS as f32;
            let u = ((x - screen.left()) / screen.width()) as f64;
            let v = ((y - screen.top()) / screen.height()) as f64;
            let c = bg_color(u, v, t, aspect, accent);
            mesh.colored_vertex(Pos2::new(x, y), Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha));
        }
    }
    let stride = COLS + 1;
    for j in 0..ROWS {
        for i in 0..COLS {
            let a = j * stride + i;
            mesh.add_triangle(a, a + 1, a + stride);
            mesh.add_triangle(a + 1, a + stride + 1, a + stride);
        }
    }
    ui.painter().add(egui::Shape::mesh(mesh));
}

/// Hauteur du fondu du bas d'écran et de la marge basse du défilement.
const BOTTOM_FADE_H: f32 = 80.0;

/// Écart unique entre les cartes (horizontal et vertical) sur l'écran Aperçu.
const GAP: f32 = 20.0;
/// Hauteur utile commune des deux cartes de listes (titre + 4 lignes) : elles restent alignées.
const LIST_CARD_MIN_H: f32 = 258.0;

/// Rayon des cartes de verre.
const CARD_RADIUS: f32 = 16.0;

/// Ombre portée diffuse, dessinée **autour** de la carte seulement (anneau dégradé) :
/// le verre étant translucide, une ombre posée dessous salirait l'intérieur de la carte.
/// Le bord interne colle à la carte (opacité max) ; le bord externe, élargi de `blur` et décalé de `offset`, vaut 0.
/// Points (centre de coin, direction) qui décrivent un rectangle à coins arrondis, dans l'ordre.
fn corner_dirs(rect: Rect, r: f32, arc: usize) -> Vec<(Pos2, Vec2)> {
    let quarter = std::f32::consts::FRAC_PI_2;
    let corners = [
        (Pos2::new(rect.right() - r, rect.bottom() - r), 0.0),
        (Pos2::new(rect.left() + r, rect.bottom() - r), quarter),
        (Pos2::new(rect.left() + r, rect.top() + r), 2.0 * quarter),
        (Pos2::new(rect.right() - r, rect.top() + r), 3.0 * quarter),
    ];
    let mut out = Vec::new();
    for (center, start) in corners {
        for k in 0..=arc {
            out.push((center, Vec2::angled(start + quarter * k as f32 / arc as f32)));
        }
    }
    out
}

fn ring_indices(mesh: &mut egui::epaint::Mesh, rings: u32, per_ring: u32) {
    for ring in 0..rings {
        for i in 0..per_ring {
            let j = (i + 1) % per_ring;
            let (a0, a1) = (ring * per_ring, (ring + 1) * per_ring);
            mesh.add_triangle(a0 + i, a1 + i, a1 + j);
            mesh.add_triangle(a0 + i, a1 + j, a0 + j);
        }
    }
}

/// Ombre portée diffuse, dessinée **autour** de la carte seulement (anneau dégradé) :
/// le verre étant translucide, une ombre posée dessous salirait l'intérieur de la carte.
/// Le bord interne colle à la carte (opacité max) ; le bord externe, élargi de `blur` et décalé de `offset`, vaut 0.
fn shadow_ring(rect: Rect, r: f32, blur: f32, offset: Vec2, alpha: u8) -> egui::Shape {
    let dirs = corner_dirs(rect, r, 8);
    let inner: Vec<Pos2> = dirs.iter().map(|(c, d)| *c + *d * r).collect();
    let outer: Vec<Pos2> = dirs.iter().map(|(c, d)| *c + offset + *d * (r + blur)).collect();
    let stops = [(0.0f32, 1.0f32), (0.35, 0.55), (0.7, 0.2), (1.0, 0.0)];
    let mut mesh = egui::epaint::Mesh::default();
    for (t, m) in stops {
        let color = Color32::from_rgba_unmultiplied(0, 0, 0, (alpha as f32 * m) as u8);
        for i in 0..inner.len() {
            mesh.colored_vertex(inner[i].lerp(outer[i], t), color);
        }
    }
    ring_indices(&mut mesh, stops.len() as u32 - 1, inner.len() as u32);
    egui::Shape::mesh(mesh)
}

/// Halos de couleur **contenus dans un rectangle à coins arrondis** (maillage qui suit le contour,
/// du bord vers le centre) : aucun angle carré ne dépasse de la carte.
/// `layers` : (centre, rayon, couleur, opacité max).
fn glow_in_rounded(painter: &egui::Painter, rect: Rect, r: f32, layers: &[(Pos2, f32, Color32, u8)]) {
    let outline: Vec<Pos2> = corner_dirs(rect, r, 8).iter().map(|(c, d)| *c + *d * r).collect();
    let n = outline.len() as u32;
    let center = rect.center();
    let color_at = |p: Pos2| -> Color32 {
        let (mut pr, mut pg, mut pb, mut pa) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
        for (c, radius, col, alpha) in layers {
            let d = p.distance(*c) / radius;
            let a = *alpha as f32 / 255.0 * (-d * d).exp();
            pr += col.r() as f32 * a;
            pg += col.g() as f32 * a;
            pb += col.b() as f32 * a;
            pa += a;
        }
        let pa = pa.min(1.0);
        let cap = pa * 255.0;
        Color32::from_rgba_premultiplied(pr.min(cap) as u8, pg.min(cap) as u8, pb.min(cap) as u8, cap as u8)
    };
    let scales = [1.0f32, 0.72, 0.44, 0.16];
    let mut mesh = egui::epaint::Mesh::default();
    for s in scales {
        for p in &outline {
            let q = center + (*p - center) * s;
            mesh.colored_vertex(q, color_at(q));
        }
    }
    let hub = mesh.vertices.len() as u32;
    mesh.colored_vertex(center, color_at(center));
    ring_indices(&mut mesh, scales.len() as u32 - 1, n);
    let last = (scales.len() as u32 - 1) * n;
    for i in 0..n {
        mesh.add_triangle(hub, last + i, last + (i + 1) % n);
    }
    painter.add(egui::Shape::mesh(mesh));
}

/// Liseré blanc fin des cartes (« 30 % » perçu : l'alpha brut est bien plus bas, le mélange étant linéaire).
const CARD_BORDER: Color32 = Color32::from_rgba_premultiplied(60, 60, 60, 60);

/// Surface de verre : ombre diffuse autour, voile blanc translucide et liseré fin.
fn glass_shapes(rect: Rect) -> egui::Shape {
    let blur = (rect.height() * 0.18).clamp(14.0, 24.0);
    egui::Shape::Vec(vec![
        shadow_ring(rect, CARD_RADIUS, blur, Vec2::new(0.0, blur * 0.35), 34),
        egui::Shape::rect_filled(rect, Rounding::same(CARD_RADIUS), glass(11)),
        egui::Shape::rect_stroke(rect, Rounding::same(CARD_RADIUS), Stroke::new(1.0f32, CARD_BORDER)),
    ])
}

/// Carte de verre de taille fixe.
fn card(ui: &mut egui::Ui, size: Vec2, add: impl FnOnce(&mut egui::Ui)) {
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    ui.painter().add(glass_shapes(rect));
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(Vec2::new(22.0, 18.0)))
            .layout(Layout::top_down(Align::Min)),
    );
    child.set_min_size(size - Vec2::new(44.0, 36.0));
    add(&mut child);
}

fn pill(ui: &mut egui::Ui, text: &str, accent: Color32, primary: bool, big: bool) -> egui::Response {
    let h = if big { 52.0 } else { 34.0 };
    let pad = if big { 30.0 } else { 18.0 };
    let font = if big { 17.0 } else { 13.5 };
    let galley = ui.painter().layout_no_wrap(text.to_string(), egui::FontId::proportional(font), Color32::WHITE);
    let size = Vec2::new(galley.size().x + pad * 2.0, h);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    if !response.enabled() {
        ui.painter().rect_filled(rect, Rounding::same(h / 2.0), glass(30));
    } else {
        let hover = response.hovered();
        let fill = if primary {
            if hover { lighten(accent, 18) } else { accent }
        } else {
            with_alpha(accent, if hover { 56 } else { 34 })
        };
        if primary {
            for (off, alpha) in [(5.0f32, 26u8), (2.0, 40)] {
                ui.painter().rect_filled(
                    rect.translate(Vec2::new(0.0, off)),
                    Rounding::same(h / 2.0),
                    Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), alpha),
                );
            }
        }
        ui.painter().rect_filled(rect, Rounding::same(h / 2.0), fill);
        let tc = Color32::WHITE;
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::proportional(font),
            tc,
        );
    }
    response
}

fn lighten(c: Color32, d: u8) -> Color32 {
    Color32::from_rgb(
        c.r().saturating_add(d),
        c.g().saturating_add(d),
        c.b().saturating_add(d),
    )
}

fn icon_char(icon: Icon) -> char {
    match icon {
        Icon::Chart => '\u{e871}',   // dashboard
        Icon::Folder => '\u{f775}',  // folder_managed
        Icon::List => '\u{f1c2}',    // rule
        Icon::Clock => '\u{e8b3}',   // history
        Icon::Sliders => '\u{e429}', // tune
        Icon::Funnel => '\u{ef4f}',  // filter_alt
        Icon::Bolt => '\u{ea0b}',    // bolt
        Icon::Grip => '\u{e945}',    // drag_indicator
        Icon::Bell => '\u{e7f5}',    // notifications
        Icon::Gear => '\u{e8b8}',    // settings
        Icon::Search => '\u{ef7a}',  // search
        Icon::Cross => '\u{e5cd}',   // close
        Icon::Add => '\u{e145}',     // add
        Icon::Trash => '\u{e92e}',   // delete
        Icon::FolderOpen => '\u{e2c8}', // folder_open
        Icon::Refresh => '\u{e5d5}',  // refresh
        Icon::Tick => '\u{e668}',     // check
        Icon::Undo => '\u{e166}',     // undo
    }
}

/// Glyphe Material Symbols centré en `c` (`s` = taille de la boîte d'icône, en px).
fn draw_icon(painter: &egui::Painter, c: Pos2, icon: Icon, color: Color32, s: f32) {
    painter.text(
        c,
        egui::Align2::CENTER_CENTER,
        icon_char(icon).to_string(),
        egui::FontId::new(s * 1.25, icons_family()),
        color,
    );
}

#[derive(Clone, Copy, PartialEq)]
enum Btn {
    Primary,
    Ghost,
    Sky,
    Danger,
}

/// Bouton 32 px de haut (rayon 8) : primaire orange, fantôme, bleu azur ou danger.
fn btn(ui: &mut egui::Ui, text: &str, kind: Btn, accent: Color32, icon: Option<Icon>) -> egui::Response {
    let font = egui::FontId::new(14.5, semibold());
    let galley = ui.painter().layout_no_wrap(text.to_string(), font.clone(), Color32::WHITE);
    let icon_w = if icon.is_some() { 20.0 } else { 0.0 };
    let size = Vec2::new(galley.size().x + 28.0 + icon_w, 32.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let enabled = resp.enabled();
    let hover = resp.hovered() && enabled;
    let (fill, fg) = match kind {
        Btn::Primary => (if hover { lighten(accent, 16) } else { accent }, Color32::WHITE),
        Btn::Ghost => (glass(if hover { 28 } else { 16 }), text_primary()),
        Btn::Sky => (with_alpha(SKY, if hover { 60 } else { 36 }), SKY),
        Btn::Danger => (with_alpha(RED, if hover { 60 } else { 34 }), RED),
    };
    let (fill, fg) = if enabled { (fill, fg) } else { (glass(12), with_alpha(Color32::WHITE, 70)) };
    let p = ui.painter();
    if kind == Btn::Primary && enabled {
        p.rect_filled(rect.expand(3.0), Rounding::same(10.0), with_alpha(accent, 40));
    }
    if matches!(kind, Btn::Sky | Btn::Danger) {
        p.rect_filled(rect, Rounding::same(8.0), with_alpha(CANVAS, 140));
    }
    p.rect_filled(rect, Rounding::same(8.0), fill);
    let mut x = rect.left() + 14.0;
    if let Some(ic) = icon {
        draw_icon(p, Pos2::new(x + 7.0, rect.center().y), ic, fg, 15.0);
        x += icon_w;
    }
    p.text(Pos2::new(x, rect.center().y), egui::Align2::LEFT_CENTER, text, font, fg);
    resp
}

/// Pastille d'information (22 px) : badge compteur, version, statut.
fn chip(ui: &mut egui::Ui, text: &str, fg: Color32, bg: Color32) -> egui::Response {
    let font = egui::FontId::new(14.0, semibold());
    let galley = ui.painter().layout_no_wrap(text.to_string(), font, fg);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(galley.size().x + 18.0, 22.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, Rounding::same(11.0), with_alpha(CANVAS, 140));
    ui.painter().rect_filled(rect, Rounding::same(11.0), bg);
    let pos = Pos2::new(rect.left() + 9.0, rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(pos, galley, fg);
    resp
}

/// Pastille cliquable (22 px) : même rendu que `chip`, avec survol.
fn chip_button(ui: &mut egui::Ui, text: &str, fg: Color32, bg: Color32) -> egui::Response {
    let font = egui::FontId::new(14.0, semibold());
    let galley = ui.painter().layout_no_wrap(text.to_string(), font, fg);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(galley.size().x + 18.0, 22.0), egui::Sense::click());
    ui.painter().rect_filled(rect, Rounding::same(11.0), with_alpha(CANVAS, 140));
    ui.painter().rect_filled(rect, Rounding::same(11.0), if resp.hovered() { with_alpha(fg, 70) } else { bg });
    let pos = Pos2::new(rect.left() + 9.0, rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(pos, galley, fg);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Interrupteur façon macOS.
fn check_box(ui: &mut egui::Ui, on: &mut bool, label: &str, accent: Color32) -> egui::Response {
    let font = egui::FontId::proportional(15.0);
    let galley = ui.painter().layout_no_wrap(label.to_string(), font.clone(), text_primary());
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(26.0 + galley.size().x, 22.0), egui::Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let b = Rect::from_min_size(Pos2::new(rect.left(), rect.center().y - 9.0), Vec2::splat(18.0));
    let fill = if *on { accent } else { glass(if resp.hovered() { 36 } else { 24 }) };
    ui.painter().rect_filled(b, Rounding::same(5.0), fill);
    if *on {
        let st = Stroke::new(2.0f32, Color32::WHITE);
        let m = Pos2::new(b.left() + 8.0, b.bottom() - 5.0);
        ui.painter().line_segment([Pos2::new(b.left() + 4.5, b.center().y), m], st);
        ui.painter().line_segment([m, Pos2::new(b.right() - 4.0, b.top() + 5.0)], st);
    }
    ui.painter().text(
        Pos2::new(rect.left() + 26.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        font,
        text_primary(),
    );
    resp
}

/// Curseur fin (rail + pastille blanche, sans contour) ; true si la valeur change.
fn slim_slider(ui: &mut egui::Ui, value: &mut u32, min: u32, max: u32, accent: Color32) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(170.0, 22.0), egui::Sense::click_and_drag());
    let (x0, x1) = (rect.left() + 8.0, rect.right() - 8.0);
    let mut changed = false;
    if resp.dragged() || resp.clicked() {
        if let Some(p) = resp.interact_pointer_pos() {
            let t = ((p.x - x0) / (x1 - x0)).clamp(0.0, 1.0);
            let v = (min as f32 + t * (max - min) as f32).round() as u32;
            if v != *value {
                *value = v;
                changed = true;
            }
        }
    }
    let t = (*value - min) as f32 / (max - min) as f32;
    let y = rect.center().y;
    let knob = Pos2::new(x0 + t * (x1 - x0), y);
    ui.painter().rect_filled(Rect::from_min_max(Pos2::new(x0, y - 2.0), Pos2::new(x1, y + 2.0)), Rounding::same(2.0), glass(36));
    ui.painter().rect_filled(Rect::from_min_max(Pos2::new(x0, y - 2.0), Pos2::new(knob.x, y + 2.0)), Rounding::same(2.0), accent);
    ui.painter().circle_filled(knob + Vec2::new(0.0, 1.0), 8.5, Color32::from_rgba_unmultiplied(0, 0, 0, 70));
    ui.painter().circle_filled(knob, 8.0, Color32::WHITE);
    changed
}

fn toggle(ui: &mut egui::Ui, on: &mut bool, accent: Color32) -> egui::Response {
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(38.0, 22.0), egui::Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let t = ui.ctx().animate_bool(resp.id, *on);
    let fill = if *on { accent } else { Color32::from_rgb(0x3a, 0x3d, 0x4a) };
    if *on {
        ui.painter().rect_filled(rect.expand(2.0), Rounding::same(13.0), with_alpha(accent, 40));
    }
    ui.painter().rect_filled(rect, Rounding::same(11.0), fill);
    let x = egui::lerp((rect.left() + 11.0)..=(rect.right() - 11.0), t);
    ui.painter().circle_filled(Pos2::new(x, rect.center().y), 8.5, Color32::WHITE);
    resp
}

/// Contrôle segmenté ; renvoie l'index cliqué.
fn segmented(ui: &mut egui::Ui, id: egui::Id, labels: &[&str], selected: usize, active: Color32) -> Option<usize> {
    let font = egui::FontId::new(14.0, semibold());
    let widths: Vec<f32> = labels
        .iter()
        .map(|l| ui.painter().layout_no_wrap(l.to_string(), font.clone(), Color32::WHITE).size().x + 28.0)
        .collect();
    let total = widths.iter().sum::<f32>() + 8.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(total, 34.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, Rounding::same(9.0), with_alpha(CANVAS, 150));
    let mut x = rect.left() + 4.0;
    let mut out = None;
    for (i, label) in labels.iter().enumerate() {
        let seg = Rect::from_min_size(Pos2::new(x, rect.top() + 4.0), Vec2::new(widths[i], 26.0));
        let resp = ui.interact(seg, id.with(i), egui::Sense::click());
        let on = i == selected;
        if on {
            ui.painter().rect_filled(seg, Rounding::same(6.0), active);
        } else if resp.hovered() {
            ui.painter().rect_filled(seg, Rounding::same(6.0), glass(18));
        }
        ui.painter().text(
            seg.center(),
            egui::Align2::CENTER_CENTER,
            label,
            font.clone(),
            if on { Color32::WHITE } else { text_dim() },
        );
        if resp.clicked() {
            out = Some(i);
        }
        x += widths[i];
    }
    out
}

/// Bouton-icône carré (28 px) ; `danger` = teinte rouge au survol. true si cliqué.
fn icon_button(ui: &mut egui::Ui, icon: Icon, tip: &str, danger: bool) -> bool {
    let (r, resp) = ui.allocate_exact_size(Vec2::splat(28.0), egui::Sense::click());
    let hot = resp.hovered();
    if hot {
        ui.painter().rect_filled(r, Rounding::same(7.0), if danger { with_alpha(RED, 36) } else { glass(24) });
    }
    let col = if hot && danger { RED } else if hot { text_primary() } else { text_dim() };
    draw_icon(&ui.painter().clone(), r.center(), icon, col, 16.0);
    resp.on_hover_text(tip).clicked()
}

fn cross_button(ui: &mut egui::Ui, tip: &str) -> bool {
    icon_button(ui, Icon::Cross, tip, true)
}

/// Ligne glissable d'une liste (conditions = 0, actions = 1).
#[derive(Clone, Copy, PartialEq)]
struct DragRow {
    list: u8,
    idx: usize,
}

/// Indicateur de dépôt + détection du relâchement : renvoie (source, position d'insertion).
fn drop_target(ui: &egui::Ui, resp: &egui::Response, list: u8, idx: usize) -> Option<(usize, usize)> {
    let ptr = ui.input(|i| i.pointer.interact_pos())?;
    let payload = resp.dnd_hover_payload::<DragRow>()?;
    if payload.list != list {
        return None;
    }
    let rect = resp.rect;
    let (insert, y) = if ptr.y < rect.center().y { (idx, rect.top() - 3.0) } else { (idx + 1, rect.bottom() + 3.0) };
    ui.painter().hline(rect.x_range(), y, Stroke::new(2.0f32, SKY));
    resp.dnd_release_payload::<DragRow>().map(|p| (p.idx, insert))
}

fn reorder<T>(v: &mut Vec<T>, from: usize, insert: usize) {
    if from >= v.len() {
        return;
    }
    let item = v.remove(from);
    let at = if insert > from { insert - 1 } else { insert };
    v.insert(at.min(v.len()), item);
}

/// Carte de verre pleine largeur ; `tint` ajoute un voile coloré (ex. zone de dépôt active).
/// Renvoie le contenu et le rectangle occupé.
fn panel_card_with<R>(ui: &mut egui::Ui, tint: Option<Color32>, margin: f32, add: impl FnOnce(&mut egui::Ui) -> R) -> (R, Rect) {
    let slot = ui.painter().add(egui::Shape::Noop);
    let out = egui::Frame::none().inner_margin(egui::Margin::same(margin)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        add(ui)
    });
    let rect = out.response.rect;
    let mut shapes = vec![glass_shapes(rect)];
    if let Some(t) = tint {
        shapes.push(egui::Shape::rect_filled(rect, Rounding::same(CARD_RADIUS), t));
    }
    ui.painter().set(slot, egui::Shape::Vec(shapes));
    (out.inner, rect)
}

/// Carte de section pleine largeur, en verre.
fn panel_card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    panel_card_with(ui, None, 20.0, add).0
}

/// Ligne imbriquée dans une carte (condition ou action).
fn row_frame<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> egui::InnerResponse<R> {
    egui::Frame::none()
        .fill(glass(12))
        .rounding(Rounding::same(10.0))
        .inner_margin(egui::Margin::symmetric(12.0, 10.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
}

fn caps(text: &str) -> RichText {
    RichText::new(text).size(12.5).family(semibold()).color(text_label())
}

fn rule_tab(ui: &mut egui::Ui, label: &str, active: bool, enabled: bool, accent: Color32) -> egui::Response {
    let font = egui::FontId::new(14.5, if active { semibold() } else { egui::FontFamily::Proportional });
    let fg = if active { Color32::WHITE } else if enabled { text_dim() } else { text_label() };
    let galley = ui.painter().layout_no_wrap(label.to_string(), font.clone(), fg);
    let lead = if active { 30.0 } else { 16.0 };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(galley.size().x + lead + 16.0, 34.0), egui::Sense::click());
    let fill = glass(if active { 30 } else if resp.hovered() { 20 } else { 11 });
    ui.painter().rect_filled(rect, Rounding::same(9.0), fill);
    if active {
        ui.painter().circle_filled(Pos2::new(rect.left() + 16.0, rect.center().y), 3.5, accent);
    }
    ui.painter().text(Pos2::new(rect.left() + lead, rect.center().y), egui::Align2::LEFT_CENTER, label, font, fg);
    resp
}

fn section_title(ui: &mut egui::Ui, icon: Icon, title: &str) {
    let (r, _) = ui.allocate_exact_size(Vec2::splat(20.0), egui::Sense::hover());
    draw_icon(&ui.painter().clone(), r.center(), icon, SKY, 18.0);
    ui.label(RichText::new(title).size(15.0).family(semibold()).color(Color32::WHITE));
}

/// « il y a 5 min », « hier »… à partir d'un instant passé.
fn ago(t: SystemTime, now: SystemTime) -> String {
    let secs = now.duration_since(t).map(|d| d.as_secs()).unwrap_or(0);
    match secs {
        0..=44 => "à l'instant".to_string(),
        45..=3599 => format!("il y a {} min", (secs + 30) / 60),
        3600..=86_399 => format!("il y a {} h", secs / 3600),
        86_400..=172_799 => "hier".to_string(),
        _ => format!("il y a {} j", secs / 86_400),
    }
}

/// Ce que l'écran Aperçu met en avant selon l'état de l'appli.
#[derive(Clone, Copy, PartialEq, Debug)]
enum HeroState {
    NoFolder,
    NoRule,
    Ready,
    Done,
}

fn hero_state(folders: usize, active_rules: usize, last_run_was_real: bool) -> HeroState {
    if folders == 0 {
        HeroState::NoFolder
    } else if active_rules == 0 {
        HeroState::NoRule
    } else if last_run_was_real {
        HeroState::Done
    } else {
        HeroState::Ready
    }
}

/// Lien texte cliquable ; true si cliqué.
fn link(ui: &mut egui::Ui, text: &str, color: Color32) -> bool {
    ui.add(egui::Label::new(RichText::new(text).size(14.0).family(semibold()).color(color)).sense(egui::Sense::click()))
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

/// Anneau de progression : piste de verre + arc accent aux extrémités arrondies.
fn ring(painter: &egui::Painter, center: Pos2, radius: f32, frac: f32, accent: Color32) {
    painter.circle_stroke(center, radius, Stroke::new(10.0f32, glass(44)));
    if frac > 0.0 {
        let steps = ((frac * 72.0).ceil() as usize).max(2);
        let pts: Vec<Pos2> = (0..=steps)
            .map(|i| {
                let a = -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * frac * (i as f32 / steps as f32);
                center + Vec2::angled(a) * radius
            })
            .collect();
        painter.circle_filled(pts[0], 5.0, accent);
        painter.circle_filled(pts[pts.len() - 1], 5.0, accent);
        painter.add(egui::Shape::line(pts, Stroke::new(10.0f32, accent)));
    }
}

/// Tuile de chiffre cliquable : pastille d'icône, grand chiffre, légende ; s'élève au survol.
fn kpi_tile(ui: &mut egui::Ui, w: f32, icon: Icon, label: &str, value: &str, sub: &str, accent: Color32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 134.0), egui::Sense::click());
    let lift = ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), 0.15) * 3.0;
    let r = rect.translate(Vec2::new(0.0, -lift));
    let p = ui.painter().clone();
    p.add(glass_shapes(r));
    if resp.hovered() {
        p.rect_filled(r, Rounding::same(CARD_RADIUS), glass(12));
    }
    let tile = Rect::from_min_size(r.min + Vec2::new(20.0, 20.0), Vec2::splat(34.0));
    p.rect_filled(tile, Rounding::same(10.0), with_alpha(accent, 70));
    draw_icon(&p, tile.center(), icon, Color32::WHITE, 17.0);
    p.text(
        Pos2::new(tile.right() + 10.0, tile.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::new(12.5, semibold()),
        text_label(),
    );
    p.text(
        Pos2::new(r.left() + 20.0, r.top() + 86.0),
        egui::Align2::LEFT_CENTER,
        value,
        egui::FontId::new(38.0, bold()),
        Color32::WHITE,
    );
    p.text(
        Pos2::new(r.left() + 20.0, r.bottom() - 20.0),
        egui::Align2::LEFT_CENTER,
        sub,
        egui::FontId::proportional(14.5),
        text_dim(),
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn status_color(s: Status) -> Color32 {
    match s {
        Status::Applied => Color32::from_rgb(0x22, 0xb5, 0x73),
        Status::Undone => Color32::from_rgb(0x3d, 0x7b, 0xf6),
        Status::DryRun => Color32::from_rgb(0x8b, 0x5c, 0xf6),
        Status::BlockedByConflict => Color32::from_rgb(0xf5, 0x8c, 0x2c),
        Status::Failed => Color32::from_rgb(0xe0, 0x5c, 0x5c),
    }
}

fn combo(ui: &mut egui::Ui, id: &str, current: &str, options: &[impl AsRef<str>]) -> Option<String> {
    let mut chosen = None;
    egui::ComboBox::from_id_source(id)
        .selected_text(current)
        .width(150.0)
        .show_ui(ui, |ui| {
            for o in options {
                let label = o.as_ref();
                if ui.selectable_label(label == current, label).clicked() && label != current {
                    chosen = Some(label.to_string());
                }
            }
        });
    chosen
}

// ------------------------------------------------------------------ render

impl eframe::App for App {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.persist();
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame += 1;
        if self.start_minimized && self.frame == 2 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }

        // --screenshot: draw ~120 frames, save the window as PNG, exit.
        if self.screenshot.is_some() {
            if self.frame == 120 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
            }
            let shots: Vec<std::sync::Arc<egui::ColorImage>> = ctx.input(|i| {
                i.raw
                    .events
                    .iter()
                    .filter_map(|e| match e {
                        egui::Event::Screenshot { image, .. } => Some(image.clone()),
                        _ => None,
                    })
                    .collect()
            });
            if let Some(data) = shots.last() {
                if let Some(path) = self.screenshot.clone() {
                    save_png(&path, data);
                    self.shot_done = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            } else if self.frame > 400 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            ctx.request_repaint();
        }

        // Sauvegarde : au plus une vérification par seconde, écriture seulement si quelque chose a changé.
        let clock = ctx.input(|i| i.time);
        if clock >= self.next_save_check {
            self.next_save_check = clock + 1.0;
            self.sync_watcher();
            self.persist();
        }
        self.poll_watcher();
        ctx.request_repaint_after(std::time::Duration::from_millis(1100));

        let screen = ctx.screen_rect();
        let accent_now = self.accent();
        if self.tab != self.last_tab {
            self.last_tab = self.tab;
            self.tab_since = ctx.input(|i| i.time);
        }
        let bg_time = if self.animate_bg {
            ctx.request_repaint_after(std::time::Duration::from_millis(33));
            ctx.input(|i| i.time) + self.bg_offset
        } else {
            self.bg_offset
        };
        egui::CentralPanel::default().frame(egui::Frame::none()).show(ctx, |ui| {
            backdrop(ui, screen, self.accent(), bg_time);

            egui::TopBottomPanel::top("topbar")
                .exact_height(56.0)
                .frame(egui::Frame::none().fill(PANEL_GLASS))
                .show_inside(ui, |ui| {
                    self.top_bar(ui);
                });

            egui::SidePanel::left("sidebar")
                .resizable(false)
                .exact_width(256.0)
                .frame(egui::Frame::none().fill(PANEL_GLASS))
                .show_inside(ui, |ui| {
                    self.sidebar(ui);
                });

            // Contenu : le défilement va jusqu'au bas de la fenêtre ; un fondu efface les cartes en bas.
            let content = ui.available_rect_before_wrap();
            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .show(ui, |ui| {
                    egui::Frame::none().inner_margin(egui::Margin::symmetric(28.0, 0.0)).show(ui, |ui| {
                        ui.add_space(24.0);
                        match self.tab {
                            Tab::Dashboard => self.dashboard(ui),
                            Tab::Folders => self.folders_ui(ui),
                            Tab::Rules => self.rules_ui(ui),
                            Tab::History => self.history_ui(ui),
                            Tab::Settings => self.settings_ui(ui),
                        }
                        ui.add_space(BOTTOM_FADE_H);
                    });
                });
            bottom_fade(ui, content, screen, bg_time, accent_now);

            // Message / bandeau « Annuler » par-dessus le fondu.
            let bar = Rect::from_min_max(
                Pos2::new(content.left() + 28.0, content.bottom() - 52.0),
                Pos2::new(content.right() - 28.0, content.bottom() - 4.0),
            );
            let mut bar_ui = ui.new_child(egui::UiBuilder::new().max_rect(bar).layout(Layout::left_to_right(Align::Center)));
            let pending = self.undo_folder.as_ref().map(|u| {
                (u.folder.path.display().to_string(), u.rules.len(), u.at.elapsed().as_secs())
            });
            match pending {
                Some((path, n_rules, secs)) if secs < UNDO_SECONDS => {
                    bar_ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
                    bar_ui.label(
                        RichText::new(format!(
                            "Dossier retiré : {} · {}",
                            shorten(&path),
                            match n_rules {
                                0 => "sans règle".to_string(),
                                1 => "1 règle supprimée".to_string(),
                                n => format!("{n} règles supprimées"),
                            }
                        ))
                        .size(14.5)
                        .color(text_primary()),
                    );
                    if btn(&mut bar_ui, "Annuler", Btn::Primary, accent_now, Some(Icon::Undo)).clicked() {
                        self.restore_folder();
                    }
                }
                _ => {
                    self.undo_folder = None;
                    if let Some(t) = &self.toast {
                        bar_ui.label(RichText::new(format!("•  {t}")).size(14.5).color(text_dim()));
                    }
                }
            }
        });
    }
}

impl App {
    fn top_bar(&mut self, ui: &mut egui::Ui) {
        let accent = self.accent();
        let ctx = ui.ctx().clone();
        let rect = ui.max_rect();
        let wide = rect.width() >= 1280.0;

        // La barre sert de poignée pour déplacer la fenêtre (barre de titre masquée).
        let drag = ui.interact(rect, ui.id().with("drag"), egui::Sense::click_and_drag());
        if drag.drag_started_by(egui::PointerButton::Primary) {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }

        // ⌘K : focus sur la recherche.
        let search_id = egui::Id::new("global-search");
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::K)) {
            ctx.memory_mut(|m| m.request_focus(search_id));
        }

        // La recherche part 25 px à gauche du début du contenu (barre latérale 256 px + marge 28 px)
        // et prend tout l'espace libre jusqu'aux boutons de droite.
        let right_w = if wide { 492.0 } else { 420.0 };
        let search_left = rect.left() + 256.0 + 28.0 - 25.0;
        let search_right = rect.right() - 16.0 - right_w - 12.0;
        let search_w = (search_right - search_left).clamp(220.0, 640.0);
        let search_rect = Rect::from_min_size(Pos2::new(search_left, rect.center().y - 17.0), Vec2::new(search_w, 34.0));

        // --- gauche
        let left = Rect::from_min_max(
            Pos2::new(rect.left() + 16.0, rect.top()),
            Pos2::new(search_rect.left() - 10.0, rect.bottom()),
        );
        let mut lui = ui.new_child(egui::UiBuilder::new().max_rect(left).layout(Layout::left_to_right(Align::Center)));
        if cfg!(target_os = "macos") {
            lui.add_space(76.0); // feux natifs de la fenêtre
        } else {
            for c in [(0xff, 0x5f, 0x56), (0xff, 0xbd, 0x2e), (0x27, 0xc9, 0x3f)] {
                let (r, _) = lui.allocate_exact_size(Vec2::splat(12.0), egui::Sense::hover());
                lui.painter().circle_filled(r.center(), 6.0, Color32::from_rgb(c.0, c.1, c.2));
                lui.add_space(2.0);
            }
            lui.add_space(10.0);
        }

        // --- centre : recherche
        ui.painter().rect_filled(search_rect, Rounding::same(8.0), with_alpha(CANVAS, 180));
        let mut sui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(search_rect.shrink2(Vec2::new(10.0, 0.0)))
                .layout(Layout::left_to_right(Align::Center)),
        );
        let (ic, _) = sui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::hover());
        draw_icon(&sui.painter().clone(), ic.center(), Icon::Search, text_dim(), 15.0);
        sui.add(
            egui::TextEdit::singleline(&mut self.search)
                .id(search_id)
                .frame(false)
                .desired_width(search_w - 90.0)
                .hint_text("Rechercher règles, dossiers…"),
        );
        let key = if cfg!(target_os = "macos") { "⌘ K" } else { "Ctrl K" };
        ui.painter().text(
            Pos2::new(search_rect.right() - 12.0, search_rect.center().y),
            egui::Align2::RIGHT_CENTER,
            key,
            egui::FontId::proportional(13.5),
            text_dim(),
        );

        // --- droite
        let right = Rect::from_min_max(
            Pos2::new(search_rect.right() + 10.0, rect.top()),
            Pos2::new(rect.right() - 16.0, rect.bottom()),
        );
        let mut rui = ui.new_child(egui::UiBuilder::new().max_rect(right).layout(Layout::right_to_left(Align::Center)));
        if wide {
            let (r, resp) = rui.allocate_exact_size(Vec2::splat(32.0), egui::Sense::click());
            if resp.hovered() {
                rui.painter().rect_filled(r, Rounding::same(8.0), glass(20));
            }
            draw_icon(&rui.painter().clone(), r.center(), Icon::Gear, text_dim(), 17.0);
            if resp.on_hover_text("Réglages").clicked() {
                self.tab = Tab::Settings;
            }
            let (r, resp) = rui.allocate_exact_size(Vec2::splat(32.0), egui::Sense::click());
            if resp.hovered() {
                rui.painter().rect_filled(r, Rounding::same(8.0), glass(20));
            }
            draw_icon(&rui.painter().clone(), r.center(), Icon::Bell, text_dim(), 17.0);
            if self.history.iter().any(|h| h.status == Status::BlockedByConflict) {
                rui.painter().circle_filled(r.center() + Vec2::new(7.0, -7.0), 3.5, accent);
            }
            if resp.on_hover_text("Historique").clicked() {
                self.tab = Tab::History;
            }
            rui.add_space(4.0);
        }
        let enabled = self.app_enabled;
        let run_label = if rect.width() >= 1300.0 { "Organiser maintenant" } else { "Organiser" };
        let run = rui.add_enabled_ui(enabled, |ui| btn(ui, run_label, Btn::Primary, accent, Some(Icon::Bolt))).inner;
        let run = run
            .on_hover_text("Lance toutes les règles maintenant (annulable dans l'Historique).")
            .on_disabled_hover_text("Application en veille.");
        if run.clicked() {
            self.run_batch(false);
        }
        let preview = btn(&mut rui, "Aperçu simulation", Btn::Ghost, accent, None)
            .on_hover_text("Simule, sans toucher aux fichiers.");
        if preview.clicked() {
            self.run_batch(true);
        }
        // Statut : pastille verte pulsée.
        let label = if self.app_enabled { "Actif" } else { "En veille" };
        let dot = if self.app_enabled { GREEN } else { text_dim() };
        let (r, resp) = rui.allocate_exact_size(Vec2::new(112.0, 32.0), egui::Sense::click());
        rui.painter().rect_filled(r, Rounding::same(16.0), with_alpha(dot, if resp.hovered() { 36 } else { 22 }));
        let dc = Pos2::new(r.left() + 16.0, r.center().y);
        if self.app_enabled {
            let t = (ctx.input(|i| i.time) as f32 * 1.4).fract();
            rui.painter().circle_filled(dc, 4.0 + 5.0 * t, with_alpha(dot, ((1.0 - t) * 90.0) as u8));
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
        rui.painter().circle_filled(dc, 4.0, dot);
        rui.painter().text(
            Pos2::new(r.left() + 28.0, r.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(14.5),
            text_primary(),
        );
        if resp.on_hover_text("Activer / mettre en veille").clicked() {
            self.app_enabled = !self.app_enabled;
        }
    }

    /// Règles de tous les dossiers, dans l'ordre d'affichage (dossier, puis priorité).
    fn rule_order(&self) -> Vec<u64> {
        let mut order: Vec<&Rule> = self.rules.iter().collect();
        order.sort_by_key(|r| {
            let folder_pos = self.folders.iter().position(|f| f.id == r.folder_id).unwrap_or(usize::MAX);
            (folder_pos, r.priority, r.id)
        });
        order.into_iter().map(|r| r.id).collect()
    }

    fn new_rule(&mut self) {
        let current = self.rule_order().get(self.selected_rule).copied();
        let folder_id = current
            .and_then(|id| self.rules.iter().find(|r| r.id == id))
            .map(|r| r.folder_id)
            .or_else(|| self.folders.first().map(|f| f.id));
        match folder_id {
            Some(id) => self.new_rule_in(id),
            None => self.toast("Ajoutez d'abord un dossier."),
        }
    }

    fn new_rule_in(&mut self, folder_id: u64) {
        let count = self.rules.iter().filter(|r| r.folder_id == folder_id).count();
        let id = self.next_id;
        self.next_id += 1;
        self.rules.push(Rule {
            id,
            folder_id,
            name: "Nouvelle règle".into(),
            enabled: true,
            match_all: true,
            recursion_depth: 0,
            priority: count as i64,
            conditions: vec![Condition {
                field: Field::Extension,
                op: Op::Equals,
                value: String::new(),
            }],
            actions: vec![Action::new(ActionKind::Move)],
        });
        self.selected_rule = self.rule_order().iter().position(|r| *r == id).unwrap_or(0);
        self.tab = Tab::Rules;
    }

    /// Ouvre l'écran Règles sur la première règle du dossier.
    fn show_rules_of(&mut self, folder_id: u64) {
        let first = self.rule_order().iter().position(|id| {
            self.rules.iter().any(|r| r.id == *id && r.folder_id == folder_id)
        });
        if let Some(pos) = first {
            self.selected_rule = pos;
        }
        self.tab = Tab::Rules;
    }

    /// Ajoute des dossiers (sélecteur, dépôt, raccourcis, saisie) ; explique ce qui a été refusé.
    fn add_folders(&mut self, paths: Vec<PathBuf>) {
        let mut added = 0;
        let mut refused: Vec<String> = Vec::new();
        for p in paths {
            match validate_new_folder(&p, &self.folders) {
                Ok(path) => {
                    self.folders.push(Folder { id: self.next_id, path, enabled: true, priority: self.folders.len() as i64 });
                    self.next_id += 1;
                    added += 1;
                }
                Err(why) => refused.push(format!("{} : {why}", shorten(&p.display().to_string()))),
            }
        }
        let msg = match (added, refused.first()) {
            (0, Some(first)) if refused.len() > 1 => format!("Refusé : {first} (+{} autres)", refused.len() - 1),
            (0, Some(first)) => format!("Refusé : {first}"),
            (1, None) => "Dossier ajouté".to_string(),
            (n, None) => format!("{n} dossiers ajoutés"),
            (n, Some(first)) => format!("{n} ajouté(s), refusé : {first}"),
        };
        if added > 0 || !refused.is_empty() {
            self.toast(msg);
        }
    }

    fn restore_folder(&mut self) {
        if let Some(u) = self.undo_folder.take() {
            let at = u.index.min(self.folders.len());
            self.folders.insert(at, u.folder);
            self.rules.extend(u.rules);
            self.toast("Dossier rétabli");
        }
    }

    fn folder_count(&mut self, id: u64, path: &std::path::Path) -> Option<usize> {
        *self.folder_counts.entry(id).or_insert_with(|| count_top_level_files(path))
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        let accent = self.accent();
        // Identité de l'app, avant le menu.
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            ui.add_space(20.0);
            let (r, _) = ui.allocate_exact_size(Vec2::splat(36.0), egui::Sense::hover());
            let logo = self.logo.get_or_insert_with(|| {
                let img = app_icon_image();
                ui.ctx().load_texture("app-logo", img, egui::TextureOptions::LINEAR)
            });
            ui.painter().image(
                logo.id(),
                r,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
            ui.add_space(8.0);
            ui.label(RichText::new("OrganizeMyFiles").size(17.0).family(semibold()).color(Color32::WHITE));
        });
        ui.add_space(22.0);
        ui.horizontal(|ui| {
            ui.add_space(22.0);
            ui.label(RichText::new("MENU").size(13.5).family(semibold()).color(text_label()));
        });
        ui.add_space(8.0);

        for (tab, label, icon, _desc) in TABS {
            let active = self.tab == *tab;
            let badge = match tab {
                Tab::Folders => Some(self.folders.len()),
                Tab::Rules => Some(self.rules.len()),
                _ => None,
            };
            ui.horizontal(|ui| {
                ui.add_space(12.0);
                let (rect, response) = ui.allocate_exact_size(Vec2::new(232.0, 40.0), egui::Sense::click());
                let p = ui.painter();
                if active {
                    p.rect_filled(rect, Rounding::same(10.0), with_alpha(accent, 38));
                    p.rect_filled(
                        Rect::from_min_size(rect.left_top() + Vec2::new(0.0, 9.0), Vec2::new(3.0, 22.0)),
                        Rounding::same(2.0),
                        accent,
                    );
                } else if response.hovered() {
                    p.rect_filled(rect, Rounding::same(10.0), glass(12));
                }
                let fg = if active { accent } else { text_dim() };
                draw_icon(p, Pos2::new(rect.left() + 26.0, rect.center().y), *icon, fg, 17.0);
                p.text(
                    Pos2::new(rect.left() + 46.0, rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    *label,
                    egui::FontId::proportional(15.5),
                    if active { accent } else { text_primary() },
                );
                if let Some(n) = badge {
                    let text = n.to_string();
                    let badge_fg = if active { Color32::WHITE } else { fg };
                    let g = p.layout_no_wrap(text.clone(), egui::FontId::proportional(13.5), badge_fg);
                    let w = (g.size().x + 14.0).max(22.0);
                    let b = Rect::from_center_size(Pos2::new(rect.right() - 12.0 - w / 2.0, rect.center().y), Vec2::new(w, 20.0));
                    p.rect_filled(b, Rounding::same(10.0), if active { with_alpha(accent, 60) } else { glass(16) });
                    p.text(b.center(), egui::Align2::CENTER_CENTER, text, egui::FontId::proportional(13.5), badge_fg);
                    if active && *tab == Tab::Rules {
                        p.circle_filled(Pos2::new(b.left() - 9.0, rect.center().y), 3.0, accent);
                    }
                }
                if response.clicked() {
                    self.tab = *tab;
                }
            });
            ui.add_space(2.0);
        }

        // Pied : état de l'application, collé en bas de la barre.
        let avail = ui.max_rect();
        let rect = Rect::from_min_size(Pos2::new(avail.left() + 12.0, avail.bottom() - 16.0 - 84.0), Vec2::new(232.0, 84.0));
        ui.painter().rect_filled(rect, Rounding::same(12.0), with_alpha(CANVAS, 150));
        let mut c = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink(12.0)).layout(Layout::top_down(Align::Min)));
        c.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(caps("APPLICATION"));
                ui.label(
                    RichText::new(if self.app_enabled { "Activée" } else { "En veille" })
                        .size(15.5)
                        .color(Color32::WHITE),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                toggle(ui, &mut self.app_enabled, accent);
            });
        });
        c.add_space(4.0);
        let nothing_watched = self.watcher.as_ref().is_some_and(|w| w.watched_folders() == 0);
        let (dot, txt) = if self.app_enabled && nothing_watched {
            (text_dim(), "Aucun dossier surveillé • v0.1")
        } else if self.app_enabled {
            (GREEN, "Surveillance active • v0.1")
        } else {
            (text_dim(), "v0.1")
        };
        c.horizontal(|ui| {
            if self.app_enabled {
                let (r, _) = ui.allocate_exact_size(Vec2::splat(8.0), egui::Sense::hover());
                ui.painter().circle_filled(r.center(), 3.5, dot);
            }
            ui.label(RichText::new(txt).size(13.5).color(text_dim()));
        });
    }

    fn header(&mut self, ui: &mut egui::Ui, title: &str, desc: &str) {
        ui.label(RichText::new(title).size(26.0).family(bold()).color(text_primary()));
        ui.add_space(4.0);
        ui.label(RichText::new(desc).size(15.0).color(text_dim()));
        ui.add_space(16.0);
    }

    fn dashboard(&mut self, ui: &mut egui::Ui) {
        let accent = self.accent();
        let ctx = ui.ctx().clone();
        // Les chiffres montent à l'ouverture de l'écran.
        let enter = (((ctx.input(|i| i.time) - self.tab_since) / 0.8) as f32).clamp(0.0, 1.0);
        let ease = 1.0 - (1.0 - enter).powi(3);
        if enter < 1.0 {
            ctx.request_repaint();
        }
        let count = |n: usize| ((n as f32) * ease).round() as usize;

        let folders_total = self.folders.len();
        let folders_active = self.folders.iter().filter(|f| f.enabled).count();
        let rules_total = self.rules.len();
        let rules_active = self.rules.iter().filter(|r| r.enabled).count();
        let state = hero_state(folders_total, rules_active, self.last_run.is_some() && !self.last_run_dry);

        // ---------------------------------------------------------------- héros
        let (title, sub) = match state {
            HeroState::NoFolder => ("Commençons".to_string(), "Ajoutez un dossier à surveiller.".to_string()),
            HeroState::NoRule => ("Aucune règle active".to_string(), "Créez une règle pour ranger vos fichiers.".to_string()),
            HeroState::Ready | HeroState::Done => (
                if state == HeroState::Done { "Tout est en ordre" } else { "Prêt à organiser" }.to_string(),
                format!(
                    "{folders_active} dossier{} · {rules_active} règle{} active{}",
                    if folders_active > 1 { "s" } else { "" },
                    if rules_active > 1 { "s" } else { "" },
                    if rules_active > 1 { "s" } else { "" },
                ),
            ),
        };
        let avail = ui.available_width();
        let vgap = GAP - ui.spacing().item_spacing.y;
        let (hero, _) = ui.allocate_exact_size(Vec2::new(avail, 214.0), egui::Sense::hover());
        ui.painter().add(glass_shapes(hero));
        {
            let t = if self.animate_bg { ctx.input(|i| i.time) } else { 0.0 };
            let drift = (t * 0.3).sin() as f32 * 0.05;
            glow_in_rounded(
                ui.painter(),
                hero.shrink(1.0),
                CARD_RADIUS - 1.0,
                &[
                    (Pos2::new(hero.left() + hero.width() * (0.30 + drift), hero.top() + 10.0), hero.width() * 0.42, accent, 14),
                    (Pos2::new(hero.right() - hero.width() * 0.12, hero.bottom()), hero.width() * 0.36, SKY_DEEP, 18),
                ],
            );
        }
        let mut go: Option<Tab> = None;
        let mut organise = false;
        let mut preview = false;
        let mut new_rule = false;
        let text_w = (avail - 250.0).max(260.0);
        let inner = Rect::from_min_size(hero.min + Vec2::new(28.0, 24.0), Vec2::new(text_w, hero.height() - 48.0));
        let mut c = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(Layout::top_down(Align::Min)));
        c.label(RichText::new("APERÇU").size(12.5).family(semibold()).color(text_label()));
        c.add_space(4.0);
        c.label(RichText::new(&title).size(34.0).family(bold()).color(Color32::WHITE));
        c.add_space(2.0);
        c.label(RichText::new(&sub).size(16.0).color(text_dim()));
        c.add_space(18.0);
        c.horizontal(|ui| match state {
            HeroState::NoFolder => {
                if pill(ui, "Ajouter un dossier", accent, true, true).clicked() {
                    go = Some(Tab::Folders);
                }
            }
            HeroState::NoRule => {
                if pill(ui, "Créer une règle", accent, true, true).clicked() {
                    new_rule = true;
                }
            }
            HeroState::Ready | HeroState::Done => {
                let run = pill(ui, "Organiser maintenant", accent, true, true)
                    .on_hover_text("Lance toutes les règles maintenant (annulable dans l'Historique).");
                organise = run.clicked() && self.app_enabled;
                ui.add_space(10.0);
                preview = pill(ui, "Aperçu", accent, false, true)
                    .on_hover_text("Simule, sans toucher aux fichiers.")
                    .clicked();
            }
        });
        // Anneau : part des fichiers concernés lors du dernier passage.
        let center = Pos2::new(hero.right() - 100.0, hero.center().y - 4.0);
        let (frac, pct_text, caption, kicker) = match self.last_run {
            Some((matched, _, scanned)) if scanned > 0 => (
                matched as f32 / scanned as f32,
                format!("{}%", (100.0 * matched as f32 / scanned as f32 * ease).round() as u32),
                format!("{matched} sur {scanned} fichiers"),
                if self.last_run_dry { "DERNIER APERÇU" } else { "DERNIER LOT" },
            ),
            _ => (0.0, "—".to_string(), "Pas encore d'analyse".to_string(), "ANALYSE"),
        };
        ring(ui.painter(), center, 50.0, frac * ease, accent);
        let p = ui.painter();
        p.text(
            Pos2::new(center.x, center.y - 78.0),
            egui::Align2::CENTER_CENTER,
            kicker,
            egui::FontId::new(12.0, semibold()),
            text_label(),
        );
        p.text(center, egui::Align2::CENTER_CENTER, pct_text, egui::FontId::new(26.0, bold()), Color32::WHITE);
        p.text(
            Pos2::new(center.x, center.y + 76.0),
            egui::Align2::CENTER_CENTER,
            caption,
            egui::FontId::proportional(14.0),
            text_dim(),
        );
        ui.add_space(vgap);

        // ---------------------------------------------------------------- tuiles
        let gap = GAP;
        let w = ((avail - 3.0 * gap) / 4.0).max(120.0);
        let (matched_actions, actions_sub) = match self.last_run {
            Some((_, actions, _)) => (count(actions).to_string(), "actions au dernier passage"),
            None => ("—".to_string(), "aucun passage"),
        };
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            if kpi_tile(ui, w, Icon::Folder, "DOSSIERS ACTIFS", &count(folders_active).to_string(), &format!("sur {folders_total} suivi{}", if folders_total > 1 { "s" } else { "" }), accent).clicked() {
                go = Some(Tab::Folders);
            }
            if kpi_tile(ui, w, Icon::List, "RÈGLES ACTIVES", &count(rules_active).to_string(), &format!("sur {rules_total} règle{}", if rules_total > 1 { "s" } else { "" }), accent).clicked() {
                go = Some(Tab::Rules);
            }
            if kpi_tile(ui, w, Icon::Clock, "FICHIERS CE MOIS", &count(self.monthly_used as usize).to_string(), "organisés", accent).clicked() {
                go = Some(Tab::History);
            }
            if kpi_tile(ui, w, Icon::Bolt, "DERNIER LOT", &matched_actions, actions_sub, accent).clicked() {
                go = Some(Tab::History);
            }
        });
        ui.add_space(vgap);

        // ---------------------------------------------------------- dossiers + activité
        if avail >= 760.0 {
            let old_gap = ui.spacing().item_spacing.x;
            ui.spacing_mut().item_spacing.x = GAP;
            ui.columns(2, |cols| {
                self.dash_folders(&mut cols[0], accent);
                self.dash_activity(&mut cols[1], accent);
            });
            ui.spacing_mut().item_spacing.x = old_gap;
        } else {
            self.dash_folders(ui, accent);
            ui.add_space(vgap);
            self.dash_activity(ui, accent);
        }

        if let Some(tab) = go {
            self.tab = tab;
        }
        if new_rule {
            self.new_rule();
        }
        if preview {
            self.run_batch(true);
        }
        if organise {
            self.run_batch(false);
        }
    }

    /// Carte « Dossiers suivis » de l'Aperçu : aperçu des 4 premiers, un clic ouvre l'écran Dossiers.
    fn dash_folders(&mut self, ui: &mut egui::Ui, accent: Color32) {
        let folders = self.folders.clone();
        let mut open = false;
        panel_card(ui, |ui| {
            ui.set_min_height(LIST_CARD_MIN_H);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Dossiers suivis").size(17.0).family(semibold()).color(Color32::WHITE));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if link(ui, "Tout voir", accent) {
                        open = true;
                    }
                });
            });
            ui.add_space(8.0);
            if folders.is_empty() {
                ui.label(RichText::new("Aucun dossier suivi.").size(15.0).color(text_dim()));
                ui.add_space(10.0);
                if btn(ui, "Ajouter un dossier", Btn::Sky, accent, Some(Icon::Add)).clicked() {
                    open = true;
                }
                return;
            }
            for f in folders.iter().take(4) {
                let exists = f.path.is_dir();
                let n_rules = self.rules.iter().filter(|r| r.folder_id == f.id).count();
                let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 54.0), egui::Sense::click());
                if resp.hovered() {
                    ui.painter().rect_filled(rect, Rounding::same(10.0), glass(14));
                }
                let tile = Rect::from_min_size(rect.min + Vec2::new(8.0, 10.0), Vec2::splat(34.0));
                ui.painter().rect_filled(tile, Rounding::same(10.0), with_alpha(if exists { accent } else { RED }, 70));
                draw_icon(&ui.painter().clone(), tile.center(), Icon::Folder, Color32::WHITE, 17.0);
                let name = f.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                let clip = Rect::from_min_max(rect.min, Pos2::new(rect.right() - 170.0, rect.bottom()));
                let tp = ui.painter().with_clip_rect(clip);
                tp.text(
                    Pos2::new(tile.right() + 12.0, rect.center().y - 9.0),
                    egui::Align2::LEFT_CENTER,
                    name,
                    egui::FontId::new(15.0, semibold()),
                    text_primary(),
                );
                tp.text(
                    Pos2::new(tile.right() + 12.0, rect.center().y + 11.0),
                    egui::Align2::LEFT_CENTER,
                    shorten(&f.path.display().to_string()),
                    egui::FontId::proportional(13.0),
                    text_dim(),
                );
                let mut right = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(rect.shrink2(Vec2::new(8.0, 0.0)))
                        .layout(Layout::right_to_left(Align::Center)),
                );
                if !exists {
                    chip(&mut right, "Introuvable", RED, with_alpha(RED, 38));
                } else if f.enabled {
                    chip(&mut right, "Actif", GREEN, with_alpha(GREEN, 38));
                } else {
                    chip(&mut right, "En pause", text_dim(), glass(30));
                }
                chip(
                    &mut right,
                    &match n_rules {
                        0 => "0 règle".to_string(),
                        1 => "1 règle".to_string(),
                        n => format!("{n} règles"),
                    },
                    SKY,
                    with_alpha(SKY, 38),
                );
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    open = true;
                }
            }
        });
        if open {
            self.tab = Tab::Folders;
        }
    }

    /// Carte « Activité récente » : les 4 dernières actions, annulables quand c'est possible.
    fn dash_activity(&mut self, ui: &mut egui::Ui, accent: Color32) {
        let now = SystemTime::now();
        let items: Vec<(usize, HistoryEntry)> = self.history.iter().cloned().enumerate().rev().take(4).collect();
        let mut open = false;
        let mut undo_idx: Option<usize> = None;
        panel_card(ui, |ui| {
            ui.set_min_height(LIST_CARD_MIN_H);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Activité récente").size(17.0).family(semibold()).color(Color32::WHITE));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if link(ui, "Tout l'historique", accent) {
                        open = true;
                    }
                });
            });
            ui.add_space(8.0);
            if items.is_empty() {
                ui.label(RichText::new("Rien pour l'instant.").size(15.0).color(text_dim()));
                ui.label(RichText::new("Lancez un aperçu ou un lot.").size(13.5).color(text_dim()));
                return;
            }
            for (idx, e) in &items {
                let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 54.0), egui::Sense::hover());
                ui.painter().circle_filled(Pos2::new(rect.left() + 14.0, rect.center().y), 5.0, status_color(e.status));
                let can_undo = e.reversible && e.status == Status::Applied && e.undo.is_some();
                let clip_right = if can_undo { 120.0 } else { 8.0 };
                let tp = ui.painter().with_clip_rect(Rect::from_min_max(rect.min, Pos2::new(rect.right() - clip_right, rect.bottom())));
                tp.text(
                    Pos2::new(rect.left() + 32.0, rect.center().y - 9.0),
                    egui::Align2::LEFT_CENTER,
                    &e.rule_name,
                    egui::FontId::new(15.0, semibold()),
                    text_primary(),
                );
                tp.text(
                    Pos2::new(rect.left() + 32.0, rect.center().y + 11.0),
                    egui::Align2::LEFT_CENTER,
                    format!("{} · {}", e.action_label, ago(e.created, now)),
                    egui::FontId::proportional(13.0),
                    text_dim(),
                );
                if can_undo {
                    let mut right = ui.new_child(
                        egui::UiBuilder::new().max_rect(rect).layout(Layout::right_to_left(Align::Center)),
                    );
                    if btn(&mut right, "Annuler", Btn::Ghost, accent, Some(Icon::Undo)).clicked() {
                        undo_idx = Some(*idx);
                    }
                }
            }
        });
        if open {
            self.tab = Tab::History;
        }
        if let Some(i) = undo_idx {
            self.undo(i);
        }
    }

    fn folders_ui(&mut self, ui: &mut egui::Ui) {
        let accent = self.accent();
        let ctx = ui.ctx().clone();

        // Dossiers déposés depuis le Finder : survol (halo) puis ajout au relâchement.
        let hovering = ctx.input(|i| !i.raw.hovered_files.is_empty());
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|d| d.path.clone()).collect());
        if !dropped.is_empty() {
            self.add_folders(dropped);
        }

        ui.horizontal(|ui| {
            ui.label(RichText::new("Dossiers suivis").size(24.0).family(bold()).color(Color32::WHITE));
            ui.add_space(6.0);
            if !self.folders.is_empty() {
                let active = self.folders.iter().filter(|f| f.enabled).count();
                chip(
                    ui,
                    &format!("{active} {} sur {}", if active > 1 { "actifs" } else { "actif" }, self.folders.len()),
                    SKY,
                    with_alpha(SKY, 38),
                );
            }
        });
        ui.add_space(2.0);
        ui.label(RichText::new("Les règles s'appliquent du premier dossier au dernier.").size(15.0).color(text_dim()));
        ui.add_space(16.0);

        self.folder_add_card(ui, accent, hovering);
        ui.add_space(14.0);
        self.folder_list(ui, accent);
    }

    /// Carte d'ajout : dépôt, sélecteur natif, raccourcis, saisie du chemin (repliée).
    fn folder_add_card(&mut self, ui: &mut egui::Ui, accent: Color32, hovering: bool) {
        const SHORTCUTS: [(&str, &str); 6] = [
            ("Bureau", "Desktop"),
            ("Téléchargements", "Downloads"),
            ("Documents", "Documents"),
            ("Images", "Pictures"),
            ("Musique", "Music"),
            ("Vidéos", "Movies"),
        ];
        let empty = self.folders.is_empty();
        let (title, sub) = if hovering {
            ("Relâchez pour ajouter", "Ils seront suivis aussitôt.")
        } else if empty {
            (
                "Aucun dossier suivi",
                "Glissez des dossiers du Finder, ou choisissez-les.",
            )
        } else {
            ("Ajouter un dossier", "Glissez un dossier du Finder, ou choisissez-le.")
        };
        let mut picked: Vec<PathBuf> = Vec::new();
        let mut typed_submit = false;
        panel_card_with(ui, hovering.then(|| with_alpha(accent, 46)), 20.0, |ui| {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::splat(44.0), egui::Sense::hover());
                ui.painter().rect_filled(r, Rounding::same(12.0), with_alpha(accent, 40));
                draw_icon(&ui.painter().clone(), r.center(), if hovering { Icon::Add } else { Icon::Folder }, Color32::WHITE, 22.0);
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new(title).size(15.0).family(semibold()).color(if hovering { accent } else { Color32::WHITE }));
                    ui.label(RichText::new(sub).size(14.0).color(text_dim()));
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if btn(ui, "Choisir un dossier…", Btn::Primary, accent, Some(Icon::FolderOpen)).clicked() {
                        if let Some(list) = rfd::FileDialog::new().set_directory(home_dir()).pick_folders() {
                            picked = list;
                        }
                    }
                });
            });
            ui.add_space(14.0);
            ui.label(caps("RACCOURCIS"));
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                for (label, dir) in SHORTCUTS {
                    let path = PathBuf::from(format!("{}/{}", home_dir(), dir));
                    let tracked = self.folders.iter().any(|f| f.path == path);
                    ui.add_enabled_ui(!tracked, |ui| {
                        let icon = if tracked { Icon::Tick } else { Icon::Add };
                        if btn(ui, label, Btn::Ghost, accent, Some(icon)).clicked() {
                            picked.push(path.clone());
                        }
                    });
                }
            });
            ui.add_space(10.0);
            let toggle_label = if self.show_path_input { "Masquer la saisie" } else { "Saisir un chemin" };
            let link = ui.add(egui::Label::new(RichText::new(toggle_label).size(14.0).color(accent)).sense(egui::Sense::click()));
            if link.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                self.show_path_input = !self.show_path_input;
            }
            if self.show_path_input {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let edit = ui.add(
                        egui::TextEdit::singleline(&mut self.add_folder_text)
                            .margin(egui::Margin::symmetric(8.0, 7.0))
                            .desired_width((ui.available_width() - 130.0).max(160.0))
                            .hint_text("~/Downloads"),
                    );
                    let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if btn(ui, "Ajouter", Btn::Sky, accent, None).clicked() || enter {
                        let text = self.add_folder_text.trim().to_string();
                        if !text.is_empty() {
                            picked.push(PathBuf::from(shim_expand(&text)));
                            typed_submit = true;
                        }
                    }
                });
            }
        });
        if !picked.is_empty() {
            let before = self.folders.len();
            self.add_folders(picked);
            if typed_submit && self.folders.len() > before {
                self.add_folder_text.clear();
            }
        }
    }

    /// Liste des dossiers suivis : priorité (glisser), état, règles, fichiers, actions.
    fn folder_list(&mut self, ui: &mut egui::Ui, accent: Color32) {
        if self.folders.is_empty() {
            return;
        }
        let mut remove: Option<usize> = None;
        let mut replace: Option<usize> = None;
        let mut finder: Option<usize> = None;
        let mut rules_of: Option<u64> = None;
        let mut reorder_f: Option<(usize, usize)> = None;
        let mut refresh = false;

        ui.horizontal(|ui| {
            ui.label(caps("PRIORITÉ"));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if icon_button(ui, Icon::Refresh, "Recompter les fichiers", false) {
                    refresh = true;
                }
            });
        });
        ui.add_space(6.0);

        for i in 0..self.folders.len() {
            let f = self.folders[i].clone();
            let exists = f.path.is_dir();
            let n_rules = self.rules.iter().filter(|r| r.folder_id == f.id).count();
            let files = if exists { self.folder_count(f.id, &f.path) } else { None };
            let name = f
                .path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| f.path.display().to_string());
            let (_, rect) = panel_card_with(ui, None, 16.0, |ui| {
                ui.horizontal(|ui| {
                    ui.dnd_drag_source(egui::Id::new(("fdrag", f.id)), DragRow { list: 2, idx: i }, |ui| {
                        let (r, _) = ui.allocate_exact_size(Vec2::new(14.0, 44.0), egui::Sense::hover());
                        draw_icon(&ui.painter().clone(), r.center(), Icon::Grip, text_dim(), 16.0);
                    });
                    let (b, resp) = ui.allocate_exact_size(Vec2::splat(26.0), egui::Sense::hover());
                    ui.painter().circle_filled(b.center(), 13.0, glass(34));
                    ui.painter().text(
                        b.center(),
                        egui::Align2::CENTER_CENTER,
                        (i + 1).to_string(),
                        egui::FontId::new(14.0, semibold()),
                        text_primary(),
                    );
                    resp.on_hover_text(format!(
                        "Priorité {} : traité {}",
                        i + 1,
                        if i == 0 { "en premier" } else { "après les précédents" }
                    ));
                    let (t, _) = ui.allocate_exact_size(Vec2::splat(42.0), egui::Sense::hover());
                    let tile = if exists { accent } else { RED };
                    ui.painter().rect_filled(t, Rounding::same(11.0), with_alpha(tile, 40));
                    draw_icon(&ui.painter().clone(), t.center(), Icon::Folder, Color32::WHITE, 20.0);
                    ui.add_space(4.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&name).size(15.5).family(semibold()).color(text_primary()));
                        ui.label(RichText::new(shorten(&f.path.display().to_string())).size(14.0).color(text_dim()));
                        ui.add_space(2.0);
                        ui.horizontal(|ui| {
                            if !exists {
                                chip(ui, "Introuvable", RED, with_alpha(RED, 38));
                            } else if f.enabled {
                                chip(ui, "Actif", GREEN, with_alpha(GREEN, 38));
                            } else {
                                chip(ui, "En pause", text_dim(), glass(30));
                            }
                            let rules_text = match n_rules {
                                0 => "Aucune règle · créer".to_string(),
                                1 => "1 règle".to_string(),
                                n => format!("{n} règles"),
                            };
                            if chip_button(ui, &rules_text, SKY, with_alpha(SKY, 38)).clicked() {
                                if n_rules == 0 {
                                    self.new_rule_in(f.id);
                                } else {
                                    rules_of = Some(f.id);
                                }
                            }
                            match files {
                                Some(0) => {
                                    chip(ui, "Dossier vide", text_dim(), glass(30));
                                }
                                Some(1) => {
                                    chip(ui, "1 fichier", text_dim(), glass(30));
                                }
                                Some(n) => {
                                    chip(ui, &format!("{n} fichiers"), text_dim(), glass(30));
                                }
                                None => {}
                            }
                        });
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let mut en = f.enabled;
                        if toggle(ui, &mut en, accent).on_hover_text("Suivre ce dossier").changed() {
                            self.folders[i].enabled = en;
                        }
                        if icon_button(ui, Icon::Trash, "Retirer ce dossier", true) {
                            remove = Some(i);
                        }
                        if exists {
                            if icon_button(ui, Icon::FolderOpen, "Ouvrir dans le Finder", false) {
                                finder = Some(i);
                            }
                        } else if btn(ui, "Changer d'emplacement", Btn::Sky, accent, None).clicked() {
                            replace = Some(i);
                        }
                    });
                });
            });
            let zone = ui.interact(rect, egui::Id::new(("fdrop", f.id)), egui::Sense::hover());
            if let Some(m) = drop_target(ui, &zone, 2, i) {
                reorder_f = Some(m);
            }
            ui.add_space(10.0);
        }

        if refresh {
            self.folder_counts.clear();
        }
        if let Some(id) = rules_of {
            self.show_rules_of(id);
        }
        if let Some(i) = finder {
            let program = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
            if std::process::Command::new(program).arg(&self.folders[i].path).spawn().is_err() {
                self.toast("Ouverture impossible.");
            }
        }
        if let Some(i) = replace {
            if let Some(new_path) = rfd::FileDialog::new().set_directory(home_dir()).pick_folder() {
                match validate_new_folder(&new_path, &self.folders) {
                    Ok(path) => {
                        let id = self.folders[i].id;
                        self.folders[i].path = path;
                        self.folder_counts.remove(&id);
                        self.toast("Emplacement mis à jour");
                    }
                    Err(why) => self.toast(format!("Non modifié — {why}")),
                }
            }
        }
        if let Some((from, insert)) = reorder_f {
            let keep = self.rule_order().get(self.selected_rule).copied();
            reorder(&mut self.folders, from, insert);
            if let Some(id) = keep {
                self.selected_rule = self.rule_order().iter().position(|r| *r == id).unwrap_or(0);
            }
        }
        if let Some(i) = remove {
            let removed = self.folders.remove(i);
            let (gone, kept): (Vec<Rule>, Vec<Rule>) =
                std::mem::take(&mut self.rules).into_iter().partition(|r| r.folder_id == removed.id);
            self.rules = kept;
            self.undo_folder = Some(UndoFolder { folder: removed, rules: gone, index: i, at: std::time::Instant::now() });
        }
        for (i, f) in self.folders.iter_mut().enumerate() {
            f.priority = i as i64;
        }
    }

    fn rule_matches(&self, rule: &Rule, q: &str) -> bool {
        let folder = self
            .folders
            .iter()
            .find(|f| f.id == rule.folder_id)
            .map(|f| f.path.display().to_string().to_lowercase())
            .unwrap_or_default();
        rule.name.to_lowercase().contains(q)
            || folder.contains(q)
            || rule.actions.iter().any(|a| a.kind.label().to_lowercase().contains(q))
    }

    fn rules_ui(&mut self, ui: &mut egui::Ui) {
        let accent = self.accent();

        ui.horizontal(|ui| {
            ui.label(RichText::new("Règles").size(24.0).family(bold()).color(Color32::WHITE));
            ui.add_space(6.0);
            let n = self.rules.iter().filter(|r| r.enabled).count();
            chip(
                ui,
                &format!("{n} {}", if n > 1 { "actives" } else { "active" }),
                SKY,
                with_alpha(SKY, 38),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if btn(ui, "Nouvelle règle", Btn::Sky, accent, Some(Icon::Add)).clicked() {
                    self.new_rule();
                }
            });
        });
        ui.add_space(2.0);
        ui.label(RichText::new("Conditions remplies : actions dans l'ordre.").size(15.0).color(text_dim()));
        ui.add_space(16.0);

        if self.folders.is_empty() {
            ui.label(RichText::new("Ajoutez d'abord un dossier.").color(text_dim()));
            return;
        }

        let order = self.rule_order();
        if order.is_empty() {
            ui.label(RichText::new("Aucune règle.").color(text_dim()));
            ui.add_space(8.0);
            if btn(ui, "+ Nouvelle règle", Btn::Sky, accent, None).clicked() {
                self.new_rule();
            }
            return;
        }
        let sel = self.selected_rule.min(order.len() - 1);
        self.selected_rule = sel;

        // ---- onglets de règles (filtrés par la recherche globale)
        let q = self.search.trim().to_lowercase();
        ui.horizontal_wrapped(|ui| {
            for (i, rid) in order.iter().enumerate() {
                let Some(r) = self.rules.iter().find(|r| r.id == *rid) else { continue };
                if !q.is_empty() && i != sel && !self.rule_matches(r, &q) {
                    continue;
                }
                let name: String = r.name.chars().take(32).collect();
                if rule_tab(ui, &format!("Règle {} : {}", i + 1, name), i == sel, r.enabled, accent).clicked() {
                    self.selected_rule = i;
                }
            }
        });
        ui.add_space(14.0);

        let rid = order[sel];
        let Some(idx) = self.rules.iter().position(|r| r.id == rid) else { return };
        let mut deleted = false;
        let mut moved = false;

        // ---- configuration générale
        panel_card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(caps("NOM DE LA RÈGLE"));
                    let mut name = self.rules[idx].name.clone();
                    if ui.add(egui::TextEdit::singleline(&mut name).margin(egui::Margin::symmetric(8.0, 7.0)).desired_width(300.0)).changed() {
                        self.rules[idx].name = name;
                    }
                });
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.label(caps("DOSSIER CIBLE"));
                    let names: Vec<String> = self.folders.iter().map(|f| shorten(&f.path.display().to_string())).collect();
                    let cur = self
                        .folders
                        .iter()
                        .position(|f| f.id == self.rules[idx].folder_id)
                        .and_then(|p| names.get(p).cloned())
                        .unwrap_or_default();
                    if let Some(pick) = combo(ui, "rule-folder", &cur, &names) {
                        if let Some(p) = names.iter().position(|n| *n == pick) {
                            let fid = self.folders[p].id;
                            let count = self.rules.iter().filter(|r| r.folder_id == fid).count();
                            self.rules[idx].folder_id = fid;
                            self.rules[idx].priority = count as i64;
                            moved = true;
                        }
                    }
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if btn(ui, "Supprimer la règle", Btn::Danger, accent, None).clicked() {
                        deleted = true;
                    }
                    ui.add_space(8.0);
                    let mut en = self.rules[idx].enabled;
                    if check_box(ui, &mut en, "Activée", accent).changed() {
                        self.rules[idx].enabled = en;
                    }
                });
            });
            ui.add_space(14.0);
            ui.horizontal(|ui| {
                let current = if self.rules[idx].match_all { 0 } else { 1 };
                if let Some(i) = segmented(
                    ui,
                    egui::Id::new(("match", rid)),
                    &["TOUTES les conditions", "AU MOINS UNE condition"],
                    current,
                    SKY_DEEP,
                ) {
                    self.rules[idx].match_all = i == 0;
                }
                ui.add_space(20.0);
                ui.label(caps("RÉCURSION"));
                let mut depth = self.rules[idx].recursion_depth;
                if slim_slider(ui, &mut depth, 0, 8, accent) {
                    self.rules[idx].recursion_depth = depth;
                }
                chip(ui, &depth.to_string(), accent, with_alpha(accent, 38));
            });
            ui.add_space(4.0);
            ui.label(
                RichText::new("0 = premier niveau seulement ; n = n niveaux de plus.")
                    .size(13.5)
                    .color(text_dim()),
            );
        });

        if deleted {
            self.rules.retain(|r| r.id != rid);
            self.selected_rule = sel.saturating_sub(1);
            return;
        }
        if moved {
            self.selected_rule = self.rule_order().iter().position(|r| *r == rid).unwrap_or(0);
        }
        ui.add_space(14.0);

        // ---- conditions
        let mut reorder_cond: Option<(usize, usize)> = None;
        panel_card(ui, |ui| {
            ui.horizontal(|ui| {
                section_title(ui, Icon::Funnel, "Conditions");
                let n = self.rules[idx].conditions.len();
                chip(
                    ui,
                    &format!("{n} {}", if n > 1 { "critères actifs" } else { "critère actif" }),
                    SKY,
                    with_alpha(SKY, 38),
                );
            });
            ui.add_space(10.0);
            let mut drop: Option<usize> = None;
            for ci in 0..self.rules[idx].conditions.len() {
                let row = row_frame(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.dnd_drag_source(egui::Id::new(("drag", rid, 0u8, ci)), DragRow { list: 0, idx: ci }, |ui| {
                            let (r, _) = ui.allocate_exact_size(Vec2::new(14.0, 26.0), egui::Sense::hover());
                            draw_icon(&ui.painter().clone(), r.center(), Icon::Grip, text_dim(), 16.0);
                        });
                        let field = self.rules[idx].conditions[ci].field;
                        if let Some(pick) = combo(
                            ui,
                            &format!("cf-{idx}-{ci}"),
                            field.label(),
                            &FIELDS.iter().map(|f| f.label()).collect::<Vec<_>>(),
                        ) {
                            if let Some(f) = FIELDS.iter().find(|f| f.label() == pick) {
                                self.rules[idx].conditions[ci].field = *f;
                                self.rules[idx].conditions[ci].op = f.ops()[0];
                            }
                        }
                        let op = self.rules[idx].conditions[ci].op;
                        let ops = field.ops();
                        if let Some(pick) = combo(
                            ui,
                            &format!("co-{idx}-{ci}"),
                            op.label(),
                            &ops.iter().map(|o| o.label()).collect::<Vec<_>>(),
                        ) {
                            if let Some(o) = ops.iter().find(|o| o.label() == pick) {
                                self.rules[idx].conditions[ci].op = *o;
                            }
                        }
                        let hint = match field {
                            Field::Extension => ".png, .jpg",
                            Field::Size => "50 MB / 512 KB",
                            Field::DateAdded | Field::DateModified => "2024-01-31  ou  30 days",
                            Field::Kind => "document / image / video / audio / archive / folder",
                            Field::Contents => "texte à chercher",
                            _ => "valeur",
                        };
                        let mut val = self.rules[idx].conditions[ci].value.clone();
                        let w = (ui.available_width() - 60.0).max(80.0);
                        if ui.add(egui::TextEdit::singleline(&mut val).margin(egui::Margin::symmetric(8.0, 7.0)).desired_width(w).hint_text(hint)).changed() {
                            self.rules[idx].conditions[ci].value = val;
                        }
                        if cross_button(ui, "Supprimer") {
                            drop = Some(ci);
                        }
                    });
                });
                let zone = ui.interact(row.response.rect, egui::Id::new(("drop", rid, 0u8, ci)), egui::Sense::hover());
                if let Some(m) = drop_target(ui, &zone, 0, ci) {
                    reorder_cond = Some(m);
                }
                ui.add_space(6.0);
            }
            if let Some(ci) = drop {
                self.rules[idx].conditions.remove(ci);
            }
            if btn(ui, "+ Ajouter une condition", Btn::Sky, accent, None).clicked() {
                self.rules[idx].conditions.push(Condition {
                    field: Field::Name,
                    op: Op::Contains,
                    value: String::new(),
                });
            }
        });
        if let Some((from, insert)) = reorder_cond {
            reorder(&mut self.rules[idx].conditions, from, insert);
        }
        ui.add_space(14.0);

        // ---- actions
        let mut reorder_act: Option<(usize, usize)> = None;
        panel_card(ui, |ui| {
            ui.horizontal(|ui| {
                section_title(ui, Icon::Bolt, "Actions");
                ui.label(
                    RichText::new("exécutées dans l'ordre")
                        .size(14.0)
                        .color(text_dim()),
                );
            });
            ui.add_space(10.0);
            let mut drop: Option<usize> = None;
            for ai in 0..self.rules[idx].actions.len() {
                let row = row_frame(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.dnd_drag_source(egui::Id::new(("drag", rid, 1u8, ai)), DragRow { list: 1, idx: ai }, |ui| {
                            let (r, _) = ui.allocate_exact_size(Vec2::new(14.0, 26.0), egui::Sense::hover());
                            draw_icon(&ui.painter().clone(), r.center(), Icon::Grip, text_dim(), 16.0);
                        });
                        let kind = self.rules[idx].actions[ai].kind.clone();
                        if let Some(pick) = combo(
                            ui,
                            &format!("ak-{idx}-{ai}"),
                            kind.label(),
                            &ACTION_KINDS.iter().map(|k| k.label()).collect::<Vec<_>>(),
                        ) {
                            if let Some(k) = ACTION_KINDS.iter().find(|k| k.label() == pick) {
                                self.rules[idx].actions[ai] = Action::new(k.clone());
                            }
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if cross_button(ui, "Supprimer") {
                                drop = Some(ai);
                            }
                        });
                    });
                    let kind = self.rules[idx].actions[ai].kind.clone();
                    for (k, placeholder) in kind.params() {
                        if *k == ON_CONFLICT_PARAM_KEY {
                            continue;
                        }
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.add_space(22.0);
                            let is_dest = *k == "destination";
                            let reserve = if is_dest { 110.0 } else { 24.0 };
                            let entry = self.rules[idx].actions[ai].params.entry((*k).to_string()).or_default();
                            ui.add(
                                egui::TextEdit::singleline(entry)
                                    .margin(egui::Margin::symmetric(8.0, 7.0))
                                    .desired_width((ui.available_width() - reserve).max(120.0))
                                    .hint_text(*placeholder),
                            );
                            if is_dest && ui.add(egui::Button::new(RichText::new("Choisir…").size(14.0)).fill(glass(34))).clicked() {
                                if let Some(dir) = rfd::FileDialog::new().set_directory(home_dir()).pick_folder() {
                                    entry.clear();
                                    entry.push_str(&dir.to_string_lossy());
                                }
                            }
                        });
                    }
                    if matches!(kind, ActionKind::Move | ActionKind::Copy) {
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.add_space(22.0);
                            ui.label(caps("EN CAS DE CONFLIT"));
                            let current = match self.rules[idx].actions[ai].param(ON_CONFLICT_PARAM_KEY).as_str() {
                                "replace" => 0,
                                "number" => 2,
                                _ => 1,
                            };
                            if let Some(i) = segmented(
                                ui,
                                egui::Id::new(("conflict", rid, ai)),
                                &["remplacer", "ignorer", "numéroter"],
                                current,
                                accent,
                            ) {
                                let v = ["replace", "skip", "number"][i];
                                self.rules[idx].actions[ai].params.insert(ON_CONFLICT_PARAM_KEY.into(), v.into());
                            }
                        });
                    }
                });
                let zone = ui.interact(row.response.rect, egui::Id::new(("drop", rid, 1u8, ai)), egui::Sense::hover());
                if let Some(m) = drop_target(ui, &zone, 1, ai) {
                    reorder_act = Some(m);
                }
                ui.add_space(6.0);
            }
            if let Some(ai) = drop {
                self.rules[idx].actions.remove(ai);
            }
            if btn(ui, "+ Ajouter une action", Btn::Sky, accent, None).clicked() {
                self.rules[idx].actions.push(Action::new(ActionKind::Rename));
            }
        });
        if let Some((from, insert)) = reorder_act {
            reorder(&mut self.rules[idx].actions, from, insert);
        }
    }

    fn history_ui(&mut self, ui: &mut egui::Ui) {
        let accent = self.accent();
        self.header(ui, "Historique", "Actions par lot, annulables si possible.");

        if self.history.is_empty() {
            ui.label(RichText::new("Rien pour l'instant. Lancez un lot ou un aperçu.").color(text_dim()));
            return;
        }

        let total = self.history.len();
        let start = total.saturating_sub(HISTORY_PAGE_SIZE);
        let mut undo_idx: Option<usize> = None;
        let mut batch: Option<u64> = None;

        for (i, e) in self.history.iter().enumerate().skip(start) {
            if batch != Some(e.batch_id) {
                batch = Some(e.batch_id);
                ui.label(
                    RichText::new(format!(
                        "Lot n°{}  ·  {}  ·  {}",
                        e.batch_id,
                        date_str(e.created),
                        time_str(e.created)
                    ))
                    .size(14.0)
                    .family(semibold())
                    .color(text_dim()),
                );
                ui.add_space(8.0);
            }
            card(ui, Vec2::new(756.0, 72.0), |ui| {
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(10.0, 10.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 5.0, status_color(e.status));
                    ui.add_space(12.0);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(format!("{} — {}", e.rule_name, e.action_label))
                                .size(15.0)
                                .family(semibold())
                                .color(text_primary()),
                        );
                        ui.label(
                            RichText::new(format!(
                                "{}  →  {}  ·  {}",
                                shorten(&e.original_path.display().to_string()),
                                shorten(&e.result_path.display().to_string()),
                                fr_message(&e.message)
                            ))
                            .size(13.5)
                            .color(text_dim()),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if e.reversible && e.status == Status::Applied {
                            if pill(ui, "Annuler", accent, false, false).clicked() {
                                undo_idx = Some(i);
                            }
                        } else {
                            ui.label(
                                RichText::new(e.status.label())
                                    .size(13.5)
                                    .color(status_color(e.status)),
                            );
                        }
                    });
                });
            });
            ui.add_space(8.0);
        }

        if total > HISTORY_PAGE_SIZE {
            ui.label(
                RichText::new(format!(
                    "{HISTORY_PAGE_SIZE} dernières entrées sur {total}."
                ))
                .size(13.5)
                .color(text_dim()),
            );
        }
        if let Some(i) = undo_idx {
            self.undo(i);
        }
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) {
        let accent = self.accent();
        self.header(ui, "Réglages", "Appliqués immédiatement.");

        card(ui, Vec2::new(756.0, 132.0), |ui| {
            ui.label(RichText::new("Couleur d'accent").size(15.0).family(semibold()).color(text_primary()));
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                for (i, (name, c)) in ACCENTS.iter().enumerate() {
                    let (rect, resp) = ui.allocate_exact_size(Vec2::new(34.0, 34.0), egui::Sense::click());
                    ui.painter().rect_filled(rect, Rounding::same(11.0), *c);
                    if self.accent == i {
                        ui.painter().circle_filled(rect.center(), 5.0, Color32::WHITE);
                    }
                    if resp.clicked() {
                        self.accent = i;
                    }
                    if resp.hovered() {
                        ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
                    }
                    let _ = name;
                    ui.add_space(8.0);
                }
            });
            ui.add_space(8.0);
            ui.label(RichText::new("Orange, bleu, violet, rose, jaune, vert, graphite.").size(13.5).color(text_dim()));
        });

        ui.add_space(16.0);
        card(ui, Vec2::new(756.0, 314.0), |ui| {
            let before = self.launch_at_login;
            check_box(ui, &mut self.launch_at_login, "Lancer à l'ouverture de la session", accent);
            if self.launch_at_login != before {
                if let Err(e) = crate::login::set_enabled(self.launch_at_login) {
                    self.launch_at_login = before;
                    self.toast(format!("Lancement au démarrage impossible : {e}"));
                }
            }
            ui.add_space(8.0);
            check_box(ui, &mut self.menu_bar, "Afficher dans la barre des menus", accent);
            ui.add_space(8.0);
            check_box(ui, &mut self.notify, "Prévenir après chaque organisation", accent);
            ui.add_space(8.0);
            check_box(ui, &mut self.animate_bg, "Animer le fond", accent);
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Garder l'historique").size(14.5).color(text_primary()));
                ui.add_space(8.0);
                slim_slider(ui, &mut self.retention_days, 1, 365, accent);
                chip(ui, &format!("{} jours", self.retention_days), accent, with_alpha(accent, 38));
            });
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!(
                    "Les entrées plus anciennes sont purgées.",
                ))
                .size(13.5)
                .color(text_dim()),
            );
            ui.label(
                RichText::new(format!("{HISTORY_RETENTION_SETTING_KEY} · de 1 à 365 jours"))
                    .size(13.5)
                    .color(text_dim()),
            );
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new(RichText::new("Purger l'ancien historique").size(14.0))
                            .fill(glass(34)),
                    )
                    .clicked()
                {
                    let removed = purge_history(&mut self.history, self.retention_days, SystemTime::now());
                    self.toast(format!("{removed} entrée(s) purgée(s)"));
                }
            });
        });

        ui.add_space(16.0);
        card(ui, Vec2::new(756.0, 110.0), |ui| {
            ui.label(RichText::new("À propos").size(15.0).family(semibold()).color(text_primary()));
            ui.add_space(8.0);
            ui.label(
                RichText::new(format!("OrganizeMyFiles · version {}", env!("CARGO_PKG_VERSION")))
                    .size(14.5)
                    .color(text_primary()),
            );
            ui.add_space(4.0);
            ui.label(
                RichText::new("Outil libre et open source. Tout reste sur ton Mac : aucun envoi de données.")
                    .size(13.5)
                    .color(text_dim()),
            );
        });
        let _ = accent;
    }
}

fn shorten(p: &str) -> String {
    shorten_with_home(p, &home_dir())
}

/// Raccourcit un chemin : `~/…` sous le dossier personnel, chemin absolu sinon.
fn shorten_with_home(p: &str, home: &str) -> String {
    let full = match std::path::Path::new(p).strip_prefix(home) {
        Ok(rel) if rel.as_os_str().is_empty() => "~".to_string(),
        Ok(rel) => format!("~/{}", rel.to_string_lossy()),
        Err(_) => p.to_string(),
    };
    let len = full.chars().count();
    if len > 44 {
        let tail: String = full.chars().skip(len - 44).collect();
        format!("…{tail}")
    } else {
        full
    }
}

fn save_png(path: &str, shot: &egui::ColorImage) {
    let w = shot.width() as u32;
    let h = shot.height() as u32;
    let mut rgba = Vec::with_capacity(shot.pixels.len() * 4);
    for p in &shot.pixels {
        rgba.extend_from_slice(&p.to_array());
    }
    if let Some(parent) = std::path::Path::new(path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = image::save_buffer(path, &rgba, w, h, image::ColorType::Rgba8);
    eprintln!("screenshot saved to {path} ({w}x{h})");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn shorten_handles_home_and_outside_home() {
        assert_eq!(shorten_with_home("/Users/a/Downloads", "/Users/a"), "~/Downloads");
        assert_eq!(shorten_with_home("/Users/a", "/Users/a"), "~");
        assert_eq!(shorten_with_home("/tmp", "/Users/a"), "/tmp");
        assert_eq!(shorten_with_home("/Users/ab/x", "/Users/a"), "/Users/ab/x");
    }

    #[test]
    fn ago_formats_relative_time() {
        let now = SystemTime::now();
        assert_eq!(ago(now - Duration::from_secs(10), now), "à l'instant");
        assert_eq!(ago(now - Duration::from_secs(5 * 60), now), "il y a 5 min");
        assert_eq!(ago(now - Duration::from_secs(3 * 3600), now), "il y a 3 h");
        assert_eq!(ago(now - Duration::from_secs(30 * 3600), now), "hier");
        assert_eq!(ago(now - Duration::from_secs(5 * 86_400), now), "il y a 5 j");
    }

    #[test]
    fn background_drifts_over_time() {
        let accent = ACCENTS[0].1;
        let (mut moved, mut total) = (0, 0);
        for (u, v) in [(0.2, 0.1), (0.5, 0.5), (0.9, 0.9), (0.3, 0.8)] {
            let (a, b) = (bg_color(u, v, 0.0, 1.5, accent), bg_color(u, v, 20.0, 1.5, accent));
            moved += (a.r() as i32 - b.r() as i32).abs() + (a.g() as i32 - b.g() as i32).abs() + (a.b() as i32 - b.b() as i32).abs();
            total += 1;
        }
        assert!(moved / total >= 6, "le fond doit visiblement évoluer en 20 s (écart {moved})");
        // Fond figé quand le temps ne change pas.
        assert_eq!(bg_color(0.4, 0.4, 3.0, 1.5, accent), bg_color(0.4, 0.4, 3.0, 1.5, accent));
    }

    #[test]
    fn app_state_survives_snapshot_and_restore() {
        let launch = || Launch { bg_time: 0.0, screenshot: None, tab: None, empty: false, background: false };
        let mut a = App::blank(launch());
        a.seed_demo();
        a.accent = 3;
        a.animate_bg = false;
        a.retention_days = 90;
        a.monthly_used = 17;
        let saved = a.snapshot();
        let json = store::to_json(&saved);

        let mut b = App::blank(launch());
        b.restore(saved);
        assert_eq!(store::to_json(&b.snapshot()), json, "restaurer puis re-photographier doit être identique");
        assert_eq!((b.accent, b.animate_bg, b.retention_days, b.monthly_used), (3, false, 90, 17));
        assert!(b.next_id > b.rules.iter().map(|r| r.id).max().unwrap(), "les identifiants ne doivent pas se recouvrir");
    }

    #[test]
    fn restore_applies_history_retention() {
        let mut a = App::blank(Launch { bg_time: 0.0, screenshot: None, tab: None, empty: false, background: false });
        a.seed_demo();
        let mut state = a.snapshot();
        state.settings.retention_days = 1;
        state.history[0].created = SystemTime::now() - std::time::Duration::from_secs(40 * 86_400);
        let before = state.history.len();
        let mut b = App::blank(Launch { bg_time: 0.0, screenshot: None, tab: None, empty: false, background: false });
        b.restore(state);
        assert_eq!(b.history.len(), before - 1);
    }

    #[test]
    fn hero_state_follows_setup_progress() {
        assert_eq!(hero_state(0, 0, false), HeroState::NoFolder);
        assert_eq!(hero_state(2, 0, false), HeroState::NoRule);
        assert_eq!(hero_state(2, 3, false), HeroState::Ready);
        assert_eq!(hero_state(2, 3, true), HeroState::Done);
    }
}
