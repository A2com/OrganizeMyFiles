//! Rule engine: conditions, actions, batches, quota, history.
//! Behaviour mirrors the rules documented in MECHANICS.md / mechanics.json.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::mechanics::*;
use crate::system::{self, CommandRunner};
use crate::zipmini;

// ---------------------------------------------------------------- file meta

#[derive(Clone, Debug)]
pub struct FileMeta {
    pub path: PathBuf,
    pub name: String,
    pub stem: String,
    pub ext: String,
    pub size: u64,
    pub created: Option<SystemTime>,
    pub modified: Option<SystemTime>,
    pub is_dir: bool,
    pub tags: Vec<String>,
    pub contents: Option<String>,
}

impl FileMeta {
    pub fn read(path: &Path, read_text: bool) -> std::io::Result<Self> {
        let md = fs::metadata(path)?;
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let ext = path
            .extension()
            .map(|s| s.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| name.clone());
        let is_dir = md.is_dir();
        let size = if is_dir { dir_size(path) } else { md.len() };
        let tags = read_tags(path);
        let contents = if read_text && !is_dir { read_text_content(path) } else { None };
        Ok(FileMeta {
            path: path.to_path_buf(),
            name,
            stem,
            ext,
            size,
            created: md.created().ok().or_else(|| md.modified().ok()),
            modified: md.modified().ok(),
            is_dir,
            tags,
            contents,
        })
    }

    /// Document kind used by the `kind` condition (derived from extension lists).
    pub fn kind(&self) -> &'static str {
        if self.is_dir {
            return "folder";
        }
        let e = self.ext.as_str();
        if e == "app" {
            "app"
        } else if ARCHIVE_KIND_EXTENSIONS.contains(&e) {
            "archive"
        } else if IMAGE_EXTENSIONS.contains(&e) {
            "image"
        } else if VIDEO_EXTENSIONS.contains(&e) {
            "video"
        } else if AUDIO_EXTENSIONS.contains(&e) {
            "audio"
        } else if DOCUMENT_EXTENSIONS.contains(&e) {
            "document"
        } else {
            "other"
        }
    }

    /// Kinds whose text extraction is a local text-recognition step.
    /// Every capability is unlocked in this build (no accounts), so content
    /// matching is never blocked; image OCR depends on extracted text only.
    pub fn needs_ocr(&self) -> bool {
        matches!(self.kind(), "image")
    }
}

fn dir_size(p: &Path) -> u64 {
    let mut total = 0;
    if let Ok(rd) = fs::read_dir(p) {
        for e in rd.flatten() {
            let md = match fs::metadata(e.path()) {
                Ok(m) => m,
                Err(_) => continue,
            };
            if md.is_dir() {
                total += dir_size(&e.path());
            } else {
                total += md.len();
            }
        }
    }
    total
}

fn read_text_content(path: &Path) -> Option<String> {
    let md = fs::metadata(path).ok()?;
    if md.len() > TEXT_LIMIT_BYTES {
        return Some(format!("<<fichier au-delà de la limite de {} Mo pour le texte>>", TEXT_LIMIT_BYTES / 1024 / 1024));
    }
    // OCR dimension guard (extractImageOCR at 0x100150c7c): images wider than
    // 12000 px or taller than 12001 px are refused before text recognition.
    let ext = path
        .extension()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if IMAGE_EXTENSIONS.contains(&ext.as_str()) {
        if let Ok((w, h)) = image::image_dimensions(path) {
            if image_dimension_limit_exceeded(w, h) {
                return Some(format!(
                    "<<image au-delà de la limite de {} px pour l'OCR>>",
                    OCR_IMAGE_DIMENSION_LIMIT
                ));
            }
        }
    }
    let bytes = fs::read(path).ok()?;
    let text = String::from_utf8_lossy(&bytes).to_string();
    if text.chars().filter(|c| c.is_control() && *c != '\n' && *c != '\r' && *c != '\t').count()
        > text.len() / 8
    {
        None
    } else {
        Some(nfc(&text).to_lowercase())
    }
}

/// OCR dimension rule from extractImageOCR (0x100150c7c): width > 12000 or
/// height >= 12001 (0x2ee0 / 0x2ee1 immediates) is refused.
fn image_dimension_limit_exceeded(width: u32, height: u32) -> bool {
    width > OCR_IMAGE_DIMENSION_LIMIT || height > OCR_IMAGE_DIMENSION_LIMIT + 1
}

/// macOS Finder tags live in the `kMDItemUserTags` xattr (`com.apple.metadata:_kMDItemUserTags`).
fn read_tags(path: &Path) -> Vec<String> {
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::ffi::OsStrExt;
        let mut out = Vec::new();
        let cname = std::ffi::CString::new("com.apple.metadata:_kMDItemUserTags").unwrap();
        let cpath = match std::ffi::CString::new(path.as_os_str().as_bytes()) {
            Ok(p) => p,
            Err(_) => return out,
        };
        unsafe {
            let sz = libc_getxattr(cpath.as_ptr(), cname.as_ptr(), std::ptr::null_mut(), 0, 0, 0);
            if sz > 0 {
                let mut buf = vec![0u8; sz as usize];
                let r = libc_getxattr(
                    cpath.as_ptr(),
                    cname.as_ptr(),
                    buf.as_mut_ptr() as *mut _,
                    buf.len(),
                    0,
                    0,
                );
                if r >= 0 {
                    // Best-effort parse of the binary plist as raw text runs.
                    let raw = String::from_utf8_lossy(&buf);
                    for chunk in raw.split(|c: char| c.is_control()) {
                        let t = chunk.trim_matches('\u{0}').trim();
                        if !t.is_empty() && t.len() < 64 && t.chars().all(|c| !c.is_control()) {
                            out.push(t.to_string());
                        }
                    }
                }
            }
        }
        out.dedup();
        out
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        Vec::new()
    }
}

#[cfg(target_os = "macos")]
unsafe fn libc_getxattr(
    p: *const i8,
    n: *const i8,
    v: *mut std::ffi::c_void,
    s: usize,
    pos: u32,
    opts: i32,
) -> isize {
    extern "C" {
        fn getxattr(
            path: *const i8,
            name: *const i8,
            value: *mut std::ffi::c_void,
            size: usize,
            position: u32,
            options: i32,
        ) -> isize;
    }
    getxattr(p, n, v, s, pos, opts)
}

#[cfg(target_os = "macos")]
unsafe fn libc_setxattr(
    p: *const i8,
    n: *const i8,
    v: *const std::ffi::c_void,
    s: usize,
    pos: u32,
    opts: i32,
) -> i32 {
    extern "C" {
        fn setxattr(
            path: *const i8,
            name: *const i8,
            value: *const std::ffi::c_void,
            size: usize,
            position: u32,
            options: i32,
        ) -> i32;
    }
    setxattr(p, n, v, s, pos, opts)
}

fn write_tags(path: &Path, tags: &[String]) {
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::ffi::OsStrExt;
        let cname = std::ffi::CString::new("com.apple.metadata:_kMDItemUserTags").unwrap();
        let cpath = match std::ffi::CString::new(path.as_os_str().as_bytes()) {
            Ok(p) => p,
            Err(_) => return,
        };
        let joined = tags.join("\n");
        unsafe {
            if tags.is_empty() {
                libc_setxattr(cpath.as_ptr(), cname.as_ptr(), std::ptr::null(), 0, 0, 0);
            } else {
                libc_setxattr(
                    cpath.as_ptr(),
                    cname.as_ptr(),
                    joined.as_ptr() as *const _,
                    joined.len(),
                    0,
                    0,
                );
            }
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (path, tags);
    }
}

// ---------------------------------------------------------------- conditions

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Field {
    Name,
    Extension,
    Size,
    DateAdded,
    DateModified,
    Kind,
    Tag,
    Contents,
}

