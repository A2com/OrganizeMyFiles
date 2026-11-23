//! Interface: soft glass cards, rounded pill buttons, colour sidebar —
//! rebuilt from scratch (no original assets or branding).

use std::path::PathBuf;
use std::time::SystemTime;

use eframe::egui::{self, Align, Color32, Layout, Pos2, Rect, RichText, Rounding, Stroke, Vec2};

use crate::alerts;
use crate::engine::*;
use crate::history_view::{self, ago};
use crate::intro::{self, IntroFrame};
use crate::mechanics::*;
use crate::menubar::{self, MenuBar, MenuCommand, MenuScreen};
use crate::menubar_info::{self, MenuFolder, MenuInfo};
use crate::sound::{Sound, Sounds};
use crate::store::{self, Loaded, RulesMode, Settings, State};
use crate::system;
use crate::tags;
use crate::toast::{self, Toast, ToastKind};
use crate::update;
use crate::watcher::{self, WatchConfig, Watcher};
use simple_rules::SimpleView;

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
    Sparkle,
    ExpandMore,
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
/// Le thème est forcé sombre : sinon egui suit le réglage macOS et, en mode clair,
/// les champs de saisie deviennent blancs avec un texte illisible.
fn apply_theme(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::Dark);
    let mut v = egui::Visuals::dark();
    v.panel_fill = Color32::TRANSPARENT;
    v.window_fill = Color32::from_rgb(0x1a, 0x1d, 0x27);
    v.extreme_bg_color = with_alpha(CANVAS, 150); // fond des champs de saisie
    v.faint_bg_color = glass(10);
    v.override_text_color = Some(text_primary());
    v.selection.bg_fill = with_alpha(ACCENTS[0].1, 90);
    v.selection.stroke = Stroke::new(1.0_f32, ACCENTS[0].1);
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
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, with_alpha(ACCENTS[0].1, 120));
    v.widgets.active.bg_stroke = Stroke::new(1.0_f32, ACCENTS[0].1);
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
    /// Règles supprimées avec le dossier (il en était la seule cible).
    rules: Vec<Rule>,
    /// Règles gardées, qui ont seulement perdu ce dossier.
    detached: Vec<u64>,
    index: usize,
    at: std::time::Instant,
}

/// Intensité des halos lumineux (1 = d'origine) : réduite, ils étaient trop brillants. Un seul réglage pour tous.
const GLOW_SOFTNESS: f32 = 0.45;

const UNDO_SECONDS: u64 = 8;

/// Barre du haut : largeurs fixes et places à réserver.
const STATUS_PILL_W: f32 = 112.0;
const ICON_BUTTON_W: f32 = 32.0;
/// La recherche ne descend pas sous cette largeur ; le raccourci « ⌘ K » disparaît sous la seconde.
const SEARCH_MIN_W: f32 = 160.0;
const SEARCH_HINT_MIN_W: f32 = 240.0;

/// Largeur réelle du groupe de droite de la barre du haut (pastille, boutons, et en grande fenêtre cloche et
/// réglages), écarts compris, plus une petite marge. La recherche prend le reste : jamais de chevauchement.
fn top_bar_right_width(run_w: f32, preview_w: f32, wide: bool, gap: f32) -> f32 {
    let mut widths = vec![STATUS_PILL_W, preview_w, run_w];
    if wide {
        widths.extend([ICON_BUTTON_W, ICON_BUTTON_W]);
    }
    let bell_extra = if wide { 4.0 } else { 0.0 };
    widths.iter().sum::<f32>() + gap * (widths.len() - 1) as f32 + bell_extra + 4.0
}

const APP_ICON_PNG: &[u8] = include_bytes!("../../assets/icons/app-256.png");

/// Icône de la fenêtre (barre des tâches). Sous macOS, on n'en impose aucune : le Dock affiche l'icône du `.app`,
/// qui respecte le gabarit Apple (corps de 824 px sur 1024) ; l'image pleine taille la ferait paraître trop grosse.
pub fn app_icon() -> egui::IconData {
    if cfg!(target_os = "macos") {
        return egui::IconData::default();
    }
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
    /// Montre l'écran d'introduction (option `--intro`).
    pub show_intro: bool,
    /// Fige l'introduction à cet instant, en secondes (captures d'écran).
    pub intro_time: Option<f64>,
}

/// Écran d'introduction : avancement de l'animation.
struct IntroState {
    /// Instant (horloge egui) du premier affichage.
    started: Option<f64>,
    /// Passée : on saute à l'état final.
    skipped: bool,
    /// Instant du clic sur « Commencer » (début du fondu de sortie).
    leaving: Option<f64>,
    /// Instant figé (captures d'écran) : pas d'avancement automatique, pas de sortie.
    frozen: Option<f32>,
}

impl IntroState {
    fn new(frozen: Option<f32>) -> Self {
        IntroState { started: None, skipped: false, leaving: None, frozen }
    }
}

