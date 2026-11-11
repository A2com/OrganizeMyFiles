//! Interface: soft glass cards, rounded pill buttons, colour sidebar —
//! rebuilt from scratch (no original assets or branding).

use std::path::PathBuf;
use std::time::SystemTime;

use eframe::egui::{self, Align, Color32, Layout, Pos2, Rect, RichText, Rounding, Stroke, Vec2};

use crate::engine::*;
use crate::mechanics::*;

// ------------------------------------------------------------------- theme

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Dashboard,
    Folders,
    Rules,
    History,
    Plan,
    Settings,
}

/// Icon glyphs drawn with painter primitives — coherent set, no initials.
#[derive(Clone, Copy, PartialEq)]
enum Icon {
    Chart,
    Folder,
    List,
    Clock,
    Check,
    Sliders,
    Sort,
}

const TABS: &[(Tab, &str, Icon, &str)] = &[
    (Tab::Dashboard, "Aperçu", Icon::Chart, "Tout en un coup d'œil"),
    (Tab::Folders, "Dossiers suivis", Icon::Folder, "Les lieux à organiser"),
    (Tab::Rules, "Règles", Icon::List, "Si ceci, alors cela"),
    (Tab::History, "Historique", Icon::Clock, "Ce qui s'est passé, annulable"),
    (Tab::Plan, "Capacités", Icon::Check, "Tout est débloqué — sans compte"),
    (Tab::Settings, "Réglages", Icon::Sliders, "Ajuster l'application"),
];

// Accent swatches, retuned for the dark gradient theme (blue and teal sampled
// from the supplied theme picture).
const ACCENTS: &[(&str, Color32)] = &[
    ("blue", Color32::from_rgb(0x50, 0x87, 0xcc)),
    ("purple", Color32::from_rgb(0xa7, 0x8b, 0xfa)),
    ("pink", Color32::from_rgb(0xf4, 0x72, 0xb6)),
    ("orange", Color32::from_rgb(0xf5, 0x9e, 0x5c)),
    ("yellow", Color32::from_rgb(0xed, 0xc1, 0x3f)),
    ("green", Color32::from_rgb(0x35, 0xa7, 0x8f)),
    ("graphite", Color32::from_rgb(0x8d, 0x99, 0xaa)),
];

// Supplied gradient theme: diagonal slate-blue, lighter at the top-left
// (corner colours sampled from picture-1.jpg).
fn bg_tl() -> Color32 {
    Color32::from_rgb(0x50, 0x6e, 0x8a)
}
fn bg_tr() -> Color32 {
    Color32::from_rgb(0x2e, 0x3e, 0x4f)
}
fn bg_bl() -> Color32 {
    Color32::from_rgb(0x33, 0x44, 0x56)
}
fn bg_br() -> Color32 {
    Color32::from_rgb(0x29, 0x35, 0x43)
}
fn card_fill() -> Color32 {
    Color32::from_rgba_unmultiplied(0xff, 0xff, 0xff, 26)
}
fn card_stroke() -> Color32 {
    Color32::from_rgba_unmultiplied(0xff, 0xff, 0xff, 56)
}
fn text_primary() -> Color32 {
    Color32::from_rgb(0xff, 0xff, 0xff)
}
fn text_dim() -> Color32 {
    Color32::from_rgb(0xb3, 0xc1, 0xd4)
}

// --------------------------------------------------------------------- app

pub struct App {
    tab: Tab,
    folders: Vec<Folder>,
    rules: Vec<Rule>,
    history: Vec<HistoryEntry>,
    next_id: u64,
    next_batch: u64,
    is_pro: bool,
    monthly_used: u32,
    accent: usize,
    menu_bar: bool,
    notify: bool,
    retention_days: u32,
    add_folder_text: String,
    selected_folder: usize,
    selected_rule: usize,
    toast: Option<String>,
    last_run: Option<(usize, usize, usize)>, // matched, actions, scanned
    frame: u32,
    screenshot: Option<String>,
    shot_done: bool,
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>, screenshot: Option<String>) -> Self {
        let mut app = App {
            tab: Tab::Dashboard,
            folders: Vec::new(),
            rules: Vec::new(),
            history: Vec::new(),
            next_id: 1,
            next_batch: 1,
            is_pro: true, // no account system: every capability is unlocked
            monthly_used: 0,
            accent: 0,
            menu_bar: true,
            notify: true,
            retention_days: HISTORY_RETENTION_DEFAULT_DAYS,
            add_folder_text: String::new(),
            selected_folder: 0,
            selected_rule: 0,
            toast: None,
            last_run: None,
            frame: 0,
            screenshot,
            shot_done: false,
        };
        if app.screenshot.is_some() {
            app.seed_demo();
        }
        app
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
        let f0 = self.folders[0].id;
        self.rules.push(sample_rule(self.next_id, f0, "Invoices to Archive", 0));
        self.next_id += 1;
        self.rules.push(sample_rule(self.next_id, f0, "Screenshots to Pictures", 1));
        self.next_id += 1;
        self.rules.push(sample_rule(self.next_id, self.folders[1].id, "Music to Library", 0));
        self.next_id += 1;
        self.monthly_used = 42;
        let now = SystemTime::now();
        for i in 0..6 {
            self.history.push(HistoryEntry {
                batch_id: 1,
                rule_name: ["Invoices to Archive", "Screenshots to Pictures"][i % 2].into(),
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
                    "A file already exists at the destination.".into()
                } else {
                    "moved to Archive/2026-11".into()
                },
                created: now,
            });
        }
        self.last_run = Some((12, 18, 30));
        self.next_batch = 2;
    }

    fn accent(&self) -> Color32 {
        ACCENTS[self.accent].1
    }

    fn toast(&mut self, msg: impl Into<String>) {
        self.toast = Some(msg.into());
    }

    fn run_batch(&mut self, dry_run: bool) {
        if self.folders.is_empty() {
            self.toast("Ajoutez d'abord un dossier suivi");
            return;
        }
        let mut runner = Runner {
            monthly_used: self.monthly_used,
            batch_id: self.next_batch,
            dry_run,
        };
        let outcome = runner.run(&self.folders, &self.rules, SystemTime::now());
        self.next_batch += 1;
        if !dry_run {
            self.monthly_used = runner.monthly_used;
        }
        let actions = outcome.history.len();
        self.last_run = Some((outcome.matched, actions, outcome.scanned));
        for e in outcome.history {
            self.history.push(e);
        }
        if dry_run {
            self.toast(format!("Aperçu seulement — {} fichier(s) correspondant(s)", outcome.matched));
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
                    self.toast("Annulé");
                }
                Err(msg) => self.toast(format!("Annulation impossible : {msg}")),
            }
        }
    }
}

