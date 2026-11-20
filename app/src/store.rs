//! Sauvegarde et chargement de l'état de l'appli : un fichier JSON, écrit de façon atomique
//! (fichier temporaire puis renommage) pour ne jamais laisser un fichier à moitié écrit.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::engine::{home_dir, Folder, HistoryEntry, Rule};

/// Version du format du fichier : à incrémenter si la structure change, avec une migration.
pub const SCHEMA_VERSION: u32 = 1;

/// Réglages choisis par l'utilisateur.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Settings {
    pub accent: usize,
    pub menu_bar: bool,
    pub notify: bool,
    pub animate_bg: bool,
    pub retention_days: u32,
    pub app_enabled: bool,
    /// Lancer l'appli à l'ouverture de session (absent des anciens fichiers : faux).
    #[serde(default)]
    pub launch_at_login: bool,
    /// La carte de bienvenue (explication des autorisations macOS) a déjà été montrée.
    #[serde(default)]
    pub welcome_seen: bool,
    /// L'écran d'introduction a déjà été montré (absent des anciens fichiers : on le considère comme vu).
    #[serde(default = "default_true")]
    pub intro_seen: bool,
    /// Jouer les sons de l'appli (absent des anciens fichiers : oui).
    #[serde(default = "default_true")]
    pub sounds: bool,
    /// Passage complet automatique (au lancement puis tous les N minutes) ; 0 = désactivé (absent des anciens fichiers).
    #[serde(default)]
    pub auto_organize_minutes: u32,
    /// Présentation de l'écran Règles (absent des anciens fichiers : avancé, pour ne rien changer aux habitués).
    #[serde(default)]
    pub rules_mode: RulesMode,
    /// Chercher les nouvelles versions au lancement puis une fois par jour (absent des anciens fichiers : oui).
    #[serde(default = "default_true")]
    pub check_updates: bool,
}

/// Présentation de l'écran Règles : assistant en trois étapes ou écran complet.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RulesMode {
    Simple,
    #[default]
    Advanced,
}

fn default_true() -> bool {
    true
}

/// Tout ce qui doit survivre à la fermeture de l'appli.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct State {
    pub schema_version: u32,
    pub folders: Vec<Folder>,
    pub rules: Vec<Rule>,
    pub history: Vec<HistoryEntry>,
    pub settings: Settings,
    pub next_id: u64,
    pub next_batch: u64,
    pub monthly_used: u32,
    pub last_run: Option<(usize, usize, usize)>,
    pub last_run_dry: bool,
}

/// Résultat d'une tentative de chargement.
pub enum Loaded {
    /// Aucun fichier : premier lancement.
    Fresh,
    Restored(Box<State>),
    /// Fichier illisible ou d'une version plus récente : mis de côté, l'appli démarre vide.
    Unreadable {
        backup: PathBuf,
        reason: String,
    },
}

/// Dossier de données de l'appli (`ORGANIZEMYFILES_STATE_DIR` permet de le rediriger, par exemple pour les tests).
pub fn state_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("ORGANIZEMYFILES_STATE_DIR") {
        return PathBuf::from(dir);
    }
    let home = PathBuf::from(home_dir());
    if cfg!(target_os = "macos") {
        home.join("Library/Application Support/OrganizeMyFiles")
    } else if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        PathBuf::from(xdg).join("OrganizeMyFiles")
    } else {
        home.join(".local/share/OrganizeMyFiles")
    }
}

pub fn state_path() -> PathBuf {
    state_dir().join("state.json")
}

pub fn to_json(state: &State) -> String {
    serde_json::to_string_pretty(state).unwrap_or_default()
}

/// Met le fichier de côté sous `state.corrupt-<horodatage>.json` ; renvoie son nouveau chemin.
fn quarantine(path: &Path) -> PathBuf {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let backup = path.with_file_name(format!("state.corrupt-{stamp}.json"));
    match fs::rename(path, &backup) {
        Ok(()) => backup,
        Err(_) => path.to_path_buf(),
    }
}

pub fn load(path: &Path) -> Loaded {
    let Ok(text) = fs::read_to_string(path) else {
        return if path.exists() {
            Loaded::Unreadable { backup: quarantine(path), reason: "fichier illisible".into() }
        } else {
            Loaded::Fresh
        };
    };
    let value: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => return Loaded::Unreadable { backup: quarantine(path), reason: format!("JSON invalide ({e})") },
    };
    let version = value.get("schema_version").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    if version == 0 || version > SCHEMA_VERSION {
        return Loaded::Unreadable {
            backup: quarantine(path),
            reason: format!("version {version} non prise en charge (attendue : {SCHEMA_VERSION})"),
        };
    }
    match serde_json::from_value::<State>(value) {
        Ok(state) => Loaded::Restored(Box::new(state)),
        Err(e) => Loaded::Unreadable { backup: quarantine(path), reason: format!("contenu inattendu ({e})") },
    }
}