/// Illustration du dossier (écran Règles, mode simple).
const INTRO_ICON_PNG: &[u8] = include_bytes!("../../assets/icons/intro-600.png");
/// Taille du mot « Bienvenue » par rapport à sa taille d'origine (réduit de moitié à la demande d'Andy).
const WORD_SCALE: f32 = 0.5;
/// Largeurs d'affichage du logotype (points) : barre latérale, écran de bienvenue, « À propos ».
const WORDMARK_SIDEBAR_W: f32 = 128.0;
const WORDMARK_INTRO_W: f32 = 170.0;
const WORDMARK_ABOUT_W: f32 = 150.0;
/// Logotype « OrganizeMyFiles » (texte blanc, dégradé sur « O » et « My »), fourni par Andy (2216 px), réduit à l'affichage par `fitted_texture`.
const WORDMARK_PNG: &[u8] = include_bytes!("../../assets/logo/wordmark-white.png");

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
    /// Réglage macOS « Réduire les animations » (lu au lancement) : coupe les animations en plus de « Animer le fond ».
    system_reduce_motion: bool,
    bg_offset: f64,
    retention_days: u32,
    add_folder_text: String,
    selected_rule: usize,
    /// Étape courante de l'assistant du mode simple (1 à 3).
    simple_step: u8,
    /// Écran du mode simple des règles : accueil, assistant ou confirmation.
    simple_view: SimpleView,
    app_enabled: bool,
    launch_at_login: bool,
    /// Passage complet automatique toutes les N minutes ; 0 = désactivé.
    auto_organize_minutes: u32,
    /// Écran Règles : assistant simple ou écran complet.
    rules_mode: RulesMode,
    /// Chercher les mises à jour automatiquement.
    check_updates: bool,
    update: update::State,
    update_rx: Option<std::sync::mpsc::Receiver<update::Msg>>,
    /// Version que l'utilisateur a repoussée (« Plus tard ») : le bandeau ne la remontre pas avant le prochain lancement.
    update_dismissed: Option<String>,
    next_update_check: f64,
    welcome_seen: bool,
    /// Écran d'introduction en cours (premier lancement, ou « Revoir l'introduction »).
    intro: Option<IntroState>,
    intro_seen: bool,
    /// Réglage « Jouer les sons ».
    sounds_on: bool,
    /// Lecteur de sons ; `None` en capture d'écran et dans les tests.
    sounds: Option<Sounds>,
    /// Le son d'ouverture de l'Aperçu a déjà été traité pour ce lancement.
    launch_sound_done: bool,
    /// Dossiers suivis que macOS ne laisse pas lire (rafraîchi régulièrement).
    denied_folders: Vec<PathBuf>,
    next_access_check: f64,
    /// Surveillance automatique ; `None` en mode capture d'écran et dans les tests.
    watcher: Option<Watcher>,
    watch_key: String,
    /// Icône et menu de la barre des menus (macOS).
    menubar: Option<MenuBar>,
    menubar_failed: bool,
    /// Vrai quand l'utilisateur a choisi « Quitter » : la fermeture n'est plus interceptée.
    quitting: bool,
    /// La fenêtre est fermée (l'appli tourne dans la barre des menus).
    window_hidden: bool,
    search: String,
    history_filter: history_view::StatusFilter,
    /// Nombre d'actions affichées dans l'historique (+50 à chaque « Afficher plus »).
    history_limit: usize,
    toast: Option<Toast>,
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
    start_hidden: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, launch: Launch) -> Self {
        apply_theme(&cc.egui_ctx);
        let empty = launch.empty;
        let tab = launch.tab.clone();
        let (show_intro, intro_time) = (launch.show_intro, launch.intro_time);
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
        if show_intro {
            app.intro = Some(IntroState::new(intro_time.map(|t| t as f32)));
        }
        if app.screenshot.is_none() {
            app.sounds = Some(Sounds::new());
        }
        if app.screenshot.is_some() && std::env::var_os("ORGANIZEMYFILES_SHOW_WELCOME").is_none() {
            app.welcome_seen = true;
        }
        app.tab = match tab.as_deref() {
            Some("folders") => Tab::Folders,
            Some("rules") => Tab::Rules,
            Some("history") => Tab::History,
            Some("settings") => Tab::Settings,
            _ => Tab::Dashboard,
        };
        if app.screenshot.is_some() {
            // Captures de l'écran Règles : OMF_RULES_STEP (1 à 3) et OMF_RULE_INDEX (0 = première règle).
            let read = |key: &str| std::env::var(key).ok().and_then(|v| v.parse::<usize>().ok());
            app.simple_step = read("OMF_RULES_STEP").map_or(1, |n| n.clamp(1, 3) as u8);
            app.selected_rule = read("OMF_RULE_INDEX").unwrap_or(0);
            // OMF_RULES_VIEW : home (par défaut), wizard ou done ; OMF_RULES_STEP force l'assistant.
            let done = app.rule_order().get(app.selected_rule).copied().map(SimpleView::Done);
            app.simple_view = match std::env::var("OMF_RULES_VIEW").as_deref() {
                Ok("done") => done.unwrap_or(SimpleView::Home),
                Ok("wizard") => SimpleView::Wizard,
                _ if std::env::var_os("OMF_RULES_STEP").is_some() => SimpleView::Wizard,
                _ => SimpleView::Home,
            };
        }
        app
    }

    /// État initial sans aucun contexte graphique (utilisable dans les tests).
    fn blank(launch: Launch) -> Self {
        let Launch { screenshot, tab: _, empty: _, bg_time, background, show_intro: _, intro_time: _ } = launch;
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
            system_reduce_motion: system_reduces_motion(),
            bg_offset: bg_time,
            retention_days: HISTORY_RETENTION_DEFAULT_DAYS,
            add_folder_text: String::new(),
            selected_rule: 0,
            simple_step: 1,
            simple_view: SimpleView::Home,
            app_enabled: true,
            launch_at_login: false,
            auto_organize_minutes: 0,
            rules_mode: RulesMode::Simple,
            check_updates: true,
            update: update::State::Idle,
            update_rx: None,
            update_dismissed: None,
            next_update_check: 0.0,
            welcome_seen: false,
            intro: None,
            intro_seen: true,
            sounds_on: true,
            sounds: None,
            launch_sound_done: false,
            denied_folders: Vec::new(),
            next_access_check: 0.0,
            watcher: None,
            watch_key: String::new(),
            menubar: None,
            menubar_failed: false,
            quitting: false,
            window_hidden: false,
            search: String::new(),
            history_filter: history_view::StatusFilter::All,
            history_limit: HISTORY_PAGE_SIZE,
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
            start_hidden: background,
        }
    }

    /// Demo content so the saved picture shows a living app.
    fn seed_demo(&mut self) {
        let home = home_dir();
        for (p, prio) in [("Downloads", 0i64), ("Desktop", 1), ("Documents", 2)] {
            self.folders.push(Folder { id: self.next_id, path: PathBuf::from(format!("{home}/{p}")), enabled: true, priority: prio });
            self.next_id += 1;
        }
        self.folders.push(Folder { id: self.next_id, path: PathBuf::from(format!("{home}/Projets-Clients")), enabled: false, priority: 3 });
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
                action_label: ["Déplacer vers un dossier", "Renommer avec un motif", "Compresser (ZIP)"][i % 3].into(),
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
                message: if i == 5 { "un fichier existe déjà à la destination".into() } else { "déplacé → Archive/2026-11".into() },
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

    /// Message d'état en bas de la fenêtre : pastille verte.
    fn toast(&mut self, msg: impl Into<String>) {
        self.toast = Some(Toast { text: msg.into(), kind: ToastKind::Ok, since: std::time::Instant::now() });
    }

    fn show_toast(&mut self, text: String, kind: ToastKind) {
        match kind {
            ToastKind::Ok => self.toast(text),
            ToastKind::Error => self.toast_error(text),
        }
    }

    /// Message d'erreur en bas de la fenêtre : pastille rouge.
    fn toast_error(&mut self, msg: impl Into<String>) {
        self.toast = Some(Toast { text: msg.into(), kind: ToastKind::Error, since: std::time::Instant::now() });
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
                auto_organize_minutes: self.auto_organize_minutes,
                rules_mode: self.rules_mode,
                check_updates: self.check_updates,
                welcome_seen: self.welcome_seen,
                intro_seen: self.intro_seen,
                sounds: self.sounds_on,
            },
            next_id: self.next_id,
            next_batch: self.next_batch,
            monthly_used: self.monthly_used,
            last_run: self.last_run,
            last_run_dry: self.last_run_dry,
        }
    }

    fn restore(&mut self, state: State) {
        let max_id = state.folders.iter().map(|f| f.id).chain(state.rules.iter().map(|r| r.id)).max().unwrap_or(0);
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
        self.auto_organize_minutes = state.settings.auto_organize_minutes;
        self.rules_mode = state.settings.rules_mode;
        self.check_updates = state.settings.check_updates;
        self.welcome_seen = state.settings.welcome_seen;
        self.intro_seen = state.settings.intro_seen;
        self.sounds_on = state.settings.sounds;
        if !self.intro_seen && !self.start_hidden {
            self.intro = Some(IntroState::new(None));
        }
        self.next_id = state.next_id.max(max_id + 1);
        self.next_batch = state.next_batch.max(1);
        self.monthly_used = state.monthly_used;
        self.last_run = state.last_run;
        self.last_run_dry = state.last_run_dry;
        // La rétention s'applique dès le chargement.
        purge_history(&mut self.history, self.retention_days, SystemTime::now());
    }

    /// Premier lancement : rien à faire. Sinon restaure les données. Un fichier illisible est mis de côté
    /// et, si une copie de secours existe, elle est restaurée automatiquement.
    fn load_saved_state(&mut self) {
        let path = store::state_path();
        let loaded = store::load_with_backup(&path);
        let restored_from_backup = matches!(&loaded, Loaded::RestoredFromBackup { .. });
        match loaded {
            Loaded::Fresh => {
                // Tout premier lancement : on commence par l'écran d'introduction (sauf démarrage en arrière-plan).
                if !self.start_hidden {
                    self.intro_seen = false;
                    self.intro = Some(IntroState::new(None));
                }
            }
            Loaded::Restored(state) => self.restore(*state),
            Loaded::RestoredFromBackup { state, backup, reason } => {
                self.restore(*state);
                self.toast_error(format!("Données abîmées ({reason}) : copie de secours restaurée depuis {}", backup.display()));
            }
            Loaded::Unreadable { backup, reason } => {
                self.toast_error(format!("Données illisibles ({reason}) : copie gardée dans {}", backup.display()));
            }
        }
        self.last_saved = store::to_json(&self.snapshot());
        if restored_from_backup {
            // Le fichier principal a été mis en quarantaine : on le réécrit tout de suite, sinon les données
            // restaurées seraient perdues au prochain démarrage si l'utilisateur ne change rien.
            if let Err(e) = store::save(&path, &self.last_saved) {
                self.toast_error(format!("Sauvegarde impossible : {e}"));
            }
        }
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
            Err(e) => self.toast_error(format!("Sauvegarde impossible : {e}")),
        }
    }

    fn run_batch(&mut self, dry_run: bool) {
        if self.folders.is_empty() {
            self.toast_error("Ajoutez d'abord un dossier");
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
        if !dry_run && self.notify {
            if let Some(text) = alerts::summary(&outcome.history) {
                alerts::notify(text);
            }
        }
        let failed = outcome.history.iter().filter(|e| matches!(e.status, Status::Failed | Status::BlockedByConflict)).count();
        for e in outcome.history {
            self.history.push(e);
        }
        purge_history(&mut self.history, self.retention_days, SystemTime::now());
        let (text, kind) = toast::batch_toast(if dry_run { "Aperçu : " } else { "" }, "", outcome.matched, failed);
        self.show_toast(text, kind);
    }

    /// Annule une entrée de l'historique et prévient la surveillance de ne pas la retraiter.
    fn undo_one(&mut self, idx: usize) -> Result<(), String> {
        let e = self.history.get_mut(idx).ok_or("entrée introuvable")?;
        undo_entry(e)?;
        e.status = Status::Undone;
        e.message = "restored".into();
        let paths = vec![e.original_path.clone(), e.result_path.clone()];
        if let Some(w) = &self.watcher {
            w.ignore(paths);
        }
        Ok(())
    }

    fn undo(&mut self, idx: usize) {
        match self.undo_one(idx) {
            Ok(()) => self.toast("Annulé"),
            Err(msg) => self.toast_error(format!("Annulation impossible : {msg}")),
        }
    }

    /// Annule tout un lot, de la dernière action à la première ; continue si l'une échoue.
    fn undo_batch(&mut self, entries: &[usize]) {
        let (mut done, mut failed) = (0, 0);
        for idx in history_view::undo_order(&self.history, entries) {
            match self.undo_one(idx) {
                Ok(()) => done += 1,
                Err(_) => failed += 1,
            }
        }
        match (done, failed) {
            (d, 0) => self.toast(format!("{d} action(s) annulée(s)")),
            (0, f) => self.toast_error(format!("Annulation impossible pour {f} action(s)")),
            (d, f) => self.toast_error(format!("{d} action(s) annulée(s), {f} impossible(s)")),
        }
    }

    /// Écran d'introduction : icône, « Bienvenue » écrit à la main, présentation et bouton « Commencer »,
    /// sur le fond animé. Remplace toute l'interface tant qu'il est affiché.
    fn intro_ui(&mut self, ui: &mut egui::Ui, screen: Rect) {
        let ctx = ui.ctx().clone();
        let now = ctx.input(|i| i.time);
        let accent = self.accent();
        let animated = self.animated();
        let intro_icon_size = (ctx.screen_rect().height() * 0.34).min(300.0);
        let texture = fitted_texture(&ctx, "intro-icon", INTRO_ICON_PNG, intro_icon_size);
        let Some(state) = self.intro.as_mut() else { return };

        // Temps de l'animation : figé (captures), final (mouvement réduit ou passée) ou horloge.
        let first_frame = state.started.is_none();
        let started = *state.started.get_or_insert(now);
        let reduced = !animated && state.frozen.is_none();
        // Le son d'introduction part avec l'animation (pas pour une capture figée, ni en mouvement réduit).
        let play_intro = first_frame && state.frozen.is_none() && !reduced;
        let t = match state.frozen {
            Some(t) => t,
            None if state.skipped || reduced => intro::END_TIME,
            None => (now - started) as f32,
        };
        let f: IntroFrame = intro::frame(t);
        let finished = t >= intro::END_TIME;
        let fade_out = state.leaving.map_or(1.0, |l| 1.0 - ((now - l) as f32 / intro::LEAVE_SECS).clamp(0.0, 1.0));
        if state.leaving.is_some() && fade_out <= 0.0 {
            self.intro = None;
            self.intro_seen = true;
            self.tab = Tab::Dashboard;
            ctx.request_repaint();
            return;
        }
        if state.frozen.is_none() && (!finished || state.leaving.is_some()) {
            ctx.request_repaint_after(std::time::Duration::from_millis(33));
        }

        // Fond : on part du noir, puis le dégradé animé apparaît ; fondu final vers l'appli.
        let painter = ui.painter().clone();
        painter.rect_filled(
            screen,
            Rounding::ZERO,
            Color32::from_black_alpha((235.0 * (1.0 - f.backdrop) + 235.0 * (1.0 - fade_out)).min(255.0) as u8),
        );

        // Disposition : bloc centré (icône, mot, présentation, bouton).
        let icon_size = (screen.height() * 0.34).min(300.0);
        let word_w = (icon_size * 1.85).min(screen.width() * 0.7) * WORD_SCALE;
        let strokes = intro::word();
        let k = word_w / strokes.width;
        let word_h = strokes.height * k;
        let gap = 22.0;
        let block_h = icon_size + gap + word_h + gap + 24.0 + gap + 52.0;
        let top = (screen.center().y - block_h / 2.0).max(screen.top() + 16.0);
        let cx = screen.center().x;
        let alpha = |a: f32| ((a * fade_out).clamp(0.0, 1.0) * 255.0) as u8;

        // Icône : halo doux, puis l'image qui apparaît avec un petit rebond.
        let icon_center = Pos2::new(cx, top + icon_size / 2.0);
        let breath = if animated { 1.0 + 0.05 * ((now as f32) * 1.6).sin() } else { 1.0 };
        radial_glow(&painter, icon_center, icon_size * 0.95 * breath * f.icon_scale, accent, 170.0 * f.icon_alpha * fade_out);
        let icon_rect = Rect::from_center_size(icon_center, Vec2::splat(icon_size * f.icon_scale));
        paint_fitted(&painter, &texture, icon_rect, Color32::from_white_alpha(alpha(f.icon_alpha)));

        // « Bienvenue » : tracé du stylo, bouts arrondis, pointe lumineuse (pas d'ombre).
        let word_left = cx - word_w / 2.0;
        let word_top = top + icon_size + gap;
        let to_screen = |p: intro::Point| Pos2::new(word_left + p[0] * k, word_top + p[1] * k);
        let pen = (strokes.pen * k * 3.0).max(3.0);
        let (drawn, tip) = strokes.drawn(f.write);
        for stroke in &drawn {
            let pts: Vec<Pos2> = stroke.iter().map(|p| to_screen(*p)).collect();
            painter.add(egui::Shape::line(pts.clone(), Stroke::new(pen, Color32::from_white_alpha(alpha(1.0)))));
            for end in [pts.first(), pts.last()].into_iter().flatten() {
                painter.circle_filled(*end, pen / 2.0, Color32::from_white_alpha(alpha(1.0)));
            }
        }
        if let (Some(tip), true) = (tip, f.write < 1.0 && f.write > 0.0) {
            let p = to_screen(tip);
            radial_glow(&painter, p, pen * 4.5, Color32::WHITE, 150.0 * fade_out);
        }

        // Présentation.
        painter.text(
            Pos2::new(cx, word_top + word_h + gap),
            egui::Align2::CENTER_TOP,
            "OrganizeMyFiles range vos fichiers pour vous.",
            egui::FontId::proportional(18.0),
            with_alpha(text_primary(), alpha(f.subtitle * 0.92)),
        );

        // Signature discrète : le logotype en pied d'écran, il apparaît après le bouton.
        {
            let w = WORDMARK_INTRO_W;
            let mark = fitted_texture(&ctx, "wordmark", WORDMARK_PNG, w);
            let h = w * mark.size_vec2().y / mark.size_vec2().x;
            let rect = Rect::from_center_size(Pos2::new(cx, screen.bottom() - 44.0 - h / 2.0), Vec2::new(w, h));
            paint_fitted(&painter, &mark, rect, Color32::from_white_alpha(alpha(f.button * 0.55)));
        }

        // Clic n'importe où ou Entrée / Espace / Échap : on saute à la fin (sans quitter l'écran).
        let anywhere = ui.interact(screen, ui.id().with("intro-skip-anywhere"), egui::Sense::click());
        let key = ctx.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space) || i.key_pressed(egui::Key::Escape));
        let mut start_app = false;
        let mut fade_intro_sound = false;
        if !finished && (anywhere.clicked() || key) {
            if let Some(state) = self.intro.as_mut() {
                state.skipped = true;
            }
            fade_intro_sound = true;
        } else if finished && key {
            start_app = true;
        }

        // « Passer » (discret) tant que l'animation n'est pas terminée.
        if !finished {
            let skip_rect = Rect::from_min_size(Pos2::new(screen.right() - 110.0, screen.bottom() - 44.0), Vec2::new(92.0, 28.0));
            let resp = ui.interact(skip_rect, ui.id().with("intro-skip"), egui::Sense::click());
            painter.text(
                skip_rect.center(),
                egui::Align2::CENTER_CENTER,
                "Passer  ›",
                egui::FontId::proportional(14.0),
                with_alpha(text_primary(), if resp.hovered() { 230 } else { 140 }),
            );
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                if let Some(state) = self.intro.as_mut() {
                    state.skipped = true;
                }
                fade_intro_sound = true;
            }
        }

        // Bouton « Commencer » : apparaît en dernier, avec le halo qui pulse.
        if f.button > 0.01 {
            let btn_rect = Rect::from_center_size(Pos2::new(cx, word_top + word_h + gap + 24.0 + gap + 26.0), Vec2::new(210.0, 52.0));
            let mut child = ui
                .new_child(egui::UiBuilder::new().max_rect(btn_rect).layout(Layout::centered_and_justified(egui::Direction::LeftToRight)));
            child.set_opacity((f.button * fade_out).clamp(0.0, 1.0));
            let slot = child.painter().add(egui::Shape::Noop);
            let run = pill(&mut child, "Commencer", accent, true, true);
            let phase = if animated && state_is_live(&self.intro) { (now / 1.8).fract() as f32 } else { 0.3 };
            child.painter().set(
                slot,
                outer_glow(
                    &ctx,
                    run.rect,
                    run.rect.height() / 2.0,
                    accent,
                    glow_breath(phase, 0.02, 0.02) * f.button,
                    2.0 * (phase * std::f32::consts::TAU).sin().abs(),
                ),
            );
            if run.clicked() {
                start_app = true;
            }
        }
        let mut play_dashboard = false;
        if start_app && state_is_live(&self.intro) {
            if let Some(state) = self.intro.as_mut() {
                if state.leaving.is_none() {
                    state.leaving = Some(now);
                    // Entrée dans l'appli : le son de l'Aperçu part avec le fondu.
                    play_dashboard = true;
                    fade_intro_sound = true;
                }
            }
        }
        if play_intro {
            self.launch_sound_done = true;
            self.play_sound(Sound::Intro);
        }
        if fade_intro_sound {
            if let Some(sounds) = &mut self.sounds {
                sounds.fade_out_intro();
            }
        }
        if play_dashboard {
            self.play_sound(Sound::Dashboard);
        }
    }

    /// Joue un son si le réglage « Jouer les sons » est activé (jamais en test ni en capture : pas de lecteur).
    fn play_sound(&mut self, sound: Sound) {
        if !self.sounds_on {
            return;
        }
        if let Some(sounds) = &mut self.sounds {
            sounds.play(sound);
        }
    }

    /// Son d'ouverture de l'Aperçu au lancement de l'appli, et avancement du fondu du son d'introduction.
    fn sound_tick(&mut self, ctx: &egui::Context) {
        if let Some(sounds) = &mut self.sounds {
            sounds.tick();
            if sounds.is_fading() {
                ctx.request_repaint_after(std::time::Duration::from_millis(16));
            }
        }
        if self.launch_sound_done || self.frame < 3 {
            return;
        }
        // Une seule fois par lancement : jamais pendant l'introduction (elle a le sien), ni fenêtre cachée.
        if self.intro.is_some() || self.start_hidden || self.window_hidden {
            self.launch_sound_done = true;
        } else if self.tab == Tab::Dashboard {
            self.launch_sound_done = true;
            self.play_sound(Sound::Dashboard);
        }
    }

    /// Relit l'état d'accès des dossiers suivis (macOS peut refuser la lecture sans autorisation).
    fn refresh_access(&mut self) {
        self.denied_folders =
            self.folders.iter().filter(|f| f.enabled && folder_access(&f.path) == Access::Denied).map(|f| f.path.clone()).collect();
    }

    /// Encarts affichés en haut des écrans : bienvenue (une fois) et aide sur les autorisations macOS.
    fn notices(&mut self, ui: &mut egui::Ui) {
        let accent = self.accent();
        self.update_banner(ui, accent);
        if !self.welcome_seen && self.tab == Tab::Dashboard {
            panel_card_with(ui, Some(with_alpha(accent, 22)), 18.0, |ui| {
                ui.label(RichText::new("Bienvenue dans OrganizeMyFiles").size(17.0).family(semibold()).color(text_primary()));
                ui.add_space(4.0);
                ui.label(
                    RichText::new(
                        "macOS peut demander l'autorisation d'accéder à vos dossiers (Bureau, Documents, Téléchargements). \
                         Acceptez : sans cela, OrganizeMyFiles ne peut pas ranger vos fichiers. Si un dossier reste bloqué, \
                         ajoutez l'appli dans Réglages Système › Confidentialité et sécurité › Accès complet au disque.",
                    )
                    .size(14.0)
                    .color(text_dim()),
                );
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if btn(ui, "Compris", Btn::Primary, accent, None).clicked() {
                        self.welcome_seen = true;
                    }
                    if btn(ui, "Ouvrir les réglages de confidentialité", Btn::Ghost, accent, None).clicked() {
                        system::open_privacy_settings();
                    }
                });
            });
            ui.add_space(14.0);
        }
        if self.tab == Tab::Settings {
            return;
        }
        let blocked = self.denied_folders.len();
        let refused_actions = history_view::latest_permission_failures(&self.history);
        if blocked == 0 && refused_actions == 0 {
            return;
        }
        panel_card_with(ui, Some(with_alpha(RED, 26)), 16.0, |ui| {
            let text = match (blocked, refused_actions) {
                (b, 0) => format!(
                    "macOS empêche OrganizeMyFiles de lire {b} dossier{} suivi{}.",
                    if b > 1 { "s" } else { "" },
                    if b > 1 { "s" } else { "" }
                ),
                (0, a) => format!("macOS a refusé {a} action{} du dernier lot.", if a > 1 { "s" } else { "" }),
                (b, a) => format!(
                    "macOS bloque {b} dossier{} et a refusé {a} action{} du dernier lot.",
                    if b > 1 { "s" } else { "" },
                    if a > 1 { "s" } else { "" }
                ),
            };
            ui.horizontal(|ui| {
                // Réserve la largeur exacte du bouton : les textes se replient
                // au lieu de passer dessous en fenêtre étroite.
                let btn_w = ui
                    .painter()
                    .layout_no_wrap("Ouvrir les réglages".to_string(), egui::FontId::new(14.5, semibold()), Color32::WHITE)
                    .size()
                    .x
                    + 28.0;
                let text_w = (ui.available_width() - btn_w - 20.0).max(120.0);
                ui.vertical(|ui| {
                    ui.set_width(text_w);
                    ui.label(RichText::new(text).size(15.0).family(semibold()).color(text_primary()));
                    ui.label(
                        RichText::new("Accordez l'accès dans Réglages Système › Confidentialité et sécurité, puis revenez ici.")
                            .size(13.5)
                            .color(text_dim()),
                    );
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if btn(ui, "Ouvrir les réglages", Btn::Primary, accent, None).clicked() {
                        system::open_privacy_settings();
                    }
                });
            });
        });
        ui.add_space(14.0);
    }

    /// Les fichiers que l'appli vient de déplacer ne doivent pas être retraités par la surveillance.
    fn ignore_in_watcher(&self, history: &[HistoryEntry]) {
        if let Some(w) = &self.watcher {
            w.ignore(history.iter().flat_map(|e| [e.original_path.clone(), e.result_path.clone()]).collect());
        }
    }

    /// Ce que le menu de la barre des menus doit afficher en ce moment.
    fn menu_info(&self) -> MenuInfo {
        MenuInfo {
            enabled: self.app_enabled,
            folders: self
                .folders
                .iter()
                .map(|f| MenuFolder {
                    id: f.id,
                    label: tags::folder_label(&self.folders, f.id),
                    active: f.enabled,
                    denied: self.denied_folders.contains(&f.path),
                })
                .collect(),
            rules_active: self.rules.iter().filter(|r| r.enabled).count(),
            last_batch: menubar_info::last_batch_line(&self.history, SystemTime::now()),
        }
    }

    /// Ferme la fenêtre sans quitter : l'appli reste active dans la barre des menus, sans icône dans le Dock.
    fn hide_window(&mut self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        menubar::set_dock_visible(false);
        self.window_hidden = true;
    }

    /// Rouvre la fenêtre (et l'icône du Dock) au premier plan.
    fn show_window(&mut self, ctx: &egui::Context) {
        menubar::set_dock_visible(true);
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        self.window_hidden = false;
    }

    /// Icône et menu de la barre des menus, commandes du menu, et « bouton rouge = fermer la fenêtre ».
    fn handle_menubar(&mut self, ctx: &egui::Context) {
        if self.screenshot.is_none() {
            if !self.menu_bar {
                self.menubar = None;
                self.menubar_failed = false;
            } else if self.menubar.is_none() && !self.menubar_failed {
                let wake = ctx.clone();
                self.menubar = MenuBar::new(move || wake.request_repaint());
                self.menubar_failed = self.menubar.is_none();
            }
        }
        // Sans icône dans la barre, une fenêtre cachée serait inaccessible : on la rouvre.
        if self.window_hidden && self.menubar.is_none() {
            self.show_window(ctx);
        }
        // Clic sur l'icône du Dock : on rouvre la fenêtre (elle peut avoir été fermée avec le bouton rouge).
        if menubar::take_reopen_request() {
            self.show_window(ctx);
        }
        let commands = self.menubar.as_ref().map(MenuBar::poll).unwrap_or_default();
        if self.menubar.is_some() {
            let info = self.menu_info();
            if let Some(bar) = &mut self.menubar {
                bar.update(&info);
            }
        }
        for cmd in commands {
            match cmd {
                MenuCommand::Open(screen) => {
                    self.tab = match screen {
                        MenuScreen::Dashboard => Tab::Dashboard,
                        MenuScreen::Folders => Tab::Folders,
                        MenuScreen::Rules => Tab::Rules,
                        MenuScreen::History => Tab::History,
                        MenuScreen::Settings => Tab::Settings,
                    };
                    self.show_window(ctx);
                }
                MenuCommand::Organize => {
                    if self.app_enabled {
                        self.run_batch(false);
                    }
                }
                MenuCommand::Preview => self.run_batch(true),
                MenuCommand::ToggleEnabled => self.app_enabled = !self.app_enabled,
                MenuCommand::RevealFolder(id) => {
                    if let Some(f) = self.folders.iter().find(|f| f.id == id) {
                        system::reveal_in_finder(&f.path);
                    }
                }
                MenuCommand::Quit => {
                    self.quitting = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
        // Bouton rouge : avec l'icône dans la barre des menus, la fenêtre se ferme mais l'appli (et la
        // surveillance) continue ; sans l'icône, c'est la fin de l'appli. « Quitter » arrête vraiment.
        if self.menubar.is_some() && !self.quitting && ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.hide_window(ctx);
        }
    }

    /// Envoie à la surveillance les dossiers et règles actuels, seulement s'ils ont changé.
    fn sync_watcher(&mut self) {
        let Some(w) = &self.watcher else { return };
        let key = serde_json::to_string(&(&self.folders, &self.rules, self.app_enabled, self.auto_organize_minutes)).unwrap_or_default();
        if key != self.watch_key {
            self.watch_key = key;
            w.set_config(WatchConfig {
                folders: self.folders.clone(),
                rules: self.rules.clone(),
                enabled: self.app_enabled,
                auto_every: (self.auto_organize_minutes > 0)
                    .then(|| std::time::Duration::from_secs(u64::from(self.auto_organize_minutes) * 60)),
            });
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
            let failed = r.history.iter().filter(|e| matches!(e.status, Status::Failed | Status::BlockedByConflict)).count();
            let (text, kind) = toast::batch_toast("", " automatiquement", r.matched, failed);
            self.show_toast(text, kind);
            if self.notify {
                if let Some(text) = alerts::summary(&r.history) {
                    alerts::notify(text);
                }
            }
            self.history.extend(r.history);
        }
        purge_history(&mut self.history, self.retention_days, SystemTime::now());
    }
}

fn sample_rule(id: u64, folder_id: u64, name: &str, prio: i64) -> Rule {
    let mut r = Rule {
        id,
        folder_ids: vec![folder_id],
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
        [1.0, 1.0, 0.0],
        [-1.0, 1.0, 0.0],
        [1.0, -1.0, 0.0],
        [-1.0, -1.0, 0.0],
        [1.0, 0.0, 1.0],
        [-1.0, 0.0, 1.0],
        [1.0, 0.0, -1.0],
        [-1.0, 0.0, -1.0],
        [0.0, 1.0, 1.0],
        [0.0, -1.0, 1.0],
        [0.0, 1.0, -1.0],
        [0.0, -1.0, -1.0],
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
        let noise =
            snoise(cx + tt * (1.0 + fi) / 30.0 * 2.0 + seed, cy + seed, tt * (1.0 + fi) * 0.11 + seed) - 0.1 * fi + 0.5 * BG_BLENDING;
        let noise = noise.clamp(0.0, 0.9 + fi * 0.02);
        let k = smoothstep(0.0, BG_BLENDING, noise);
        c = [c[0] + (col.r() as f32 - c[0]) * k, c[1] + (col.g() as f32 - c[1]) * k, c[2] + (col.b() as f32 - c[2]) * k];
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
            if hover {
                lighten(accent, 18)
            } else {
                accent
            }
        } else {
            with_alpha(accent, if hover { 56 } else { 34 })
        };
        if primary {
            ui.painter().add(outer_glow(ui.ctx(), rect, h / 2.0, accent, 0.03, 0.0));
        }
        ui.painter().rect_filled(rect, Rounding::same(h / 2.0), fill);
        let tc = Color32::WHITE;
        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, text, egui::FontId::proportional(font), tc);
    }
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, response.enabled(), text));
    response
}

