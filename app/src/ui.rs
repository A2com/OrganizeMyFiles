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
    Funnel,
    Bolt,
    Grip,
    Bell,
    Gear,
    Search,
}

const TABS: &[(Tab, &str, Icon, &str)] = &[
    (Tab::Dashboard, "Aperçu", Icon::Chart, "Tout en un coup d'œil"),
    (Tab::Folders, "Dossiers suivis", Icon::Folder, "Les lieux à organiser"),
    (Tab::Rules, "Règles", Icon::List, "Si ceci, alors cela"),
    (Tab::History, "Historique", Icon::Clock, "Ce qui s'est passé, annulable"),
    (Tab::Plan, "Capacités", Icon::Check, "Tout est débloqué — sans compte"),
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
const PANEL: Color32 = Color32::from_rgb(0x11, 0x13, 0x1a);
const CONTENT: Color32 = Color32::from_rgb(0x16, 0x18, 0x21);
const CARD: Color32 = Color32::from_rgb(0x19, 0x1b, 0x23);
const CARD_HI: Color32 = Color32::from_rgb(0x1f, 0x22, 0x2d);
const TAB_ACTIVE: Color32 = Color32::from_rgb(0x24, 0x28, 0x35);
const SKY: Color32 = Color32::from_rgb(0x38, 0xbd, 0xf8);
const SKY_DEEP: Color32 = Color32::from_rgb(0x02, 0x84, 0xc7);
const GREEN: Color32 = Color32::from_rgb(0x27, 0xc9, 0x3f);

fn card_fill() -> Color32 {
    CARD
}
fn card_stroke() -> Color32 {
    Color32::from_rgba_unmultiplied(0xff, 0xff, 0xff, 15)
}
fn field_stroke() -> Color32 {
    Color32::from_rgba_unmultiplied(0xff, 0xff, 0xff, 26)
}
fn text_primary() -> Color32 {
    Color32::from_rgb(0xf1, 0xf5, 0xf9)
}
fn text_dim() -> Color32 {
    Color32::from_rgb(0x94, 0xa3, 0xb8)
}
fn with_alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

/// Applique le style sombre global : champs `#0c0e15`, focus orange, coins doux.
fn apply_theme(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = CONTENT;
    v.window_fill = CARD_HI;
    v.extreme_bg_color = CANVAS; // fond des champs de saisie
    v.faint_bg_color = CARD;
    v.override_text_color = Some(text_primary());
    v.selection.bg_fill = with_alpha(ACCENTS[0].1, 90);
    v.selection.stroke = Stroke::new(1.0, ACCENTS[0].1);
    let r = Rounding::same(8.0);
    for w in [&mut v.widgets.noninteractive, &mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
        w.rounding = r;
    }
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, card_stroke());
    v.widgets.inactive.bg_fill = CANVAS;
    v.widgets.inactive.weak_bg_fill = CANVAS;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, field_stroke());
    v.widgets.hovered.bg_fill = CARD_HI;
    v.widgets.hovered.weak_bg_fill = CARD_HI;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, with_alpha(ACCENTS[0].1, 120));
    v.widgets.active.bg_stroke = Stroke::new(1.0, ACCENTS[0].1);
    ctx.set_visuals(v);
    ctx.style_mut(|s| {
        s.spacing.scroll = egui::style::ScrollStyle::thin();
        s.spacing.item_spacing = Vec2::new(8.0, 6.0);
    });
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
    app_enabled: bool,
    search: String,
    toast: Option<String>,
    last_run: Option<(usize, usize, usize)>, // matched, actions, scanned
    frame: u32,
    screenshot: Option<String>,
    shot_done: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, screenshot: Option<String>) -> Self {
        apply_theme(&cc.egui_ctx);
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
            app_enabled: true,
            search: String::new(),
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

fn flat_bg(ui: &mut egui::Ui, rect: Rect) {
    ui.painter().rect_filled(rect, Rounding::ZERO, CONTENT);
}

/// Soft shadow + white card with rounded corners.
fn card(ui: &mut egui::Ui, size: Vec2, add: impl FnOnce(&mut egui::Ui)) {
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter().clone();
    for (off, alpha) in [(6.0f32, 46u8), (3.0, 28), (1.0, 16)] {
        painter.rect_filled(
            rect.translate(Vec2::new(0.0, off)),
            Rounding::same(12.0),
            Color32::from_rgba_unmultiplied(0x02, 0x06, 0x0c, alpha),
        );
    }
    painter.rect_filled(rect, Rounding::same(12.0), card_fill());
    painter.rect_stroke(rect, Rounding::same(12.0), Stroke::new(1.0f32, card_stroke()));
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
        Icon::Funnel => {
            for (a, b) in [
                ((-8.0, -6.0), (8.0, -6.0)),
                ((-8.0, -6.0), (-2.0, 1.0)),
                ((8.0, -6.0), (2.0, 1.0)),
                ((-2.0, 1.0), (-2.0, 7.0)),
                ((2.0, 1.0), (2.0, 5.0)),
                ((-2.0, 7.0), (2.0, 5.0)),
            ] {
                seg(a, b, 2.0);
            }
        }
        Icon::Bolt => {
            let pts = [(2.0, -8.0), (-4.0, 1.0), (1.0, 1.0), (-2.0, 8.0), (5.0, -2.0), (0.0, -2.0), (2.0, -8.0)];
            for w in pts.windows(2) {
                seg(w[0], w[1], 2.0);
            }
        }
        Icon::Grip => {
            for x in [-2.5f32, 2.5] {
                for y in [-5.0f32, 0.0, 5.0] {
                    painter.circle_filled(Pos2::new(c.x + x * u, c.y + y * u), 1.4 * u, color);
                }
            }
        }
        Icon::Bell => {
            painter.circle_stroke(Pos2::new(c.x, c.y - 1.5 * u), 5.5 * u, Stroke::new(2.0 * u, color));
            seg((-7.5, 4.0), (7.5, 4.0), 2.0);
            painter.circle_filled(Pos2::new(c.x, c.y + 7.0 * u), 1.5 * u, color);
        }
        Icon::Gear => {
            painter.circle_stroke(c, 3.0 * u, Stroke::new(2.0 * u, color));
            painter.circle_stroke(c, 7.0 * u, Stroke::new(2.0 * u, color));
            for (a, b) in [((0.0, -10.0), (0.0, -7.0)), ((0.0, 7.0), (0.0, 10.0)), ((-10.0, 0.0), (-7.0, 0.0)), ((7.0, 0.0), (10.0, 0.0))] {
                seg(a, b, 2.4);
            }
        }
        Icon::Search => {
            painter.circle_stroke(Pos2::new(c.x - 2.0 * u, c.y - 2.0 * u), 5.5 * u, Stroke::new(2.0 * u, color));
            seg((2.0, 2.0), (7.5, 7.5), 2.2);
        }
        Icon::Sort => {
            seg((-8.0, -6.0), (8.0, -6.0), 2.4);
            seg((-5.0, 0.0), (5.0, 0.0), 2.4);
            seg((-2.0, 6.0), (2.0, 6.0), 2.4);
        }
    }
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
    let font = egui::FontId::proportional(12.5);
    let galley = ui.painter().layout_no_wrap(text.to_string(), font.clone(), Color32::WHITE);
    let icon_w = if icon.is_some() { 20.0 } else { 0.0 };
    let size = Vec2::new(galley.size().x + 28.0 + icon_w, 32.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let enabled = resp.enabled();
    let hover = resp.hovered() && enabled;
    let (fill, stroke, fg) = match kind {
        Btn::Primary => (if hover { lighten(accent, 16) } else { accent }, Color32::TRANSPARENT, Color32::WHITE),
        Btn::Ghost => (
            if hover { Color32::from_rgba_unmultiplied(0xff, 0xff, 0xff, 14) } else { Color32::TRANSPARENT },
            field_stroke(),
            text_primary(),
        ),
        Btn::Sky => (with_alpha(SKY, if hover { 40 } else { 22 }), with_alpha(SKY, 100), SKY),
        Btn::Danger => (
            with_alpha(Color32::from_rgb(0xf8, 0x71, 0x71), if hover { 36 } else { 0 }),
            with_alpha(Color32::from_rgb(0xf8, 0x71, 0x71), 90),
            Color32::from_rgb(0xf8, 0x71, 0x71),
        ),
    };
    let (fill, fg) = if enabled { (fill, fg) } else { (with_alpha(Color32::WHITE, 12), with_alpha(Color32::WHITE, 70)) };
    let p = ui.painter();
    if kind == Btn::Primary && enabled {
        p.rect_filled(rect.expand(3.0), Rounding::same(10.0), with_alpha(accent, 40));
    }
    p.rect_filled(rect, Rounding::same(8.0), fill);
    if stroke != Color32::TRANSPARENT {
        p.rect_stroke(rect, Rounding::same(8.0), Stroke::new(1.0, stroke));
    }
    let mut x = rect.left() + 14.0;
    if let Some(ic) = icon {
        draw_icon(p, Pos2::new(x + 7.0, rect.center().y), ic, fg, 15.0);
        x += icon_w;
    }
    p.text(Pos2::new(x, rect.center().y), egui::Align2::LEFT_CENTER, text, font, fg);
    resp
}

/// Pastille d'information (22 px) : badge compteur, version, statut.
fn chip(ui: &mut egui::Ui, text: &str, fg: Color32, bg: Color32, stroke: Color32) -> egui::Response {
    let font = egui::FontId::proportional(11.5);
    let galley = ui.painter().layout_no_wrap(text.to_string(), font, fg);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(galley.size().x + 18.0, 22.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, Rounding::same(11.0), bg);
    ui.painter().rect_stroke(rect, Rounding::same(11.0), Stroke::new(1.0, stroke));
    let pos = Pos2::new(rect.left() + 9.0, rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(pos, galley, fg);
    resp
}

/// Interrupteur façon macOS.
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
    let font = egui::FontId::proportional(12.0);
    let widths: Vec<f32> = labels
        .iter()
        .map(|l| ui.painter().layout_no_wrap(l.to_string(), font.clone(), Color32::WHITE).size().x + 28.0)
        .collect();
    let total = widths.iter().sum::<f32>() + 8.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(total, 34.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, Rounding::same(9.0), CANVAS);
    ui.painter().rect_stroke(rect, Rounding::same(9.0), Stroke::new(1.0, field_stroke()));
    let mut x = rect.left() + 4.0;
    let mut out = None;
    for (i, label) in labels.iter().enumerate() {
        let seg = Rect::from_min_size(Pos2::new(x, rect.top() + 4.0), Vec2::new(widths[i], 26.0));
        let resp = ui.interact(seg, id.with(i), egui::Sense::click());
        let on = i == selected;
        if on {
            ui.painter().rect_filled(seg, Rounding::same(6.0), active);
        } else if resp.hovered() {
            ui.painter().rect_filled(seg, Rounding::same(6.0), CARD_HI);
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

/// Carte de section pleine largeur.
fn panel_card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::none()
        .fill(CARD)
        .rounding(Rounding::same(12.0))
        .stroke(Stroke::new(1.0f32, card_stroke()))
        .inner_margin(egui::Margin::same(18.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

/// Ligne imbriquée dans une carte (condition ou action).
fn row_frame<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> egui::InnerResponse<R> {
    egui::Frame::none()
        .fill(CARD_HI)
        .rounding(Rounding::same(10.0))
        .stroke(Stroke::new(1.0f32, card_stroke()))
        .inner_margin(egui::Margin::symmetric(12.0, 10.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
}

fn caps(text: &str) -> RichText {
    RichText::new(text).size(10.0).strong().color(Color32::from_rgb(0x64, 0x74, 0x8b))
}

fn rule_tab(ui: &mut egui::Ui, label: &str, active: bool, enabled: bool, accent: Color32) -> egui::Response {
    let font = egui::FontId::proportional(12.5);
    let fg = if active { Color32::WHITE } else if enabled { text_dim() } else { Color32::from_rgb(0x64, 0x74, 0x8b) };
    let galley = ui.painter().layout_no_wrap(label.to_string(), font.clone(), fg);
    let lead = if active { 30.0 } else { 16.0 };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(galley.size().x + lead + 16.0, 34.0), egui::Sense::click());
    let fill = if active { TAB_ACTIVE } else if resp.hovered() { CARD_HI } else { CARD };
    let stroke = if active { with_alpha(accent, 77) } else { card_stroke() };
    ui.painter().rect_filled(rect, Rounding::same(9.0), fill);
    ui.painter().rect_stroke(rect, Rounding::same(9.0), Stroke::new(1.0f32, stroke));
    if active {
        ui.painter().circle_filled(Pos2::new(rect.left() + 16.0, rect.center().y), 3.5, accent);
    }
    ui.painter().text(Pos2::new(rect.left() + lead, rect.center().y), egui::Align2::LEFT_CENTER, label, font, fg);
    resp
}

fn section_title(ui: &mut egui::Ui, icon: Icon, title: &str) {
    let (r, _) = ui.allocate_exact_size(Vec2::splat(20.0), egui::Sense::hover());
    draw_icon(&ui.painter().clone(), r.center(), icon, SKY, 18.0);
    ui.label(RichText::new(title).size(15.0).strong().color(Color32::WHITE));
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
            flat_bg(ui, screen);

            egui::TopBottomPanel::top("topbar")
                .exact_height(56.0)
                .frame(egui::Frame::none().fill(PANEL).stroke(Stroke::new(1.0f32, card_stroke())))
                .show_inside(ui, |ui| {
                    self.top_bar(ui);
                });

            egui::SidePanel::left("sidebar")
                .resizable(false)
                .exact_width(256.0)
                .frame(egui::Frame::none().fill(PANEL).stroke(Stroke::new(1.0f32, card_stroke())))
                .show_inside(ui, |ui| {
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
                    egui::Frame::none().inner_margin(egui::Margin::symmetric(28.0, 0.0)).show(ui, |ui| {
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
        });
    }
}

impl App {
    fn top_bar(&mut self, ui: &mut egui::Ui) {
        let accent = self.accent();
        let ctx = ui.ctx().clone();
        let rect = ui.max_rect();
        let wide = rect.width() >= 1100.0;

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

        let search_w = (rect.width() * 0.24).clamp(190.0, 300.0);
        let search_rect = Rect::from_center_size(rect.center(), Vec2::new(search_w, 34.0));

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
        let (sep, _) = lui.allocate_exact_size(Vec2::new(1.0, 22.0), egui::Sense::hover());
        lui.painter().rect_filled(sep, Rounding::ZERO, card_stroke());
        lui.add_space(12.0);
        let (logo, _) = lui.allocate_exact_size(Vec2::splat(22.0), egui::Sense::hover());
        draw_icon(&lui.painter().clone(), logo.center(), Icon::Folder, accent, 22.0);
        lui.add_space(6.0);
        lui.label(RichText::new("OrganizeMyFiles").size(15.0).strong().color(Color32::WHITE));
        if wide {
            lui.add_space(8.0);
            let state = if self.app_enabled { "Actif" } else { "En veille" };
            chip(
                &mut lui,
                &format!("V0.1 • {state}"),
                text_dim(),
                Color32::from_rgba_unmultiplied(0xff, 0xff, 0xff, 12),
                card_stroke(),
            );
        }

        // --- centre : recherche
        ui.painter().rect_filled(search_rect, Rounding::same(8.0), with_alpha(CANVAS, 180));
        ui.painter().rect_stroke(search_rect, Rounding::same(8.0), Stroke::new(1.0f32, field_stroke()));
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
                .hint_text("Rechercher des règles, dossiers, actions..."),
        );
        let key = if cfg!(target_os = "macos") { "⌘K" } else { "Ctrl K" };
        ui.painter().text(
            Pos2::new(search_rect.right() - 12.0, search_rect.center().y),
            egui::Align2::RIGHT_CENTER,
            key,
            egui::FontId::proportional(11.0),
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
                rui.painter().rect_filled(r, Rounding::same(8.0), with_alpha(Color32::WHITE, 14));
            }
            draw_icon(&rui.painter().clone(), r.center(), Icon::Gear, text_dim(), 17.0);
            if resp.on_hover_text("Réglages").clicked() {
                self.tab = Tab::Settings;
            }
            let (r, resp) = rui.allocate_exact_size(Vec2::splat(32.0), egui::Sense::click());
            if resp.hovered() {
                rui.painter().rect_filled(r, Rounding::same(8.0), with_alpha(Color32::WHITE, 14));
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
        let run = rui.add_enabled_ui(enabled, |ui| btn(ui, "Organiser", Btn::Primary, accent, Some(Icon::Bolt))).inner;
        if run.clicked() {
            self.run_batch(false);
        }
        if btn(&mut rui, "Aperçu simulation", Btn::Ghost, accent, None).clicked() {
            self.run_batch(true);
        }
        // Statut : pastille verte pulsée.
        let label = if self.app_enabled { "Actif" } else { "En veille" };
        let dot = if self.app_enabled { GREEN } else { text_dim() };
        let (r, resp) = rui.allocate_exact_size(Vec2::new(82.0, 32.0), egui::Sense::click());
        rui.painter().rect_filled(r, Rounding::same(16.0), with_alpha(dot, if resp.hovered() { 36 } else { 22 }));
        rui.painter().rect_stroke(r, Rounding::same(16.0), Stroke::new(1.0f32, with_alpha(dot, 80)));
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
            egui::FontId::proportional(12.5),
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
        let Some(folder_id) = folder_id else {
            self.toast("Ajoutez d'abord un dossier suivi.");
            return;
        };
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

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        let accent = self.accent();
        ui.add_space(18.0);
        ui.horizontal(|ui| {
            ui.add_space(22.0);
            ui.label(RichText::new("MENU").size(11.0).strong().color(Color32::from_rgb(0x64, 0x74, 0x8b)));
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
                    p.rect_filled(rect, Rounding::same(10.0), with_alpha(Color32::WHITE, 10));
                }
                let fg = if active { accent } else { text_dim() };
                draw_icon(p, Pos2::new(rect.left() + 26.0, rect.center().y), *icon, fg, 17.0);
                p.text(
                    Pos2::new(rect.left() + 46.0, rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    *label,
                    egui::FontId::proportional(13.5),
                    if active { accent } else { text_primary() },
                );
                if let Some(n) = badge {
                    let text = n.to_string();
                    let g = p.layout_no_wrap(text.clone(), egui::FontId::proportional(11.0), fg);
                    let w = (g.size().x + 14.0).max(22.0);
                    let b = Rect::from_center_size(Pos2::new(rect.right() - 12.0 - w / 2.0, rect.center().y), Vec2::new(w, 20.0));
                    p.rect_filled(b, Rounding::same(10.0), if active { with_alpha(accent, 60) } else { with_alpha(Color32::WHITE, 16) });
                    p.text(b.center(), egui::Align2::CENTER_CENTER, text, egui::FontId::proportional(11.0), fg);
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

        ui.add_space(14.0);
        ui.horizontal(|ui| {
            ui.add_space(12.0);
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(232.0, 38.0), egui::Sense::click());
            let col = if resp.hovered() { with_alpha(accent, 130) } else { with_alpha(Color32::WHITE, 50) };
            if resp.hovered() {
                ui.painter().rect_filled(rect, Rounding::same(10.0), with_alpha(accent, 14));
            }
            let r = rect.shrink(0.5);
            let st = Stroke::new(1.0f32, col);
            for (a, b) in [
                (r.left_top() + Vec2::new(10.0, 0.0), r.right_top() - Vec2::new(10.0, 0.0)),
                (r.left_bottom() + Vec2::new(10.0, 0.0), r.right_bottom() - Vec2::new(10.0, 0.0)),
                (r.left_top() + Vec2::new(0.0, 10.0), r.left_bottom() - Vec2::new(0.0, 10.0)),
                (r.right_top() + Vec2::new(0.0, 10.0), r.right_bottom() - Vec2::new(0.0, 10.0)),
            ] {
                ui.painter().extend(egui::Shape::dashed_line(&[a, b], st, 4.0, 3.0));
            }
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "+  Nouvelle règle",
                egui::FontId::proportional(13.0),
                if resp.hovered() { accent } else { text_primary() },
            );
            if resp.clicked() {
                self.new_rule();
            }
        });

        // Pied : état de l'application.
        ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                ui.add_space(12.0);
                let (rect, _) = ui.allocate_exact_size(Vec2::new(232.0, 84.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, Rounding::same(12.0), with_alpha(CANVAS, 150));
                ui.painter().rect_stroke(rect, Rounding::same(12.0), Stroke::new(1.0f32, with_alpha(Color32::WHITE, 13)));
                let mut c = ui.new_child(
                    egui::UiBuilder::new().max_rect(rect.shrink(12.0)).layout(Layout::top_down(Align::Min)),
                );
                c.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("APPLICATION").size(10.0).strong().color(text_dim()));
                        ui.label(
                            RichText::new(if self.app_enabled { "Activée" } else { "En veille" })
                                .size(13.5)
                                .color(Color32::WHITE),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        toggle(ui, &mut self.app_enabled, accent);
                    });
                });
                c.add_space(6.0);
                let (dot, txt) = if self.app_enabled {
                    (GREEN, "Surveillance active • v0.1")
                } else {
                    (text_dim(), "En veille • v0.1")
                };
                c.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(8.0), egui::Sense::hover());
                    ui.painter().circle_filled(r.center(), 3.5, dot);
                    ui.label(RichText::new(txt).size(11.0).color(text_dim()));
                });
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
            ui.label(RichText::new("Règles").size(24.0).strong().color(Color32::WHITE));
            ui.add_space(6.0);
            let n = self.rules.iter().filter(|r| r.enabled).count();
            chip(
                ui,
                &format!("{n} {}", if n > 1 { "actives" } else { "active" }),
                SKY,
                with_alpha(SKY, 38),
                with_alpha(SKY, 77),
            );
        });
        ui.add_space(2.0);
        ui.label(RichText::new("Si ces conditions correspondent, exécutez ces actions dans l'ordre.").size(13.0).color(text_dim()));
        ui.add_space(16.0);

        if self.folders.is_empty() {
            ui.label(RichText::new("Ajoutez d'abord un dossier suivi, puis créez-y des règles.").color(text_dim()));
            return;
        }

        let order = self.rule_order();
        if order.is_empty() {
            ui.label(RichText::new("Aucune règle pour l'instant.").color(text_dim()));
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
            if btn(ui, "+", Btn::Ghost, accent, None).on_hover_text("Nouvelle règle").clicked() {
                self.new_rule();
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
                    if ui.add(egui::TextEdit::singleline(&mut name).desired_width(300.0)).changed() {
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
                    if ui.checkbox(&mut en, "Activée").changed() {
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
                ui.spacing_mut().slider_width = 150.0;
                let mut depth = self.rules[idx].recursion_depth;
                if ui.add(egui::Slider::new(&mut depth, 0..=8).show_value(false).trailing_fill(true)).changed() {
                    self.rules[idx].recursion_depth = depth;
                }
                chip(ui, &depth.to_string(), accent, with_alpha(accent, 38), with_alpha(accent, 77));
            });
            ui.add_space(4.0);
            ui.label(
                RichText::new("0 = fichiers de premier niveau seulement ; n descend de n niveaux.")
                    .size(11.0)
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
                    with_alpha(SKY, 77),
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
                            Field::Contents => "texte à chercher (documents ; images : OCR)",
                            _ => "valeur",
                        };
                        let mut val = self.rules[idx].conditions[ci].value.clone();
                        let w = (ui.available_width() - 44.0).max(80.0);
                        if ui.add(egui::TextEdit::singleline(&mut val).desired_width(w).hint_text(hint)).changed() {
                            self.rules[idx].conditions[ci].value = val;
                        }
                        if ui.add(egui::Button::new("✕").frame(false)).on_hover_text("Supprimer").clicked() {
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
                    RichText::new("exécutées en ordre, chacune voit le fichier laissé par la précédente")
                        .size(11.5)
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
                            if ui.add(egui::Button::new("✕").frame(false)).on_hover_text("Supprimer").clicked() {
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
                            let entry = self.rules[idx].actions[ai].params.entry((*k).to_string()).or_default();
                            ui.add(
                                egui::TextEdit::singleline(entry)
                                    .desired_width((ui.available_width() - 8.0).max(120.0))
                                    .hint_text(*placeholder),
                            );
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