fn sample_rule(id: u64, folder_id: u64, name: &str, prio: i64) -> Rule {
    let mut r = Rule {
        id,
        folder_id,
        name: name.into(),
        enabled: true,
        match_all: true,
        recursion_depth: if prio == 0 { 1 } else { 0 },
        priority: prio,
        conditions: Vec::new(),
        actions: Vec::new(),
    };
    match prio {
        0 => {
            r.conditions.push(Condition {
                field: Field::Extension,
                op: Op::Equals,
                value: "pdf".into(),
            });
            r.conditions.push(Condition {
                field: Field::Name,
                op: Op::Contains,
                value: "invoice".into(),
            });
            let mut a = Action::new(ActionKind::Move);
            a.params
                .insert("destination".into(), "~/Documents/Archive/{year}/{month}".into());
            a.params.insert(ON_CONFLICT_PARAM_KEY.into(), "skip".into());
            r.actions.push(a);
            let mut b = Action::new(ActionKind::AddTag);
            b.params.insert("tags".into(), "paperwork".into());
            r.actions.push(b);
        }
        1 => {
            r.conditions.push(Condition {
                field: Field::Kind,
                op: Op::Is,
                value: "image".into(),
            });
            let mut a = Action::new(ActionKind::Rename);
            a.params
                .insert("pattern".into(), "{name}-{current_date}.{extension}".into());
            r.actions.push(a);
            let mut c = Action::new(ActionKind::Compress);
            c.params.insert("pattern".into(), "{name}-{timestamp}".into());
            r.actions.push(c);
        }
        _ => {
            r.conditions.push(Condition {
                field: Field::Kind,
                op: Op::Is,
                value: "audio".into(),
            });
            let mut a = Action::new(ActionKind::ImportToLibrary);
            a.params.insert("library_type".into(), "music".into());
            r.actions.push(a);
        }
    }
    r
}

// ------------------------------------------------------------ egui helpers

fn gradient_bg(ui: &mut egui::Ui, rect: Rect) {
    let painter = ui.painter().clone();
    let (tl, tr, bl, br) = (bg_tl(), bg_tr(), bg_bl(), bg_br());
    let lerp = |a: f32, b: f32, t: f32| a * (1.0 - t) + b * t;
    // Bilinear 4-corner gradient: top-left light slate -> bottom-right deep slate.
    let steps_x = 24;
    let steps_y = 18;
    for gy in 0..steps_y {
        for gx in 0..steps_x {
            let u = (gx as f32 + 0.5) / steps_x as f32;
            let v = (gy as f32 + 0.5) / steps_y as f32;
            let c = Color32::from_rgb(
                lerp(lerp(tl.r() as f32, tr.r() as f32, u), lerp(bl.r() as f32, br.r() as f32, u), v) as u8,
                lerp(lerp(tl.g() as f32, tr.g() as f32, u), lerp(bl.g() as f32, br.g() as f32, u), v) as u8,
                lerp(lerp(tl.b() as f32, tr.b() as f32, u), lerp(bl.b() as f32, br.b() as f32, u), v) as u8,
            );
            let x0 = rect.left() + rect.width() * gx as f32 / steps_x as f32;
            let x1 = rect.left() + rect.width() * (gx + 1) as f32 / steps_x as f32 + 1.0;
            let y0 = rect.top() + rect.height() * gy as f32 / steps_y as f32;
            let y1 = rect.top() + rect.height() * (gy + 1) as f32 / steps_y as f32 + 1.0;
            painter.rect_filled(
                Rect::from_min_max(Pos2::new(x0, y0), Pos2::new(x1, y1)),
                Rounding::ZERO,
                c,
            );
        }
    }
}

