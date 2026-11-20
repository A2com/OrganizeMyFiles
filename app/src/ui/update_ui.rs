//! Mises à jour côté interface : vérification en arrière-plan, bandeau « Version X disponible », réglages.

use std::sync::mpsc;

use eframe::egui::{self, RichText};

use super::*;
use crate::system::SystemCommands;
use crate::update::{self, Msg, State};

/// Délai entre deux vérifications automatiques.
const CHECK_EVERY_SECONDS: f64 = 24.0 * 3600.0;
/// Laisse l'appli finir de démarrer avant de toucher au réseau.
const FIRST_CHECK_AFTER_FRAMES: u32 = 120;

impl App {
    /// Appelé à chaque image : récupère les réponses des fils de travail, lance la vérification du jour.
    pub(super) fn update_tick(&mut self, ctx: &egui::Context, clock: f64) {
        self.poll_update();
        let idle = matches!(self.update, State::Idle | State::UpToDate | State::Failed(_));
        if self.screenshot.is_none()
            && self.check_updates
            && idle
            && self.frame >= FIRST_CHECK_AFTER_FRAMES
            && clock >= self.next_update_check
        {
            self.next_update_check = clock + CHECK_EVERY_SECONDS;
            self.start_update_check(ctx, false);
        }
    }

    pub(super) fn start_update_check(&mut self, ctx: &egui::Context, manual: bool) {
        if matches!(self.update, State::Checking | State::Installing(_)) {
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.update_rx = Some(rx);
        self.update = State::Checking;
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = update::check(&SystemCommands, env!("CARGO_PKG_VERSION"));
            let _ = tx.send(Msg::Checked { manual, result });
            ctx.request_repaint();
        });
    }

    fn start_update_install(&mut self, ctx: &egui::Context, release: update::Release) {
        let Some(bundle) = std::env::current_exe().ok().and_then(|exe| update::bundle_path(&exe)) else {
            system::open_url(&release.page_url);
            return;
        };
        let (tx, rx) = mpsc::channel();
        self.update_rx = Some(rx);
        self.update = State::Installing("Préparation…".into());
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let work = std::env::temp_dir().join(format!("organizemyfiles-update-{}", std::process::id()));
            let progress = |step: &str| {
                let _ = tx.send(Msg::Progress(step.to_string()));
                ctx.request_repaint();
            };
            let result = update::install(&SystemCommands, &release, &bundle, &work, &progress);
            let _ = tx.send(Msg::Installed(result));
            ctx.request_repaint();
        });
    }

    pub(super) fn poll_update(&mut self) {
        let Some(rx) = &self.update_rx else { return };
        let messages: Vec<Msg> = rx.try_iter().collect();
        for msg in messages {
            match msg {
                Msg::Checked { manual, result } => {
                    self.update = match result {
                        Ok(Some(release)) => State::Available(release),
                        Ok(None) => State::UpToDate,
                        Err(e) if manual => State::Failed(e),
                        // Vérification automatique : pas de réseau ne dérange pas l'utilisateur.
                        Err(_) => State::Idle,
                    };
                    if manual {
                        self.update_dismissed = None;
                    }
                }
                Msg::Progress(step) => self.update = State::Installing(step),
                Msg::Installed(Ok(())) => self.finish_update(),
                Msg::Installed(Err(e)) => self.update = State::Failed(e),
            }
        }
    }

    /// Nouvelle version en place : on sauvegarde, on programme la relance et on quitte vraiment (pas la barre des menus).
    fn finish_update(&mut self) {
        let Some(bundle) = std::env::current_exe().ok().and_then(|exe| update::bundle_path(&exe)) else { return };
        self.persist();
        update::relaunch(&bundle);
        std::process::exit(0);
    }

    /// Bandeau en haut de l'écran : version disponible, installation en cours ou erreur.
    pub(super) fn update_banner(&mut self, ui: &mut egui::Ui, accent: Color32) {
        let ctx = ui.ctx().clone();
        match self.update.clone() {
            State::Available(release) if self.update_dismissed.as_deref() != Some(release.version.as_str()) => {
                panel_card_with(ui, Some(with_alpha(accent, 22)), 16.0, |ui| {
                    ui.label(
                        RichText::new(format!("OrganizeMyFiles {} est disponible", release.version))
                            .size(16.5)
                            .family(semibold())
                            .color(text_primary()),
                    );
                    ui.label(
                        RichText::new(format!("Vous utilisez la version {}.", env!("CARGO_PKG_VERSION"))).size(14.0).color(text_dim()),
                    );
                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        let can_install =
                            release.dmg_url.is_some() && std::env::current_exe().ok().and_then(|exe| update::bundle_path(&exe)).is_some();
                        let label = if can_install { "Installer et redémarrer" } else { "Télécharger" };
                        if btn(ui, label, Btn::Primary, accent, None).clicked() {
                            if can_install {
                                self.start_update_install(&ctx, release.clone());
                            } else {
                                system::open_url(&release.page_url);
                            }
                        }
                        if btn(ui, "Notes de version", Btn::Ghost, accent, None).clicked() {
                            system::open_url(&release.page_url);
                        }
                        if btn(ui, "Plus tard", Btn::Ghost, accent, None).clicked() {
                            self.update_dismissed = Some(release.version.clone());
                        }
                    });
                });
                ui.add_space(14.0);
            }
            State::Installing(step) => {
                ctx.request_repaint_after(std::time::Duration::from_millis(200));
                panel_card_with(ui, Some(with_alpha(accent, 22)), 16.0, |ui| {
                    ui.label(RichText::new(format!("Mise à jour : {step}")).size(16.0).family(semibold()).color(text_primary()));
                    ui.label(RichText::new("OrganizeMyFiles va redémarrer tout seul.").size(14.0).color(text_dim()));
                });
                ui.add_space(14.0);
            }
            State::Failed(message) if self.tab != Tab::Settings => {
                panel_card_with(ui, Some(with_alpha(RED, 26)), 16.0, |ui| {
                    ui.label(RichText::new("La mise à jour n'a pas abouti").size(16.0).family(semibold()).color(text_primary()));
                    ui.label(RichText::new(message).size(14.0).color(text_dim()));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if btn(ui, "Fermer", Btn::Ghost, accent, None).clicked() {
                            self.update = State::Idle;
                        }
                    });
                });
                ui.add_space(14.0);
            }
            _ => {}
        }
    }

    /// Bloc « Mises à jour » de Réglages › À propos.
    pub(super) fn update_settings(&mut self, ui: &mut egui::Ui, accent: Color32) {
        let ctx = ui.ctx().clone();
        ui.horizontal(|ui| {
            let busy = matches!(self.update, State::Checking | State::Installing(_));
            ui.add_enabled_ui(!busy, |ui| {
                if btn(ui, "Rechercher des mises à jour", Btn::Ghost, accent, None).clicked() {
                    self.start_update_check(&ctx, true);
                }
            });
            ui.add_space(8.0);
            let status = match &self.update {
                State::Checking => "Recherche en cours…".to_string(),
                State::UpToDate => "Vous avez la dernière version.".to_string(),
                State::Available(r) => format!("La version {} est disponible (voir le bandeau en haut).", r.version),
                State::Installing(step) => format!("Mise à jour : {step}"),
                State::Failed(e) => e.clone(),
                State::Idle => String::new(),
            };
            ui.label(RichText::new(status).size(13.5).color(text_dim()));
        });
        ui.add_space(6.0);
        check_box(ui, &mut self.check_updates, "Vérifier automatiquement (au lancement, puis une fois par jour)", accent);
    }
}
