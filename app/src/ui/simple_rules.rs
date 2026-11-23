//! Mode simple de l'écran Règles : un assistant en trois étapes (dossiers, conditions, action).
//! Première partie : logique pure (compatibilité, raccourcis, étapes, phrase récapitulative), testée sans fenêtre.
//! Seconde partie : affichage egui de l'assistant.

use eframe::egui::{self, RichText};

use super::*;
use crate::engine::{Action, ActionKind, Condition, Field, Folder, Op, Rule};
use crate::mechanics::{parse_size, parse_span};
use crate::tags;

/// Étape de l'assistant : 1 = dossiers, 2 = conditions, 3 = action.
pub const LAST_STEP: u8 = 3;

/// Actions proposées en mode simple : celles qui ne détruisent rien de façon irréversible.
pub const SIMPLE_ACTIONS: &[ActionKind] =
    &[ActionKind::Move, ActionKind::Copy, ActionKind::Rename, ActionKind::Trash, ActionKind::AddTag, ActionKind::Compress];

/// Icônes illustrées de l'assistant (`assets/ui/rules/`, préparées par `scripts/generer-icones-regles.sh`).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum RuleArt {
    Image,
    Pdf,
    Video,
    Document,
    Audio,
    Archive,
    Calendar,
    Text,
    Ruler,
    Folder,
    FolderMove,
    FolderCopy,
    FolderRename,
    FolderDelete,
    FolderTag,
    FolderZip,
}

impl RuleArt {
    #[cfg(test)]
    pub const ALL: [RuleArt; 16] = [
        RuleArt::Image,
        RuleArt::Pdf,
        RuleArt::Video,
        RuleArt::Document,
        RuleArt::Audio,
        RuleArt::Archive,
        RuleArt::Calendar,
        RuleArt::Text,
        RuleArt::Ruler,
        RuleArt::Folder,
        RuleArt::FolderMove,
        RuleArt::FolderCopy,
        RuleArt::FolderRename,
        RuleArt::FolderDelete,
        RuleArt::FolderTag,
        RuleArt::FolderZip,
    ];

    fn png(self) -> &'static [u8] {
        macro_rules! art {
            ($name:literal) => {
                include_bytes!(concat!("../../../assets/ui/rules/", $name, ".png"))
            };
        }
        match self {
            RuleArt::Image => art!("images-file"),
            RuleArt::Pdf => art!("pdf-file"),
            RuleArt::Video => art!("video-file"),
            RuleArt::Document => art!("document-file"),
            RuleArt::Audio => art!("audio-file"),
            RuleArt::Archive => art!("archive-file"),
            RuleArt::Calendar => art!("calandar-30J-file"),
            RuleArt::Text => art!("tt"),
            RuleArt::Ruler => art!("rules"),
            RuleArt::Folder => art!("folder-empty"),
            RuleArt::FolderMove => art!("folder-move"),
            RuleArt::FolderCopy => art!("folder-copy"),
            RuleArt::FolderRename => art!("folder-rename"),
            RuleArt::FolderDelete => art!("folder-delete"),
            RuleArt::FolderTag => art!("folder-tag"),
            RuleArt::FolderZip => art!("folder-zip"),
        }
    }
}

/// Raccourci de condition : une carte illustrée (libellé, icône, teinte du halo) et la condition qu'elle ajoute.
pub struct Preset {
    pub label: &'static str,
    pub art: RuleArt,
    pub tint: Color32,
    pub make: fn() -> Condition,
}

fn cond(field: Field, op: Op, value: &str) -> Condition {
    Condition { field, op, value: value.into() }
}

// Teintes des pastilles illustrées : une couleur par famille de fichier, lisibles sur le thème sombre.
const BLUE: Color32 = Color32::from_rgb(0x2b, 0x8c, 0xf0);
const RED_TINT: Color32 = Color32::from_rgb(0xe5, 0x4d, 0x4d);
const VIOLET: Color32 = Color32::from_rgb(0x8b, 0x5c, 0xf6);
const INDIGO: Color32 = Color32::from_rgb(0x55, 0x6b, 0xf0);
const PINK_TINT: Color32 = Color32::from_rgb(0xd9, 0x4f, 0xa0);
const AMBER: Color32 = Color32::from_rgb(0xc2, 0x7c, 0x0e);
const ORANGE_TINT: Color32 = Color32::from_rgb(0xe0, 0x55, 0x1c);
const TEAL: Color32 = Color32::from_rgb(0x0f, 0x8f, 0x88);
const GREEN_TINT: Color32 = Color32::from_rgb(0x1f, 0x95, 0x4e);
const CYAN: Color32 = Color32::from_rgb(0x0e, 0x8f, 0xb0);

pub const PRESETS: &[Preset] = &[
    Preset { label: "C'est une image", art: RuleArt::Image, tint: BLUE, make: || cond(Field::Kind, Op::Is, "image") },
    Preset { label: "C'est un PDF", art: RuleArt::Pdf, tint: RED_TINT, make: || cond(Field::Extension, Op::Is, "pdf") },
    Preset { label: "C'est une vidéo", art: RuleArt::Video, tint: VIOLET, make: || cond(Field::Kind, Op::Is, "video") },
    Preset { label: "C'est un document", art: RuleArt::Document, tint: INDIGO, make: || cond(Field::Kind, Op::Is, "document") },
    Preset { label: "C'est un son", art: RuleArt::Audio, tint: PINK_TINT, make: || cond(Field::Kind, Op::Is, "audio") },
    Preset { label: "C'est une archive", art: RuleArt::Archive, tint: AMBER, make: || cond(Field::Kind, Op::Is, "archive") },
    Preset {
        label: "Plus ancien que 30 jours",
        art: RuleArt::Calendar,
        tint: ORANGE_TINT,
        make: || cond(Field::DateModified, Op::NotWithinLast, "30 days"),
    },
    Preset { label: "Le nom contient…", art: RuleArt::Text, tint: TEAL, make: || cond(Field::Name, Op::Contains, "") },
    Preset { label: "Plus gros que…", art: RuleArt::Ruler, tint: GREEN_TINT, make: || cond(Field::Size, Op::Greater, "100 MB") },
];

/// La carte est-elle « allumée » ? Oui si une condition de la règle correspond : même critère et même opérateur, et pour
/// les types et extensions (valeur fixe) la même valeur. Les cartes à valeur libre (nom, taille, ancienneté) comptent dès
/// que le critère est présent, quelle que soit la valeur choisie.
pub fn preset_active(rule: &Rule, preset: &Preset) -> bool {
    rule.conditions.iter().any(|c| preset_matches(c, &(preset.make)()))
}

fn preset_matches(have: &Condition, want: &Condition) -> bool {
    if have.field != want.field || have.op != want.op {
        return false;
    }
    match want.field {
        Field::Kind | Field::Extension => have.value.trim().eq_ignore_ascii_case(want.value.trim()),
        _ => true,
    }
}

/// Clic sur une carte : ajoute la condition, ou la retire si elle y est déjà.
pub fn toggle_preset(rule: &mut Rule, preset: &Preset) {
    let want = (preset.make)();
    match rule.conditions.iter().position(|c| preset_matches(c, &want)) {
        Some(i) => {
            rule.conditions.remove(i);
        }
        None => rule.conditions.push(want),
    }
}

/// Icône et teinte d'un critère de condition (ligne de la liste, étape 2).
pub fn condition_icon(field: Field) -> (RuleArt, Color32) {
    match field {
        Field::Name => (RuleArt::Text, TEAL),
        Field::Extension | Field::Contents => (RuleArt::Document, INDIGO),
        Field::Size => (RuleArt::Ruler, GREEN_TINT),
        Field::DateAdded | Field::DateModified => (RuleArt::Calendar, ORANGE_TINT),
        Field::Kind => (RuleArt::Image, BLUE),
        Field::Tag => (RuleArt::FolderTag, AMBER),
    }
}