/// Écrit `json` dans `path` de façon atomique : fichier temporaire, synchronisation disque, renommage.
pub fn save(path: &Path, json: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(json.as_bytes())?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Action, ActionKind, Condition, Field, Op, Status, Undo};
    use std::time::Duration;

    fn temp_dir(tag: &str) -> PathBuf {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
        let dir = std::env::temp_dir().join(format!("organizemyfiles-store-{tag}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sample_state() -> State {
        let mut action = Action::new(ActionKind::Move);
        action.params.insert("destination".into(), "~/Pictures/Triées".into());
        action.params.insert("on_conflict".into(), "number".into());
        State {
            schema_version: SCHEMA_VERSION,
            folders: vec![Folder { id: 1, path: PathBuf::from("/tmp/Téléchargements"), enabled: true, priority: 0 }],
            rules: vec![Rule {
                id: 2,
                folder_ids: vec![1],
                name: "Tri des images".into(),
                enabled: true,
                match_all: false,
                recursion_depth: 2,
                priority: 0,
                conditions: vec![Condition { field: Field::Extension, op: Op::Equals, value: ".png, .jpg".into() }],
                actions: vec![action],
            }],
            history: vec![HistoryEntry {
                batch_id: 7,
                rule_name: "Tri des images".into(),
                action_label: "Déplacer".into(),
                original_path: PathBuf::from("/tmp/a.png"),
                result_path: PathBuf::from("/tmp/b/a.png"),
                undo: Some(Undo { kind: ActionKind::Move, from: PathBuf::from("/tmp/b/a.png"), to: PathBuf::from("/tmp/a.png") }),
                reversible: true,
                status: Status::Applied,
                message: "déplacé".into(),
                created: UNIX_EPOCH + Duration::from_secs(1_700_000_000),
            }],
            settings: Settings {
                accent: 3,
                menu_bar: false,
                notify: true,
                animate_bg: false,
                retention_days: 45,
                app_enabled: false,
                launch_at_login: true,
                welcome_seen: true,
                intro_seen: true,
                sounds: true,
                auto_organize_minutes: 60,
                rules_mode: RulesMode::Simple,
                check_updates: false,
            },
            next_id: 9,
            next_batch: 8,
            monthly_used: 42,
            last_run: Some((12, 18, 30)),
            last_run_dry: false,
        }
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = temp_dir("roundtrip");
        let path = dir.join("state.json");
        let state = sample_state();
        let json = to_json(&state);
        save(&path, &json).unwrap();
        assert!(!path.with_extension("json.tmp").exists(), "le fichier temporaire doit disparaître");
        match load(&path) {
            Loaded::Restored(back) => assert_eq!(to_json(&back), json),
            _ => panic!("l'état aurait dû être restauré"),
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_file_is_a_fresh_start() {
        let dir = temp_dir("fresh");
        assert!(matches!(load(&dir.join("state.json")), Loaded::Fresh));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_file_is_set_aside_not_deleted() {
        let dir = temp_dir("corrupt");
        let path = dir.join("state.json");
        fs::write(&path, "{ pas du json").unwrap();
        match load(&path) {
            Loaded::Unreadable { backup, .. } => {
                assert!(!path.exists());
                assert_eq!(fs::read_to_string(backup).unwrap(), "{ pas du json");
            }
            _ => panic!("un fichier corrompu doit être mis de côté"),
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn newer_schema_is_never_overwritten() {
        let dir = temp_dir("newer");
        let path = dir.join("state.json");
        let mut value: serde_json::Value = serde_json::from_str(&to_json(&sample_state())).unwrap();
        value["schema_version"] = serde_json::json!(SCHEMA_VERSION + 1);
        fs::write(&path, value.to_string()).unwrap();
        assert!(matches!(load(&path), Loaded::Unreadable { .. }));
        assert!(fs::read_dir(&dir).unwrap().flatten().any(|e| e.file_name().to_string_lossy().starts_with("state.corrupt-")));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_replaces_the_previous_file_completely() {
        let dir = temp_dir("replace");
        let path = dir.join("state.json");
        save(&path, &"x".repeat(5000)).unwrap();
        save(&path, "{}").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "{}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn old_settings_without_launch_at_login_still_load() {
        let json = r#"{"accent":1,"menu_bar":true,"notify":true,"animate_bg":true,"retention_days":30,"app_enabled":true}"#;
        let s: Settings = serde_json::from_str(json).unwrap();
        assert!(!s.launch_at_login);
        assert!(!s.welcome_seen);
        assert!(s.intro_seen, "un ancien fichier ne remontre pas l'introduction");
        assert!(s.sounds, "un ancien fichier garde les sons activés");
        assert_eq!(s.rules_mode, RulesMode::Advanced, "un ancien fichier garde l'écran Règles complet");
        assert!(s.check_updates, "un ancien fichier cherche les mises à jour");
    }
}