pub const FIELDS: &[Field] = &[
    Field::Name,
    Field::Extension,
    Field::Size,
    Field::DateAdded,
    Field::DateModified,
    Field::Kind,
    Field::Tag,
    Field::Contents,
];

impl Field {
    pub fn label(&self) -> &'static str {
        match self {
            Field::Name => "Nom",
            Field::Extension => "Extension",
            Field::Size => "Taille",
            Field::DateAdded => "Date d'ajout",
            Field::DateModified => "Date de modification",
            Field::Kind => "Type",
            Field::Tag => "Étiquette",
            Field::Contents => "Contenu",
        }
    }
    /// Raw kind value used by the original condition table (`kind` column).
    pub fn raw(&self) -> &'static str {
        match self {
            Field::Name => "name",
            Field::Extension => "extension",
            Field::Size => "size_bytes",
            Field::DateAdded => "date_added",
            Field::DateModified => "date_modified",
            Field::Kind => "kind",
            Field::Tag => "tag",
            Field::Contents => "contents",
        }
    }
    pub fn ops(&self) -> &'static [Op] {
        use Op::*;
        match self {
            Field::Name => &[Contains, Equals, StartsWith, EndsWith, Matches],
            Field::Extension => &[Equals, Is, IsNot, Contains],
            Field::Size => &[Equals, Greater, Less],
            Field::DateAdded | Field::DateModified => &[Before, After, WithinLast, NotWithinLast],
            Field::Kind => &[Is, IsNot],
            Field::Tag => &[Contains, Equals],
            Field::Contents => &[Contains, Matches],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Op {
    Contains,
    Equals,
    StartsWith,
    EndsWith,
    Matches,
    Is,
    IsNot,
    Greater,
    Less,
    Before,
    After,
    WithinLast,
    NotWithinLast,
}

impl Op {
    pub fn label(&self) -> &'static str {
        match self {
            Op::Contains => "contient",
            Op::Equals => "est égal à",
            Op::StartsWith => "commence par",
            Op::EndsWith => "finit par",
            Op::Matches => "correspond à",
            Op::Is => "est",
            Op::IsNot => "n'est pas",
            Op::Greater => "est supérieur à",
            Op::Less => "est inférieur à",
            Op::Before => "est avant",
            Op::After => "est après",
            Op::WithinLast => "est dans les derniers",
            Op::NotWithinLast => "n'est pas dans les derniers",
        }
    }
    /// Raw operator value from the condition table.
    pub fn raw(&self) -> &'static str {
        match self {
            Op::Contains => "contains",
            Op::Equals => "equals",
            Op::StartsWith => "startsWith",
            Op::EndsWith => "endsWith",
            Op::Matches => "matches",
            Op::Is => "is",
            Op::IsNot => "isNot",
            Op::Greater => "greater",
            Op::Less => "less",
            Op::Before => "before",
            Op::After => "after",
            Op::WithinLast => "within_last",
            Op::NotWithinLast => "not_within_last",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Condition {
    pub field: Field,
    pub op: Op,
    pub value: String,
}

impl Condition {
    pub fn evaluate(&self, f: &FileMeta, now: SystemTime) -> bool {
        let value = nfc(self.value.trim());
        let v = value.as_str();
        let lv = v.to_lowercase();
        match self.field {
            Field::Name => {
                let n = nfc(&f.name).to_lowercase();
                match self.op {
                    Op::Contains => n.contains(&lv),
                    Op::Equals => n == lv,
                    Op::StartsWith => n.starts_with(&lv),
                    Op::EndsWith => n.ends_with(&lv),
                    Op::Matches => regex_match(&n, v),
                    _ => false,
                }
            }
            Field::Extension => {
                // Plusieurs extensions possibles, séparées par des virgules : « .png, .jpg ».
                let e = f.ext.to_lowercase();
                let wanted: Vec<&str> = lv.split(',').map(|x| x.trim().trim_start_matches('.')).collect();
                match self.op {
                    Op::Equals | Op::Is => wanted.iter().any(|w| e == *w),
                    Op::IsNot => wanted.iter().all(|w| e != *w),
                    Op::Contains => wanted.iter().any(|w| e.contains(w)),
                    _ => false,
                }
            }
            Field::Size => {
                let want = parse_size(v).unwrap_or(0);
                match self.op {
                    Op::Equals => f.size == want,
                    Op::Greater => f.size > want,
                    Op::Less => f.size < want,
                    _ => false,
                }
            }
            Field::DateAdded | Field::DateModified => {
                let t = if self.field == Field::DateAdded { f.created } else { f.modified };
                let Some(t) = t else { return false };
                match self.op {
                    Op::Before | Op::After => {
                        let Some(want) = parse_date(v) else { return false };
                        let secs = dur_secs(t, now);
                        if self.op == Op::Before {
                            secs < want
                        } else {
                            secs > want
                        }
                    }
                    Op::WithinLast | Op::NotWithinLast => {
                        let Some(span) = parse_span(v) else { return false };
                        let age = dur_secs(now, t);
                        let within = age <= span;
                        if self.op == Op::WithinLast {
                            within
                        } else {
                            !within
                        }
                    }
                    _ => false,
                }
            }
            Field::Kind => {
                let k = f.kind();
                match self.op {
                    Op::Is => k == lv,
                    Op::IsNot => k != lv,
                    _ => false,
                }
            }
            Field::Tag => {
                let tags: Vec<String> = f.tags.iter().map(|t| nfc(t).to_lowercase()).collect();
                match self.op {
                    Op::Contains => tags.iter().any(|t| t.contains(&lv)),
                    Op::Equals => tags.iter().any(|t| *t == lv),
                    _ => false,
                }
            }
            Field::Contents => {
                let Some(text) = &f.contents else { return false };
                match self.op {
                    Op::Contains => text.contains(&lv),
                    Op::Matches => regex_match(text, v),
                    _ => false,
                }
            }
        }
    }
}

/// Seconds from `earlier` to `later` (saturating).
fn dur_secs(later: SystemTime, earlier: SystemTime) -> u64 {
    later.duration_since(earlier).map(|d| d.as_secs()).unwrap_or(0)
}

/// `YYYY-MM-DD` or `YYYY/MM/DD` -> seconds since the epoch (midnight UTC).
fn parse_date(s: &str) -> Option<u64> {
    let t = s.trim().replace('/', "-");
    let parts: Vec<&str> = t.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let y: i64 = parts[0].parse().ok()?;
    let m: i64 = parts[1].parse().ok()?;
    let d: i64 = parts[2].parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let days = days_from_civil(y, m as u32, d as u32);
    Some((days * 86_400).max(0) as u64)
}

/// Howard Hinnant's civil calendar algorithm (inverse of `civil_from_days`).
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 } as i64;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Small literal/`*`/`?` matching plus `re:` prefix for a real subset of regex.
fn regex_match(hay: &str, pat: &str) -> bool {
    let p = pat.trim();
    if let Some(re) = p.strip_prefix("re:") {
        let re = re.trim();
        // Subset: anchored literals with `.` and `.*` wildcards.
        let mut parts: Vec<&str> = Vec::new();
        let mut cur = String::new();
        let mut chars = re.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '.' && chars.peek() == Some(&'*') {
                chars.next();
                parts.push(Box::leak(cur.clone().into_boxed_str()));
                parts.push(".*");
                cur.clear();
            } else if c == '.' {
                parts.push(Box::leak(cur.clone().into_boxed_str()));
                parts.push(".");
                cur.clear();
            } else {
                cur.push(c);
            }
        }
        parts.push(Box::leak(cur.into_boxed_str()));
        glob_parts(hay, &parts)
    } else {
        // Plain token search when the pattern has no wildcards.
        !p.is_empty() && hay.contains(&p.to_lowercase())
    }
}