/// Icône et teinte d'une action (cartes de l'étape 3).
pub fn action_icon(kind: &ActionKind) -> (RuleArt, Color32) {
    match kind {
        ActionKind::Move => (RuleArt::FolderMove, BLUE),
        ActionKind::Copy => (RuleArt::FolderCopy, CYAN),
        ActionKind::Rename => (RuleArt::FolderRename, VIOLET),
        ActionKind::Trash => (RuleArt::FolderDelete, RED_TINT),
        ActionKind::AddTag => (RuleArt::FolderTag, AMBER),
        ActionKind::Compress => (RuleArt::FolderZip, ORANGE_TINT),
        _ => (RuleArt::Folder, INDIGO),
    }
}

/// Opérateurs proposés en mode simple : tout sauf l'expression régulière (« correspond à »).
pub fn simple_ops(field: Field) -> Vec<Op> {
    field.ops().iter().copied().filter(|op| *op != Op::Matches).collect()
}

/// Libellé d'un opérateur pour les novices (plus parlant que « est supérieur à »).
pub fn simple_op_label(field: Field, op: Op) -> &'static str {
    match (field, op) {
        (Field::Size, Op::Greater) => "plus gros que",
        (Field::Size, Op::Less) => "plus petit que",
        (Field::Size, Op::Equals) => "exactement",
        (Field::DateAdded | Field::DateModified, Op::NotWithinLast) => "il y a plus de",
        (Field::DateAdded | Field::DateModified, Op::WithinLast) => "dans les derniers",
        (Field::DateAdded | Field::DateModified, Op::Before) => "avant le",
        (Field::DateAdded | Field::DateModified, Op::After) => "après le",
        _ => op.label(),
    }
}

/// Libellé d'une action pour les novices.
pub fn simple_action_label(kind: &ActionKind) -> &'static str {
    match kind {
        ActionKind::Move => "Déplacer",
        ActionKind::Copy => "Copier",
        ActionKind::Rename => "Renommer",
        ActionKind::Trash => "Mettre à la corbeille",
        ActionKind::AddTag => "Ajouter une étiquette",
        ActionKind::Compress => "Compresser en ZIP",
        other => other.label(),
    }
}

/// La règle tient-elle dans l'assistant ? Sinon on la laisse intacte et on propose le mode avancé.
pub fn is_simple_compatible(rule: &Rule) -> bool {
    let one_condition = rule.conditions.len() <= 1;
    rule.recursion_depth == 0
        && (rule.match_all || one_condition)
        && rule.conditions.iter().all(|c| c.op != Op::Matches)
        && rule.actions.len() <= 1
        && rule.actions.iter().all(|a| SIMPLE_ACTIONS.contains(&a.kind))
}

/// L'étape est-elle complète ? Sinon : la phrase qui dit quoi faire.
pub fn step_ready(step: u8, rule: &Rule) -> Result<(), &'static str> {
    match step {
        1 if rule.folder_ids.is_empty() => Err("Choisissez au moins un dossier."),
        1 => Ok(()),
        2 => conditions_ready(&rule.conditions),
        _ => action_ready(&rule.actions),
    }
}

fn conditions_ready(conditions: &[Condition]) -> Result<(), &'static str> {
    if conditions.is_empty() {
        return Err("Ajoutez au moins une condition.");
    }
    for c in conditions {
        let value = c.value.trim();
        if value.is_empty() {
            return Err("Complétez chaque condition.");
        }
        let understood = match (c.field, c.op) {
            (Field::Size, _) => parse_size(value).is_some(),
            (Field::DateAdded | Field::DateModified, Op::WithinLast | Op::NotWithinLast) => parse_span(value).is_some(),
            _ => true,
        };
        if !understood {
            return Err("Une valeur n'est pas comprise : vérifiez-la.");
        }
    }
    Ok(())
}

fn action_ready(actions: &[Action]) -> Result<(), &'static str> {
    let [action] = actions else { return Err("Choisissez ce qu'il faut faire des fichiers.") };
    let missing = |key: &str| action.param(key).is_empty();
    match action.kind {
        ActionKind::Move | ActionKind::Copy if missing("destination") => Err("Choisissez le dossier de destination."),
        ActionKind::Rename if missing("pattern") => Err("Indiquez le nouveau nom."),
        ActionKind::AddTag if missing("tags") => Err("Indiquez l'étiquette à ajouter."),
        _ => Ok(()),
    }
}

/// « 30 days » → (« 30 », « jours ») ; « 100 MB » → (« 100 », « MB »). Sert aux champs « nombre + unité ».
pub fn split_amount(value: &str) -> (String, String) {
    let t = value.trim();
    let end = t.find(|c: char| !(c.is_ascii_digit() || c == '.' || c == ',')).unwrap_or(t.len());
    (t[..end].trim().to_string(), t[end..].trim().to_string())
}

fn unit_fr(unit: &str, plural: bool) -> String {
    let base = match unit.to_lowercase().trim_end_matches('s') {
        "day" | "" => "jour",
        "week" => "semaine",
        "month" => "mois",
        "year" => return if plural { "ans".into() } else { "an".into() },
        _ => return unit.to_string(),
    };
    if plural && base != "mois" {
        format!("{base}s")
    } else {
        base.to_string()
    }
}

fn span_fr(value: &str) -> String {
    let (n, unit) = split_amount(value);
    let plural = n.replace(',', ".").parse::<f64>().map_or(true, |x| x > 1.0);
    format!("{n} {}", unit_fr(&unit, plural))
}

fn kind_phrase(value: &str, negative: bool) -> String {
    let noun = match value.trim().to_lowercase().as_str() {
        "image" => "une image",
        "document" => "un document",
        "video" => "une vidéo",
        "audio" => "de type audio",
        "archive" => "une archive",
        "folder" => "un dossier",
        "app" => "une application",
        "other" => "d'un autre type",
        other => return format!("{} {other}", if negative { "n'est pas du type" } else { "est du type" }),
    };
    format!("{} {noun}", if negative { "n'est pas" } else { "est" })
}

/// Une condition en français, au début de « quand un fichier … ».
pub fn condition_phrase(c: &Condition) -> String {
    let v = c.value.trim();
    let shown = if v.is_empty() { "…" } else { v };
    match (c.field, c.op) {
        (Field::Kind, Op::Is) => kind_phrase(v, false),
        (Field::Kind, Op::IsNot) => kind_phrase(v, true),
        (Field::Extension, Op::IsNot) => format!("n'a pas l'extension {shown}"),
        (Field::Extension, _) if shown.contains(',') => format!("a l'une des extensions {shown}"),
        (Field::Extension, _) => format!("a l'extension {shown}"),
        (Field::Name, Op::Contains) => format!("a un nom qui contient « {shown} »"),
        (Field::Name, Op::StartsWith) => format!("a un nom qui commence par « {shown} »"),
        (Field::Name, Op::EndsWith) => format!("a un nom qui finit par « {shown} »"),
        (Field::Name, _) => format!("s'appelle « {shown} »"),
        (Field::Size, Op::Greater) => format!("pèse plus de {shown}"),
        (Field::Size, Op::Less) => format!("pèse moins de {shown}"),
        (Field::Size, _) => format!("pèse exactement {shown}"),
        (Field::DateAdded | Field::DateModified, Op::WithinLast | Op::NotWithinLast) => {
            let verb = if c.field == Field::DateAdded { "a été ajouté" } else { "a été modifié" };
            if c.op == Op::NotWithinLast {
                format!("{verb} il y a plus de {}", span_fr(shown))
            } else {
                format!("{verb} dans les {} derniers", span_fr(shown))
            }
        }
        (Field::DateAdded | Field::DateModified, op) => {
            let what = if c.field == Field::DateAdded { "a été ajouté" } else { "a été modifié" };
            format!("{what} {} {shown}", if op == Op::Before { "avant le" } else { "après le" })
        }
        (Field::Tag, _) => format!("porte l'étiquette « {shown} »"),
        (Field::Contents, _) => format!("contient le texte « {shown} »"),
        (field, op) => format!("{} {} {shown}", field.label().to_lowercase(), op.label()),
    }
}