/// Soft shadow + white card with rounded corners.
fn card(ui: &mut egui::Ui, size: Vec2, add: impl FnOnce(&mut egui::Ui)) {
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter().clone();
    for (off, alpha) in [(6.0f32, 46u8), (3.0, 28), (1.0, 16)] {
        painter.rect_filled(
            rect.translate(Vec2::new(0.0, off)),
            Rounding::same(18.0),
            Color32::from_rgba_unmultiplied(0x02, 0x06, 0x0c, alpha),
        );
    }
    painter.rect_filled(rect, Rounding::same(18.0), card_fill());
    painter.rect_stroke(rect, Rounding::same(18.0), Stroke::new(1.0f32, card_stroke()));
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(Vec2::new(20.0, 16.0)))
            .layout(Layout::top_down(Align::Min)),
    );
    child.set_min_size(size - Vec2::new(40.0, 32.0));
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
        ui.painter().rect_filled(rect, Rounding::same(h / 2.0), Color32::from_rgba_unmultiplied(0xff, 0xff, 0xff, 30));
    } else {
        let hover = response.hovered();
        let fill = if primary {
            if hover { lighten(accent, 18) } else { accent }
        } else if hover {
            Color32::from_rgba_unmultiplied(0xff, 0xff, 0xff, 34)
        } else {
            card_fill()
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
        if !primary {
            ui.painter().rect_stroke(rect, Rounding::same(h / 2.0), Stroke::new(1.2f32, accent));
        }
        let tc = if primary { Color32::WHITE } else { accent };
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

/// Draw one icon glyph centred in `c`, in a `s` px box.
fn draw_icon(painter: &egui::Painter, c: Pos2, icon: Icon, color: Color32, s: f32) {
    let u = s / 26.0;
    let seg = |a: (f32, f32), b: (f32, f32), w: f32| {
        painter.line_segment(
            [Pos2::new(c.x + a.0 * u, c.y + a.1 * u), Pos2::new(c.x + b.0 * u, c.y + b.1 * u)],
            Stroke::new(w * u, color),
        );
    };
    let rect = |min: (f32, f32), max: (f32, f32), r: f32| {
        painter.rect_stroke(
            Rect::from_min_max(
                Pos2::new(c.x + min.0 * u, c.y + min.1 * u),
                Pos2::new(c.x + max.0 * u, c.y + max.1 * u),
            ),
            Rounding::same(r * u),
            Stroke::new(2.0 * u, color),
        );
    };
    match icon {
        Icon::Chart => {
            // three bars of growing height
            seg((-7.0, 2.0), (-7.0, 7.0), 3.0);
            seg((-1.5, -2.0), (-1.5, 7.0), 3.0);
            seg((4.0, -7.0), (4.0, 7.0), 3.0);
        }
        Icon::Folder => {
            // folder: tab + body
            painter.rect_filled(
                Rect::from_min_max(
                    Pos2::new(c.x - 8.0 * u, c.y - 8.0 * u),
                    Pos2::new(c.x - 1.0 * u, c.y - 4.5 * u),
                ),
                Rounding::same(1.5 * u),
                color,
            );
            rect((-8.0, -5.0), (8.0, 7.0), 2.0);
        }
        Icon::List => {
            seg((-7.0, -5.0), (7.0, -5.0), 2.2);
            seg((-7.0, 0.5), (7.0, 0.5), 2.2);
            seg((-7.0, 6.0), (2.0, 6.0), 2.2);
        }
        Icon::Clock => {
            painter.circle_stroke(c, 8.0 * u, Stroke::new(2.0 * u, color));
            seg((0.0, -4.5), (0.0, 0.0), 2.0);
            seg((0.0, 0.0), (3.5, 2.5), 2.0);
        }
        Icon::Check => {
            painter.circle_stroke(c, 8.0 * u, Stroke::new(2.0 * u, color));
            seg((-3.5, 0.5), (-1.0, 3.5), 2.2);
            seg((-1.0, 3.5), (4.0, -3.0), 2.2);
        }
        Icon::Sliders => {
            for (y, kx) in [(-5.0f32, 3.0f32), (0.5, -3.0), (6.0, 1.5)] {
                seg((-8.0, y), (8.0, y), 2.0);
                painter.circle_filled(
                    Pos2::new(c.x + kx * u, c.y + y * u),
                    2.6 * u,
                    color,
                );
            }
        }
        Icon::Sort => {
            seg((-8.0, -6.0), (8.0, -6.0), 2.4);
            seg((-5.0, 0.0), (5.0, 0.0), 2.4);
            seg((-2.0, 6.0), (2.0, 6.0), 2.4);
        }
    }
}

fn icon_square(ui: &mut egui::Ui, icon: Icon, accent: Color32, active: bool) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(26.0, 26.0), egui::Sense::hover());
    let fill = if active { Color32::WHITE } else { Color32::from_rgba_unmultiplied(0xff, 0xff, 0xff, 44) };
    ui.painter().rect_filled(rect, Rounding::same(8.0), fill);
    draw_icon(
        &ui.painter().clone(),
        rect.center(),
        icon,
        if active { accent } else { Color32::WHITE },
        15.0,
    );
}