/// Pastille d'état : fond teinté (vert = tout va bien, rouge = erreur), point qui pulse, texte.
/// Renvoie vrai si on a cliqué dessus pour la fermer.
fn toast_pill(ui: &mut egui::Ui, text: &str, kind: ToastKind, look: toast::ToastLook, time: f64) -> bool {
    let color = match kind {
        ToastKind::Ok => GREEN,
        ToastKind::Error => RED,
    };
    let fade = |a: f32| (a * look.opacity).round().clamp(0.0, 255.0) as u8;
    let galley = ui.painter().layout_no_wrap(text.to_string(), egui::FontId::proportional(14.5), Color32::WHITE);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(galley.size().x + 46.0, 32.0), egui::Sense::click());
    ui.painter().rect_filled(rect, Rounding::same(16.0), with_alpha(CANVAS, fade(170.0)));
    ui.painter().rect_filled(rect, Rounding::same(16.0), with_alpha(color, fade(if resp.hovered() { 44.0 } else { 30.0 })));
    let dot = Pos2::new(rect.left() + 16.0, rect.center().y);
    if look.pulsing {
        let t = (time as f32 * 1.4).fract();
        ui.painter().circle_filled(dot, 4.0 + 5.0 * t, with_alpha(color, fade((1.0 - t) * 90.0)));
    }
    ui.painter().circle_filled(dot, 4.0, with_alpha(color, fade(255.0)));
    ui.painter().galley(
        Pos2::new(rect.left() + 28.0, rect.center().y - galley.size().y / 2.0),
        ui.painter().layout_no_wrap(text.to_string(), egui::FontId::proportional(14.5), with_alpha(text_primary(), fade(255.0))),
        text_primary(),
    );
    resp.on_hover_text("Cliquer pour fermer").clicked()
}

/// Halo « lueur externe » lisse autour d'un bouton à coins arrondis (`radius`) : flou gaussien mis en cache, peint
/// avec la couleur `color` à l'opacité `intensity` (0 à 1). `grow` agrandit légèrement le halo (pulsation).
fn outer_glow(ctx: &egui::Context, rect: Rect, radius: f32, color: Color32, intensity: f32, grow: f32) -> egui::Shape {
    type Cache = std::sync::Arc<std::sync::Mutex<std::collections::HashMap<(u32, u32, u32), egui::TextureHandle>>>;
    let cache: Cache = ctx.data_mut(|d| d.get_temp_mut_or_insert_with(egui::Id::new("outer-glow-cache"), Cache::default).clone());
    let key = (rect.width().round() as u32, rect.height().round() as u32, radius.round() as u32);
    let texture = {
        let mut textures = cache.lock().unwrap_or_else(|e| e.into_inner());
        textures
            .entry(key)
            .or_insert_with(|| {
                let map = crate::glow::glow_map(key.0 as f32, key.1 as f32, key.2 as f32, crate::glow::SOFT);
                let pixels = map.alpha.iter().map(|a| Color32::from_white_alpha((a * 255.0).round() as u8)).collect();
                let image = egui::ColorImage { size: [map.width, map.height], pixels };
                ctx.load_texture("outer-glow", image, egui::TextureOptions::LINEAR)
            })
            .clone()
    };
    let margin = crate::glow::margin(crate::glow::SOFT) as f32 + grow;
    let tint = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), (intensity.clamp(0.0, 1.0) * GLOW_SOFTNESS * 255.0) as u8);
    egui::Shape::image(texture.id(), rect.expand(margin).translate(Vec2::ZERO), Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), tint)
}

/// Intensité de la pulsation du halo (0 à 1) : « respire » doucement autour de `base` sur ± `amplitude`.
fn glow_breath(phase: f32, base: f32, amplitude: f32) -> f32 {
    let breath = 0.5 - 0.5 * (phase * std::f32::consts::TAU).cos();
    base + amplitude * breath
}

/// Vrai si l'intro avance en direct (pas figée pour une capture).
fn state_is_live(intro: &Option<IntroState>) -> bool {
    intro.as_ref().is_some_and(|s| s.frozen.is_none())
}

/// Halo doux : disque dont l'opacité décroît du centre (`alpha` au centre) vers le bord (transparent).
/// Vrai si macOS demande de réduire les animations (Réglages Système › Accessibilité › Écran).
#[cfg(target_os = "macos")]
fn system_reduces_motion() -> bool {
    std::process::Command::new("defaults")
        .args(["read", "com.apple.universalaccess", "reduceMotion"])
        .output()
        .is_ok_and(|o| String::from_utf8_lossy(&o.stdout).trim() == "1")
}

#[cfg(not(target_os = "macos"))]
fn system_reduces_motion() -> bool {
    false
}

/// Texture d'une image embarquée, **réduite à la taille exacte d'affichage** (`width_pt` points, soit `width_pt ×
/// pixels_per_point` pixels) par un filtre de Lanczos. Les contours sont lisses (échantillonnage précis) et nets (un
/// pixel d'image = un pixel d'écran, voir `paint_fitted`). Calculée une fois par taille et gardée en mémoire.
/// La taille de la texture est en pixels physiques : le rapport largeur/hauteur reste celui de l'image.
fn fitted_texture(ctx: &egui::Context, key: &str, png: &[u8], width_pt: f32) -> egui::TextureHandle {
    let width_px = (width_pt * ctx.pixels_per_point()).round().max(1.0) as u32;
    let id = egui::Id::new(("fitted-texture", key, width_px));
    if let Some(tex) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return tex;
    }
    let source = image::load_from_memory(png).expect("image embarquée valide").to_rgba8();
    let image = resize_premultiplied(&source, width_px);
    let tex = ctx.load_texture(format!("{key}-{width_px}"), image, egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, tex.clone()));
    tex
}

/// Réduit `source` à `width_px` de large (jamais d'agrandissement), en alpha **prémultiplié** : sans cela, la couleur
/// des pixels transparents déteint sur les contours (liseré sombre autour d'un logo blanc). Les valeurs sont ramenées
/// à 0-1 : c'est la plage que `imageops::resize` conserve pour des flottants.
fn resize_premultiplied(source: &image::RgbaImage, width_px: u32) -> egui::ColorImage {
    let (w, h) = source.dimensions();
    let width_px = width_px.min(w);
    let height_px = ((h as f32 * width_px as f32 / w as f32).round() as u32).max(1);
    let premultiplied: Vec<f32> = source
        .pixels()
        .flat_map(|p| {
            let a = f32::from(p[3]) / 255.0;
            [f32::from(p[0]) / 255.0 * a, f32::from(p[1]) / 255.0 * a, f32::from(p[2]) / 255.0 * a, a]
        })
        .collect();
    let buffer = image::ImageBuffer::<image::Rgba<f32>, Vec<f32>>::from_raw(w, h, premultiplied).expect("dimensions cohérentes");
    let out =
        if width_px == w { buffer } else { image::imageops::resize(&buffer, width_px, height_px, image::imageops::FilterType::Lanczos3) };
    let bytes: Vec<u8> = out
        .pixels()
        .flat_map(|p| {
            let a = p[3].clamp(0.0, 1.0);
            [p[0].clamp(0.0, a), p[1].clamp(0.0, a), p[2].clamp(0.0, a), a].map(|v| (v * 255.0).round() as u8)
        })
        .collect();
    egui::ColorImage::from_rgba_premultiplied([width_px as usize, height_px as usize], &bytes)
}

/// Dessine une texture de `fitted_texture`. Quand `rect` a la taille prévue, l'image est posée **sur la grille de pixels** :
/// un pixel d'image tombe pile sur un pixel d'écran (net). Sinon (animation d'échelle), elle est simplement étirée.
fn paint_fitted(painter: &egui::Painter, tex: &egui::TextureHandle, rect: Rect, tint: Color32) {
    let ppp = painter.ctx().pixels_per_point();
    let exact = tex.size_vec2() / ppp;
    let rect = if (rect.width() - exact.x).abs() * ppp < 0.5 {
        Rect::from_min_size(painter.round_pos_to_pixels(rect.center() - exact / 2.0), exact)
    } else {
        rect
    };
    painter.image(tex.id(), rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), tint);
}