/// Une action en français, après « … , ».
pub fn action_phrase(a: &Action) -> String {
    let quoted = |key: &str| {
        let v = a.param(key);
        if v.is_empty() {
            "…".to_string()
        } else {
            format!("« {v} »")
        }
    };
    match a.kind {
        ActionKind::Move => format!("le déplacer vers {}", quoted("destination")),
        ActionKind::Copy => format!("le copier vers {}", quoted("destination")),
        ActionKind::Rename => format!("le renommer avec le motif {}", quoted("pattern")),
        ActionKind::Trash => "le mettre à la corbeille".into(),
        ActionKind::AddTag => format!("lui ajouter l'étiquette {}", quoted("tags")),
        ActionKind::Compress => "le compresser en archive ZIP".into(),
        _ => a.kind.label().to_lowercase(),
    }
}

/// La règle en une phrase, découpée en segments : `(texte, mis_en_valeur)`. Les mots clés (dossiers, conditions, action)
/// sont mis en valeur par l'écran ; les autres relient la phrase.
pub fn rule_sentence_parts(rule: &Rule, folders: &[Folder]) -> Vec<(String, bool)> {
    let names: Vec<String> = rule.folder_ids.iter().map(|id| tags::folder_label(folders, *id)).filter(|n| !n.is_empty()).collect();
    let place = match names.as_slice() {
        [] => "…".to_string(),
        [one] => one.clone(),
        [init @ .., last] => format!("{} et {last}", init.join(", ")),
    };
    let when = if rule.conditions.is_empty() {
        "…".to_string()
    } else {
        rule.conditions.iter().map(condition_phrase).collect::<Vec<_>>().join(" et ")
    };
    let what = rule.actions.first().map_or_else(|| "…".to_string(), action_phrase);
    let prefix = if names.len() > 1 { "Dans les dossiers " } else { "Dans " };
    vec![
        (prefix.to_string(), false),
        (place, true),
        (", quand un fichier ".to_string(), false),
        (when, true),
        (", ".to_string(), false),
        (what, true),
        (".".to_string(), false),
    ]
}

/// La règle en une phrase : « Dans Téléchargements, quand un fichier est une image, le déplacer vers « Images ». »
pub fn rule_sentence(rule: &Rule, folders: &[Folder]) -> String {
    rule_sentence_parts(rule, folders).into_iter().map(|(text, _)| text).collect()
}

// ------------------------------------------------------------------ affichage

const STEP_NAMES: [&str; 3] = ["Dossiers", "Conditions", "Action"];
const SIZE_UNITS: [&str; 3] = ["KB", "MB", "GB"];
const SPAN_UNITS: [(&str, &str); 3] = [("days", "jours"), ("weeks", "semaines"), ("months", "mois")];
const CONFLICT_CHOICES: [(&str, &str); 3] = [(CONFLICT_NUMBER, "Garder les deux"), ("skip", "Ne rien faire"), ("replace", "Remplacer")];

/// Cartes illustrées (modèles de conditions, actions) : taille fixe, elles passent à la ligne.
const CARD_W: f32 = 148.0;
const CARD_H: f32 = 134.0;
/// Côté de l'icône illustrée d'une carte.
const ART_SIZE: f32 = 78.0;
const FOLDER_TILE_W: f32 = 262.0;
const FOLDER_TILE_H: f32 = 58.0;
/// Au-delà de ce nombre de dossiers suivis, le panneau de sélection propose une recherche.
const SEARCH_FROM: usize = 6;

fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn step_title(ui: &mut egui::Ui, title: &str, hint: &str) {
    ui.label(RichText::new(title).size(21.0).family(semibold()).color(text_primary()));
    ui.add_space(2.0);
    ui.label(RichText::new(hint).size(14.5).color(text_dim()));
    ui.add_space(14.0);
}

/// Pastille illustrée : carré arrondi **plat** (couleur unie, sans reflet ni contour) et pictogramme blanc bien lisible.
fn icon_tile(painter: &egui::Painter, rect: Rect, icon: Icon, tint: Color32) {
    painter.rect_filled(rect, Rounding::same(rect.width() * 0.28), tint);
    draw_icon(painter, rect.center(), icon, Color32::WHITE, rect.width() * 0.62);
}

/// Texture d'une icône illustrée (décodée une seule fois, gardée dans la mémoire temporaire d'egui).
fn rule_art_texture(ctx: &egui::Context, art: RuleArt) -> egui::TextureHandle {
    let id = egui::Id::new(("rule-art", art));
    if let Some(tex) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return tex;
    }
    let tex = smooth_texture(ctx, &format!("rule-art-{art:?}"), art.png());
    ctx.data_mut(|d| d.insert_temp(id, tex.clone()));
    tex
}

/// Dessine une icône illustrée centrée en `center`, avec la taille `size`.
fn paint_art(painter: &egui::Painter, ctx: &egui::Context, art: RuleArt, center: Pos2, size: f32) {
    let tex = rule_art_texture(ctx, art);
    painter.image(
        tex.id(),
        Rect::from_center_size(center, Vec2::splat(size)),
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        Color32::WHITE,
    );
}