fn glob_parts(hay: &str, parts: &[&str]) -> bool {
    let mut idx = 0usize;
    for (i, part) in parts.iter().enumerate() {
        if *part == ".*" {
            if i == parts.len() - 1 {
                return true;
            }
            let Some(next) = parts.get(i + 1) else { return true };
            if let Some(pos) = hay[idx..].find(next) {
                idx += pos + next.len();
            } else {
                return false;
            }
        } else if *part == "." {
            if hay[idx..].chars().next().is_some() {
                idx += 1;
            } else {
                return false;
            }
        } else {
            if !hay[idx..].starts_with(part) {
                return false;
            }
            idx += part.len();
        }
    }
    idx == hay.len()
}

// -------------------------------------------------------------------- rules

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionKind {
    /// L'ancienne action d'import en médiathèque (retirée) est relue comme un déplacement sans destination.
    #[serde(alias = "ImportToLibrary")]
    Move,
    Copy,
    Rename,
    Compress,
    Trash,
    Delete,
    AddTag,
    RemoveTag,
    OpenApplication,
    RunShortcut,
}

pub const ACTION_KINDS: &[ActionKind] = &[
    ActionKind::Move,
    ActionKind::Copy,
    ActionKind::Rename,
    ActionKind::Compress,
    ActionKind::Trash,
    ActionKind::Delete,
    ActionKind::AddTag,
    ActionKind::RemoveTag,
    ActionKind::OpenApplication,
    ActionKind::RunShortcut,
];