fn stat_tile(ui: &mut egui::Ui, title: &str, big: &str, sub: &str, accent: Color32) {
    card(ui, Vec2::new(240.0, 128.0), |ui| {
        ui.label(RichText::new(title).size(12.5).color(text_dim()));
        ui.add_space(8.0);
        ui.label(RichText::new(big).size(30.0).color(accent).strong());
        ui.add_space(4.0);
        ui.label(RichText::new(sub).size(12.0).color(text_primary()));
    });
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

        let screen = ctx.screen_rect();
        egui::CentralPanel::default().frame(egui::Frame::none()).show(ctx, |ui| {
            gradient_bg(ui, screen);

            egui::SidePanel::left("sidebar")
                .resizable(false)
                .exact_width(232.0)
                .frame(egui::Frame::none())
                .show_animated_inside(ui, true, |ui| {
                    self.sidebar(ui);
                });

            egui::TopBottomPanel::bottom("toast")
                .exact_height(44.0)
                .frame(egui::Frame::none())
                .show_inside(ui, |ui| {
                    ui.add_space(8.0);
                    if let Some(t) = &self.toast {
                        ui.label(
                            RichText::new(format!("•  {t}"))
                                .size(12.5)
                                .color(text_dim()),
                        );
                    }
                });

            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .show(ui, |ui| {
                    ui.add_space(24.0);
                    match self.tab {
                        Tab::Dashboard => self.dashboard(ui),
                        Tab::Folders => self.folders_ui(ui),
                        Tab::Rules => self.rules_ui(ui),
                        Tab::History => self.history_ui(ui),
                        Tab::Plan => self.plan_ui(ui),
                        Tab::Settings => self.settings_ui(ui),
                    }
                    ui.add_space(32.0);
                });
        });
    }
}