/// Carte illustrée cliquable : grande icône sur un halo de sa teinte, libellé, état « choisi » (contour et coche
/// d'accent), l'icône grossit et monte légèrement au survol.
fn choice_card(ui: &mut egui::Ui, art: RuleArt, tint: Color32, label: &str, selected: bool, accent: Color32, dur: f32) -> egui::Response {
    let (slot, resp) = ui.allocate_exact_size(Vec2::new(CARD_W, CARD_H), egui::Sense::click());
    let ctx = ui.ctx().clone();
    let hover = ctx.animate_bool_with_time(resp.id.with("hover"), resp.hovered(), dur);
    let chosen = ctx.animate_bool_with_time(resp.id.with("chosen"), selected, dur);
    let rect = slot.translate(Vec2::new(0.0, -2.0 * hover));
    let painter = ui.painter();
    painter.rect_filled(rect, Rounding::same(14.0), glass(((12.0 + 10.0 * hover) as u8).max(10)));
    if chosen > 0.0 {
        painter.rect_filled(rect, Rounding::same(14.0), with_alpha(accent, (40.0 * chosen) as u8));
        painter.rect_stroke(rect, Rounding::same(14.0), Stroke::new(1.5_f32, with_alpha(accent, (230.0 * chosen) as u8)));
    } else {
        painter.rect_stroke(rect, Rounding::same(14.0), Stroke::new(1.0_f32, glass(22)));
    }
    let center = Pos2::new(rect.center().x, rect.top() + 10.0 + ART_SIZE / 2.0 - 3.0 * hover);
    radial_glow(painter, center, ART_SIZE * 0.62, tint, 40.0 + 70.0 * chosen + 30.0 * hover);
    paint_art(painter, &ctx, art, center, ART_SIZE * (1.0 + 0.06 * hover));
    let mut job = egui::text::LayoutJob::simple(label.to_string(), egui::FontId::new(13.5, semibold()), text_primary(), CARD_W - 16.0);
    job.halign = Align::Center;
    let galley = painter.layout_job(job);
    painter.galley(Pos2::new(rect.center().x, rect.top() + 10.0 + ART_SIZE + 2.0), galley, text_primary());
    if chosen > 0.0 {
        let c = Pos2::new(rect.right() - 13.0, rect.top() + 13.0);
        painter.circle_filled(c, 9.0 * chosen, accent);
        if chosen > 0.6 {
            draw_icon(painter, c, Icon::Tick, Color32::WHITE, 12.0);
        }
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Déroule `content` avec une animation de hauteur et de fondu ; `None` tant que c'est fermé. Sans animation
/// (`dur` à 0, mouvement réduit), le contenu apparaît d'un coup.
fn unfold<R>(ui: &mut egui::Ui, id: egui::Id, open: bool, dur: f32, content: impl FnOnce(&mut egui::Ui) -> R) -> Option<R> {
    let ctx = ui.ctx().clone();
    let t = ctx.animate_bool_with_time(id.with("t"), open, dur);
    if t <= 0.0 {
        return None;
    }
    let height_id = id.with("h");
    if t >= 1.0 {
        let out = ui.scope(content);
        ctx.data_mut(|d| d.insert_temp(height_id, out.response.rect.height()));
        return Some(out.inner);
    }
    let full: f32 = ctx.data(|d| d.get_temp(height_id)).unwrap_or(180.0);
    let width = ui.available_width();
    let (outer, _) = ui.allocate_exact_size(Vec2::new(width, full * ease(t)), egui::Sense::hover());
    let mut child = ui.new_child(
        egui::UiBuilder::new().max_rect(Rect::from_min_size(outer.min, Vec2::new(width, 4000.0))).layout(Layout::top_down(Align::Min)),
    );
    child.set_clip_rect(outer.intersect(ui.clip_rect()));
    child.set_opacity(ease(t));
    let out = content(&mut child);
    ctx.data_mut(|d| d.insert_temp(height_id, child.min_rect().height()));
    Some(out)
}

impl App {
    /// Durée des animations de l'assistant : nulle quand le mouvement est réduit (« Animer le fond » coupé).
    fn motion_secs(&self) -> f32 {
        if self.animate_bg {
            0.24
        } else {
            0.0
        }
    }

    /// Corps de l'écran Règles en mode simple : frise, étape courante, navigation, phrase récapitulative.
    /// Renvoie vrai si les dossiers cibles ont changé (l'ordre des onglets peut en dépendre).
    pub(super) fn simple_rules_ui(&mut self, ui: &mut egui::Ui, idx: usize) -> bool {
        let accent = self.accent();
        if !is_simple_compatible(&self.rules[idx]) {
            self.advanced_only_card(ui, idx, accent);
            return false;
        }
        let step = self.simple_step.clamp(1, LAST_STEP);
        self.step_bar(ui, idx, step, accent);
        ui.add_space(14.0);

        let mut moved = false;
        panel_card_with(ui, None, 22.0, |ui| match step {
            1 => moved = self.step_folders(ui, idx, accent),
            2 => self.step_conditions(ui, idx, accent),
            _ => self.step_action(ui, idx, accent),
        });
        ui.add_space(14.0);

        let ready = step_ready(step, &self.rules[idx]);
        ui.horizontal(|ui| {
            if step > 1 && btn(ui, "Précédent", Btn::Ghost, accent, None).clicked() {
                self.simple_step = step - 1;
            }
            if let Err(why) = ready {
                ui.add_space(8.0);
                ui.label(RichText::new(why).size(14.5).color(text_dim()));
            }
            // « Suivant » (ou « Terminer ») à droite : c'est là qu'on l'attend.
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_enabled_ui(ready.is_ok(), |ui| {
                    if step < LAST_STEP {
                        if btn(ui, "Suivant", Btn::Primary, accent, None).clicked() {
                            self.simple_step = step + 1;
                        }
                    } else if btn(ui, "Terminer", Btn::Primary, accent, Some(Icon::Tick)).clicked() {
                        self.toast("Règle enregistrée : elle s'applique dès maintenant.".to_string());
                        self.simple_step = 1;
                    }
                });
            });
        });
        ui.add_space(16.0);
        self.summary_card(ui, idx, accent, step);
        ui.add_space(14.0);
        moved
    }

    /// « En résumé » : la règle en une phrase, mise en évidence (fond et contour d'accent, mots clés colorés).
    fn summary_card(&mut self, ui: &mut egui::Ui, idx: usize, accent: Color32, step: u8) {
        let parts = rule_sentence_parts(&self.rules[idx], &self.folders);
        let (_, rect) = panel_card_with(ui, Some(with_alpha(accent, 24)), 20.0, |ui| {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::splat(28.0), egui::Sense::hover());
                icon_tile(ui.painter(), r, Icon::Sparkle, accent);
                ui.add_space(4.0);
                ui.label(RichText::new("EN RÉSUMÉ").size(13.5).family(semibold()).color(lighten(accent, 40)));
            });
            ui.add_space(10.0);
            let mut job = egui::text::LayoutJob::default();
            job.wrap.max_width = ui.available_width();
            for (text, key) in parts {
                // Mots clés en gras seulement : même couleur que le reste de la phrase, pas de fond.
                let font = if key { egui::FontId::new(19.5, semibold()) } else { egui::FontId::proportional(19.5) };
                job.append(&text, 0.0, egui::text::TextFormat { font_id: font, color: text_primary(), ..Default::default() });
            }
            ui.label(job);
            if step == LAST_STEP {
                ui.add_space(12.0);
                ui.label(
                    RichText::new(
                        "Rien n'est perdu : chaque rangement apparaît dans l'Historique et, le plus souvent, peut être annulé. \
                         Vous pouvez d'abord faire un essai : rien ne bouge, le résultat s'affiche dans l'Historique.",
                    )
                    .size(14.0)
                    .color(text_dim()),
                );
                ui.add_space(10.0);
                if btn(ui, "Tester sans rien toucher", Btn::Sky, accent, None).clicked() {
                    self.run_batch(true);
                }
            }
        });
        // Contour fin d'accent, sans halo : la carte se distingue par sa teinte.
        ui.painter().rect_stroke(rect, Rounding::same(CARD_RADIUS), Stroke::new(1.5_f32, with_alpha(accent, 170)));
    }

    /// Règle qui utilise des réglages que l'assistant ne sait pas présenter : on la laisse intacte.
    fn advanced_only_card(&mut self, ui: &mut egui::Ui, idx: usize, accent: Color32) {
        let sentence = rule_sentence(&self.rules[idx], &self.folders);
        panel_card(ui, |ui| {
            ui.label(RichText::new("Cette règle utilise des réglages avancés").size(18.0).family(semibold()).color(text_primary()));
            ui.add_space(4.0);
            ui.label(
                RichText::new(
                    "Elle fonctionne comme avant et n'a pas été modifiée. Pour la changer, passez en mode avancé : \
                     vous y retrouvez tous ses réglages.",
                )
                .size(14.5)
                .color(text_dim()),
            );
            ui.add_space(10.0);
            ui.label(RichText::new(sentence).size(15.0).color(text_primary()));
            ui.add_space(12.0);
            if btn(ui, "Modifier en mode avancé", Btn::Sky, accent, None).clicked() {
                self.rules_mode = RulesMode::Advanced;
            }
        });
    }

    /// Frise « 1 Dossiers — 2 Conditions — 3 Action » : étape courante en accent, étapes faites cochées et cliquables
    /// (on revient en arrière librement, on avance étape par étape).
    fn step_bar(&mut self, ui: &mut egui::Ui, idx: usize, step: u8, accent: Color32) {
        let seg = ((ui.available_width() - 40.0) / 3.0).clamp(150.0, 240.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(seg * 3.0, 44.0), egui::Sense::hover());
        let dur = self.motion_secs();
        let ctx = ui.ctx().clone();
        let mut prev_label_end = rect.left();
        for (i, name) in STEP_NAMES.iter().enumerate() {
            let n = i as u8 + 1;
            let origin = rect.left() + seg * i as f32;
            let center = Pos2::new(origin + 18.0, rect.center().y);
            let hit = Rect::from_min_size(Pos2::new(origin, rect.top()), Vec2::new(seg - 8.0, rect.height()));
            let resp = ui.interact(hit, ui.id().with(("simple-step", n)), egui::Sense::click());
            let current = n == step;
            let done = n < step && step_ready(n, &self.rules[idx]).is_ok();
            let on = ctx.animate_bool_with_time(resp.id.with("on"), current || done, dur);
            let painter = ui.painter();
            if i > 0 {
                let from = Pos2::new(prev_label_end + 10.0, center.y);
                let to = Pos2::new(center.x - 22.0, center.y);
                if to.x - from.x > 12.0 {
                    painter.line_segment([from, to], Stroke::new(2.0_f32, if n <= step { with_alpha(accent, 170) } else { glass(40) }));
                }
            }
            if current {
                painter.circle_filled(center, 21.0, with_alpha(accent, 26));
            }
            painter.circle_filled(center, 15.0, if on > 0.0 { accent } else { glass(26) });
            if done {
                draw_icon(painter, center, Icon::Tick, Color32::WHITE, 15.0);
            } else {
                let color = if current { Color32::WHITE } else { text_dim() };
                painter.text(center, egui::Align2::CENTER_CENTER, n.to_string(), egui::FontId::new(15.0, semibold()), color);
            }
            let label_color = if current { text_primary() } else { text_dim() };
            let font = egui::FontId::new(if current { 16.0 } else { 15.0 }, semibold());
            let label_w = painter.layout_no_wrap(name.to_string(), font.clone(), label_color).size().x;
            painter.text(Pos2::new(center.x + 24.0, center.y), egui::Align2::LEFT_CENTER, *name, font, label_color);
            prev_label_end = center.x + 24.0 + label_w;
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                let reachable = (1..n).all(|s| step_ready(s, &self.rules[idx]).is_ok());
                if reachable {
                    self.simple_step = n;
                }
            }
        }
    }

    /// Étape 1 : la grande illustration du dossier ; un clic déroule la liste de tous les dossiers suivis.
    fn step_folders(&mut self, ui: &mut egui::Ui, idx: usize, accent: Color32) -> bool {
        let ctx = ui.ctx().clone();
        let rid = self.rules[idx].id;
        let dur = self.motion_secs();
        let open_id = egui::Id::new(("simple-folders-open", rid));
        // Capture d'écran : OMF_FOLDERS_OPEN ouvre d'emblée le panneau des dossiers.
        if self.screenshot.is_some() && self.frame <= 1 && std::env::var_os("OMF_FOLDERS_OPEN").is_some() {
            ctx.data_mut(|d| d.insert_temp(open_id, true));
        }
        let open: bool = ctx.data(|d| d.get_temp(open_id)).unwrap_or(false);
        let art = rule_art_texture(&ctx, RuleArt::Folder);
        let float = if self.animate_bg { (ctx.input(|i| i.time) as f32 * 1.2).sin() * 2.5 } else { 0.0 };
        let mut changed = false;
        let mut toggle = false;
        let mut refused_last = false;

        ui.horizontal(|ui| {
            let h = 128.0;
            let w = h * art.size_vec2().x / art.size_vec2().y;
            let (slot, resp) = ui.allocate_exact_size(Vec2::new(w + 28.0, h + 24.0), egui::Sense::click());
            let hover = ctx.animate_bool_with_time(resp.id.with("hover"), resp.hovered() || open, dur);
            let painter = ui.painter();
            radial_glow(painter, slot.center(), h * (0.75 + 0.1 * hover), accent, 55.0 + 55.0 * hover);
            let scale = 1.0 + 0.05 * hover;
            let img = Rect::from_center_size(slot.center() + Vec2::new(0.0, float - 2.0 * hover), Vec2::new(w, h) * scale);
            painter.image(art.id(), img, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Choisir les dossiers à ranger").clicked() {
                toggle = true;
            }
            if self.animate_bg {
                ctx.request_repaint_after(std::time::Duration::from_millis(40));
            }
            ui.add_space(6.0);
            ui.vertical(|ui| {
                ui.set_width(ui.available_width());
                ui.add_space(6.0);
                step_title(
                    ui,
                    "Quels dossiers voulez-vous ranger ?",
                    "Touchez le dossier pour choisir. Vos fichiers ne bougent que si les conditions de l'étape suivante sont remplies.",
                );
                ui.horizontal_wrapped(|ui| {
                    let ids = self.rules[idx].folder_ids.clone();
                    for fid in &ids {
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
                });
                ui.add_space(8.0);
                let label = if open { "Fermer la liste" } else { "Choisir les dossiers" };
                if btn(ui, label, Btn::Sky, accent, Some(Icon::ExpandMore)).clicked() {
                    toggle = true;
                }
            });
        });
        if toggle {
            ctx.data_mut(|d| d.insert_temp(open_id, !open));
        }

        let panel_open = ctx.data(|d| d.get_temp::<bool>(open_id)).unwrap_or(false);
        ui.add_space(4.0);
        let closed = unfold(ui, open_id.with("panel"), panel_open, dur, |ui| {
            ui.add_space(10.0);
            self.folder_panel(ui, idx, accent, &mut changed, &mut refused_last)
        });
        if closed == Some(true) {
            ctx.data_mut(|d| d.insert_temp(open_id, false));
        }
        if refused_last {
            self.toast_error("Une règle doit garder au moins un dossier.");
        }
        changed
    }

    /// Panneau « tous les dossiers suivis » : une tuile par dossier, coche animée. Renvoie vrai quand on le referme.
    fn folder_panel(&mut self, ui: &mut egui::Ui, idx: usize, accent: Color32, changed: &mut bool, refused_last: &mut bool) -> bool {
        let dur = self.motion_secs();
        let mut close = false;
        if self.folders.is_empty() {
            ui.label(RichText::new("Aucun dossier suivi : ajoutez-en dans « Dossiers suivis ».").size(14.0).color(text_dim()));
            return close;
        }
        let query_id = egui::Id::new(("simple-folders-query", self.rules[idx].id));
        let mut query: String = ui.data_mut(|d| d.get_temp(query_id)).unwrap_or_default();
        if self.folders.len() > SEARCH_FROM {
            ui.add(
                egui::TextEdit::singleline(&mut query)
                    .margin(egui::Margin::symmetric(8.0, 6.0))
                    .desired_width(300.0)
                    .hint_text("Rechercher un dossier…"),
            );
            ui.add_space(8.0);
        }
        ui.data_mut(|d| d.insert_temp(query_id, query.clone()));
        let q = tags::fold(&query);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(10.0, 10.0);
            for fi in 0..self.folders.len() {
                let (fid, path) = (self.folders[fi].id, self.folders[fi].path.display().to_string());
                if !q.is_empty() && !tags::fold(&path).contains(&q) {
                    continue;
                }
                let label = tags::folder_label(&self.folders, fid);
                let selected = self.rules[idx].folder_ids.contains(&fid);
                if folder_tile(ui, &label, &shorten(&path), selected, accent, dur).clicked() {
                    let ids = &mut self.rules[idx].folder_ids;
                    if !selected {
                        ids.push(fid);
                        *changed = true;
                    } else if ids.len() > 1 {
                        ids.retain(|id| *id != fid);
                        *changed = true;
                    } else {
                        *refused_last = true;
                    }
                }
            }
        });
        ui.add_space(10.0);
        if btn(ui, "Terminé", Btn::Primary, accent, Some(Icon::Tick)).clicked() {
            close = true;
        }
        ui.add_space(4.0);
        close
    }

    fn step_conditions(&mut self, ui: &mut egui::Ui, idx: usize, accent: Color32) {
        let dur = self.motion_secs();
        step_title(
            ui,
            "Quels fichiers sont concernés ?",
            "Touchez les modèles qui vous parlent. Un fichier est rangé seulement s'il remplit toutes les conditions.",
        );
        ui.label(caps("MODÈLES"));
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(10.0, 10.0);
            for preset in PRESETS {
                let on = preset_active(&self.rules[idx], preset);
                if choice_card(ui, preset.art, preset.tint, preset.label, on, accent, dur).clicked() {
                    toggle_preset(&mut self.rules[idx], preset);
                }
            }
        });
        ui.add_space(14.0);

        let rid = self.rules[idx].id;
        if !self.rules[idx].conditions.is_empty() {
            ui.label(caps("VOS CONDITIONS"));
            ui.add_space(6.0);
        }
        let mut remove: Option<usize> = None;
        for ci in 0..self.rules[idx].conditions.len() {
            row_frame(ui, |ui| {
                ui.horizontal(|ui| {
                    let (art, tint) = condition_icon(self.rules[idx].conditions[ci].field);
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(36.0), egui::Sense::hover());
                    radial_glow(ui.painter(), r.center(), 24.0, tint, 50.0);
                    paint_art(ui.painter(), ui.ctx(), art, r.center(), 36.0);
                    ui.add_space(4.0);
                    if self.simple_condition_row(ui, idx, ci, rid, accent) {
                        remove = Some(ci);
                    }
                });
            });
            ui.add_space(6.0);
        }
        if let Some(ci) = remove {
            self.rules[idx].conditions.remove(ci);
        }
        if btn(ui, "+ Autre critère", Btn::Ghost, accent, None).clicked() {
            self.rules[idx].conditions.push(Condition { field: Field::Name, op: Op::Contains, value: String::new() });
        }
    }

    /// Une ligne « champ · opérateur · valeur ». Renvoie vrai si la croix est cliquée.
    fn simple_condition_row(&mut self, ui: &mut egui::Ui, idx: usize, ci: usize, rid: u64, accent: Color32) -> bool {
        let field = self.rules[idx].conditions[ci].field;
        if let Some(pick) = combo(ui, &format!("sf-{idx}-{ci}"), field.label(), &FIELDS.iter().map(|f| f.label()).collect::<Vec<_>>()) {
            if let Some(f) = FIELDS.iter().find(|f| f.label() == pick) {
                let c = &mut self.rules[idx].conditions[ci];
                c.field = *f;
                c.op = simple_ops(*f)[0];
                c.value = match f {
                    Field::Kind => KIND_CHOICES[0].0.into(),
                    Field::Size => "100 MB".into(),
                    Field::DateAdded | Field::DateModified => "30 days".into(),
                    _ => String::new(),
                };
            }
        }
        let field = self.rules[idx].conditions[ci].field;
        let op = self.rules[idx].conditions[ci].op;
        let ops = simple_ops(field);
        let labels: Vec<&str> = ops.iter().map(|o| simple_op_label(field, *o)).collect();
        if let Some(pick) = combo(ui, &format!("so-{idx}-{ci}"), simple_op_label(field, op), &labels) {
            if let Some(o) = ops.iter().find(|o| simple_op_label(field, **o) == pick) {
                self.rules[idx].conditions[ci].op = *o;
            }
        }
        let value = self.rules[idx].conditions[ci].value.clone();
        let width = (ui.available_width() - 60.0).max(80.0);
        let new_value = match (field, op) {
            (Field::Extension, _) => extension_tags(ui, egui::Id::new(("sext", rid, ci)), &value, width, accent),
            (Field::Kind, _) => {
                let current = kind_label(&value).unwrap_or("Autre");
                let labels: Vec<&str> = KIND_CHOICES.iter().map(|(_, l)| *l).collect();
                combo(ui, &format!("sk-{idx}-{ci}"), current, &labels)
                    .and_then(|pick| KIND_CHOICES.iter().find(|(_, l)| *l == pick).map(|(v, _)| (*v).to_string()))
            }
            (Field::Size, _) => amount_editor(ui, &format!("ss-{idx}-{ci}"), &value, &SIZE_UNITS.map(|u| (u, u))),
            (Field::DateAdded | Field::DateModified, Op::WithinLast | Op::NotWithinLast) => {
                amount_editor(ui, &format!("sd-{idx}-{ci}"), &value, &SPAN_UNITS)
            }
            _ => {
                let mut text = value.clone();
                let hint = match field {
                    Field::DateAdded | Field::DateModified => "2024-01-31",
                    Field::Contents => "texte à chercher",
                    _ => "mot ou début de nom",
                };
                let edited = ui
                    .add(
                        egui::TextEdit::singleline(&mut text)
                            .margin(egui::Margin::symmetric(8.0, 7.0))
                            .desired_width(width)
                            .hint_text(hint),
                    )
                    .changed();
                edited.then_some(text)
            }
        };
        if let Some(v) = new_value {
            self.rules[idx].conditions[ci].value = v;
        }
        cross_button(ui, "Retirer cette condition")
    }

    fn step_action(&mut self, ui: &mut egui::Ui, idx: usize, accent: Color32) {
        let dur = self.motion_secs();
        step_title(ui, "Que faire de ces fichiers ?", "Choisissez une action. Rien n'est supprimé définitivement.");
        let current = self.rules[idx].actions.first().map(|a| a.kind.clone());
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(10.0, 10.0);
            for kind in SIMPLE_ACTIONS {
                let on = current.as_ref() == Some(kind);
                let (art, tint) = action_icon(kind);
                if choice_card(ui, art, tint, simple_action_label(kind), on, accent, dur).clicked() && !on {
                    let mut action = Action::new(kind.clone());
                    if matches!(kind, ActionKind::Move | ActionKind::Copy) {
                        action.params.insert(ON_CONFLICT_PARAM_KEY.into(), CONFLICT_NUMBER.into());
                    }
                    self.rules[idx].actions = vec![action];
                }
            }
        });
        let Some(kind) = self.rules[idx].actions.first().map(|a| a.kind.clone()) else { return };
        ui.add_space(14.0);
        // Les réglages de l'action apparaissent en fondu à chaque changement d'action.
        let appear = ui.ctx().animate_bool_with_time(egui::Id::new(("simple-action-params", self.rules[idx].id, kind.raw())), true, dur);
        ui.scope(|ui| {
            ui.set_opacity(appear);
            match kind {
                ActionKind::Move | ActionKind::Copy => self.destination_fields(ui, idx, accent),
                ActionKind::Rename => {
                    ui.label(caps("NOUVEAU NOM"));
                    self.param_field(ui, idx, "pattern", "{name}-{current_date}.{extension}");
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("{name} = nom actuel · {extension} = extension · {current_date} = date du jour.")
                            .size(13.5)
                            .color(text_dim()),
                    );
                }
                ActionKind::AddTag => {
                    ui.label(caps("ÉTIQUETTE"));
                    self.param_field(ui, idx, "tags", "ex. Factures");
                }
                ActionKind::Trash => {
                    ui.label(
                        RichText::new("Les fichiers vont dans la corbeille : vous pouvez les récupérer.").size(14.5).color(text_dim()),
                    );
                }
                ActionKind::Compress => {
                    ui.label(
                        RichText::new("Chaque fichier est compressé dans une archive ZIP, posée à côté de lui.")
                            .size(14.5)
                            .color(text_dim()),
                    );
                }
                _ => {}
            }
        });
    }

    fn param_field(&mut self, ui: &mut egui::Ui, idx: usize, key: &str, hint: &str) {
        let entry = self.rules[idx].actions[0].params.entry(key.to_string()).or_default();
        ui.add(
            egui::TextEdit::singleline(entry)
                .margin(egui::Margin::symmetric(8.0, 7.0))
                .desired_width((ui.available_width() - 24.0).max(120.0))
                .hint_text(hint),
        );
    }

    fn destination_fields(&mut self, ui: &mut egui::Ui, idx: usize, accent: Color32) {
        ui.label(caps("DOSSIER DE DESTINATION"));
        ui.horizontal(|ui| {
            let entry = self.rules[idx].actions[0].params.entry("destination".to_string()).or_default();
            ui.add(
                egui::TextEdit::singleline(entry)
                    .margin(egui::Margin::symmetric(8.0, 7.0))
                    .desired_width((ui.available_width() - 110.0).max(120.0))
                    .hint_text("ex. ~/Images"),
            );
            if btn(ui, "Choisir…", Btn::Ghost, accent, None).clicked() {
                if let Some(dir) = rfd::FileDialog::new().set_directory(home_dir()).pick_folder() {
                    entry.clear();
                    entry.push_str(&dir.to_string_lossy());
                }
            }
        });
        ui.add_space(10.0);
        ui.label(caps("SI UN FICHIER DU MÊME NOM EXISTE DÉJÀ LÀ-BAS"));
        ui.add_space(4.0);
        let stored = self.rules[idx].actions[0].param(ON_CONFLICT_PARAM_KEY);
        let current = CONFLICT_CHOICES.iter().position(|(v, _)| *v == stored).unwrap_or(1);
        let labels: Vec<&str> = CONFLICT_CHOICES.iter().map(|(_, l)| *l).collect();
        if let Some(i) = segmented(ui, egui::Id::new(("sconflict", self.rules[idx].id)), &labels, current, SKY_DEEP) {
            self.rules[idx].actions[0].params.insert(ON_CONFLICT_PARAM_KEY.into(), CONFLICT_CHOICES[i].0.into());
        }
    }
}