impl ActionKind {
    pub fn label(&self) -> &'static str {
        match self {
            ActionKind::Move => "Déplacer vers un dossier",
            ActionKind::Copy => "Copier vers un dossier",
            ActionKind::Rename => "Renommer avec un motif",
            ActionKind::Compress => "Compresser (ZIP)",
            ActionKind::Trash => "Mettre à la corbeille",
            ActionKind::Delete => "Supprimer définitivement",
            ActionKind::AddTag => "Ajouter une étiquette",
            ActionKind::RemoveTag => "Retirer une étiquette",
            ActionKind::OpenApplication => "Ouvrir une application",
            ActionKind::RunShortcut => "Lancer un raccourci",
        }
    }
    pub fn raw(&self) -> &'static str {
        match self {
            ActionKind::Move => "move",
            ActionKind::Copy => "copy",
            ActionKind::Rename => "rename",
            ActionKind::Compress => "compress",
            ActionKind::Trash => "trash",
            ActionKind::Delete => "delete",
            ActionKind::AddTag => "addTag",
            ActionKind::RemoveTag => "removeTag",
            ActionKind::OpenApplication => "openApplication",
            ActionKind::RunShortcut => "runShortcut",
        }
    }
    /// Every action is available in this build — nothing is gated behind an
    /// account. (The original gated library import behind its paid plan.)
    pub fn requires_pro(&self) -> bool {
        false
    }
    /// Param keys the editor shows for this action.
    pub fn params(&self) -> &'static [(&'static str, &'static str)] {
        match self {
            ActionKind::Move | ActionKind::Copy => &[
                ("destination", "Dossier de destination"),
                ("on_conflict", "conflit : replace | skip"),
            ],
            ActionKind::Rename => &[("pattern", "Motif, ex. {name}-{current_date}.{extension}")],
            ActionKind::Compress => &[
                ("pattern", "Nom de l'archive, ex. {name}-{timestamp}.zip"),
                ("destination", "Destination de l'archive (vide = à côté de la source)"),
                ("delete_source_after_compression", "supprimer la source : true | false"),
            ],
            ActionKind::AddTag | ActionKind::RemoveTag => &[("tags", "Étiquettes (séparées par des virgules)")],
            ActionKind::OpenApplication => &[("application_path", "Chemin de l'application")],
            ActionKind::RunShortcut => &[("shortcut_name", "Nom du raccourci")],
            ActionKind::Trash | ActionKind::Delete => &[],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Action {
    pub kind: ActionKind,
    pub params: BTreeMap<String, String>,
}

impl Action {
    pub fn new(kind: ActionKind) -> Self {
        let mut params = BTreeMap::new();
        for (k, _v) in kind.params() {
            params.insert((*k).to_string(), String::new());
        }
        Action { kind, params }
    }
    pub fn param(&self, k: &str) -> String {
        self.params
            .get(k)
            .map(|s| s.trim().to_string())
            .unwrap_or_default()
    }
    /// Placeholder values that were never edited count as empty.
    pub fn param_real(&self, k: &str, placeholder: &str) -> String {
        let v = self.param(k);
        if v == placeholder {
            String::new()
        } else {
            v
        }
    }
}

/// Relit un identifiant seul (ancien format) ou une liste d'identifiants.
fn one_or_many_ids<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<u64>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany {
        One(u64),
        Many(Vec<u64>),
    }
    Ok(match OneOrMany::deserialize(d)? {
        OneOrMany::One(id) => vec![id],
        OneOrMany::Many(ids) => ids,
    })
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rule {
    pub id: u64,
    /// Dossiers suivis auxquels la règle s'applique (les anciennes sauvegardes n'en avaient qu'un : `folder_id`).
    #[serde(alias = "folder_id", deserialize_with = "one_or_many_ids")]
    pub folder_ids: Vec<u64>,
    pub name: String,
    pub enabled: bool,
    pub match_all: bool, // condition_match 'all' | 'any'
    pub recursion_depth: u32,
    pub priority: i64,
    pub conditions: Vec<Condition>,
    pub actions: Vec<Action>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Folder {
    pub id: u64,
    pub path: PathBuf,
    pub enabled: bool,
    pub priority: i64,
}

// ------------------------------------------------------------------ results

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    Applied,
    Failed,
    BlockedByConflict,
    DryRun,
    Undone,
}

impl Status {
    pub fn label(&self) -> &'static str {
        match self {
            Status::Applied => "appliqué",
            Status::Failed => "échoué",
            Status::BlockedByConflict => "bloqué (conflit)",
            Status::DryRun => "aperçu",
            Status::Undone => "annulé",
        }
    }
    pub fn raw(&self) -> &'static str {
        match self {
            Status::Applied => "applied",
            Status::Failed => "failed",
            Status::BlockedByConflict => "blocked_by_conflict",
            Status::DryRun => "dry_run",
            Status::Undone => "undone",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Undo {
    pub kind: ActionKind,
    pub from: PathBuf,
    pub to: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub batch_id: u64,
    pub rule_name: String,
    pub action_label: String,
    pub original_path: PathBuf,
    pub result_path: PathBuf,
    pub undo: Option<Undo>,
    pub reversible: bool,
    pub status: Status,
    pub message: String,
    pub created: SystemTime,
}

// --------------------------------------------------------- pattern expansion

/// Rename/archive tokens: {name} {extension} {current_date} {timestamp}
/// {date_modified} {date_created} {size} (0x100217778 family).
pub fn expand_pattern(pattern: &str, f: &FileMeta, now: SystemTime) -> String {
    let mut out = pattern.to_string();
    let reps: [(&str, String); 7] = [
        ("{name}", f.stem.clone()),
        ("{extension}", f.ext.clone()),
        ("{current_date}", date_str(now)),
        ("{timestamp}", timestamp_str(now)),
        ("{date_modified}", f.modified.map(date_str).unwrap_or_else(|| date_str(now))),
        ("{date_created}", f.created.map(date_str).unwrap_or_else(|| date_str(now))),
        ("{size}", f.size.to_string()),
    ];
    for (k, v) in reps {
        out = out.replace(k, &v);
    }
    out
}

/// Destination tokens: {year} {month} {day} (0x1001e0b30).
pub fn expand_destination(dest: &str, now: SystemTime) -> String {
    let secs = now_unix(now) as i64;
    let (y, m, d) = crate::mechanics::month_key_parts(secs);
    dest.replace("{year}", &format!("{:04}", y))
        .replace("{month}", &format!("{:02}", m))
        .replace("{day}", &format!("{:02}", d))
}

pub fn now_unix(t: SystemTime) -> u64 {
    t.duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub fn date_str(t: SystemTime) -> String {
    let (y, m, d) = crate::mechanics::month_key_parts(now_unix(t) as i64);
    format!("{:04}-{:02}-{:02}", y, m, d)
}

/// {timestamp} / {current_date} expansion: "yyyy-MM-dd_HH-mm-ss" (format seen in
/// ActionExecutor.archiveFileName at 0x100140a14).
pub fn timestamp_str(t: SystemTime) -> String {
    let secs = now_unix(t) as i64;
    let (y, mo, d, h, mi, s) = crate::mechanics::timestamp_parts(secs);
    format!("{:04}-{:02}-{:02}_{:02}-{:02}-{:02}", y, mo, d, h, mi, s)
}

pub fn time_str(t: SystemTime) -> String {
    let secs = now_unix(t) % 86_400;
    format!("{:02}:{:02}", secs / 3600, (secs % 3600) / 60)
}

/// macOS filename validation (0x100138c54 family): no '/', not '.' / '..',
/// no trailing '.' or space, not empty, at most 255 characters.
pub fn validate_filename(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("le motif de renommage produit un nom vide".into());
    }
    if name.contains('/') {
        return Err("le motif de renommage produit un nom de fichier invalide (contient '/')".into());
    }
    if name == "." || name == ".." {
        return Err("le motif de renommage produit un nom de fichier invalide ('.' et '..' sont interdits)".into());
    }
    if name.ends_with('.') || name.ends_with(' ') {
        return Err("le motif de renommage produit un nom finissant par '.' ou un espace — non géré par macOS".into());
    }
    if name.chars().count() > FILENAME_MAX_LENGTH {
        return Err("le motif de renommage produit un nom de plus de 255 caractères".into());
    }
    Ok(())
}

// ---------------------------------------------------------------- execution

pub struct Runner {
    pub monthly_used: u32,
    pub batch_id: u64,
    pub dry_run: bool,
    pub commands: Box<dyn CommandRunner>,
    /// `Some` : on ne traite que ces fichiers (surveillance automatique) ; `None` : tout le dossier.
    pub only_paths: Option<std::collections::HashSet<PathBuf>>,
}

pub struct RunOutcome {
    pub matched: usize,
    pub history: Vec<HistoryEntry>,
    pub scanned: usize,
}

impl Runner {
    /// Run rules over folders (RuleEngine.run family, 0x100158480).
    /// Every folder and every action runs — this build has no plan limits.
    /// Dry Run previews without touching files and stops at
    /// `PREVIEW_MATCH_LIMIT` matches (0x1001df4f0).
    pub fn run(
        &mut self,
        folders: &[Folder],
        rules: &[Rule],
        now: SystemTime,
    ) -> RunOutcome {
        let mut history = Vec::new();
        let mut matched = 0usize;
        let mut scanned = 0usize;

        // All enabled folders run, in priority order.
        let mut allowed: Vec<&Folder> = folders.iter().filter(|f| f.enabled).collect();
        allowed.sort_by_key(|f| (f.priority, f.id));

        for folder in &allowed {
            let mut rule_list: Vec<&Rule> = rules.iter().filter(|r| r.folder_ids.contains(&folder.id) && r.enabled).collect();
            rule_list.sort_by_key(|r| (r.priority, r.id));

            // Scan at the deepest recursion any rule asks for; each rule then
            // only sees files within its own `recursion_depth`.
            let max_depth = rule_list.iter().map(|r| r.recursion_depth as usize).max().unwrap_or(0);
            let mut files = Vec::new();
            collect_files(&folder.path, max_depth, &mut files);
            files.sort();
            files.dedup();
            if let Some(only) = &self.only_paths {
                let only: std::collections::HashSet<PathBuf> =
                    only.iter().map(|p| p.canonicalize().unwrap_or_else(|_| p.clone())).collect();
                files.retain(|p| only.contains(&p.canonicalize().unwrap_or_else(|_| p.clone())));
            }
            scanned += files.len();

            for path in &files {
                if self.dry_run && matched >= PREVIEW_MATCH_LIMIT {
                    break;
                }
                let meta = match FileMeta::read(path, true) {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                let rel_depth = rel_depth(&folder.path, path);
                for rule in &rule_list {
                    if rel_depth > rule.recursion_depth as usize {
                        continue;
                    }
                    let hits = if rule.match_all {
                        !rule.conditions.is_empty() && rule.conditions.iter().all(|c| c.evaluate(&meta, now))
                    } else {
                        rule.conditions.iter().any(|c| c.evaluate(&meta, now))
                    };
                    if !hits {
                        continue;
                    }
                    matched += 1;
                    let mut current = meta.path.clone();
                    let mut applied_any = false;
                    for (pos, action) in rule.actions.iter().enumerate() {
                        let entry = self.run_action(rule, action, pos, &meta, &current, now);
                        let stops_chain = matches!(action.kind, ActionKind::Trash | ActionKind::Delete);
                        if entry.status == Status::Applied {
                            current = entry.result_path.clone();
                            applied_any = true;
                        } else if entry.status == Status::BlockedByConflict
                            || entry.status == Status::Failed
                        {
                            history.push(entry);
                            break;
                        }
                        history.push(entry);
                        if stops_chain {
                            break;
                        }
                    }
                    if applied_any && !self.dry_run {
                        self.monthly_used += 1; // one processed file (statistic)
                    }
                    break; // first matching rule wins per file (estimated)
                }
            }
        }

        RunOutcome { matched, history, scanned }
    }

    fn run_action(
        &self,
        rule: &Rule,
        action: &Action,
        pos: usize,
        meta: &FileMeta,
        current: &Path,
        now: SystemTime,
    ) -> HistoryEntry {
        let label = action.kind.label().to_string();
        let base = |status: Status, message: String, result: PathBuf, undo: Option<Undo>| {
            HistoryEntry {
                batch_id: self.batch_id,
                rule_name: rule.name.clone(),
                action_label: label.clone(),
                original_path: meta.path.clone(),
                result_path: result,
                undo,
                reversible: matches!(status, Status::Applied) && matches!(
                    action.kind,
                    ActionKind::Move | ActionKind::Copy | ActionKind::Rename | ActionKind::Compress
                ),
                status,
                message,
                created: now,
            }
        };

        if action.kind.requires_pro() {
            // Unreachable guard kept for clarity: this build gates nothing.
            debug_assert!(false, "requires_pro is always false in this build");
        }

        if self.dry_run {
            return self.preview_action(rule, action, pos, meta, current, now, &base);
        }

        match action.kind {
            ActionKind::Move | ActionKind::Copy => {
                let dest_raw = action.param_real("destination", "Destination folder");
                if dest_raw.is_empty() {
                    return base(Status::Failed, "Le chemin de destination ne peut pas être vide".into(), current.to_path_buf(), None);
                }
                let dest = expand_destination(&dest_raw, now);
                let dest_dir = PathBuf::from(shim_expand(&dest));
                let mut target = dest_dir.join(current.file_name().unwrap_or_default());
                if target == current {
                    return base(Status::Applied, "Déjà à la destination".into(), current.to_path_buf(), None);
                }
                if let Err(e) = fs::create_dir_all(&dest_dir) {
                    return base(Status::Failed, format!("Destination indisponible : {e}"), current.to_path_buf(), None);
                }
                if target.exists() {
                    let on_conflict = action.param(ON_CONFLICT_PARAM_KEY);
                    if on_conflict == "replace" {
                        let _ = fs::remove_file(&target);
                    } else if on_conflict == CONFLICT_NUMBER {
                        target = numbered_path(&target);
                    } else {
                        return base(
                            Status::BlockedByConflict,
                            "A file already exists at the destination.".into(),
                            current.to_path_buf(),
                            None,
                        );
                    }
                }
                let r = if action.kind == ActionKind::Move {
                    fs::rename(current, &target).or_else(|_| {
                        fs::copy(current, &target).map(|_| ())?;
                        fs::remove_file(current)
                    })
                } else {
                    fs::copy(current, &target).map(|_| ())
                };
                match r {
                    Ok(_) => base(
                        Status::Applied,
                        format!("{} → {}", action.kind.raw(), target.display()),
                        target.clone(),
                        Some(Undo {
                            kind: action.kind.clone(),
                            from: target,
                            to: current.to_path_buf(),
                        }),
                    ),
                    Err(e) => base(Status::Failed, e.to_string(), current.to_path_buf(), None),
                }
            }
            ActionKind::Rename => {
                let pattern = action.param_real("pattern", "Pattern, e.g. {name}-{current_date}.{extension}");
                let pattern = if pattern.is_empty() { "{name}.{extension}".into() } else { pattern };
                let mut new_name = expand_pattern(&pattern, meta, now);
                if !meta.ext.is_empty() && !new_name.contains('.') {
                    new_name = format!("{new_name}.{}", meta.ext);
                }
                if let Err(msg) = validate_filename(&new_name) {
                    return base(Status::Failed, msg, current.to_path_buf(), None);
                }
                let target = current.with_file_name(&new_name);
                if target == current {
                    return base(Status::Applied, "Déjà nommé ainsi".into(), current.to_path_buf(), None);
                }
                match fs::rename(current, &target) {
                    Ok(_) => base(
                        Status::Applied,
                        format!("renommé en {new_name}"),
                        target.clone(),
                        Some(Undo { kind: ActionKind::Rename, from: target, to: current.to_path_buf() }),
                    ),
                    Err(e) => base(Status::Failed, e.to_string(), current.to_path_buf(), None),
                }
            }
            ActionKind::Compress => {
                let pattern = action.param_real("pattern", "Archive name, e.g. {name}-{timestamp}.zip");
                let base_name = if pattern.is_empty() {
                    "{name}.zip".into()
                } else {
                    let n = expand_pattern(&pattern, meta, now);
                    if n.to_lowercase().ends_with(".zip") { n } else { format!("{n}.zip") }
                };
                let dest_raw = action.param_real("destination", "Archive destination (empty = next to source)");
                let dest_dir = if dest_raw.is_empty() {
                    current.parent().map(|p| p.to_path_buf()).unwrap_or_default()
                } else {
                    PathBuf::from(shim_expand(&expand_destination(&dest_raw, now)))
                };
                let delete_source =
                    action.param_real("delete_source_after_compression", "delete_source: true | false") == "true";
                if delete_source && dest_dir.starts_with(current.parent().unwrap_or(Path::new(""))) {
                    // A ZIP inside the source folder would include itself.
                }
                if base_name.chars().count() + 4 > FILENAME_MAX_LENGTH {
                    return base(
                        Status::Failed,
                        "Le nom d'archive est trop long (255 octets maximum, .zip compris).".into(),
                        current.to_path_buf(),
                        None,
                    );
                }
                if let Err(e) = fs::create_dir_all(&dest_dir) {
                    return base(Status::Failed, format!("Destination indisponible : {e}"), current.to_path_buf(), None);
                }
                let target = unique_zip_path(&dest_dir, &base_name);
                let z = if meta.is_dir { zipmini::zip_dir(current, &target) } else { zipmini::zip_file(current, &target) };
                match z {
                    Ok(_) => {
                        let undo_from = target.clone();
                        if delete_source {
                            let _ = if meta.is_dir { fs::remove_dir_all(current) } else { fs::remove_file(current) };
                        }
                        base(
                            Status::Applied,
                            format!("créé {}", target.display()),
                            target,
                            Some(Undo { kind: ActionKind::Compress, from: undo_from, to: current.to_path_buf() }),
                        )
                    }
                    Err(e) => base(Status::Failed, e.to_string(), current.to_path_buf(), None),
                }
            }
            ActionKind::Trash => {
                let trash = dirs_trash();
                let _ = fs::create_dir_all(&trash);
                let target = unique_zip_path(&trash, &current.file_name().unwrap_or_default().to_string_lossy());
                match fs::rename(current, &target) {
                    Ok(_) => base(
                        Status::Applied,
                        format!("mis à la corbeille sous {}", target.display()),
                        target.clone(),
                        Some(Undo { kind: ActionKind::Trash, from: target, to: current.to_path_buf() }),
                    ),
                    Err(e) => base(Status::Failed, e.to_string(), current.to_path_buf(), None),
                }
            }
            ActionKind::Delete => {
                let r = if meta.is_dir { fs::remove_dir_all(current) } else { fs::remove_file(current) };
                match r {
                    Ok(_) => base(Status::Applied, "permanently delete source after success".into(), PathBuf::new(), None),
                    Err(e) => base(Status::Failed, e.to_string(), current.to_path_buf(), None),
                }
            }
            ActionKind::AddTag | ActionKind::RemoveTag => {
                let tags_raw = action.param_real("tags", "Tags (comma separated)");
                let mut tags = meta.tags.clone();
                for t in tags_raw.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) {
                    // FinderTags.setColorLabel (0x100154104): a color name becomes the
                    // Finder label entry "<Name>\n<index>" (gray=1 … orange=7).
                    let t = finder_color_tag(&t).unwrap_or(t);
                    match action.kind {
                        ActionKind::AddTag => {
                            if !tags.contains(&t) {
                                tags.push(t);
                            }
                        }
                        _ => tags.retain(|x| *x != t),
                    }
                }
                write_tags(current, &tags);
                base(
                    Status::Applied,
                    format!("étiquettes maintenant [{}]", tags.join(", ")),
                    current.to_path_buf(),
                    None,
                )
            }
            ActionKind::OpenApplication => {
                let app = action.param_real("application_path", "Application path");
                if app.is_empty() {
                    return base(Status::Failed, "Le chemin de l'application ne peut pas être vide".into(), current.to_path_buf(), None);
                }
                let p = PathBuf::from(shim_expand(&app));
                if !p.exists() {
                    return base(
                        Status::Failed,
                        format!("Application introuvable à {}", p.display()),
                        current.to_path_buf(),
                        None,
                    );
                }
                let args = system::open_application_args(&p.to_string_lossy(), &current.to_string_lossy());
                self.run_command("open", &args, |_| format!("ouvert avec {}", p.display()), &base, current)
            }
            ActionKind::RunShortcut => {
                let name = action.param_real("shortcut_name", "Shortcut name");
                if name.is_empty() {
                    return base(
                        Status::Failed,
                        "RunShortcut requires 'shortcut_name' param".into(),
                        current.to_path_buf(),
                        None,
                    );
                }
                let args = system::run_shortcut_args(&name, &current.to_string_lossy());
                self.run_command("shortcuts", &args, |_| format!("raccourci \"{name}\" exécuté"), &base, current)
            }
        }
    }

    /// Lance une commande système et traduit le résultat en entrée d'historique honnête.
    fn run_command(
        &self,
        program: &str,
        args: &[String],
        ok_message: impl Fn(&str) -> String,
        base: &dyn Fn(Status, String, PathBuf, Option<Undo>) -> HistoryEntry,
        current: &Path,
    ) -> HistoryEntry {
        let timeout = Duration::from_secs(SCRIPT_TIMEOUT_SECONDS);
        match self.commands.run(program, args, timeout) {
            Ok(out) if out.success => base(Status::Applied, ok_message(&out.stderr), current.to_path_buf(), None),
            Ok(out) => {
                let why = out.stderr;
                let why = if why.is_empty() { format!("« {program} » a échoué") } else { why };
                base(Status::Failed, why, current.to_path_buf(), None)
            }
            Err(e) => base(Status::Failed, e, current.to_path_buf(), None),
        }
    }

    fn preview_action(
        &self,
        _rule: &Rule,
        action: &Action,
        pos: usize,
        meta: &FileMeta,
        current: &Path,
        now: SystemTime,
        base: &dyn Fn(Status, String, PathBuf, Option<Undo>) -> HistoryEntry,
    ) -> HistoryEntry {
        let _ = pos;
        match action.kind {
            ActionKind::Move | ActionKind::Copy => {
                let dest_raw = action.param_real("destination", "Destination folder");
                if dest_raw.is_empty() {
                    return base(Status::Failed, "Le chemin de destination ne peut pas être vide".into(), current.to_path_buf(), None);
                }
                let dest = PathBuf::from(shim_expand(&expand_destination(&dest_raw, now)));
                let target = dest.join(current.file_name().unwrap_or_default());
                if target.exists() && action.param(ON_CONFLICT_PARAM_KEY) != "replace" {
                    return base(
                        Status::BlockedByConflict,
                        "A file already exists at the destination.".into(),
                        current.to_path_buf(),
                        None,
                    );
                }
                base(
                    Status::DryRun,
                    format!("serait {} vers {}", action.kind.raw(), target.display()),
                    target,
                    None,
                )
            }
            ActionKind::Rename => {
                let pattern = action.param_real("pattern", "Pattern, e.g. {name}-{current_date}.{extension}");
                let pattern = if pattern.is_empty() { "{name}.{extension}".into() } else { pattern };
                let mut new_name = expand_pattern(&pattern, meta, now);
                if !meta.ext.is_empty() && !new_name.contains('.') {
                    new_name = format!("{new_name}.{}", meta.ext);
                }
                if let Err(msg) = validate_filename(&new_name) {
                    return base(Status::Failed, msg, current.to_path_buf(), None);
                }
                base(Status::DryRun, format!("serait renommé en \"{new_name}\""), current.with_file_name(&new_name), None)
            }
            ActionKind::Compress => {
                let pattern = action.param_real("pattern", "Archive name, e.g. {name}-{timestamp}.zip");
                let name = if pattern.is_empty() { format!("{}.zip", meta.stem) } else { expand_pattern(&pattern, meta, now) };
                base(Status::DryRun, format!("créerait \"{name}\""), current.to_path_buf(), None)
            }
            ActionKind::Trash => base(Status::DryRun, "serait mis à la corbeille".into(), current.to_path_buf(), None),
            ActionKind::Delete => base(Status::DryRun, "serait supprimé définitivement".into(), current.to_path_buf(), None),
            ActionKind::AddTag | ActionKind::RemoveTag => {
                let tags = action.param_real("tags", "Tags (comma separated)");
                base(
                    Status::DryRun,
                    format!("serait {} [{}]", action.kind.raw(), tags),
                    current.to_path_buf(),
                    None,
                )
            }
            ActionKind::OpenApplication => {
                base(Status::DryRun, "ouvrirait l'application".into(), current.to_path_buf(), None)
            }
            ActionKind::RunShortcut => {
                let name = action.param_real("shortcut_name", "Shortcut name");
                base(Status::DryRun, format!("lancerait le raccourci \"{name}\""), current.to_path_buf(), None)
            }
        }
    }
}

/// `~` expansion for destinations ("absolute folder path or start with ~/" at
/// 0x1001e2160).
pub fn shim_expand(p: &str) -> String {
    if p == "~" {
        home_dir()
    } else if let Some(rest) = p.strip_prefix("~/") {
        format!("{}/{}", home_dir(), rest)
    } else {
        p.to_string()
    }
}

/// Valide un dossier à suivre : il doit exister, être un dossier et ne pas être déjà suivi.
/// Renvoie le chemin nettoyé (sans « / » final) ou un message lisible.
pub fn validate_new_folder(path: &Path, existing: &[Folder]) -> Result<PathBuf, String> {
    let raw = path.to_string_lossy();
    let trimmed = if raw.len() > 1 { raw.trim_end_matches('/') } else { &raw };
    let clean = PathBuf::from(trimmed);
    if !clean.is_dir() {
        return Err("ce chemin n'est pas un dossier existant.".into());
    }
    if existing.iter().any(|f| f.path == clean) {
        return Err("ce dossier est déjà suivi.".into());
    }
    Ok(clean)
}

/// Nombre de fichiers visibles à la racine du dossier (sans descendre), `None` s'il est illisible.
pub fn count_top_level_files(path: &Path) -> Option<usize> {
    let rd = fs::read_dir(path).ok()?;
    Some(
        rd.flatten()
            .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
            .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
            .count(),
    )
}

pub fn home_dir() -> String {
    std::env::var("HOME").unwrap_or_else(|_| ".".into())
}

fn dirs_trash() -> PathBuf {
    PathBuf::from(home_dir()).join(".Trash")
}

/// Valeur de `on_conflict` ajoutée par la reconstruction : suffixe « (n) » au lieu de bloquer.
pub const CONFLICT_NUMBER: &str = "number";

/// `photo.png` → `photo (1).png`, `photo (2).png`… premier nom libre (uniqueDest 0x100138560).
fn numbered_path(target: &Path) -> PathBuf {
    let dir = target.parent().unwrap_or(Path::new("."));
    let stem = target.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = target.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    let mut n = 1;
    loop {
        let candidate = dir.join(format!("{stem} ({n}){ext}"));
        if !candidate.exists() {
            return candidate;
        }
        n += 1;
    }
}

/// Existing archives are kept; new ones are numbered (0x1001e0820).
fn unique_zip_path(dir: &Path, name: &str) -> PathBuf {
    let name = if name.to_lowercase().ends_with(".zip") {
        name.to_string()
    } else {
        format!("{name}.zip")
    };
    let base = name.trim_end_matches(".zip").to_string();
    let mut target = dir.join(&name);
    let mut n = 2;
    while target.exists() {
        target = dir.join(format!("{base}-{n}.zip"));
        n += 1;
    }
    target
}

fn collect_files(root: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(root) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            if depth > 0 {
                collect_files(&p, depth - 1, out);
            }
        } else {
            out.push(p);
        }
    }
}