fn radial_glow(painter: &egui::Painter, center: Pos2, radius: f32, color: Color32, alpha: f32) {
    const SEGMENTS: usize = 64;
    let mut mesh = egui::epaint::Mesh::default();
    let inner = with_alpha(color, (alpha * GLOW_SOFTNESS).clamp(0.0, 255.0) as u8);
    let outer = with_alpha(color, 0);
    mesh.colored_vertex(center, inner);
    for i in 0..=SEGMENTS {
        let angle = i as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
        mesh.colored_vertex(center + Vec2::angled(angle) * radius, outer);
    }
    for i in 1..=SEGMENTS as u32 {
        mesh.add_triangle(0, i, i + 1);
    }
    painter.add(egui::Shape::mesh(mesh));
}

/// Tronque `text` avec « … » pour qu'il tienne dans `max_w` (mesure de la fonte réelle).
fn ellipsize(painter: &egui::Painter, text: &str, font: &egui::FontId, max_w: f32) -> String {
    let width = |s: String| painter.layout_no_wrap(s, font.clone(), Color32::WHITE).size().x;
    if width(text.to_string()) <= max_w {
        return text.to_string();
    }
    let mut out = String::new();
    for c in text.chars() {
        if width(format!("{out}{c}…")) > max_w {
            return format!("{}…", out.trim_end());
        }
        out.push(c);
    }
    text.to_string()
}

fn lighten(c: Color32, d: u8) -> Color32 {
    Color32::from_rgb(c.r().saturating_add(d), c.g().saturating_add(d), c.b().saturating_add(d))
}

fn icon_char(icon: Icon) -> char {
    match icon {
        Icon::Chart => '\u{e871}',      // dashboard
        Icon::Folder => '\u{f775}',     // folder_managed
        Icon::List => '\u{f1c2}',       // rule
        Icon::Clock => '\u{e8b3}',      // history
        Icon::Sliders => '\u{e429}',    // tune
        Icon::Funnel => '\u{ef4f}',     // filter_alt
        Icon::Bolt => '\u{ea0b}',       // bolt
        Icon::Grip => '\u{e945}',       // drag_indicator
        Icon::Bell => '\u{e7f5}',       // notifications
        Icon::Gear => '\u{e8b8}',       // settings
        Icon::Search => '\u{ef7a}',     // search
        Icon::Cross => '\u{e5cd}',      // close
        Icon::Add => '\u{e145}',        // add
        Icon::Trash => '\u{e92e}',      // delete
        Icon::FolderOpen => '\u{e2c8}', // folder_open
        Icon::Refresh => '\u{e5d5}',    // refresh
        Icon::Tick => '\u{e668}',       // check
        Icon::Undo => '\u{e166}',       // undo
        Icon::Sparkle => '\u{e65f}',    // auto_awesome
        Icon::ExpandMore => '\u{e5cf}', // expand_more
    }
}

/// Glyphe Material Symbols centré en `c` (`s` = taille de la boîte d'icône, en px).
fn draw_icon(painter: &egui::Painter, c: Pos2, icon: Icon, color: Color32, s: f32) {
    painter.text(c, egui::Align2::CENTER_CENTER, icon_char(icon).to_string(), egui::FontId::new(s * 1.25, icons_family()), color);
}

#[derive(Clone, Copy, PartialEq)]
enum Btn {
    Primary,
    Ghost,
    Sky,
}

/// Marges horizontales d'un bouton `btn` et place prise par son icône.
const BTN_PAD_X: f32 = 28.0;
const BTN_ICON_W: f32 = 20.0;

/// Largeur qu'aura un bouton `btn` : sert à réserver la bonne place avant de le dessiner.
fn btn_width(ui: &egui::Ui, text: &str, with_icon: bool) -> f32 {
    let galley = ui.painter().layout_no_wrap(text.to_string(), egui::FontId::new(14.5, semibold()), Color32::WHITE);
    galley.size().x + BTN_PAD_X + if with_icon { BTN_ICON_W } else { 0.0 }
}

/// Bouton 32 px de haut (rayon 8) : primaire orange, fantôme, bleu azur ou danger.
fn btn(ui: &mut egui::Ui, text: &str, kind: Btn, accent: Color32, icon: Option<Icon>) -> egui::Response {
    let font = egui::FontId::new(14.5, semibold());
    let galley = ui.painter().layout_no_wrap(text.to_string(), font.clone(), Color32::WHITE);
    let icon_w = if icon.is_some() { BTN_ICON_W } else { 0.0 };
    let size = Vec2::new(galley.size().x + BTN_PAD_X + icon_w, 32.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let enabled = resp.enabled();
    let hover = resp.hovered() && enabled;
    let (fill, fg) = match kind {
        Btn::Primary => (if hover { lighten(accent, 16) } else { accent }, Color32::WHITE),
        Btn::Ghost => (glass(if hover { 28 } else { 16 }), text_primary()),
        Btn::Sky => (with_alpha(SKY, if hover { 60 } else { 36 }), SKY),
    };
    let (fill, fg) = if enabled { (fill, fg) } else { (glass(12), with_alpha(Color32::WHITE, 70)) };
    let p = ui.painter();
    if kind == Btn::Primary && enabled {
        p.rect_filled(rect.expand(3.0), Rounding::same(10.0), with_alpha(accent, (40.0 * GLOW_SOFTNESS) as u8));
    }
    if matches!(kind, Btn::Sky) {
        p.rect_filled(rect, Rounding::same(8.0), with_alpha(CANVAS, 140));
    }
    p.rect_filled(rect, Rounding::same(8.0), fill);
    let mut x = rect.left() + 14.0;
    if let Some(ic) = icon {
        draw_icon(p, Pos2::new(x + 7.0, rect.center().y), ic, fg, 15.0);
        x += icon_w;
    }
    p.text(Pos2::new(x, rect.center().y), egui::Align2::LEFT_CENTER, text, font, fg);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, text));
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

/// Pastille discrète d'un libellé du héros : fond accent translucide, texte secondaire inchangé.
fn hero_sub_pill(ui: &mut egui::Ui, text: &str, accent: Color32) {
    let galley = ui.painter().layout_no_wrap(text.to_string(), egui::FontId::proportional(16.0), text_dim());
    let (rect, _) = ui.allocate_exact_size(Vec2::new(galley.size().x + 20.0, 26.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, Rounding::same(13.0), with_alpha(accent, 30));
    ui.painter().galley(Pos2::new(rect.left() + 10.0, rect.center().y - galley.size().y / 2.0), galley, text_dim());
}

/// Pastille cliquable (22 px) : même rendu que `chip`, avec survol.
fn chip_button(ui: &mut egui::Ui, text: &str, fg: Color32, bg: Color32) -> egui::Response {
    let font = egui::FontId::new(14.0, semibold());
    let galley = ui.painter().layout_no_wrap(text.to_string(), font, fg);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(galley.size().x + 22.0, 26.0), egui::Sense::click());
    ui.painter().rect_filled(rect, Rounding::same(13.0), with_alpha(CANVAS, 140));
    ui.painter().rect_filled(rect, Rounding::same(13.0), if resp.hovered() { with_alpha(fg, 70) } else { bg });
    let pos = Pos2::new(rect.left() + 11.0, rect.center().y - galley.size().y / 2.0);
    let label = text.to_string();
    ui.painter().galley(pos, galley, fg);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label.clone()));
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Interrupteur façon macOS.
fn check_box(ui: &mut egui::Ui, on: &mut bool, label: &str, accent: Color32) -> egui::Response {
    let font = egui::FontId::proportional(15.0);
    let galley = ui.painter().layout_no_wrap(label.to_string(), font.clone(), text_primary());
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(26.0 + galley.size().x, 26.0), egui::Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, *on, label));
    let b = Rect::from_min_size(Pos2::new(rect.left(), rect.center().y - 9.0), Vec2::splat(18.0));
    let fill = if *on { accent } else { glass(if resp.hovered() { 36 } else { 24 }) };
    ui.painter().rect_filled(b, Rounding::same(5.0), fill);
    if !*on {
        ui.painter().rect_stroke(b, Rounding::same(5.0), Stroke::new(1.5_f32, with_alpha(text_dim(), 150)));
    }
    if *on {
        let st = Stroke::new(2.0f32, Color32::WHITE);
        let m = Pos2::new(b.left() + 8.0, b.bottom() - 5.0);
        ui.painter().line_segment([Pos2::new(b.left() + 4.5, b.center().y), m], st);
        ui.painter().line_segment([m, Pos2::new(b.right() - 4.0, b.top() + 5.0)], st);
    }
    ui.painter().text(Pos2::new(rect.left() + 26.0, rect.center().y), egui::Align2::LEFT_CENTER, label, font, text_primary());
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
    resp.widget_info(|| egui::WidgetInfo::slider(true, f64::from(*value), "Réglette"));
    let t = (*value - min) as f32 / (max - min) as f32;
    let y = rect.center().y;
    let knob = Pos2::new(x0 + t * (x1 - x0), y);
    ui.painter().rect_filled(Rect::from_min_max(Pos2::new(x0, y - 2.0), Pos2::new(x1, y + 2.0)), Rounding::same(2.0), glass(36));
    ui.painter().rect_filled(Rect::from_min_max(Pos2::new(x0, y - 2.0), Pos2::new(knob.x, y + 2.0)), Rounding::same(2.0), accent);
    ui.painter().circle_filled(knob + Vec2::new(0.0, 1.0), 8.5, Color32::from_rgba_unmultiplied(0, 0, 0, 70));
    ui.painter().circle_filled(knob, 8.0, Color32::WHITE);
    changed
}

/// Sélecteur « TOUTES / AU MOINS UNE condition » d'une règle.
fn rule_match_selector(ui: &mut egui::Ui, rule: &mut Rule, rid: u64) {
    let current = if rule.match_all { 0 } else { 1 };
    if let Some(i) = segmented(ui, egui::Id::new(("match", rid)), &["TOUTES les conditions", "AU MOINS UNE condition"], current, SKY_DEEP) {
        rule.match_all = i == 0;
    }
}

/// Réglette « Récursion » : profondeur de sous-dossiers prise en compte par la règle.
fn recursion_control(ui: &mut egui::Ui, rule: &mut Rule, accent: Color32) {
    ui.label(caps("RÉCURSION"));
    let mut depth = rule.recursion_depth;
    if slim_slider(ui, &mut depth, 0, 8, accent) {
        rule.recursion_depth = depth;
    }
    chip(ui, &rule.recursion_depth.to_string(), accent, with_alpha(accent, 38));
}

fn toggle(ui: &mut egui::Ui, on: &mut bool, accent: Color32) -> egui::Response {
    let (slot, mut resp) = ui.allocate_exact_size(Vec2::new(38.0, 26.0), egui::Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, *on, "Interrupteur"));
    let rect = Rect::from_center_size(slot.center(), Vec2::new(38.0, 22.0));
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
    let widths: Vec<f32> =
        labels.iter().map(|l| ui.painter().layout_no_wrap(l.to_string(), font.clone(), Color32::WHITE).size().x + 28.0).collect();
    let total = segmented_width(&widths);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(total, 34.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, Rounding::same(9.0), with_alpha(CANVAS, 150));
    let mut x = rect.left() + SEGMENT_PAD;
    let mut out = None;
    for (i, label) in labels.iter().enumerate() {
        let seg = Rect::from_min_size(Pos2::new(x, rect.top() + 4.0), Vec2::new(widths[i], 26.0));
        let resp = ui.interact(seg, id.with(i), egui::Sense::click());
        let on = i == selected;
        resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::RadioButton, true, on, *label));
        if on {
            ui.painter().rect_filled(seg, Rounding::same(6.0), active);
        } else if resp.hovered() {
            ui.painter().rect_filled(seg, Rounding::same(6.0), glass(18));
        }
        ui.painter().text(seg.center(), egui::Align2::CENTER_CENTER, label, font.clone(), if on { Color32::WHITE } else { text_dim() });
        if resp.clicked() {
            out = Some(i);
        }
        x += widths[i] + SEGMENT_GAP;
    }
    out
}

/// Espace autour et entre les segments d'un sélecteur (points) : régulier, pour que les boutons ne se touchent pas.
const SEGMENT_PAD: f32 = 4.0;
const SEGMENT_GAP: f32 = 4.0;

/// Largeur totale d'un sélecteur : segments, espaces entre eux et marge de chaque côté.
fn segmented_width(widths: &[f32]) -> f32 {
    widths.iter().sum::<f32>() + SEGMENT_GAP * widths.len().saturating_sub(1) as f32 + 2.0 * SEGMENT_PAD
}

/// Bouton-icône carré (28 px) ; `danger` = teinte rouge au survol. true si cliqué.
fn icon_button(ui: &mut egui::Ui, icon: Icon, tip: &str, danger: bool) -> bool {
    let (r, resp) = ui.allocate_exact_size(Vec2::splat(28.0), egui::Sense::click());
    let hot = resp.hovered();
    if hot {
        ui.painter().rect_filled(r, Rounding::same(7.0), if danger { with_alpha(RED, 36) } else { glass(24) });
    }
    let col = if hot && danger {
        RED
    } else if hot {
        text_primary()
    } else {
        text_dim()
    };
    draw_icon(&ui.painter().clone(), r.center(), icon, col, 16.0);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tip));
    resp.on_hover_text(tip).clicked()
}

/// Pastille avec une croix pour la retirer ; renvoie vrai si la croix est cliquée.
fn removable_chip(ui: &mut egui::Ui, text: &str, tip: &str, accent: Color32) -> bool {
    let font = egui::FontId::new(14.0, semibold());
    let galley = ui.painter().layout_no_wrap(text.to_string(), font, accent);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(galley.size().x + 36.0, 28.0), egui::Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("{tip} : {text}")));
    ui.painter().rect_filled(rect, Rounding::same(12.0), with_alpha(CANVAS, 140));
    ui.painter().rect_filled(rect, Rounding::same(12.0), with_alpha(accent, 38));
    ui.painter().galley(Pos2::new(rect.left() + 10.0, rect.center().y - galley.size().y / 2.0), galley, accent);
    let cross = Rect::from_center_size(Pos2::new(rect.right() - 13.0, rect.center().y), Vec2::splat(20.0));
    let over_cross = resp.hover_pos().is_some_and(|p| cross.contains(p));
    if over_cross {
        ui.painter().circle_filled(cross.center(), 8.0, with_alpha(RED, 60));
    }
    draw_icon(&ui.painter().clone(), cross.center(), Icon::Cross, if over_cross { RED } else { text_dim() }, 12.0);
    let hovered = resp.on_hover_text(tip);
    hovered.clicked() && hovered.interact_pointer_pos().is_some_and(|p| cross.contains(p))
}