/// Tuile d'un dossier suivi (panneau de l'étape 1) : pastille, nom, chemin court, coche animée.
fn folder_tile(ui: &mut egui::Ui, name: &str, path: &str, selected: bool, accent: Color32, dur: f32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(FOLDER_TILE_W, FOLDER_TILE_H), egui::Sense::click());
    let ctx = ui.ctx().clone();
    let hover = ctx.animate_bool_with_time(resp.id.with("hover"), resp.hovered(), dur);
    let chosen = ctx.animate_bool_with_time(resp.id.with("chosen"), selected, dur);
    let painter = ui.painter();
    painter.rect_filled(rect, Rounding::same(12.0), glass((12.0 + 10.0 * hover) as u8));
    if chosen > 0.0 {
        painter.rect_filled(rect, Rounding::same(12.0), with_alpha(accent, (36.0 * chosen) as u8));
        painter.rect_stroke(rect, Rounding::same(12.0), Stroke::new(1.5_f32, with_alpha(accent, (220.0 * chosen) as u8)));
    } else {
        painter.rect_stroke(rect, Rounding::same(12.0), Stroke::new(1.0_f32, glass(22)));
    }
    let tile = Rect::from_min_size(rect.left_top() + Vec2::new(10.0, 10.0), Vec2::splat(38.0));
    radial_glow(painter, tile.center(), 26.0, if selected { accent } else { BLUE }, 60.0);
    paint_art(painter, &ctx, RuleArt::Folder, tile.center(), 42.0);
    let text_x = tile.right() + 12.0;
    let max_w = rect.right() - 44.0 - text_x;
    let font = egui::FontId::new(14.5, semibold());
    let name = ellipsize(painter, name, &font, max_w);
    painter.text(Pos2::new(text_x, rect.top() + 11.0), egui::Align2::LEFT_TOP, name, font, text_primary());
    let small = egui::FontId::proportional(12.5);
    let path = ellipsize(painter, path, &small, max_w);
    painter.text(Pos2::new(text_x, rect.top() + 32.0), egui::Align2::LEFT_TOP, path, small, text_dim());
    let c = Pos2::new(rect.right() - 22.0, rect.center().y);
    painter.circle_stroke(c, 10.0, Stroke::new(1.5_f32, with_alpha(text_dim(), 150)));
    if chosen > 0.0 {
        painter.circle_filled(c, 10.0 * chosen, accent);
        if chosen > 0.6 {
            draw_icon(painter, c, Icon::Tick, Color32::WHITE, 13.0);
        }
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Champ « nombre + unité » : renvoie la nouvelle valeur enregistrée (« 30 days », « 100 MB ») si elle change.
fn amount_editor(ui: &mut egui::Ui, id: &str, value: &str, units: &[(&str, &str)]) -> Option<String> {
    let (mut number, unit) = split_amount(value);
    let stored_unit = units.iter().find(|(raw, _)| raw.eq_ignore_ascii_case(unit.trim_end_matches('s')) || raw.eq_ignore_ascii_case(&unit));
    let (raw_unit, shown_unit) = stored_unit.copied().unwrap_or(units[0]);
    let mut out = None;
    if ui.add(egui::TextEdit::singleline(&mut number).margin(egui::Margin::symmetric(8.0, 7.0)).desired_width(70.0)).changed() {
        out = Some(format!("{} {raw_unit}", number.trim()));
    }
    let labels: Vec<&str> = units.iter().map(|(_, l)| *l).collect();
    if let Some(pick) = combo(ui, id, shown_unit, &labels) {
        if let Some((raw, _)) = units.iter().find(|(_, l)| *l == pick) {
            out = Some(format!("{} {raw}", number.trim()));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn folder(id: u64, path: &str) -> Folder {
        Folder { id, path: PathBuf::from(path), enabled: true, priority: 0 }
    }

    fn rule(conditions: Vec<Condition>, actions: Vec<Action>) -> Rule {
        Rule {
            id: 1,
            folder_ids: vec![1],
            name: "r".into(),
            enabled: true,
            match_all: true,
            recursion_depth: 0,
            priority: 0,
            conditions,
            actions,
        }
    }

    fn move_to(dest: &str) -> Action {
        let mut a = Action::new(ActionKind::Move);
        a.params.insert("destination".into(), dest.into());
        a
    }

    #[test]
    fn a_plain_rule_fits_the_simple_mode() {
        assert!(is_simple_compatible(&rule(vec![(PRESETS[0].make)()], vec![move_to("/x")])));
        assert!(is_simple_compatible(&rule(vec![], vec![])), "une règle vide se remplit dans l'assistant");
    }

    #[test]
    fn advanced_settings_keep_a_rule_out_of_the_simple_mode() {
        let base = || rule(vec![(PRESETS[0].make)()], vec![move_to("/x")]);
        let mut r = base();
        r.recursion_depth = 1;
        assert!(!is_simple_compatible(&r), "récursion");
        let mut r = base();
        r.conditions.push((PRESETS[1].make)());
        r.match_all = false;
        assert!(!is_simple_compatible(&r), "au moins une condition parmi plusieurs");
        let mut r = base();
        r.match_all = false;
        assert!(is_simple_compatible(&r), "avec une seule condition, le choix n'a pas d'effet");
        let mut r = base();
        r.conditions[0] = cond(Field::Name, Op::Matches, "^a");
        assert!(!is_simple_compatible(&r), "expression régulière");
        let mut r = base();
        r.actions.push(Action::new(ActionKind::Trash));
        assert!(!is_simple_compatible(&r), "plusieurs actions");
        let mut r = base();
        r.actions = vec![Action::new(ActionKind::Delete)];
        assert!(!is_simple_compatible(&r), "suppression définitive");
    }

    #[test]
    fn simple_actions_never_include_the_irreversible_ones() {
        assert!(!SIMPLE_ACTIONS.contains(&ActionKind::Delete));
        assert!(SIMPLE_ACTIONS.contains(&ActionKind::Trash));
    }

    #[test]
    fn simple_operators_drop_regular_expressions() {
        assert!(!simple_ops(Field::Name).contains(&Op::Matches));
        assert!(simple_ops(Field::Name).contains(&Op::Contains));
        assert!(!simple_ops(Field::Contents).contains(&Op::Matches));
    }

    #[test]
    fn every_illustration_is_a_valid_png() {
        for art in RuleArt::ALL {
            let img = image::load_from_memory(art.png()).unwrap_or_else(|e| panic!("{art:?} : {e}"));
            assert_eq!((img.width(), img.height()), (256, 256), "{art:?}");
        }
    }

    #[test]
    fn every_preset_and_simple_action_has_an_illustration() {
        for p in PRESETS {
            assert!(RuleArt::ALL.contains(&p.art), "{}", p.label);
        }
        for kind in SIMPLE_ACTIONS {
            assert!(RuleArt::ALL.contains(&action_icon(kind).0));
        }
    }

    #[test]
    fn every_preset_builds_a_condition_the_engine_understands() {
        for p in PRESETS {
            let c = (p.make)();
            assert!(c.field.ops().contains(&c.op), "{} : opérateur invalide", p.label);
            assert_ne!(c.op, Op::Matches);
        }
    }

    #[test]
    fn steps_say_what_is_missing() {
        let mut r = rule(vec![], vec![]);
        assert!(step_ready(1, &r).is_ok());
        r.folder_ids.clear();
        assert_eq!(step_ready(1, &r), Err("Choisissez au moins un dossier."));
        assert_eq!(step_ready(2, &r), Err("Ajoutez au moins une condition."));
        r.conditions.push(cond(Field::Name, Op::Contains, "  "));
        assert_eq!(step_ready(2, &r), Err("Complétez chaque condition."));
        r.conditions[0].value = "facture".into();
        assert!(step_ready(2, &r).is_ok());
        r.conditions.push(cond(Field::Size, Op::Greater, "beaucoup"));
        assert_eq!(step_ready(2, &r), Err("Une valeur n'est pas comprise : vérifiez-la."));
        r.conditions[1].value = "50 MB".into();
        assert!(step_ready(2, &r).is_ok());
        assert_eq!(step_ready(3, &r), Err("Choisissez ce qu'il faut faire des fichiers."));
        r.actions = vec![Action::new(ActionKind::Move)];
        assert_eq!(step_ready(3, &r), Err("Choisissez le dossier de destination."));
        r.actions = vec![move_to("/Images")];
        assert!(step_ready(3, &r).is_ok());
        r.actions = vec![Action::new(ActionKind::Trash)];
        assert!(step_ready(3, &r).is_ok(), "la corbeille n'a pas de réglage");
    }

    #[test]
    fn amounts_split_into_number_and_unit() {
        assert_eq!(split_amount("30 days"), ("30".into(), "days".into()));
        assert_eq!(split_amount("100MB"), ("100".into(), "MB".into()));
        assert_eq!(split_amount("1.5 gb"), ("1.5".into(), "gb".into()));
        assert_eq!(split_amount(""), (String::new(), String::new()));
    }

    #[test]
    fn the_sentence_reads_like_french() {
        let folders = vec![folder(1, "/Users/a/Téléchargements"), folder(2, "/Users/a/Bureau")];
        let r = rule(vec![(PRESETS[0].make)()], vec![move_to("/Users/a/Images")]);
        assert_eq!(
            rule_sentence(&r, &folders),
            "Dans Téléchargements, quand un fichier est une image, le déplacer vers « /Users/a/Images »."
        );
        let mut r = rule(vec![(PRESETS[6].make)(), (PRESETS[8].make)()], vec![Action::new(ActionKind::Trash)]);
        r.folder_ids = vec![1, 2];
        assert_eq!(
            rule_sentence(&r, &folders),
            "Dans les dossiers Téléchargements et Bureau, quand un fichier a été modifié il y a plus de 30 jours et pèse plus de 100 MB, le mettre à la corbeille."
        );
    }

    #[test]
    fn an_unfinished_rule_shows_dots_instead_of_panicking() {
        let r = rule(vec![], vec![]);
        assert_eq!(rule_sentence(&r, &[folder(1, "/a/Docs")]), "Dans Docs, quand un fichier …, ….");
    }

    #[test]
    fn labels_are_friendly() {
        assert_eq!(simple_op_label(Field::Size, Op::Greater), "plus gros que");
        assert_eq!(simple_op_label(Field::DateModified, Op::NotWithinLast), "il y a plus de");
        assert_eq!(simple_op_label(Field::Name, Op::Contains), "contient");
        for kind in SIMPLE_ACTIONS {
            assert!(!simple_action_label(kind).is_empty());
        }
    }

    fn preset(label: &str) -> &'static Preset {
        PRESETS.iter().find(|p| p.label == label).unwrap_or_else(|| panic!("modèle introuvable : {label}"))
    }

    #[test]
    fn a_preset_card_lights_up_when_its_condition_is_present_and_toggles() {
        let mut r = rule(vec![], vec![]);
        let image = preset("C'est une image");
        assert!(!preset_active(&r, image));
        toggle_preset(&mut r, image);
        assert!(preset_active(&r, image) && r.conditions.len() == 1);
        assert!(!preset_active(&r, preset("C'est une vidéo")), "un autre type n'allume pas la carte");
        toggle_preset(&mut r, image);
        assert!(r.conditions.is_empty(), "un second clic retire la condition");
    }

    #[test]
    fn free_value_presets_stay_lit_whatever_the_value() {
        let mut r = rule(vec![], vec![]);
        toggle_preset(&mut r, preset("Plus gros que…"));
        r.conditions[0].value = "2 GB".into();
        assert!(preset_active(&r, preset("Plus gros que…")), "la taille choisie ne change rien à l'état de la carte");
        r.conditions.push(cond(Field::DateModified, Op::NotWithinLast, "6 months"));
        assert!(preset_active(&r, preset("Plus ancien que 30 jours")));
        toggle_preset(&mut r, preset("Plus gros que…"));
        assert_eq!(r.conditions.len(), 1, "seule la condition de taille est retirée");
    }

    #[test]
    fn every_preset_and_action_has_an_icon_and_a_distinct_label() {
        let mut labels: Vec<&str> = PRESETS.iter().map(|p| p.label).collect();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), PRESETS.len());
        for kind in SIMPLE_ACTIONS {
            let (_, tint) = action_icon(kind);
            assert_ne!(tint, Color32::TRANSPARENT);
        }
        for field in FIELDS {
            let (_, tint) = condition_icon(*field);
            assert_ne!(tint, Color32::TRANSPARENT);
        }
    }

    #[test]
    fn the_sentence_parts_highlight_the_key_words_and_rebuild_the_sentence() {
        let folders = vec![folder(1, "/Users/a/Téléchargements")];
        let r = rule(vec![(preset("C'est une image").make)()], vec![move_to("/Users/a/Images")]);
        let parts = rule_sentence_parts(&r, &folders);
        let highlighted: Vec<&str> = parts.iter().filter(|(_, key)| *key).map(|(t, _)| t.as_str()).collect();
        assert_eq!(highlighted, ["Téléchargements", "est une image", "le déplacer vers « /Users/a/Images »"]);
        let joined: String = parts.iter().map(|(t, _)| t.as_str()).collect();
        assert_eq!(joined, rule_sentence(&r, &folders));
    }

    #[test]
    fn spans_are_translated() {
        assert_eq!(span_fr("1 week"), "1 semaine");
        assert_eq!(span_fr("2 weeks"), "2 semaines");
        assert_eq!(span_fr("3 months"), "3 mois");
        assert_eq!(span_fr("1 year"), "1 an");
    }
}