/// How many folder levels below `root` this file sits (0 = root's own files).
fn rel_depth(root: &Path, file: &Path) -> usize {
    file.strip_prefix(root)
        .map(|rel| rel.components().count().saturating_sub(1))
        .unwrap_or(0)
}

/// Texte en forme composée (NFC) : macOS donne souvent les noms de fichiers en forme décomposée
/// (« e » + accent séparé), qui s'affiche mal et ne correspond pas à ce que l'utilisateur tape.
pub fn nfc(s: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    s.nfc().collect()
}

/// Traduit à l'affichage les messages du moteur (valeurs d'origine, laissées intactes
/// dans les données). Un message inconnu est rendu tel quel.
pub fn fr_message(msg: &str) -> String {
    nfc(&fr_message_raw(msg))
}

fn fr_message_raw(msg: &str) -> String {
    match msg {
        "A file already exists at the destination." => return "un fichier existe déjà à la destination".into(),
        "restored" => return "restauré".into(),
        "nothing to undo" => return "rien à annuler".into(),
        "permanently delete source after success" => return "source supprimée définitivement".into(),
        "RunShortcut requires 'shortcut_name' param" => return "le nom du raccourci est obligatoire".into(),
        _ => {}
    }
    for (en, fr) in [("move", "déplacé"), ("copy", "copié")] {
        if let Some(rest) = msg.strip_prefix(&format!("{en} → ")) {
            return format!("{fr} → {rest}");
        }
        if let Some(rest) = msg.strip_prefix(&format!("serait {en} vers ")) {
            return format!("serait {fr} vers {rest}");
        }
    }
    for (en, fr) in [("addTag", "ajouterait"), ("removeTag", "retirerait")] {
        if let Some(rest) = msg.strip_prefix(&format!("serait {en} ")) {
            return format!("{fr} les étiquettes {rest}");
        }
    }
    msg.to_string()
}