impl App {
    fn sidebar(&mut self, ui: &mut egui::Ui) {
        let accent = self.accent();
        ui.add_space(24.0);
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(34.0, 34.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, Rounding::same(11.0), accent);
            draw_icon(&ui.painter().clone(), rect.center(), Icon::Sort, Color32::WHITE, 18.0);
            ui.add_space(12.0);
            ui.vertical(|ui| {
                ui.label(RichText::new("OrganizeMyFiles").size(16.0).strong().color(text_primary()));
                ui.label(
                    RichText::new("Toutes les fonctions débloquées")
                        .size(11.0)
                        .color(text_dim()),
                );
            });
        });
        ui.add_space(24.0);

        for (tab, label, icon, _desc) in TABS {
            let active = self.tab == *tab;
            ui.horizontal(|ui| {
                ui.add_space(12.0);
                let id = ui.next_auto_id();
                let (rect, response) =
                    ui.allocate_exact_size(Vec2::new(208.0, 42.0), egui::Sense::click());
                if active {
                    for (off, alpha) in [(5.0f32, 30u8), (2.0, 46)] {
                        ui.painter().rect_filled(
                            rect.translate(Vec2::new(0.0, off)),
                            Rounding::same(14.0),
                            Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), alpha),
                        );
                    }
                    ui.painter().rect_filled(rect, Rounding::same(14.0), accent);
                } else if response.hovered() {
                    ui.painter().rect_filled(
                        rect,
                        Rounding::same(14.0),
                        Color32::from_rgba_unmultiplied(0xff, 0xff, 0xff, 160),
                    );
                }
                let mut child = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(rect.shrink2(Vec2::new(0.0, 7.0)))
                        .layout(Layout::left_to_right(Align::Center)),
                );
                child.set_min_size(rect.size());
                child.add_space(12.0);
                icon_square(&mut child, *icon, accent, active);
                child.add_space(12.0);
                child.label(
                    RichText::new(*label).size(13.5).color(if active { Color32::WHITE } else { text_primary() }),
                );
                let _ = id;
                if response.clicked() {
                    self.tab = *tab;
                }
            });
            ui.add_space(4.0);
        }

        ui.add_space(32.0);
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{} dossiers  •  {} fichiers traités ce mois-ci (sans limites)",
                        self.folders.iter().filter(|f| f.enabled).count(),
                        self.monthly_used,
                    ))
                    .size(10.5)
                    .color(text_dim()),
                );
            });
        });
    }

    fn header(&mut self, ui: &mut egui::Ui, title: &str, desc: &str) {
        ui.label(RichText::new(title).size(26.0).strong().color(text_primary()));
        ui.add_space(4.0);
        ui.label(RichText::new(desc).size(13.0).color(text_dim()));
        ui.add_space(16.0);
    }

    fn dashboard(&mut self, ui: &mut egui::Ui) {
        let accent = self.accent();
        self.header(ui, "Bonjour", "Vos dossiers, rangés par des règles que vous maîtrisez.");

        ui.horizontal(|ui| {
            stat_tile(
                ui,
                "WATCHED FOLDERS",
                &format!("{}", self.folders.iter().filter(|f| f.enabled).count()),
                "unlimited — all folders run",
                accent,
            );
            ui.add_space(16.0);
            stat_tile(
                ui,
                "FILES THIS MONTH",
                &format!("{}", self.monthly_used),
                "processed so far (counter only)",
                accent,
            );
            ui.add_space(16.0);
            stat_tile(
                ui,
                "RULES",
                &format!("{}", self.rules.iter().filter(|r| r.enabled).count()),
                "unlimited rules and actions",
                accent,
            );
        });

        ui.add_space(20.0);
        let run_now = {
            let mut clicked = false;
            card(ui, Vec2::new(756.0, 150.0), |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.add_space(8.0);
                        ui.label(RichText::new("Prêt à organiser").size(18.0).strong().color(text_primary()));
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Les règles s'exécutent de haut en bas, les actions en ordre. L'aperçu (simulation) ne touche à aucun fichier.")
                                .size(12.5)
                                .color(text_dim()),
                        );
                    });
                });
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    if pill(ui, "Organiser", accent, true, true).clicked() {
                        clicked = true;
                    }
                    ui.add_space(12.0);
                    if pill(ui, "Aperçu (simulation)", accent, false, true).clicked() {
                        clicked = false;
                        self.run_batch(true);
                    }
                });
            });
            clicked
        };
        if run_now {
            self.run_batch(false);
        }

        ui.add_space(20.0);
        let used = self.monthly_used;
        card(ui, Vec2::new(756.0, 128.0), |ui| {
            ui.label(RichText::new("Activité de ce mois").size(15.0).strong().color(text_primary()));
            ui.add_space(8.0);
            let frac: f32 = 0.08;
            let (rect, _) = ui.allocate_exact_size(Vec2::new(688.0, 14.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, Rounding::same(7.0), Color32::from_rgba_unmultiplied(0xff, 0xff, 0xff, 32));
            let mut fill = rect;
            fill.set_width((rect.width() * frac.min(1.0)).max(8.0));
            ui.painter().rect_filled(fill, Rounding::same(7.0), accent);
            ui.add_space(8.0);
            ui.label(
                RichText::new(format!(
                    "{used} fichier(s) traité(s) — clé de mois {}. Aucune limite : c'est une simple statistique.",
                    month_key(SystemTime::now())
                ))
                .size(11.5)
                .color(text_dim()),
            );
        });

        if let Some((matched, actions, scanned)) = self.last_run {
            ui.add_space(20.0);
            card(ui, Vec2::new(756.0, 96.0), |ui| {
                ui.label(RichText::new("Dernier lot").size(15.0).strong().color(text_primary()));
                ui.add_space(8.0);
                ui.label(
                    RichText::new(format!(
                        "{matched} fichier(s) correspondant sur {scanned} analysés, {actions} action(s) enregistrée(s)"
                    ))
                    .size(12.5)
                    .color(text_dim()),
                );
            });
        }
    }

    fn folders_ui(&mut self, ui: &mut egui::Ui) {
        let accent = self.accent();
        self.header(ui, "Dossiers suivis", "Les lieux où les règles s'exécutent, par ordre de priorité.");

        let mut remove: Option<usize> = None;
        let mut move_up: Option<usize> = None;
        let mut move_down: Option<usize> = None;

        for i in 0..self.folders.len() {
            let f = self.folders[i].clone();
            card(ui, Vec2::new(756.0, 86.0), |ui| {
                ui.horizontal(|ui| {
                    icon_square(ui, Icon::Folder, accent, true);
                    ui.add_space(12.0);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(f.path.display().to_string())
                                .size(14.0)
                                .strong()
                                .color(text_primary()),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(if f.enabled {
                                "Active — rules run here"
                            } else {
                                "Paused"
                            })
                            .size(11.5)
                            .color(text_dim()),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("Retirer").clicked() {
                            remove = Some(i);
                        }
                        if ui.small_button("Descendre").clicked() {
                            move_down = Some(i);
                        }
                        if ui.small_button("Monter").clicked() {
                            move_up = Some(i);
                        }
                        let mut en = f.enabled;
                        if ui.checkbox(&mut en, "Suivi").changed() {
                            self.folders[i].enabled = en;
                        }
                    });
                });
            });
            ui.add_space(12.0);
        }

        if let Some(i) = move_up {
            if i > 0 {
                self.folders.swap(i, i - 1);
            }
        }
        if let Some(i) = move_down {
            if i + 1 < self.folders.len() {
                self.folders.swap(i, i + 1);
            }
        }
        if let Some(i) = remove {
            let id = self.folders[i].id;
            self.rules.retain(|r| r.folder_id != id);
            self.folders.remove(i);
            self.toast("Dossier retiré");
        }
        for (i, f) in self.folders.iter_mut().enumerate() {
            f.priority = i as i64;
        }

        ui.add_space(8.0);
        card(ui, Vec2::new(756.0, 104.0), |ui| {
            ui.label(RichText::new("Ajouter un dossier").size(15.0).strong().color(text_primary()));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.add_folder_text)
                        .desired_width(520.0)
                        .hint_text("~/Downloads — chemin complet ou ~/…"),
                );
                ui.add_space(12.0);
                let add_clicked = pill(ui, "Ajouter", accent, true, false).clicked();
                if add_clicked {
                    let text = self.add_folder_text.trim().to_string();
                    if !text.is_empty() {
                        let path = PathBuf::from(shim_expand(&text));
                        self.folders.push(Folder {
                            id: self.next_id,
                            path,
                            enabled: true,
                            priority: self.folders.len() as i64,
                        });
                        self.next_id += 1;
                        self.add_folder_text.clear();
                        self.toast("Dossier ajouté");
                    }
                }
            });
        });
    }

    fn rules_ui(&mut self, ui: &mut egui::Ui) {
        let accent = self.accent();
        self.header(ui, "Règles", "Si ces conditions correspondent, exécutez ces actions dans l'ordre.");

        if self.folders.is_empty() {
            ui.label(RichText::new("Ajoutez d'abord un dossier suivi, puis créez-y des règles.").color(text_dim()));
            return;
        }

        ui.horizontal(|ui| {
            ui.label(RichText::new("Dossier").size(12.0).color(text_dim()));
            ui.add_space(8.0);
            let names: Vec<String> = self
                .folders
                .iter()
                .map(|f| f.path.display().to_string())
                .collect();
            let cur = names
                .get(self.selected_folder.min(names.len() - 1))
                .cloned()
                .unwrap_or_default();
            if let Some(pick) = combo(ui, "folder-pick", &shorten(&cur), &names.iter().map(|s| shorten(s)).collect::<Vec<_>>()) {
                if let Some(idx) = names.iter().position(|s| shorten(s) == pick) {
                    self.selected_folder = idx;
                    self.selected_rule = 0;
                }
            }
        });
        ui.add_space(16.0);

        let folder_id = self.folders[self.selected_folder.min(self.folders.len() - 1)].id;
        let mut rule_ids: Vec<u64> = self
            .rules
            .iter()
            .filter(|r| r.folder_id == folder_id)
            .map(|r| r.id)
            .collect();
        rule_ids.sort_by_key(|id| {
            self.rules.iter().find(|r| r.id == *id).map(|r| (r.priority, r.id)).unwrap_or((0, 0))
        });

        ui.horizontal(|ui| {
            for (i, rid) in rule_ids.iter().enumerate() {
                let name = self.rules.iter().find(|r| r.id == *rid).map(|r| r.name.clone()).unwrap_or_default();
                let active = self.selected_rule == i;
                if pill(ui, &name, accent, active, false).clicked() {
                    self.selected_rule = i;
                }
                ui.add_space(8.0);
            }
            if pill(ui, "+ Nouvelle règle", accent, false, false).clicked() {
                let id = self.next_id;
                self.next_id += 1;
                self.rules.push(Rule {
                    id,
                    folder_id,
                    name: format!("Rule {}", rule_ids.len() + 1),
                    enabled: true,
                    match_all: true,
                    recursion_depth: 0,
                    priority: rule_ids.len() as i64,
                    conditions: vec![Condition {
                        field: Field::Extension,
                        op: Op::Equals,
                        value: String::new(),
                    }],
                    actions: vec![Action::new(ActionKind::Move)],
                });
                self.selected_rule = rule_ids.len();
            }
        });
        ui.add_space(16.0);

        if rule_ids.is_empty() {
            ui.label(RichText::new("Aucune règle pour l'instant — créez-en une ci-dessus.").color(text_dim()));
            return;
        }

        let sel = self.selected_rule.min(rule_ids.len() - 1);
        let rid = rule_ids[sel];

        // ---- rule header card
        card(ui, Vec2::new(756.0, 122.0), |ui| {
            let idx = self.rules.iter().position(|r| r.id == rid).unwrap();
            ui.horizontal(|ui| {
                ui.label(RichText::new("Nom").size(12.0).color(text_dim()));
                ui.add_space(8.0);
                let mut name = self.rules[idx].name.clone();
                if ui.add(egui::TextEdit::singleline(&mut name).desired_width(320.0)).changed() {
                    self.rules[idx].name = name;
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.small_button("Supprimer la règle").clicked() {
                        self.rules.retain(|r| r.id != rid);
                        return;
                    }
                    let mut en = self.rules[idx].enabled;
                    if ui.checkbox(&mut en, "Activée").changed() {
                        self.rules[idx].enabled = en;
                    }
                });
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let all = self.rules[idx].match_all;
                if ui.selectable_label(all, "TOUTES les conditions").clicked() {
                    self.rules[idx].match_all = true;
                }
                if ui.selectable_label(!all, "AU MOINS UNE condition").clicked() {
                    self.rules[idx].match_all = false;
                }
                ui.add_space(16.0);
                ui.label(RichText::new("Profondeur de récursion").size(12.0).color(text_dim()));
                let mut depth = self.rules[idx].recursion_depth;
                if ui.add(egui::Slider::new(&mut depth, 0..=8).logarithmic(false)).changed() {
                    self.rules[idx].recursion_depth = depth;
                }
            });
            ui.label(
                RichText::new("0 = fichiers de premier niveau seulement ; n descend de n niveaux.")
                    .size(11.0)
                    .color(text_dim()),
            );
        });
        ui.add_space(16.0);

        if self.rules.iter().all(|r| r.id != rid) {
            return;
        }
        let idx = self.rules.iter().position(|r| r.id == rid).unwrap();

        // ---- conditions
        card(ui, Vec2::new(756.0, 86.0 + 52.0 * self.rules[idx].conditions.len() as f32), |ui| {
            ui.label(RichText::new("Conditions").size(15.0).strong().color(text_primary()));
            ui.add_space(8.0);
            let mut drop: Option<usize> = None;
            for ci in 0..self.rules[idx].conditions.len() {
                ui.horizontal(|ui| {
                    let field = self.rules[idx].conditions[ci].field;
                    let flabel = field.label();
                    if let Some(pick) = combo(
                        ui,
                        &format!("cf-{idx}-{ci}"),
                        flabel,
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
                        Field::Size => "10 MB / 512 KB",
                        Field::DateAdded | Field::DateModified => "2024-01-31  or  30 days",
                        Field::Kind => "document / image / video / audio / archive / folder",
                        Field::Contents => "text to find (documents; images need OCR)",
                        _ => "value",
                    };
                    let mut val = self.rules[idx].conditions[ci].value.clone();
                    if ui
                        .add(egui::TextEdit::singleline(&mut val).desired_width(220.0).hint_text(hint))
                        .changed()
                    {
                        self.rules[idx].conditions[ci].value = val;
                    }
                    if ui.small_button("✕").clicked() {
                        drop = Some(ci);
                    }
                });
                ui.add_space(8.0);
            }
            if let Some(ci) = drop {
                self.rules[idx].conditions.remove(ci);
            }
            if pill(ui, "+ Ajouter une condition", accent, false, false).clicked() {
                self.rules[idx].conditions.push(Condition {
                    field: Field::Name,
                    op: Op::Contains,
                    value: String::new(),
                });
            }
        });
        ui.add_space(16.0);

        // ---- actions
        let extra = self.rules[idx]
            .actions
            .iter()
            .map(|a| 30.0 + 34.0 * a.kind.params().len() as f32)
            .sum::<f32>();
        card(ui, Vec2::new(756.0, 86.0 + extra), |ui| {
            ui.label(RichText::new("Actions — exécutées en ordre, chacune voit le fichier laissé par la précédente").size(15.0).strong().color(text_primary()));
            ui.add_space(8.0);
            let mut drop: Option<usize> = None;
            for ai in 0..self.rules[idx].actions.len() {
                ui.horizontal(|ui| {
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
                    if kind.requires_pro() {
                        ui.label(
                            RichText::new("avancé")
                                .size(10.5)
                                .color(accent),
                        );
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("✕").clicked() {
                            drop = Some(ai);
                        }
                        if ui.small_button("▼").clicked() && ai + 1 < self.rules[idx].actions.len() {
                            self.rules[idx].actions.swap(ai, ai + 1);
                        }
                        if ui.small_button("▲").clicked() && ai > 0 {
                            self.rules[idx].actions.swap(ai, ai - 1);
                        }
                    });
                });
                for (k, placeholder) in self.rules[idx].actions[ai].kind.params() {
                    ui.horizontal(|ui| {
                        ui.add_space(150.0);
                        let entry = self.rules[idx].actions[ai].params.entry((*k).to_string()).or_default();
                        ui.add(
                            egui::TextEdit::singleline(entry)
                                .desired_width(380.0)
                                .hint_text(*placeholder),
                        );
                    });
                    ui.add_space(4.0);
                }
                ui.add_space(8.0);
            }
            if let Some(ai) = drop {
                self.rules[idx].actions.remove(ai);
            }
            if pill(ui, "+ Ajouter une action", accent, false, false).clicked() {
                self.rules[idx].actions.push(Action::new(ActionKind::Rename));
            }
        });
    }

    fn history_ui(&mut self, ui: &mut egui::Ui) {
        let accent = self.accent();
        self.header(ui, "Historique", "Chaque action d'un lot — celles réversibles peuvent être annulées.");

        if self.history.is_empty() {
            ui.label(RichText::new("Rien pour l'instant. Lancez un lot ou un aperçu pour le voir ici.").color(text_dim()));
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
                    .size(12.0)
                    .strong()
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
                                .size(13.0)
                                .strong()
                                .color(text_primary()),
                        );
                        ui.label(
                            RichText::new(format!(
                                "{}  →  {}  ·  {}",
                                shorten(&e.original_path.display().to_string()),
                                shorten(&e.result_path.display().to_string()),
                                e.message
                            ))
                            .size(11.0)
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
                                    .size(11.0)
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
                    "Affichage des {HISTORY_PAGE_SIZE} dernières entrées sur {total}."
                ))
                .size(11.0)
                .color(text_dim()),
            );
        }
        if let Some(i) = undo_idx {
            self.undo(i);
        }
    }

    fn plan_ui(&mut self, ui: &mut egui::Ui) {
        let accent = self.accent();
        self.header(ui, "Capacités", "Toutes les capacités sont disponibles — aucun compte ni forfait à activer.");

        ui.horizontal(|ui| {
            card(ui, Vec2::new(366.0, 300.0), |ui| {
                ui.label(RichText::new("Organisation").size(20.0).strong().color(text_primary()));
                ui.add_space(4.0);
                ui.label(RichText::new("Toujours disponible").size(12.0).color(text_dim()));
                ui.add_space(12.0);
                for line in [
                    "• dossiers suivis illimités (par ordre de priorité)",
                    "• règles et actions illimitées",
                    "• correspondance sur le texte des documents",
                    "• aperçus (simulation)",
                    "• historique avec annulation",
                ] {
                    ui.label(RichText::new(line).size(12.5).color(text_primary()));
                    ui.add_space(3.0);
                }
                ui.add_space(12.0);
                ui.label(
                    RichText::new("Les limites documentées du plan gratuit de l'application d'origine (2 dossiers, 300 fichiers par mois) figurent dans MECHANICS.md mais ne sont pas appliquées ici.")
                        .size(11.0)
                        .color(text_dim()),
                );
            });
            ui.add_space(16.0);
            card(ui, Vec2::new(366.0, 300.0), |ui| {
                ui.label(RichText::new("Avancé").size(20.0).strong().color(accent));
                ui.add_space(4.0);
                ui.label(RichText::new("Tout est débloqué :").size(12.0).color(text_dim()));
                ui.add_space(12.0);
                for (title, sub) in [
                    ("Dossiers et fichiers illimités", "Aucun plafond mensuel, autant de dossiers suivis que souhaité."),
                    ("Import vers les médiathèques", "Envoyez directement les fichiers correspondants dans vos médiathèques photos, musique et vidéos."),
                    ("Reconnaissance de texte (OCR)", "Classez les PDF numérisés et les images selon les mots qu'ils contiennent — lu localement sur votre ordinateur."),
                ] {
                    ui.label(RichText::new(format!("• {title}")).size(12.5).strong().color(text_primary()));
                    ui.label(RichText::new(sub).size(11.5).color(text_dim()));
                    ui.add_space(8.0);
                }
                ui.add_space(8.0);
                pill(ui, "Toutes les fonctions actives", accent, true, false);
            });
        });

        ui.add_space(16.0);
        card(ui, Vec2::new(756.0, 96.0), |ui| {
            ui.label(RichText::new("Ce mois-ci").size(15.0).strong().color(text_primary()));
            ui.add_space(8.0);
            ui.label(
                RichText::new(format!(
                    "{} fichier(s) traité(s) jusqu'ici ({}). Statistique pure — aucune limite.",
                    self.monthly_used,
                    month_key(SystemTime::now())
                ))
                .size(12.5)
                .color(text_dim()),
            );
        });
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) {
        let accent = self.accent();
        self.header(ui, "Réglages", "Petits réglages, appliqués immédiatement.");

        card(ui, Vec2::new(756.0, 132.0), |ui| {
            ui.label(RichText::new("Couleur d'accent").size(15.0).strong().color(text_primary()));
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                for (i, (name, c)) in ACCENTS.iter().enumerate() {
                    let (rect, resp) = ui.allocate_exact_size(Vec2::new(34.0, 34.0), egui::Sense::click());
                    ui.painter().rect_filled(rect, Rounding::same(11.0), *c);
                    if self.accent == i {
                        ui.painter().rect_stroke(rect, Rounding::same(11.0), Stroke::new(2.5f32, Color32::WHITE));
                        ui.painter().rect_stroke(
                            rect.expand(3.0),
                            Rounding::same(14.0),
                            Stroke::new(1.5f32, *c),
                        );
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
            ui.label(RichText::new("Nommé d'après la palette d'origine : bleu, violet, rose, orange, jaune, vert, graphite.").size(11.0).color(text_dim()));
        });

        ui.add_space(16.0);
        card(ui, Vec2::new(756.0, 168.0), |ui| {
            ui.checkbox(&mut self.menu_bar, "Afficher l'application dans la barre des menus");
            ui.add_space(8.0);
            ui.checkbox(&mut self.notify, "Prévenir quand le surveillant organise des fichiers");
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Conserver l'historique pendant").size(12.5).color(text_primary()));
                ui.add_space(8.0);
                ui.add(egui::Slider::new(&mut self.retention_days, 1..=365).suffix(" jours").logarithmic(true));
            });
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!(
                    "Les entrées plus anciennes sont purgées automatiquement (réglage history_max_days).",
                ))
                .size(11.0)
                .color(text_dim()),
            );
            ui.label(
                RichText::new(format!("Clé de réglage : {HISTORY_RETENTION_SETTING_KEY} — plage 1 à 365 jours"))
                    .size(11.0)
                    .color(text_dim()),
            );
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new(RichText::new("Purger l'historique plus ancien").size(12.0))
                            .fill(Color32::from_rgba_unmultiplied(0xff, 0xff, 0xff, 34)),
                    )
                    .clicked()
                {
                    let removed = purge_history(&mut self.history, self.retention_days, SystemTime::now());
                    self.toast(format!("{removed} entrée(s) purgée(s)"));
                }
            });
        });

        ui.add_space(16.0);
        card(ui, Vec2::new(756.0, 92.0), |ui| {
            ui.label(RichText::new("À propos des chiffres").size(15.0).strong().color(text_primary()));
            ui.add_space(8.0);
            ui.label(
                RichText::new(format!(
                    "Constantes documentées (application d'origine, non appliquées ici) : {} fichiers / mois et {} dossiers (limites du forfait), {} Mo d'extraction de texte, {} Mo d'OCR, noms de {} caractères. Adresses sources complètes dans MECHANICS.md.",
                    MONTHLY_FILE_LIMIT,
                    FREE_FOLDER_LIMIT,
                    TEXT_LIMIT_BYTES / 1024 / 1024,
                    OCR_LIMIT_BYTES / 1024 / 1024,
                    FILENAME_MAX_LENGTH
                ))
                .size(11.5)
                .color(text_dim()),
            );
        });
        let _ = accent;
    }
}

fn shorten(p: &str) -> String {
    let home = home_dir();
    let p = p.strip_prefix(&home).unwrap_or(p);
    let p = p.trim_start_matches('/');
    if p.len() > 44 {
        format!("…{}", &p[p.len() - 44..])
    } else if p.is_empty() {
        "~".into()
    } else {
        format!("~/{p}")
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
