//! Tests d'enchaînement de bout en bout (sans fenêtre) : ce que fait un utilisateur, de l'ajout
//! d'un dossier à l'annulation, puis fermeture et relance de l'appli.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::*;

fn blank_app() -> App {
    let mut app = App::blank(Launch {
        bg_time: 0.0,
        screenshot: None,
        tab: None,
        empty: false,
        background: false,
        show_intro: false,
        intro_time: None,
    });
    app.notify = false; // pas de vraie notification pendant les tests
    app
}

fn temp_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos();
    let dir = std::env::temp_dir().join(format!("organizemyfiles-e2e-{tag}-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.canonicalize().unwrap()
}

/// Ajoute un dossier suivi et une règle « extension png → déplacer vers `dest` » ; renvoie l'id de la règle.
fn add_png_rule(app: &mut App, watched: &Path, dest: &Path) -> u64 {
    app.add_folders(vec![watched.to_path_buf()]);
    let folder_id = app.folders[0].id;
    app.new_rule_in(folder_id);
    let rule = app.rules.last_mut().expect("la règle a été créée");
    rule.name = "Images".into();
    rule.conditions = vec![Condition { field: Field::Extension, op: Op::Equals, value: "png".into() }];
    let mut action = Action::new(ActionKind::Move);
    action.params.insert("destination".into(), dest.to_string_lossy().into_owned());
    rule.actions = vec![action];
    rule.id
}

#[test]
fn full_journey_add_folder_rule_organise_undo_restart() {
    let (watched, dest) = (temp_dir("in"), temp_dir("out"));
    let mut app = blank_app();

    // 1. Dossier + règle.
    add_png_rule(&mut app, &watched, &dest);
    assert_eq!(app.folders.len(), 1);
    assert_eq!(app.rules.len(), 1);

    // 2. Deux fichiers déposés, un seul correspond à la règle.
    std::fs::write(watched.join("photo.png"), b"png").unwrap();
    std::fs::write(watched.join("notes.txt"), b"txt").unwrap();

    // 3. « Organiser » : seul le .png bouge ; l'historique le dit.
    app.run_batch(false);
    assert!(dest.join("photo.png").exists());
    assert!(watched.join("notes.txt").exists());
    assert!(!watched.join("photo.png").exists());
    assert_eq!(app.history.len(), 1);
    assert_eq!(app.history[0].status, Status::Applied);

    // 4. Annuler : le fichier revient, l'entrée passe à « Annulé ».
    app.undo(0);
    assert!(watched.join("photo.png").exists());
    assert!(!dest.join("photo.png").exists());
    assert_eq!(app.history[0].status, Status::Undone);

    // 5. Fermeture puis relance : tout est retrouvé.
    let saved = store::to_json(&app.snapshot());
    let state: State = serde_json::from_str(&saved).expect("l'état sauvegardé se relit");
    let mut relaunched = blank_app();
    relaunched.restore(state);
    assert_eq!(relaunched.folders.len(), 1);
    assert_eq!(relaunched.rules.len(), 1);
    assert_eq!(relaunched.history.len(), 1);
    assert_eq!(relaunched.history[0].status, Status::Undone);
    assert_eq!(relaunched.rules[0].name, "Images");
}

#[test]
fn preview_never_touches_files() {
    let (watched, dest) = (temp_dir("pin"), temp_dir("pout"));
    let mut app = blank_app();
    add_png_rule(&mut app, &watched, &dest);
    std::fs::write(watched.join("photo.png"), b"png").unwrap();
    app.run_batch(true);
    assert!(watched.join("photo.png").exists(), "l'aperçu ne déplace rien");
    assert!(!dest.join("photo.png").exists());
    assert_eq!(app.history.len(), 1);
    assert_eq!(app.history[0].status, Status::DryRun);
}

#[test]
fn removing_a_folder_can_be_undone_with_its_rules() {
    let (watched, dest) = (temp_dir("rin"), temp_dir("rout"));
    let mut app = blank_app();
    add_png_rule(&mut app, &watched, &dest);
    app.remove_folder(0);
    assert!(app.folders.is_empty() && app.rules.is_empty());
    app.restore_folder();
    assert_eq!(app.folders.len(), 1);
    assert_eq!(app.rules.len(), 1);
}

#[test]
fn cmd_comma_opens_settings_and_plain_comma_does_nothing() {
    let mut app = blank_app();
    app.tab = Tab::Dashboard;
    let ctx = egui::Context::default();
    let key = |modifiers| egui::Event::Key { key: egui::Key::Comma, physical_key: None, pressed: true, repeat: false, modifiers };

    // ⌘, : le raccourci macOS standard ouvre les réglages.
    ctx.begin_pass(egui::RawInput { events: vec![key(egui::Modifiers::COMMAND)], ..Default::default() });
    app.global_shortcuts(&ctx);
    let _ = ctx.end_pass();
    assert!(matches!(app.tab, Tab::Settings), "⌘, ouvre les réglages");

    // Virgule seule : aucun changement d'onglet.
    app.tab = Tab::Dashboard;
    ctx.begin_pass(egui::RawInput { events: vec![key(egui::Modifiers::NONE)], ..Default::default() });
    app.global_shortcuts(&ctx);
    let _ = ctx.end_pass();
    assert!(matches!(app.tab, Tab::Dashboard), "virgule seule ne change rien");
}

#[test]
fn intro_shows_until_it_has_been_seen_and_never_when_launched_in_background() {
    let mut seen = blank_app();
    let mut state = seen.snapshot();
    state.settings.intro_seen = true;
    seen.restore(state);
    assert!(seen.intro.is_none(), "déjà vue : pas d'introduction");

    let mut first = blank_app();
    let mut state = first.snapshot();
    state.settings.intro_seen = false;
    first.restore(state);
    assert!(first.intro.is_some(), "pas encore vue : on la montre");
    assert!(!first.intro_seen);

    // Démarrage à l'ouverture de session : jamais d'introduction.
    let mut background = App::blank(Launch {
        bg_time: 0.0,
        screenshot: None,
        tab: None,
        empty: false,
        background: true,
        show_intro: false,
        intro_time: None,
    });
    let mut state = background.snapshot();
    state.settings.intro_seen = false;
    background.restore(state);
    assert!(background.intro.is_none());
}