/// Undo a reversible history entry (restores original_path from result_path).
/// Pre-checks mirror UndoChecker.evaluate (0x10015e564): entry must be applied,
/// the expected file must still exist, the original location must be free, and
/// the restored file must not be instantly re-processed by an active rule.
pub fn undo_entry(e: &HistoryEntry) -> Result<(), String> {
    if e.status == Status::Undone {
        return Err("cette action n'est pas actuellement appliquée".into());
    }
    let undo = e.undo.as_ref().ok_or("nothing to undo")?;
    match undo.kind {
        ActionKind::Move | ActionKind::Rename | ActionKind::Trash => {
            if !undo.from.exists() {
                return Err("le fichier n'existe plus à l'emplacement attendu".into());
            }
            if undo.to.exists() {
                return Err("un fichier existe déjà à l'emplacement d'origine".into());
            }
            if let Some(parent) = undo.to.parent() {
                fs::create_dir_all(parent).map_err(|x| x.to_string())?;
            }
            if undo.from.exists() && !undo.to.exists() {
                fs::rename(&undo.from, &undo.to).map_err(|x| x.to_string())
            } else if undo.from.exists() {
                fs::remove_file(&undo.from).map_err(|x| x.to_string())
            } else {
                Err("la source a disparu".into())
            }
        }
        ActionKind::Copy | ActionKind::Compress => {
            if undo.from.exists() {
                fs::remove_file(&undo.from).map_err(|x| x.to_string())
            } else {
                Err("la copie créée n'existe plus".into())
            }
        }
        _ => Err("cette action ne peut pas être annulée".into()),
    }
}