/// Extensions affichées en pastilles, avec un champ pour en ajouter (Entrée, virgule ou espace).
/// Renvoie la nouvelle valeur (« png, jpg ») quand elle change.
fn extension_tags(ui: &mut egui::Ui, id: egui::Id, value: &str, width: f32, accent: Color32) -> Option<String> {
    let mut exts = tags::split_extensions(value);
    let mut changed = false;
    ui.allocate_ui_with_layout(Vec2::new(width, 28.0), Layout::left_to_right(Align::Center).with_main_wrap(true), |ui| {
        ui.set_width(width);
        let mut remove = None;
        for (i, e) in exts.iter().enumerate() {
            if removable_chip(ui, &format!(".{e}"), "Retirer cette extension", accent) {
                remove = Some(i);
            }
        }
        if let Some(i) = remove {
            exts.remove(i);
            changed = true;
        }
        let input_id = id.with("draft");
        let mut draft: String = ui.data_mut(|d| d.get_temp(input_id)).unwrap_or_default();
        let was_empty = draft.is_empty();
        let hint = if exts.is_empty() { "ex. png, jpg, pdf" } else { "ajouter…" };
        let invalid = tags::has_invalid_token(&draft);
        let resp = ui
            .add(
                egui::TextEdit::singleline(&mut draft)
                    .id(input_id.with("edit"))
                    .margin(egui::Margin::symmetric(8.0, 5.0))
                    .desired_width(if exts.is_empty() { 150.0 } else { 90.0 })
                    .text_color(if invalid { RED } else { text_primary() })
                    .hint_text(hint),
            )
            .on_hover_text("Entrée, virgule ou espace pour valider. Lettres, chiffres, « _ » et « - » seulement.");
        let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let (added, left) = tags::absorb_extensions(&mut exts, &draft, enter || (resp.lost_focus() && !draft.is_empty()));
        if added {
            changed = true;
        }
        if left != draft {
            draft = left;
        }
        if enter {
            resp.request_focus();
        }
        if resp.has_focus() && was_empty && ui.input(|i| i.key_pressed(egui::Key::Backspace)) && exts.pop().is_some() {
            changed = true;
        }
        ui.data_mut(|d| d.insert_temp(input_id, draft));
    });
    changed.then(|| tags::join_extensions(&exts))
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
    egui::Frame::none().fill(glass(12)).rounding(Rounding::same(10.0)).inner_margin(egui::Margin::symmetric(12.0, 10.0)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        add(ui)
    })
}

fn caps(text: &str) -> RichText {
    RichText::new(text).size(13.0).family(semibold()).color(text_label())
}

fn rule_tab(ui: &mut egui::Ui, label: &str, active: bool, enabled: bool, accent: Color32) -> egui::Response {
    let font = egui::FontId::new(14.5, if active { semibold() } else { egui::FontFamily::Proportional });
    let fg = if active {
        Color32::WHITE
    } else if enabled {
        text_dim()
    } else {
        text_label()
    };
    let galley = ui.painter().layout_no_wrap(label.to_string(), font.clone(), fg);
    let lead = if active { 30.0 } else { 16.0 };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(galley.size().x + lead + 16.0, 34.0), egui::Sense::click());
    let fill = glass(if active {
        30
    } else if resp.hovered() {
        20
    } else {
        11
    });
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
    // Piste en trait CENTRÉ sur le rayon, comme l'arc : `circle_stroke` d'epaint
    // dessine son trait à l'extérieur du rayon, ce qui décalait la piste de 5 px.
    let track: Vec<Pos2> = (0..96)
        .map(|i| {
            let a = -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * (i as f32 / 96.0);
            center + Vec2::angled(a) * radius
        })
        .collect();
    painter.add(egui::Shape::closed_line(track, Stroke::new(10.0f32, glass(44))));
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

/// Largeur de fenêtre sous laquelle la rangée KPI passe de 4 à 2 tuiles.
const KPI_FOUR_COLUMNS_MIN_W: f32 = 1200.0;

