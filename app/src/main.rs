//! Entry point: eframe/egui desktop app.
//! `--screenshot <file.png>` draws ~120 frames, saves a PNG of the window and
//! exits (used for automated checks).

mod engine;
mod mechanics;
mod ui;
mod zipmini;

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().collect();
    let mut screenshot: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--screenshot" {
            screenshot = args.get(i + 1).cloned();
            i += 2;
        } else {
            i += 1;
        }
    }

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 780.0])
            .with_min_inner_size([980.0, 640.0])
            .with_title("OrganizeMyFiles")
            // macOS : barre de titre fondue dans le contenu, feux natifs conservés.
            .with_fullsize_content_view(cfg!(target_os = "macos"))
            .with_titlebar_shown(!cfg!(target_os = "macos"))
            .with_title_shown(!cfg!(target_os = "macos")),
        ..Default::default()
    };

    eframe::run_native(
        "File Organizer",
        options,
        Box::new(move |cc| Ok(Box::new(ui::App::new(cc, screenshot)))),
    )
}