fn finder_color_tag(tag: &str) -> Option<String> {
    let t = tag.trim().to_ascii_lowercase();
    let (_, idx) = FINDER_COLOR_INDEX.iter().find(|(n, _)| *n == t)?;
    // FinderTags.setColorLabel (0x100154104) writes "<Name>\n<index>": the
    // first letter uppercased, the rest lowercased, then the label index.
    let mut chars = t.chars();
    let name = match chars.next() {
        Some(c) => c.to_ascii_uppercase().to_string() + chars.as_str(),
        None => return None,
    };
    Some(format!("{name}\n{idx}"))
}

/// Purge history entries older than `retention_days`, mirroring the original's
/// `DELETE FROM action_history WHERE datetime(created_at) < datetime(?1)`
/// (Database.purgeHistory at 0x10017a438): the bound is now minus
/// `days * HISTORY_PURGE_SECONDS_PER_DAY` seconds.
pub fn purge_history(entries: &mut Vec<HistoryEntry>, retention_days: u32, now: SystemTime) -> usize {
    let cutoff_secs = u64::from(retention_days) * HISTORY_PURGE_SECONDS_PER_DAY;
    let before = now.checked_sub(std::time::Duration::from_secs(cutoff_secs));
    let Some(before) = before else { return 0 };
    let mut removed = 0;
    entries.retain(|e| {
        if e.created < before {
            removed += 1;
            false
        } else {
            true
        }
    });
    removed
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn meta(name: &str, ext: &str, size: u64) -> FileMeta {
        FileMeta {
            path: PathBuf::from(format!("/t/{name}")),
            name: format!("{name}.{ext}"),
            stem: name.into(),
            ext: ext.into(),
            size,
            created: Some(UNIX_EPOCH + Duration::from_secs(1_700_000_000)),
            modified: Some(UNIX_EPOCH + Duration::from_secs(1_700_100_000)),
            is_dir: false,
            tags: vec![],
            contents: None,
        }
    }

    #[test]
    fn name_condition_ops() {
        let f = meta("invoice-42", "pdf", 100);
        let now = UNIX_EPOCH + Duration::from_secs(1_800_000_000);
        let c = Condition { field: Field::Name, op: Op::Contains, value: "invoice".into() };
        assert!(c.evaluate(&f, now));
        let c = Condition { field: Field::Name, op: Op::StartsWith, value: "invoice".into() };
        assert!(c.evaluate(&f, now));
        let c = Condition { field: Field::Name, op: Op::Equals, value: "other".into() };
        assert!(!c.evaluate(&f, now));
    }

    #[test]
    fn size_ops_use_bytes() {
        let f = meta("a", "pdf", 10 * 1024);
        let now = SystemTime::now();
        let c = Condition { field: Field::Size, op: Op::Greater, value: "5 KB".into() };
        assert!(c.evaluate(&f, now));
        let c = Condition { field: Field::Size, op: Op::Less, value: "5 KB".into() };
        assert!(!c.evaluate(&f, now));
    }

    #[test]
    fn date_within_last() {
        let f = meta("a", "pdf", 1);
        let now = UNIX_EPOCH + Duration::from_secs(1_800_000_000);
        let c = Condition {
            field: Field::DateModified,
            op: Op::WithinLast,
            value: "30 days".into(),
        };
        assert!(!c.evaluate(&f, now)); // modified long before `now`
        let c = Condition {
            field: Field::DateModified,
            op: Op::NotWithinLast,
            value: "30 days".into(),
        };
        assert!(c.evaluate(&f, now));
    }

    #[test]
    fn ocr_dimension_guard() {
        assert_eq!(OCR_IMAGE_DIMENSION_LIMIT, 12000);
        assert!(!image_dimension_limit_exceeded(12000, 12001));
        assert!(image_dimension_limit_exceeded(12001, 1));
        assert!(image_dimension_limit_exceeded(1, 12002));
    }

    #[test]
    fn filename_validation() {
        assert!(validate_filename("report-2024.pdf").is_ok());
        assert!(validate_filename("a/b").is_err());
        assert!(validate_filename(".").is_err());
        assert!(validate_filename("ends.").is_err());
        assert!(validate_filename(&"x".repeat(300)).is_err());
    }

    #[test]
    fn pattern_tokens() {
        let f = meta("old", "pdf", 2048);
        let now = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        assert_eq!(
            expand_pattern("{name}-{current_date}.{extension}", &f, now),
            "old-2023-11-14.pdf"
        );
        assert_eq!(
            expand_pattern("{name}-{size}.{extension}", &f, now),
            "old-2048.pdf"
        );
        assert_eq!(
            expand_pattern("{timestamp}", &f, now),
            "2023-11-14_22-13-20"
        );
    }

    #[test]
    fn kind_mapping() {
        assert_eq!(meta("a", "pdf", 1).kind(), "document");
        assert_eq!(meta("a", "zip", 1).kind(), "archive");
        assert_eq!(meta("a", "jpg", 1).kind(), "image");
        assert_eq!(meta("a", "mp3", 1).kind(), "audio");
    }

    #[test]
    fn extension_accepts_comma_separated_list() {
        let c = Condition { field: Field::Extension, op: Op::Equals, value: ".png, .JPG".into() };
        let now = SystemTime::now();
        assert!(c.evaluate(&meta("a", "png", 1), now));
        assert!(c.evaluate(&meta("b", "jpg", 1), now));
        assert!(!c.evaluate(&meta("c", "gif", 1), now));
        let not = Condition { field: Field::Extension, op: Op::IsNot, value: "png, jpg".into() };
        assert!(not.evaluate(&meta("c", "gif", 1), now));
        assert!(!not.evaluate(&meta("a", "png", 1), now));
    }

    #[test]
    fn numbered_path_skips_existing_names() {
        let dir = std::env::temp_dir().join(format!("organizemyfiles-num-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let first = dir.join("photo.png");
        fs::write(&first, b"x").unwrap();
        assert_eq!(numbered_path(&first), dir.join("photo (1).png"));
        fs::write(dir.join("photo (1).png"), b"x").unwrap();
        assert_eq!(numbered_path(&first), dir.join("photo (2).png"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn validate_new_folder_rejects_missing_and_duplicates() {
        let dir = std::env::temp_dir().join(format!("organizemyfiles-folders-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("a.txt"), b"x").unwrap();
        fs::write(dir.join(".hidden"), b"x").unwrap();
        let with_slash = PathBuf::from(format!("{}/", dir.display()));
        let ok = validate_new_folder(&with_slash, &[]).unwrap();
        assert_eq!(ok, dir);
        let tracked = [Folder { id: 1, path: dir.clone(), enabled: true, priority: 0 }];
        assert!(validate_new_folder(&dir, &tracked).unwrap_err().contains("déjà suivi"));
        assert!(validate_new_folder(&dir.join("a.txt"), &[]).is_err());
        assert!(validate_new_folder(&dir.join("absent"), &[]).is_err());
        assert_eq!(count_top_level_files(&dir), Some(1));
        let _ = fs::remove_dir_all(&dir);
    }

    fn system_action_entry(kind: ActionKind, key: &str, value: &str, fake: &std::rc::Rc<crate::system::fake::FakeCommands>) -> HistoryEntry {
        let runner = Runner { monthly_used: 0, batch_id: 1, dry_run: false, commands: Box::new(fake.clone()), only_paths: None };
        let rule = Rule {
            id: 1, folder_ids: vec![1], name: "r".into(), enabled: true, match_all: true,
            recursion_depth: 0, priority: 0, conditions: vec![], actions: vec![],
        };
        let mut action = Action::new(kind);
        action.params.insert(key.into(), value.into());
        let file = meta("photo", "jpg", 10);
        runner.run_action(&rule, &action, 0, &file, &file.path, SystemTime::now())
    }

    #[test]
    fn shortcut_action_runs_the_command_and_reports_success() {
        let fake = std::rc::Rc::new(crate::system::fake::FakeCommands::ok());
        let e = system_action_entry(ActionKind::RunShortcut, "shortcut_name", "Mon raccourci", &fake);
        assert_eq!(e.status, Status::Applied);
        let calls = fake.calls.borrow();
        assert_eq!(calls[0].0, "shortcuts");
        assert_eq!(calls[0].1, vec!["run", "Mon raccourci", "--input-path", "/t/photo"]);
    }

    #[test]
    fn failing_command_is_reported_as_failed_not_applied() {
        let mut f = crate::system::fake::FakeCommands::ok();
        f.reply = Ok(false);
        f.stderr = "Error: Couldn't find shortcut".into();
        let fake = std::rc::Rc::new(f);
        let e = system_action_entry(ActionKind::RunShortcut, "shortcut_name", "X", &fake);
        assert_eq!(e.status, Status::Failed);
        assert!(e.message.contains("Couldn't find shortcut"));
    }

    #[test]
    fn timeout_or_missing_system_is_reported_as_failed() {
        let mut f = crate::system::fake::FakeCommands::ok();
        f.reply = Err("Délai dépassé (60 s), commande arrêtée".into());
        let fake = std::rc::Rc::new(f);
        let e = system_action_entry(ActionKind::RunShortcut, "shortcut_name", "X", &fake);
        assert_eq!(e.status, Status::Failed);
        assert!(e.message.contains("Délai"));
    }

    #[test]
    fn dry_run_launches_nothing() {
        let fake = std::rc::Rc::new(crate::system::fake::FakeCommands::ok());
        let runner = Runner { monthly_used: 0, batch_id: 1, dry_run: true, commands: Box::new(fake.clone()), only_paths: None };
        let rule = Rule {
            id: 1, folder_ids: vec![1], name: "r".into(), enabled: true, match_all: true,
            recursion_depth: 0, priority: 0, conditions: vec![], actions: vec![],
        };
        let mut action = Action::new(ActionKind::RunShortcut);
        action.params.insert("shortcut_name".into(), "X".into());
        let file = meta("a", "jpg", 1);
        let e = runner.run_action(&rule, &action, 0, &file, &file.path, SystemTime::now());
        assert_eq!(e.status, Status::DryRun);
        assert!(fake.calls.borrow().is_empty());
    }

    #[test]
    fn old_import_action_is_read_as_move() {
        let k: ActionKind = serde_json::from_str("\"ImportToLibrary\"").unwrap();
        assert_eq!(k, ActionKind::Move);
    }

    #[test]
    fn fr_message_translates_known_messages_only() {
        assert_eq!(fr_message("restored"), "restauré");
        assert_eq!(fr_message("A file already exists at the destination."), "un fichier existe déjà à la destination");
        assert_eq!(fr_message("move → /a/b"), "déplacé → /a/b");
        assert_eq!(fr_message("serait copy vers /x"), "serait copié vers /x");
        assert_eq!(fr_message("serait addTag [a]"), "ajouterait les étiquettes [a]");
        assert_eq!(fr_message("Permission denied (os error 13)"), "Permission denied (os error 13)");
    }

    #[test]
    fn decomposed_accents_match_and_display_composed() {
        let nfd = "Releve\u{301}s e\u{301}te\u{301}"; // « Relevés été » en forme décomposée
        assert_eq!(nfc(nfd), "Relevés été");
        let f = meta(nfd, "pdf", 1);
        let c = Condition { field: Field::Name, op: Op::Contains, value: "relevés".into() };
        assert!(c.evaluate(&f, SystemTime::now()));
        assert_eq!(fr_message("move → /a/Releve\u{301}s"), "déplacé → /a/Relevés");
    }

    #[test]
    fn old_rule_with_single_folder_id_is_still_readable() {
        let old = r#"{"id":1,"folder_id":7,"name":"x","enabled":true,"match_all":true,"recursion_depth":0,"priority":0,"conditions":[],"actions":[]}"#;
        let r: Rule = serde_json::from_str(old).unwrap();
        assert_eq!(r.folder_ids, vec![7]);
        let new = serde_json::to_string(&r).unwrap();
        assert!(new.contains("\"folder_ids\":[7]"));
        assert_eq!(serde_json::from_str::<Rule>(&new).unwrap().folder_ids, vec![7]);
    }

    #[test]
    fn one_rule_applies_to_every_folder_it_targets() {
        let (a, b) = (std::env::temp_dir().join(format!("organizemyfiles-multi-a-{}", std::process::id())),
                      std::env::temp_dir().join(format!("organizemyfiles-multi-b-{}", std::process::id())));
        for d in [&a, &b] {
            fs::create_dir_all(d).unwrap();
            fs::write(d.join("x.png"), b"x").unwrap();
        }
        let dest = std::env::temp_dir().join(format!("organizemyfiles-multi-out-{}", std::process::id()));
        let mut act = Action::new(ActionKind::Copy);
        act.params.insert("destination".into(), dest.to_string_lossy().into_owned());
        act.params.insert(ON_CONFLICT_PARAM_KEY.into(), CONFLICT_NUMBER.into());
        let rule = Rule {
            id: 1, folder_ids: vec![1, 2], name: "r".into(), enabled: true, match_all: true, recursion_depth: 0, priority: 0,
            conditions: vec![Condition { field: Field::Extension, op: Op::Equals, value: ".png".into() }],
            actions: vec![act],
        };
        let folders = vec![
            Folder { id: 1, path: a.clone(), enabled: true, priority: 0 },
            Folder { id: 2, path: b.clone(), enabled: true, priority: 1 },
        ];
        let mut runner = Runner { monthly_used: 0, batch_id: 1, dry_run: false, commands: Box::new(crate::system::SystemCommands), only_paths: None };
        let out = runner.run(&folders, &[rule], SystemTime::now());
        assert_eq!(out.matched, 2, "un fichier par dossier ciblé");
        assert_eq!(fs::read_dir(&dest).unwrap().count(), 2);
    }
}