/// Nombre de colonnes de la rangée KPI : 2 sous 1200 px de fenêtre, 4 au-dessus.
fn kpi_columns(window_w: f32) -> usize {
    if window_w < KPI_FOUR_COLUMNS_MIN_W {
        2
    } else {
        4
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
    // Les textes sont tronqués à la largeur disponible : en fenêtre étroite,
    // ils dépassaient de la tuile (peints sans limite de largeur).
    let label = ellipsize(&p, label, &egui::FontId::new(13.0, semibold()), (r.right() - 16.0 - (tile.right() + 10.0)).max(20.0));
    p.text(
        Pos2::new(tile.right() + 10.0, tile.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::new(13.0, semibold()),
        text_label(),
    );
    let value = ellipsize(&p, value, &egui::FontId::new(38.0, bold()), (w - 32.0).max(20.0));
    p.text(Pos2::new(r.left() + 20.0, r.top() + 86.0), egui::Align2::LEFT_CENTER, value, egui::FontId::new(38.0, bold()), Color32::WHITE);
    let sub = ellipsize(&p, sub, &egui::FontId::proportional(14.5), (w - 32.0).max(20.0));
    p.text(Pos2::new(r.left() + 20.0, r.bottom() - 20.0), egui::Align2::LEFT_CENTER, sub, egui::FontId::proportional(14.5), text_dim());
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

/// Intervalles proposés pour « Organiser automatiquement » : (minutes, libellé).
const AUTO_ORGANIZE_CHOICES: &[(u32, &str)] = &[(15, "15 minutes"), (60, "1 heure"), (360, "6 heures"), (1440, "1 jour")];

fn combo(ui: &mut egui::Ui, id: &str, current: &str, options: &[impl AsRef<str>]) -> Option<String> {
    let mut chosen = None;
    egui::ComboBox::from_id_salt(id).selected_text(current).width(150.0).show_ui(ui, |ui| {
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
        if clock >= self.next_access_check {
            self.next_access_check = clock + 5.0;
            self.refresh_access();
        }
        self.poll_watcher();
        self.update_tick(ctx, clock);
        self.handle_menubar(ctx);
        self.sound_tick(ctx);
        // Démarrage à l'ouverture de session : fenêtre fermée d'emblée, l'appli vit dans la barre des menus.
        if self.start_hidden && self.frame >= 2 {
            if self.menubar.is_some() {
                self.hide_window(ctx);
                self.start_hidden = false;
            } else if self.frame > 30 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                self.start_hidden = false;
            }
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(1100));

        let screen = ctx.screen_rect();
        let accent_now = self.accent();
        if self.tab != self.last_tab {
            self.last_tab = self.tab;
            self.tab_since = ctx.input(|i| i.time);
        }
        let bg_time = if self.animated() {
            ctx.request_repaint_after(std::time::Duration::from_millis(33));
            ctx.input(|i| i.time) + self.bg_offset
        } else {
            self.bg_offset
        };
        egui::CentralPanel::default().frame(egui::Frame::none()).show(ctx, |ui| {
            backdrop(ui, screen, self.accent(), bg_time);
            if self.intro.is_some() {
                self.intro_ui(ui, screen);
                return;
            }

            egui::TopBottomPanel::top("topbar").exact_height(56.0).frame(egui::Frame::none().fill(PANEL_GLASS)).show_inside(ui, |ui| {
                self.top_bar(ui);
            });

            egui::SidePanel::left("sidebar").resizable(false).exact_width(256.0).frame(egui::Frame::none().fill(PANEL_GLASS)).show_inside(
                ui,
                |ui| {
                    self.sidebar(ui);
                },
            );

            // Contenu : le défilement va jusqu'au bas de la fenêtre ; un fondu efface les cartes en bas.
            let content = ui.available_rect_before_wrap();
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                egui::Frame::none().inner_margin(egui::Margin::symmetric(28.0, 0.0)).show(ui, |ui| {
                    ui.add_space(24.0);
                    self.notices(ui);
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
            let pending = self.undo_folder.as_ref().map(|u| (u.folder.path.display().to_string(), u.rules.len(), u.at.elapsed().as_secs()));
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
                    let shown =
                        self.toast.as_ref().and_then(|t| toast::look(t.since.elapsed().as_secs_f32(), t.kind).map(|look| (t, look)));
                    match shown {
                        Some((t, look)) => {
                            let time = ctx.input(|i| i.time);
                            let dismissed = toast_pill(&mut bar_ui, &t.text, t.kind, look, time);
                            ctx.request_repaint_after(std::time::Duration::from_millis(if look.pulsing { 50 } else { 250 }));
                            if dismissed {
                                self.toast = None;
                            }
                        }
                        None => self.toast = None,
                    }
                }
            }
        });
    }
}

impl App {
    /// Raccourcis clavier globaux : ⌘K focus recherche, ⌘, réglages (standard macOS).
    fn global_shortcuts(&mut self, ctx: &egui::Context) {
        // ⌘K : focus sur la recherche.
        let search_id = egui::Id::new("global-search");
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::K)) {
            ctx.memory_mut(|m| m.request_focus(search_id));
        }
        // ⌘, : ouvre les réglages (raccourci macOS par défaut, aussi dans le menu de la barre des menus).
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Comma)) {
            self.tab = Tab::Settings;
        }
    }

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

        self.global_shortcuts(&ctx);

        // La recherche part 25 px à gauche du début du contenu (barre latérale 256 px + marge 28 px)
        // et prend tout l'espace libre jusqu'aux boutons de droite.
        let run_label = if rect.width() >= 1300.0 { "Organiser maintenant" } else { "Organiser" };
        let right_w = top_bar_right_width(
            btn_width(ui, run_label, true),
            btn_width(ui, "Aperçu simulation", false),
            wide,
            ui.spacing().item_spacing.x,
        );
        let search_left = rect.left() + 256.0 + 28.0 - 25.0;
        let search_right = rect.right() - 16.0 - right_w - 12.0;
        let search_w = (search_right - search_left).clamp(SEARCH_MIN_W, 640.0);
        let search_rect = Rect::from_min_size(Pos2::new(search_left, rect.center().y - 17.0), Vec2::new(search_w, 34.0));

        // --- gauche
        let left = Rect::from_min_max(Pos2::new(rect.left() + 16.0, rect.top()), Pos2::new(search_rect.left() - 10.0, rect.bottom()));
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
        let search_id = egui::Id::new("global-search");
        ui.painter().rect_filled(search_rect, Rounding::same(8.0), with_alpha(CANVAS, 180));
        let mut sui = ui.new_child(
            egui::UiBuilder::new().max_rect(search_rect.shrink2(Vec2::new(10.0, 0.0))).layout(Layout::left_to_right(Align::Center)),
        );
        let (ic, _) = sui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::hover());
        draw_icon(&sui.painter().clone(), ic.center(), Icon::Search, text_dim(), 15.0);
        sui.add(
            egui::TextEdit::singleline(&mut self.search)
                .id(search_id)
                .frame(false)
                .desired_width(search_w - 90.0)
                .hint_text("Rechercher règles, dossiers, fichiers…"),
        );
        let key = if cfg!(target_os = "macos") { "⌘ K" } else { "Ctrl K" };
        if search_w >= SEARCH_HINT_MIN_W {
            ui.painter().text(
                Pos2::new(search_rect.right() - 12.0, search_rect.center().y),
                egui::Align2::RIGHT_CENTER,
                key,
                egui::FontId::proportional(13.5),
                text_dim(),
            );
        }

        // --- droite
        let right = Rect::from_min_max(Pos2::new(search_rect.right() + 10.0, rect.top()), Pos2::new(rect.right() - 16.0, rect.bottom()));
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
        let run = rui.add_enabled_ui(enabled, |ui| btn(ui, run_label, Btn::Primary, accent, Some(Icon::Bolt))).inner;
        let run = run
            .on_hover_text("Lance toutes les règles maintenant (annulable dans l'Historique).")
            .on_disabled_hover_text("Application en veille.");
        if run.clicked() {
            self.run_batch(false);
        }
        let preview = btn(&mut rui, "Aperçu simulation", Btn::Ghost, accent, None).on_hover_text("Simule, sans toucher aux fichiers.");
        if preview.clicked() {
            self.run_batch(true);
        }
        // Statut : pastille verte pulsée.
        let label = if self.app_enabled { "Actif" } else { "En veille" };
        let dot = if self.app_enabled { GREEN } else { text_dim() };
        let (r, resp) = rui.allocate_exact_size(Vec2::new(STATUS_PILL_W, 32.0), egui::Sense::click());
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
            let folder_pos = self.folders.iter().position(|f| r.folder_ids.first() == Some(&f.id)).unwrap_or(usize::MAX);
            (folder_pos, r.priority, r.id)
        });
        order.into_iter().map(|r| r.id).collect()
    }

    fn new_rule(&mut self) {
        let current = self.rule_order().get(self.selected_rule).copied();
        let folder_id = current
            .and_then(|id| self.rules.iter().find(|r| r.id == id))
            .and_then(|r| r.folder_ids.first().copied())
            .or_else(|| self.folders.first().map(|f| f.id));
        match folder_id {
            Some(id) => self.new_rule_in(id),
            None => self.toast_error("Ajoutez d'abord un dossier."),
        }
    }

    fn new_rule_in(&mut self, folder_id: u64) {
        let count = self.rules.iter().filter(|r| r.folder_ids.contains(&folder_id)).count();
        let id = self.next_id;
        self.next_id += 1;
        // En mode simple, la règle démarre sans condition (l'assistant les propose) et garde les deux fichiers en cas de doublon.
        let simple = self.rules_mode == RulesMode::Simple;
        let mut action = Action::new(ActionKind::Move);
        if simple {
            action.params.insert(ON_CONFLICT_PARAM_KEY.into(), CONFLICT_NUMBER.into());
        }
        self.rules.push(Rule {
            id,
            folder_ids: vec![folder_id],
            name: "Nouvelle règle".into(),
            enabled: true,
            match_all: true,
            recursion_depth: 0,
            priority: count as i64,
            conditions: if simple { vec![] } else { vec![Condition { field: Field::Extension, op: Op::Equals, value: String::new() }] },
            actions: vec![action],
        });
        self.simple_step = 1;
        self.simple_view = SimpleView::Wizard;
        self.selected_rule = self.rule_order().iter().position(|r| *r == id).unwrap_or(0);
        self.tab = Tab::Rules;
    }

    /// Ouvre l'écran Règles sur la première règle du dossier.
    fn show_rules_of(&mut self, folder_id: u64) {
        let first = self.rule_order().iter().position(|id| self.rules.iter().any(|r| r.id == *id && r.folder_ids.contains(&folder_id)));
        if let Some(pos) = first {
            self.selected_rule = pos;
            self.simple_step = 1;
            self.simple_view = SimpleView::Wizard;
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
        if !refused.is_empty() {
            self.toast_error(msg);
        } else if added > 0 {
            self.toast(msg);
        }
    }

    fn restore_folder(&mut self) {
        if let Some(u) = self.undo_folder.take() {
            let at = u.index.min(self.folders.len());
            for rule in &mut self.rules {
                if u.detached.contains(&rule.id) && !rule.folder_ids.contains(&u.folder.id) {
                    rule.folder_ids.push(u.folder.id);
                }
            }
            self.folders.insert(at, u.folder);
            self.rules.extend(u.rules);
            self.toast("Dossier rétabli");
        }
    }

    fn folder_count(&mut self, id: u64, path: &std::path::Path) -> Option<usize> {
        *self.folder_counts.entry(id).or_insert_with(|| count_top_level_files(path))
    }

    /// Animations actives : réglage « Animer le fond » et mouvement non réduit dans macOS.
    fn animated(&self) -> bool {
        self.animate_bg && !self.system_reduce_motion
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        let accent = self.accent();
        // Identité de l'app, avant le menu.
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            ui.add_space(20.0);
            let (r, _) = ui.allocate_exact_size(Vec2::splat(36.0), egui::Sense::hover());
            let logo = fitted_texture(ui.ctx(), "app-logo", APP_ICON_PNG, r.width());
            paint_fitted(ui.painter(), &logo, r, Color32::WHITE);
            ui.add_space(10.0);
            let width = WORDMARK_SIDEBAR_W;
            let mark = fitted_texture(ui.ctx(), "wordmark", WORDMARK_PNG, width);
            let (rect, resp) =
                ui.allocate_exact_size(Vec2::new(width, width * mark.size_vec2().y / mark.size_vec2().x), egui::Sense::hover());
            paint_fitted(ui.painter(), &mark, rect, Color32::WHITE);
            resp.on_hover_text("OrganizeMyFiles");
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
        let rect = Rect::from_min_size(Pos2::new(avail.left() + 12.0, avail.bottom() - 16.0 - 64.0), Vec2::new(232.0, 64.0));
        ui.painter().rect_filled(rect, Rounding::same(12.0), with_alpha(CANVAS, 150));
        let mut c = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink(12.0)).layout(Layout::top_down(Align::Min)));
        c.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(caps("APPLICATION"));
                ui.label(RichText::new(if self.app_enabled { "Activée" } else { "En veille" }).size(15.5).color(Color32::WHITE));
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                toggle(ui, &mut self.app_enabled, accent);
            });
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
        // `subs` : une ligne par information ; deux labels distincts quand tout est prêt.
        let (title, subs) = match state {
            HeroState::NoFolder => ("Commençons".to_string(), vec!["Ajoutez un dossier à surveiller.".to_string()]),
            HeroState::NoRule => ("Aucune règle active".to_string(), vec!["Créez une règle pour ranger vos fichiers.".to_string()]),
            HeroState::Ready | HeroState::Done => (
                if state == HeroState::Done { "Tout est en ordre" } else { "Prêt à organiser" }.to_string(),
                vec![
                    format!("{folders_active} dossier{}", if folders_active > 1 { "s" } else { "" }),
                    format!(
                        "{rules_active} règle{} active{}",
                        if rules_active > 1 { "s" } else { "" },
                        if rules_active > 1 { "s" } else { "" }
                    ),
                ],
            ),
        };
        let avail = ui.available_width();
        let vgap = GAP - ui.spacing().item_spacing.y;
        let (hero, _) = ui.allocate_exact_size(Vec2::new(avail, 214.0), egui::Sense::hover());
        ui.painter().add(glass_shapes(hero));
        {
            let t = if self.animated() { ctx.input(|i| i.time) } else { 0.0 };
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
        c.label(RichText::new("APERÇU").size(13.0).family(semibold()).color(text_label()));
        c.add_space(4.0);
        c.label(RichText::new(&title).size(34.0).family(bold()).color(Color32::WHITE));
        c.add_space(2.0);
        let badge = subs.len() > 1;
        c.horizontal(|ui| {
            for (i, s) in subs.iter().enumerate() {
                if i > 0 {
                    ui.add_space(8.0);
                }
                if badge {
                    hero_sub_pill(ui, s, accent);
                } else {
                    ui.label(RichText::new(s).size(16.0).color(text_dim()));
                }
            }
        });
        c.add_space(12.0);
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
                // Le halo est peint derrière le bouton : on réserve sa place avant de dessiner le bouton.
                let glow_slot = ui.painter().add(egui::Shape::Noop);
                let run = pill(ui, "Organiser maintenant", accent, true, true)
                    .on_hover_text("Lance toutes les règles maintenant (annulable dans l'Historique).");
                let animated = self.animated() && self.app_enabled;
                let phase = if animated { (ctx.input(|i| i.time) / 1.8).fract() as f32 } else { 0.3 };
                if self.app_enabled {
                    ui.painter().set(
                        glow_slot,
                        outer_glow(
                            &ctx,
                            run.rect,
                            run.rect.height() / 2.0,
                            accent,
                            glow_breath(phase, 0.02, 0.02),
                            2.0 * (phase * std::f32::consts::TAU).sin().abs(),
                        ),
                    );
                }
                if animated {
                    ctx.request_repaint_after(std::time::Duration::from_millis(33));
                }
                organise = run.clicked() && self.app_enabled;
                ui.add_space(10.0);
                preview = pill(ui, "Aperçu", accent, false, true).on_hover_text("Simule, sans toucher aux fichiers.").clicked();
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
            egui::FontId::new(13.0, semibold()),
            text_label(),
        );
        p.text(center, egui::Align2::CENTER_CENTER, pct_text, egui::FontId::new(26.0, bold()), Color32::WHITE);
        p.text(Pos2::new(center.x, center.y + 76.0), egui::Align2::CENTER_CENTER, caption, egui::FontId::proportional(14.0), text_dim());
        ui.add_space(vgap);

        // ---------------------------------------------------------------- tuiles
        let gap = GAP;
        let cols = kpi_columns(ctx.screen_rect().width());
        let w = ((avail - (cols - 1) as f32 * gap) / cols as f32).max(120.0);
        let (matched_actions, actions_sub) = match self.last_run {
            Some((_, actions, _)) => (count(actions).to_string(), "actions au dernier passage".to_string()),
            None => ("—".to_string(), "aucun passage".to_string()),
        };
        let tiles = [
            (
                Icon::Folder,
                "DOSSIERS ACTIFS",
                count(folders_active).to_string(),
                format!("sur {folders_total} suivi{}", if folders_total > 1 { "s" } else { "" }),
                Tab::Folders,
            ),
            (
                Icon::List,
                "RÈGLES ACTIVES",
                count(rules_active).to_string(),
                format!("sur {rules_total} règle{}", if rules_total > 1 { "s" } else { "" }),
                Tab::Rules,
            ),
            (Icon::Clock, "FICHIERS CE MOIS", count(self.monthly_used as usize).to_string(), "organisés".to_string(), Tab::History),
            (Icon::Bolt, "DERNIER LOT", matched_actions, actions_sub, Tab::History),
        ];
        for row in tiles.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for (icon, label, value, sub, tab) in row {
                    if kpi_tile(ui, w, *icon, label, value, sub, accent).clicked() {
                        go = Some(*tab);
                    }
                }
            });
            ui.add_space(vgap);
        }

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
                let n_rules = self.rules.iter().filter(|r| r.folder_ids.contains(&f.id)).count();
                let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 54.0), egui::Sense::click());
                if resp.hovered() {
                    ui.painter().rect_filled(rect, Rounding::same(10.0), glass(14));
                }
                let tile = Rect::from_min_size(rect.min + Vec2::new(8.0, 10.0), Vec2::splat(34.0));
                ui.painter().rect_filled(tile, Rounding::same(10.0), with_alpha(if exists { accent } else { RED }, 70));
                draw_icon(&ui.painter().clone(), tile.center(), Icon::Folder, Color32::WHITE, 17.0);
                let name = f.path.file_name().map(|n| nfc(&n.to_string_lossy())).unwrap_or_default();
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
                    egui::FontId::proportional(13.5),
                    text_dim(),
                );
                let mut right = ui.new_child(
                    egui::UiBuilder::new().max_rect(rect.shrink2(Vec2::new(8.0, 0.0))).layout(Layout::right_to_left(Align::Center)),
                );
                if !exists {
                    chip(&mut right, "Introuvable", RED, with_alpha(RED, 38));
                } else if self.denied_folders.contains(&f.path) {
                    chip(&mut right, "Accès refusé", RED, with_alpha(RED, 38));
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
                    egui::FontId::proportional(13.5),
                    text_dim(),
                );
                if can_undo {
                    let mut right = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(Layout::right_to_left(Align::Center)));
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
            ("Aucun dossier suivi", "Glissez des dossiers du Finder, ou choisissez-les.")
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
            // Répartition en lignes mesurée à la main : horizontal_wrapped ne détectait
            // pas le dépassement à travers add_enabled_ui et le dernier raccourci
            // sortait de la carte en fenêtre étroite.
            let max_w = ui.available_width();
            let gap = ui.spacing().item_spacing.x;
            let mut lines: Vec<Vec<(&str, &str)>> = Vec::new();
            let mut current: Vec<(&str, &str)> = Vec::new();
            let mut used = 0.0_f32;
            for (label, dir) in SHORTCUTS {
                // Même mesure que btn : texte 14.5 semibold + 28 de padding + 20 d'icône.
                let w = ui.painter().layout_no_wrap(label.to_string(), egui::FontId::new(14.5, semibold()), Color32::WHITE).size().x + 48.0;
                let extra = if current.is_empty() { w } else { w + gap };
                if !current.is_empty() && used + extra > max_w {
                    lines.push(std::mem::take(&mut current));
                    used = 0.0;
                }
                used += if current.is_empty() { w } else { w + gap };
                current.push((label, dir));
            }
            if !current.is_empty() {
                lines.push(current);
            }
            for line in lines {
                ui.horizontal(|ui| {
                    for (label, dir) in line {
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
            }
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
            let n_rules = self.rules.iter().filter(|r| r.folder_ids.contains(&f.id)).count();
            let files = if exists { self.folder_count(f.id, &f.path) } else { None };
            let name = f.path.file_name().map(|n| nfc(&n.to_string_lossy())).unwrap_or_else(|| nfc(&f.path.display().to_string()));
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
                            } else if self.denied_folders.contains(&f.path) {
                                chip(ui, "Accès refusé", RED, with_alpha(RED, 38));
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
                self.toast_error("Ouverture impossible.");
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
                    Err(why) => self.toast_error(format!("Non modifié — {why}")),
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
            self.remove_folder(i);
        }
        for (i, f) in self.folders.iter_mut().enumerate() {
            f.priority = i as i64;
        }
    }

    /// Retire un dossier : les règles qui n'en ciblaient que lui partent avec ; les autres le perdent seulement.
    fn remove_folder(&mut self, i: usize) {
        let removed = self.folders.remove(i);
        let mut gone = Vec::new();
        let mut detached = Vec::new();
        for rule in std::mem::take(&mut self.rules) {
            if !rule.folder_ids.contains(&removed.id) {
                self.rules.push(rule);
            } else if rule.folder_ids.len() == 1 {
                gone.push(rule);
            } else {
                let mut rule = rule;
                rule.folder_ids.retain(|id| *id != removed.id);
                detached.push(rule.id);
                self.rules.push(rule);
            }
        }
        self.undo_folder = Some(UndoFolder { folder: removed, rules: gone, detached, index: i, at: std::time::Instant::now() });
    }

    /// Dossiers cibles d'une règle : pastilles + panneau de sélection (recherche, cases à cocher).
    /// Renvoie vrai si la liste a changé.
    fn target_folders_ui(&mut self, ui: &mut egui::Ui, idx: usize, accent: Color32) -> bool {
        const MAX_CHIPS: usize = 6;
        let rid = self.rules[idx].id;
        let mut changed = false;
        let mut refused_last = false;
        ui.label(caps("DOSSIERS CIBLES"));
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            let ids = self.rules[idx].folder_ids.clone();
            let shown = if ids.len() > MAX_CHIPS { MAX_CHIPS - 1 } else { ids.len() };
            for fid in ids.iter().take(shown) {
                let label = tags::folder_label(&self.folders, *fid);
                let full = self.folders.iter().find(|f| f.id == *fid).map(|f| f.path.display().to_string()).unwrap_or_default();
                if removable_chip(ui, &label, &full, accent) {
                    if ids.len() > 1 {
                        self.rules[idx].folder_ids.retain(|id| id != fid);
                        changed = true;
                    } else {
                        refused_last = true;
                    }
                }
            }
            let popup_id = egui::Id::new(("folder-popup", rid));
            let hidden = ids.len() - shown;
            let label = if hidden > 0 { format!("+{hidden}  ·  Dossiers") } else { "+ Dossier".to_string() };
            let opener = chip_button(ui, &label, accent, with_alpha(accent, 38)).on_hover_text("Choisir les dossiers suivis ciblés");
            if opener.clicked() {
                ui.memory_mut(|m| m.toggle_popup(popup_id));
            }
            egui::popup_below_widget(ui, popup_id, &opener, egui::PopupCloseBehavior::CloseOnClickOutside, |ui| {
                ui.set_min_width(340.0);
                let search_id = popup_id.with("search");
                let mut query: String = ui.data_mut(|d| d.get_temp(search_id)).unwrap_or_default();
                ui.add(
                    egui::TextEdit::singleline(&mut query)
                        .margin(egui::Margin::symmetric(8.0, 6.0))
                        .desired_width(f32::INFINITY)
                        .hint_text("Rechercher un dossier…"),
                );
                ui.data_mut(|d| d.insert_temp(search_id, query.clone()));
                ui.add_space(6.0);
                if self.folders.is_empty() {
                    ui.label(RichText::new("Aucun dossier suivi : ajoutez-en dans « Dossiers suivis ».").size(13.5).color(text_dim()));
                    return;
                }
                let all_selected = self.folders.iter().all(|f| self.rules[idx].folder_ids.contains(&f.id));
                let toggle_all = if all_selected { "Tout désélectionner (garde le premier)" } else { "Tout sélectionner" };
                if ui.link(RichText::new(toggle_all).size(13.5).color(accent)).clicked() {
                    self.rules[idx].folder_ids = if all_selected {
                        self.folders.iter().take(1).map(|f| f.id).collect()
                    } else {
                        self.folders.iter().map(|f| f.id).collect()
                    };
                    changed = true;
                }
                ui.add_space(4.0);
                let q = tags::fold(&query);
                egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                    for fi in 0..self.folders.len() {
                        let (fid, path) = (self.folders[fi].id, self.folders[fi].path.display().to_string());
                        if !q.is_empty() && !tags::fold(&path).contains(&q) {
                            continue;
                        }
                        let ids = &mut self.rules[idx].folder_ids;
                        let mut on = ids.contains(&fid);
                        if check_box(ui, &mut on, &shorten(&path), accent).changed() {
                            if on {
                                ids.push(fid);
                                changed = true;
                            } else if ids.len() > 1 {
                                ids.retain(|id| *id != fid);
                                changed = true;
                            } else {
                                refused_last = true;
                            }
                        }
                    }
                });
            });
        });
        if refused_last {
            self.toast_error("Une règle doit garder au moins un dossier.");
        }
        changed
    }

    fn rule_matches(&self, rule: &Rule, q: &str) -> bool {
        let folder = self
            .folders
            .iter()
            .filter(|f| rule.folder_ids.contains(&f.id))
            .map(|f| nfc(&f.path.display().to_string()).to_lowercase())
            .collect::<Vec<_>>()
            .join(" ");
        rule.name.to_lowercase().contains(q) || folder.contains(q) || rule.actions.iter().any(|a| a.kind.label().to_lowercase().contains(q))
    }

    fn rules_ui(&mut self, ui: &mut egui::Ui) {
        let accent = self.accent();

        ui.horizontal(|ui| {
            ui.label(RichText::new("Règles").size(24.0).family(bold()).color(Color32::WHITE));
            ui.add_space(6.0);
            let n = self.rules.iter().filter(|r| r.enabled).count();
            chip(ui, &format!("{n} {}", if n > 1 { "actives" } else { "active" }), SKY, with_alpha(SKY, 38));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if btn(ui, "Nouvelle règle", Btn::Sky, accent, Some(Icon::Add)).clicked() {
                    self.new_rule();
                }
                ui.add_space(4.0); // + l'écart habituel entre widgets : 12 pt au total
                let current = if self.rules_mode == RulesMode::Simple { 0 } else { 1 };
                if let Some(i) = segmented(ui, egui::Id::new("rules-mode"), &["Simple", "Avancé"], current, SKY_DEEP) {
                    self.rules_mode = if i == 0 { RulesMode::Simple } else { RulesMode::Advanced };
                    self.simple_step = 1;
                    self.simple_view = SimpleView::Home;
                }
            });
        });
        ui.add_space(2.0);
        let subtitle = match self.rules_mode {
            RulesMode::Simple => "Dites en trois étapes quels fichiers ranger, et comment.",
            RulesMode::Advanced => "Conditions remplies : actions dans l'ordre.",
        };
        ui.label(RichText::new(subtitle).size(15.0).color(text_dim()));
        ui.add_space(16.0);

        if self.folders.is_empty() {
            ui.label(RichText::new("Ajoutez d'abord un dossier.").color(text_dim()));
            return;
        }

        let simple = self.rules_mode == RulesMode::Simple;
        match self.simple_view {
            SimpleView::Home if simple => return self.simple_home_ui(ui),
            SimpleView::Done(rid) if simple => return self.simple_done_ui(ui, rid),
            _ => {}
        }
        let order = self.rule_order();
        if order.is_empty() && simple {
            self.simple_view = SimpleView::Home;
            ui.ctx().request_repaint();
            return;
        }
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
        if simple {
            if btn(ui, "Toutes les règles", Btn::Ghost, accent, Some(Icon::Undo)).clicked() {
                self.simple_view = SimpleView::Home;
            }
        } else {
            ui.horizontal_wrapped(|ui| {
                for (i, rid) in order.iter().enumerate() {
                    let Some(r) = self.rules.iter().find(|r| r.id == *rid) else { continue };
                    if !q.is_empty() && i != sel && !self.rule_matches(r, &q) {
                        continue;
                    }
                    let name: String = r.name.chars().take(32).collect();
                    if rule_tab(ui, &format!("Règle {} : {}", i + 1, name), i == sel, r.enabled, accent).clicked() {
                        self.selected_rule = i;
                        self.simple_step = 1;
                    }
                }
            });
        }
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
                    if ui
                        .add(egui::TextEdit::singleline(&mut name).margin(egui::Margin::symmetric(8.0, 7.0)).desired_width(300.0))
                        .changed()
                    {
                        self.rules[idx].name = name;
                    }
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if icon_button(ui, Icon::Trash, "Supprimer la règle", true) {
                        deleted = true;
                    }
                    ui.add_space(8.0);
                    let mut en = self.rules[idx].enabled;
                    let label = if en { "Activée" } else { "Désactivée" };
                    ui.label(RichText::new(label).size(14.5).color(if en { text_primary() } else { text_dim() }));
                    if toggle(ui, &mut en, accent).on_hover_text("Activer ou désactiver la règle").changed() {
                        self.rules[idx].enabled = en;
                    }
                });
            });
            if self.rules_mode == RulesMode::Advanced {
                ui.add_space(12.0);
                if self.target_folders_ui(ui, idx, accent) {
                    moved = true;
                }
                ui.add_space(14.0);
                // Fenêtre étroite : la réglette passe sur sa propre ligne au lieu de sortir de la carte.
                if ui.available_width() < 700.0 {
                    rule_match_selector(ui, &mut self.rules[idx], rid);
                    ui.add_space(8.0);
                    ui.horizontal(|ui| recursion_control(ui, &mut self.rules[idx], accent));
                } else {
                    ui.horizontal(|ui| {
                        rule_match_selector(ui, &mut self.rules[idx], rid);
                        ui.add_space(20.0);
                        recursion_control(ui, &mut self.rules[idx], accent);
                    });
                }
                ui.add_space(4.0);
                ui.label(
                    RichText::new(
                        "Jusqu'où chercher dans les sous-dossiers : 0 = seulement les fichiers posés dans le dossier ; \
                         1 = aussi ceux de ses sous-dossiers ; 2 = un niveau de plus, etc. \
                         Exemple avec 1 : un fichier dans « Téléchargements/Photos » est traité aussi.",
                    )
                    .size(13.5)
                    .color(text_dim()),
                );
            }
        });

        if deleted {
            self.rules.retain(|r| r.id != rid);
            self.selected_rule = sel.saturating_sub(1);
            if simple {
                self.simple_view = SimpleView::Home;
            }
            return;
        }
        if moved {
            self.selected_rule = self.rule_order().iter().position(|r| *r == rid).unwrap_or(0);
        }
        ui.add_space(14.0);

        if self.rules_mode == RulesMode::Simple {
            if self.simple_rules_ui(ui, idx) {
                self.selected_rule = self.rule_order().iter().position(|r| *r == rid).unwrap_or(0);
            }
            return;
        }

        // ---- conditions
        let mut reorder_cond: Option<(usize, usize)> = None;
        panel_card(ui, |ui| {
            ui.horizontal(|ui| {
                section_title(ui, Icon::Funnel, "Conditions");
                let n = self.rules[idx].conditions.len();
                chip(ui, &format!("{n} {}", if n > 1 { "critères actifs" } else { "critère actif" }), SKY, with_alpha(SKY, 38));
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
                        if let Some(pick) =
                            combo(ui, &format!("cf-{idx}-{ci}"), field.label(), &FIELDS.iter().map(|f| f.label()).collect::<Vec<_>>())
                        {
                            if let Some(f) = FIELDS.iter().find(|f| f.label() == pick) {
                                self.rules[idx].conditions[ci].field = *f;
                                self.rules[idx].conditions[ci].op = f.ops()[0];
                                if *f == Field::Kind && kind_label(&self.rules[idx].conditions[ci].value).is_none() {
                                    self.rules[idx].conditions[ci].value = KIND_CHOICES[0].0.into();
                                }
                            }
                        }
                        let op = self.rules[idx].conditions[ci].op;
                        let ops = field.ops();
                        if let Some(pick) =
                            combo(ui, &format!("co-{idx}-{ci}"), op.label(), &ops.iter().map(|o| o.label()).collect::<Vec<_>>())
                        {
                            if let Some(o) = ops.iter().find(|o| o.label() == pick) {
                                self.rules[idx].conditions[ci].op = *o;
                            }
                        }
                        let hint = match field {
                            Field::Extension => ".png, .jpg",
                            Field::Size => "50 MB / 512 KB",
                            Field::DateAdded | Field::DateModified => "2024-01-31  ou  30 days",
                            Field::Contents => "texte à chercher",
                            _ => "valeur",
                        };
                        let mut val = self.rules[idx].conditions[ci].value.clone();
                        let w = (ui.available_width() - 60.0).max(80.0);
                        if field == Field::Extension {
                            if let Some(v) = extension_tags(ui, egui::Id::new(("ext", rid, ci)), &val, w, accent) {
                                self.rules[idx].conditions[ci].value = v;
                            }
                        } else if field == Field::Kind {
                            let current = match kind_label(&val) {
                                Some(label) => label.to_string(),
                                None => format!("⚠ valeur inconnue : {val}"),
                            };
                            let labels: Vec<&str> = KIND_CHOICES.iter().map(|(_, l)| *l).collect();
                            if let Some(pick) = combo(ui, &format!("ck-{idx}-{ci}"), &current, &labels) {
                                if let Some((v, _)) = KIND_CHOICES.iter().find(|(_, l)| *l == pick) {
                                    self.rules[idx].conditions[ci].value = (*v).into();
                                }
                            }
                        } else if ui
                            .add(
                                egui::TextEdit::singleline(&mut val)
                                    .margin(egui::Margin::symmetric(8.0, 7.0))
                                    .desired_width(w)
                                    .hint_text(hint),
                            )
                            .changed()
                        {
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
                self.rules[idx].conditions.push(Condition { field: Field::Name, op: Op::Contains, value: String::new() });
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
                ui.label(RichText::new("exécutées dans l'ordre").size(14.0).color(text_dim()));
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
                        if let Some(pick) =
                            combo(ui, &format!("ak-{idx}-{ai}"), kind.label(), &ACTION_KINDS.iter().map(|k| k.label()).collect::<Vec<_>>())
                        {
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
                            if let Some(i) =
                                segmented(ui, egui::Id::new(("conflict", rid, ai)), &["remplacer", "ignorer", "numéroter"], current, accent)
                            {
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
            panel_card(ui, |ui| {
                ui.label(RichText::new("Rien pour l'instant.").size(16.0).family(semibold()).color(text_primary()));
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Chaque rangement apparaîtra ici, par lot, avec la possibilité de l'annuler.")
                        .size(14.0)
                        .color(text_dim()),
                );
                ui.add_space(12.0);
                if btn(ui, "Lancer un aperçu", Btn::Sky, accent, None).clicked() {
                    self.run_batch(true);
                }
            });
            return;
        }

        // Filtres : statut (avec compteurs) et recherche de la barre du haut.
        let labels: Vec<String> = history_view::FILTERS
            .iter()
            .map(|f| format!("{} {}", f.label(), self.history.iter().filter(|e| f.matches(e.status)).count()))
            .collect();
        let current = history_view::FILTERS.iter().position(|f| *f == self.history_filter).unwrap_or(0);
        let mut clear_search = false;
        ui.horizontal(|ui| {
            let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
            if let Some(i) = segmented(ui, egui::Id::new("history-filter"), &refs, current, SKY_DEEP) {
                self.history_filter = history_view::FILTERS[i];
                self.history_limit = HISTORY_PAGE_SIZE;
            }
            if !self.search.trim().is_empty() {
                ui.add_space(8.0);
                if chip_button(ui, &format!("Recherche : {}  ×", self.search.trim()), accent, with_alpha(accent, 38))
                    .on_hover_text("Effacer la recherche")
                    .clicked()
                {
                    clear_search = true;
                }
            }
        });
        if clear_search {
            self.search.clear();
        }
        ui.add_space(14.0);

        let days = history_view::group(&self.history, SystemTime::now(), self.history_filter, &tags::fold(self.search.trim()));
        if days.is_empty() {
            ui.label(RichText::new("Aucune entrée ne correspond à ce filtre.").size(14.5).color(text_dim()));
            return;
        }

        let mut undo_idx: Option<usize> = None;
        let mut undo_batch: Option<Vec<usize>> = None;
        let mut reveal: Option<PathBuf> = None;
        let mut shown = 0usize;
        let mut more = false;
        let first_batch = days.first().and_then(|d| d.batches.first()).map(|b| b.batch_id);

        'days: for day in &days {
            ui.label(caps(&day.label.to_uppercase()));
            ui.add_space(6.0);
            for batch in &day.batches {
                if shown >= self.history_limit {
                    more = true;
                    break 'days;
                }
                shown += batch.entries.len();
                let sum = history_view::batch_summary(&self.history, &batch.entries);
                panel_card_with(ui, None, 14.0, |ui| {
                    let id = ui.make_persistent_id(("history-batch", batch.batch_id));
                    let state =
                        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, first_batch == Some(batch.batch_id));
                    state
                        .show_header(ui, |ui| {
                            ui.label(
                                RichText::new(format!("Lot n°{}", batch.batch_id)).size(15.0).family(semibold()).color(text_primary()),
                            );
                            ui.label(
                                RichText::new(format!(
                                    "·  {}  ·  {} fichier{}",
                                    time_str(batch.created),
                                    sum.files,
                                    if sum.files > 1 { "s" } else { "" }
                                ))
                                .size(13.5)
                                .color(text_dim()),
                            );
                            if sum.applied > 0 {
                                chip(
                                    ui,
                                    &format!("{} appliqué{}", sum.applied, if sum.applied > 1 { "s" } else { "" }),
                                    status_color(Status::Applied),
                                    with_alpha(status_color(Status::Applied), 38),
                                );
                            }
                            if sum.failed > 0 {
                                chip(
                                    ui,
                                    &format!("{} échec{}", sum.failed, if sum.failed > 1 { "s" } else { "" }),
                                    status_color(Status::Failed),
                                    with_alpha(status_color(Status::Failed), 38),
                                );
                            }
                            if sum.undone > 0 {
                                chip(
                                    ui,
                                    &format!("{} annulé{}", sum.undone, if sum.undone > 1 { "s" } else { "" }),
                                    status_color(Status::Undone),
                                    with_alpha(status_color(Status::Undone), 38),
                                );
                            }
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if sum.undoable > 0
                                    && chip_button(ui, "Annuler le lot", accent, with_alpha(accent, 38))
                                        .on_hover_text("Annule toutes les actions annulables de ce lot")
                                        .clicked()
                                {
                                    undo_batch = Some(batch.entries.clone());
                                }
                            });
                        })
                        .body(|ui| {
                            ui.add_space(8.0);
                            for &i in &batch.entries {
                                self.history_row(ui, i, accent, &mut undo_idx, &mut reveal);
                            }
                        });
                });
                ui.add_space(10.0);
            }
        }

        if more {
            let remaining: usize = days.iter().flat_map(|d| &d.batches).map(|b| b.entries.len()).sum::<usize>() - shown;
            if btn(ui, &format!("Afficher plus ({remaining} restantes)"), Btn::Ghost, accent, None).clicked() {
                self.history_limit += HISTORY_PAGE_SIZE;
            }
        }
        if let Some(path) = reveal {
            system::reveal_in_finder(&path);
        }
        if let Some(entries) = undo_batch {
            self.undo_batch(&entries);
        } else if let Some(i) = undo_idx {
            self.undo(i);
        }
    }

    /// Une ligne compacte : fichier → dossier d'arrivée, action, boutons. Texte tronqué, détail au survol.
    fn history_row(&self, ui: &mut egui::Ui, i: usize, accent: Color32, undo_idx: &mut Option<usize>, reveal: &mut Option<PathBuf>) {
        let e = &self.history[i];
        let failed = matches!(e.status, Status::Failed | Status::BlockedByConflict);
        let frame = egui::Frame::none().inner_margin(egui::Margin::symmetric(4.0, 4.0)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (dot, _) = ui.allocate_exact_size(Vec2::splat(10.0), egui::Sense::hover());
                ui.painter().circle_filled(dot.center(), 4.5, status_color(e.status));
                ui.add_space(6.0);
                let right_w = 190.0;
                let left_w = (ui.available_width() - right_w).max(120.0);
                let name_w = (left_w * 0.58).floor();
                let truncated = |ui: &mut egui::Ui, w: f32, text: RichText| {
                    ui.allocate_ui_with_layout(Vec2::new(w, 22.0), Layout::left_to_right(Align::Center), |ui| {
                        ui.add(egui::Label::new(text).truncate());
                    });
                };
                truncated(ui, name_w, RichText::new(history_view::file_name(e)).size(14.5).family(semibold()).color(text_primary()));
                ui.label(RichText::new("→").size(14.0).color(text_label()));
                truncated(
                    ui,
                    (left_w - name_w - 24.0).max(40.0),
                    RichText::new(history_view::destination_label(e)).size(14.0).color(text_dim()),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if history_view::is_undoable(e) && chip_button(ui, "Annuler", accent, with_alpha(accent, 38)).clicked() {
                        *undo_idx = Some(i);
                    }
                    if e.status == Status::Applied && icon_button(ui, Icon::FolderOpen, "Afficher dans le Finder", false) {
                        *reveal = Some(e.result_path.clone());
                    }
                    chip(ui, &history_view::short_action(&e.action_label), text_dim(), glass(14));
                });
            });
            if failed {
                ui.horizontal(|ui| {
                    ui.add_space(22.0);
                    ui.add(egui::Label::new(RichText::new(fr_message(&e.message)).size(13.5).color(status_color(e.status))).wrap());
                });
            }
        });
        let tip = format!(
            "{}\nDepuis : {}\nVers : {}\nRègle : {} · {}\n{}",
            e.status.label(),
            e.original_path.display(),
            e.result_path.display(),
            e.rule_name,
            e.action_label,
            fr_message(&e.message)
        );
        ui.interact(frame.response.rect, egui::Id::new(("history-row", i)), egui::Sense::hover()).on_hover_text(nfc(&tip));
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) {
        let accent = self.accent();
        self.header(ui, "Réglages", "Appliqués immédiatement.");

        panel_card(ui, |ui| {
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
        panel_card(ui, |ui| {
            let before = self.launch_at_login;
            check_box(ui, &mut self.launch_at_login, "Lancement au démarrage", accent);
            if self.launch_at_login != before {
                if let Err(e) = crate::login::set_enabled(self.launch_at_login) {
                    self.launch_at_login = before;
                    self.toast_error(format!("Lancement au démarrage impossible : {e}"));
                }
            }
            ui.add_space(8.0);
            check_box(ui, &mut self.menu_bar, "Afficher dans la barre des menus", accent);
            ui.add_space(8.0);
            check_box(ui, &mut self.notify, "Prévenir après chaque organisation", accent);
            ui.add_space(8.0);
            check_box(ui, &mut self.animate_bg, "Animer le fond", accent);
            ui.add_space(8.0);
            check_box(ui, &mut self.sounds_on, "Jouer les sons", accent);
            ui.add_space(8.0);
            let mut auto_on = self.auto_organize_minutes > 0;
            check_box(ui, &mut auto_on, "Organiser automatiquement les dossiers suivis", accent);
            if auto_on != (self.auto_organize_minutes > 0) {
                self.auto_organize_minutes = if auto_on { AUTO_ORGANIZE_CHOICES[1].0 } else { 0 };
            }
            if auto_on {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Au lancement, puis toutes les").size(13.5).color(text_dim()));
                    let current = AUTO_ORGANIZE_CHOICES
                        .iter()
                        .find(|(m, _)| *m == self.auto_organize_minutes)
                        .map_or_else(|| format!("{} min", self.auto_organize_minutes), |(_, l)| (*l).to_string());
                    let labels: Vec<&str> = AUTO_ORGANIZE_CHOICES.iter().map(|(_, l)| *l).collect();
                    if let Some(pick) = combo(ui, "auto-organize", &current, &labels) {
                        if let Some((m, _)) = AUTO_ORGANIZE_CHOICES.iter().find(|(_, l)| *l == pick) {
                            self.auto_organize_minutes = *m;
                        }
                    }
                });
            }
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Garder l'historique").size(14.5).color(text_primary()));
                ui.add_space(8.0);
                slim_slider(ui, &mut self.retention_days, 1, 365, accent);
                chip(ui, &format!("{} jours", self.retention_days), accent, with_alpha(accent, 38));
            });
            ui.add_space(4.0);
            ui.label(RichText::new("Les entrées plus anciennes sont purgées.").size(13.5).color(text_dim()));
            ui.label(RichText::new("Entre 1 et 365 jours.").size(13.5).color(text_dim()));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new(RichText::new("Purger l'ancien historique").size(14.0)).fill(glass(34))).clicked() {
                    let removed = purge_history(&mut self.history, self.retention_days, SystemTime::now());
                    self.toast(format!("{removed} entrée(s) purgée(s)"));
                }
            });
        });

        ui.add_space(16.0);
        panel_card(ui, |ui| {
            ui.label(RichText::new("À propos").size(15.0).family(semibold()).color(text_primary()));
            ui.add_space(8.0);
            let w = WORDMARK_ABOUT_W;
            let mark = fitted_texture(ui.ctx(), "wordmark", WORDMARK_PNG, w);
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, w * mark.size_vec2().y / mark.size_vec2().x), egui::Sense::hover());
            paint_fitted(ui.painter(), &mark, rect, Color32::WHITE);
            resp.on_hover_text("OrganizeMyFiles");
            ui.add_space(4.0);
            ui.label(RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION"))).size(14.5).color(text_primary()));
            ui.add_space(4.0);
            ui.label(
                RichText::new("Outil libre et open source. Tout reste sur ton Mac : aucun envoi de données (seule la recherche de mises à jour contacte GitHub).").size(13.5).color(text_dim()),
            );
            ui.add_space(8.0);
            if btn(ui, "Revoir l'introduction", Btn::Ghost, accent, None).clicked() {
                self.intro = Some(IntroState::new(None));
            }
            ui.add_space(12.0);
            self.update_settings(ui, accent);
        });
        let _ = accent;
    }
}

