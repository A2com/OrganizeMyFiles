//! Entry point: eframe/egui desktop app.
//! `--tab rules` ouvre l'écran voulu (dashboard, folders, rules, history, settings).
//! `--arriere-plan` : démarre fenêtre réduite (lancement à l'ouverture de session).
//! `--intro` : montre l'écran d'introduction ; `--intro-time <s>` le fige à cet instant (captures).
//! `--bg-time <s>` : décale l'horloge du fond animé (captures à différents instants).
//! `--empty` : avec --screenshot, capture l'appli sans données de démonstration.
//! `--size 980x640` : taille de la fenêtre (défaut 1180x780 ; la taille minimale reste 980x640).
//! `--screenshot <file.png>` draws ~120 frames, saves a PNG of the window and
//! exits (used for automated checks).

mod alerts;
mod engine;
mod extract;
mod glow;
mod history_view;
mod intro;
mod login;
mod mechanics;
mod menubar;
mod menubar_info;
mod sound;
mod store;
mod system;
mod tags;
mod toast;
mod ui;
mod update;
mod watcher;
mod zipmini;

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().collect();
    let mut screenshot: Option<String> = None;
    let mut tab: Option<String> = None;
    let mut empty = false;
    let background = args.iter().any(|a| a == "--arriere-plan");
    let mut bg_time = 0.0f64;
    let mut size = [1180.0f32, 780.0f32];
    let show_intro = args.iter().any(|a| a == "--intro");
    let mut intro_time: Option<f64> = None;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--screenshot" {
            screenshot = args.get(i + 1).cloned();
            i += 2;
        } else if args[i] == "--intro-time" {
            intro_time = args.get(i + 1).and_then(|v| v.parse().ok());
            i += 2;
        } else if args[i] == "--bg-time" {
            bg_time = args.get(i + 1).and_then(|v| v.parse().ok()).unwrap_or(0.0);
            i += 2;
        } else if args[i] == "--empty" {
            empty = true;
            i += 1;
        } else if args[i] == "--tab" {
            tab = args.get(i + 1).cloned();
            i += 2;
        } else if args[i] == "--size" {
            if let Some((w, h)) = args.get(i + 1).and_then(|v| v.split_once('x')) {
                if let (Ok(w), Ok(h)) = (w.parse::<f32>(), h.parse::<f32>()) {
                    size = [w, h];
                }
            }
            i += 2;
        } else {
            i += 1;
        }
    }

    let options = eframe::NativeOptions {
        // Lissage matériel des bords (4 échantillons par pixel) : formes vectorielles et images moins crénelées.
        multisampling: 4,
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size(size)
            .with_min_inner_size([980.0, 640.0])
            .with_title("OrganizeMyFiles")
            .with_icon(ui::app_icon())
            // macOS : barre de titre fondue dans le contenu, feux natifs conservés.
            .with_fullsize_content_view(cfg!(target_os = "macos"))
            .with_titlebar_shown(!cfg!(target_os = "macos"))
            .with_title_shown(!cfg!(target_os = "macos")),
        ..Default::default()
    };

    eframe::run_native(
        "OrganizeMyFiles",
        options,
        Box::new(move |cc| {
            Ok(Box::new(ui::App::new(cc, ui::Launch { screenshot, tab, empty, bg_time, background, show_intro, intro_time })))
        }),
    )
}