fn shorten(p: &str) -> String {
    shorten_with_home(p, &home_dir())
}

/// Raccourcit un chemin : `~/…` sous le dossier personnel, chemin absolu sinon.
fn shorten_with_home(p: &str, home: &str) -> String {
    let (p, home) = (&nfc(p), &nfc(home));
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

    #[test]
    fn the_wordmark_png_decodes_with_the_logo_proportions() {
        let img = image::load_from_memory(WORDMARK_PNG).expect("logotype valide");
        let ratio = img.width() as f32 / img.height() as f32;
        assert!((6.5..7.2).contains(&ratio), "rapport largeur/hauteur du logotype : {ratio}");
        // Fond transparent : le logotype se pose sur le thème sombre sans cadre.
        assert_eq!(img.to_rgba8().get_pixel(0, 0)[3], 0);
    }

    #[test]
    fn a_reduced_white_logo_keeps_a_valid_premultiplied_edge() {
        // Carré blanc opaque sur fond transparent « noir » : sans prémultiplication, les contours réduits seraient gris.
        let mut img = image::RgbaImage::from_pixel(64, 64, image::Rgba([0, 0, 0, 0]));
        for y in 16..48 {
            for x in 16..48 {
                img.put_pixel(x, y, image::Rgba([255, 255, 255, 255]));
            }
        }
        let small = resize_premultiplied(&img, 20);
        assert_eq!(small.size, [20, 20]);
        for p in &small.pixels {
            let [r, g, b, a] = p.to_array();
            assert!(r <= a && g <= a && b <= a, "couleur prémultipliée invalide : {p:?}");
            assert_eq!((r, g), (g, b), "le blanc reste neutre, même sur les contours");
            assert_eq!(r, a, "un blanc à demi transparent prémultiplié vaut son opacité (pas de liseré gris)");
        }
        assert!(small.pixels.iter().any(|p| p.a() == 255) && small.pixels.iter().any(|p| p.a() == 0));
    }

    #[test]
    fn an_image_is_never_enlarged() {
        let img = image::RgbaImage::from_pixel(10, 4, image::Rgba([255, 255, 255, 255]));
        assert_eq!(resize_premultiplied(&img, 50).size, [10, 4]);
    }

    #[test]
    fn a_fitted_texture_has_the_exact_pixel_size_and_is_cached() {
        let ctx = egui::Context::default();
        let a = fitted_texture(&ctx, "wordmark", WORDMARK_PNG, 128.0);
        let b = fitted_texture(&ctx, "wordmark", WORDMARK_PNG, 128.0);
        assert_eq!(a.id(), b.id(), "la texture est calculée une seule fois par taille");
        assert_eq!(a.size()[0], 128);
        assert_eq!(a.size()[1], 19, "2216 × 322 réduit à 128 px de large");
    }

    #[test]
    fn segmented_buttons_never_touch() {
        // Deux segments de 60 pt : 4 de marge de chaque côté et 4 entre eux, régulier quel que soit leur nombre.
        assert_eq!(segmented_width(&[60.0, 60.0]), 60.0 + 60.0 + 4.0 + 8.0);
        assert_eq!(segmented_width(&[60.0]), 68.0);
        assert!(segmented_width(&[50.0, 50.0, 50.0]) - 150.0 >= 2.0 * SEGMENT_GAP + 2.0 * SEGMENT_PAD);
    }

    #[test]
    fn the_search_box_never_overlaps_the_right_group_of_the_top_bar() {
        // Largeurs réalistes des boutons (mesurées sur Mac) ; la fenêtre ne descend pas sous 980 px.
        for window_w in (980..=2200).step_by(10) {
            let window_w = window_w as f32;
            let wide = window_w >= 1280.0;
            let run_w = if window_w >= 1300.0 { 190.0 } else { 113.0 };
            let right_w = top_bar_right_width(run_w, 160.0, wide, 8.0);
            let search_left = 259.0;
            let search_w = (window_w - 16.0 - right_w - 12.0 - search_left).clamp(SEARCH_MIN_W, 640.0);
            assert!(
                search_left + search_w + 10.0 + right_w <= window_w - 16.0 + 0.01,
                "chevauchement à {window_w} px : recherche jusqu'à {}, groupe de droite dès {}",
                search_left + search_w,
                window_w - 16.0 - right_w
            );
            assert!(search_w > SEARCH_MIN_W, "la recherche garde de la place à {window_w} px");
        }
    }

    #[test]
    fn the_wide_top_bar_reserves_room_for_the_bell_and_the_gear() {
        assert!(top_bar_right_width(113.0, 160.0, true, 8.0) > top_bar_right_width(113.0, 160.0, false, 8.0) + 2.0 * ICON_BUTTON_W);
    }

    #[test]
    fn ellipsize_truncates_to_width() {
        let ctx = egui::Context::default();
        ctx.begin_pass(Default::default());
        let p = egui::Painter::new(ctx.clone(), egui::LayerId::background(), egui::Rect::EVERYTHING);
        let font = egui::FontId::proportional(14.0);
        assert_eq!(ellipsize(&p, "ok", &font, 500.0), "ok", "un texte qui tient est inchangé");
        let long = ellipsize(&p, "actions au dernier passage", &font, 60.0);
        assert!(long.ends_with('…') && long.len() < "actions au dernier passage".len(), "tronqué avec ellipse : {long}");
        assert!(p.layout_no_wrap(long.clone(), font.clone(), Color32::WHITE).size().x <= 60.0, "le texte tronqué tient dans 60 px");
        // Accents : pas de coupe au milieu d'un caractère UTF-8.
        let accented = ellipsize(&p, "RÈGLES ACTIVES", &font, 40.0);
        assert!(accented.ends_with('…'));
        let _ = ctx.end_pass();
    }

    #[test]
    fn shorten_handles_home_and_outside_home() {
        assert_eq!(shorten_with_home("/Users/a/Downloads", "/Users/a"), "~/Downloads");
        assert_eq!(shorten_with_home("/Users/a", "/Users/a"), "~");
        assert_eq!(shorten_with_home("/tmp", "/Users/a"), "/tmp");
        assert_eq!(shorten_with_home("/Users/ab/x", "/Users/a"), "/Users/ab/x");
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
        let launch =
            || Launch { bg_time: 0.0, screenshot: None, tab: None, empty: false, background: false, show_intro: false, intro_time: None };
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
        let mut a = App::blank(Launch {
            bg_time: 0.0,
            screenshot: None,
            tab: None,
            empty: false,
            background: false,
            show_intro: false,
            intro_time: None,
        });
        a.seed_demo();
        let mut state = a.snapshot();
        state.settings.retention_days = 1;
        state.history[0].created = SystemTime::now() - std::time::Duration::from_secs(40 * 86_400);
        let before = state.history.len();
        let mut b = App::blank(Launch {
            bg_time: 0.0,
            screenshot: None,
            tab: None,
            empty: false,
            background: false,
            show_intro: false,
            intro_time: None,
        });
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

    #[test]
    fn kpi_columns_switches_at_1200() {
        assert_eq!(kpi_columns(980.0), 2);
        assert_eq!(kpi_columns(1199.9), 2);
        assert_eq!(kpi_columns(1200.0), 4);
        assert_eq!(kpi_columns(1280.0), 4);
    }

    fn rule_on(id: u64, folders: &[u64]) -> Rule {
        let mut r = sample_rule(id, folders[0], "r", 0);
        r.folder_ids = folders.to_vec();
        r
    }

    #[test]
    fn removing_a_shared_folder_keeps_the_rule_and_undo_restores_it() {
        let mut app = App::blank(Launch {
            bg_time: 0.0,
            screenshot: None,
            tab: None,
            empty: false,
            background: false,
            show_intro: false,
            intro_time: None,
        });
        app.folders = vec![
            Folder { id: 1, path: "/a".into(), enabled: true, priority: 0 },
            Folder { id: 2, path: "/b".into(), enabled: true, priority: 1 },
        ];
        app.rules = vec![rule_on(10, &[1, 2]), rule_on(11, &[1])];
        app.remove_folder(0);
        assert_eq!(app.rules.len(), 1, "la règle propre au dossier part avec lui");
        assert_eq!(app.rules[0].folder_ids, vec![2]);
        app.restore_folder();
        assert_eq!(app.rules.len(), 2);
        let shared = app.rules.iter().find(|r| r.id == 10).unwrap();
        assert!(shared.folder_ids.contains(&1) && shared.folder_ids.contains(&2));
        assert_eq!(app.folders.len(), 2);
    }

    fn history_entry(
        batch: u64,
        kind: ActionKind,
        from: &std::path::Path,
        to: &std::path::Path,
        original: &std::path::Path,
        result: &std::path::Path,
    ) -> HistoryEntry {
        HistoryEntry {
            batch_id: batch,
            rule_name: "r".into(),
            action_label: kind.label().into(),
            original_path: original.to_path_buf(),
            result_path: result.to_path_buf(),
            undo: Some(Undo { kind, from: from.to_path_buf(), to: to.to_path_buf() }),
            reversible: true,
            status: Status::Applied,
            message: String::new(),
            created: SystemTime::now(),
        }
    }

    #[test]
    fn undoing_a_batch_reverts_the_last_action_first() {
        let dir = std::env::temp_dir().join(format!("organizemyfiles-undo-batch-{}", std::process::id()));
        let sub = dir.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        // Le fichier a été déplacé dans sub/ puis renommé en b.txt.
        std::fs::write(sub.join("b.txt"), b"x").unwrap();
        let (orig, moved, renamed) = (dir.join("a.txt"), sub.join("a.txt"), sub.join("b.txt"));
        let mut app = App::blank(Launch {
            bg_time: 0.0,
            screenshot: None,
            tab: None,
            empty: false,
            background: false,
            show_intro: false,
            intro_time: None,
        });
        app.history = vec![
            history_entry(1, ActionKind::Move, &moved, &orig, &orig, &moved),
            history_entry(1, ActionKind::Rename, &renamed, &moved, &moved, &renamed),
        ];
        app.undo_batch(&[0, 1]);
        assert!(orig.exists(), "le fichier est revenu à sa place d'origine");
        assert!(app.history.iter().all(|e| e.status == Status::Undone));
    }
}

mod simple_rules;
mod update_ui;

#[cfg(test)]
mod e2e;
