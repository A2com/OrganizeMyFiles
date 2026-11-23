//! Textes et données du menu de la barre des menus : fonctions pures, testées.
// Utilisées par le menu macOS ; ailleurs, seuls les tests les appellent.
#![cfg_attr(not(target_os = "macos"), allow(dead_code))]

use std::collections::HashSet;
use std::time::SystemTime;

use crate::engine::{HistoryEntry, Status};
use crate::history_view::ago;

#[derive(Clone, Debug, PartialEq)]
pub struct MenuFolder {
    pub id: u64,
    /// Nom court du dossier (déjà désambiguïsé).
    pub label: String,
    pub active: bool,
    pub denied: bool,
}

/// Tout ce que le menu affiche ; comparé d'une image à l'autre pour ne mettre à jour que ce qui change.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MenuInfo {
    pub enabled: bool,
    pub folders: Vec<MenuFolder>,
    pub rules_active: usize,
    pub last_batch: Option<String>,
}

pub fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n > 1 { many } else { one })
}

impl MenuInfo {
    pub fn folders_active(&self) -> usize {
        self.folders.iter().filter(|f| f.active).count()
    }
}

pub fn status_line(info: &MenuInfo) -> &'static str {
    if info.enabled {
        "Surveillance active"
    } else {
        "En veille"
    }
}

/// « 3 dossiers suivis · 5 règles actives » (avec « (1 en pause) » si besoin).
pub fn counts_line(info: &MenuInfo) -> String {
    let total = info.folders.len();
    let folders = match total {
        0 => "Aucun dossier suivi".to_string(),
        n => {
            let paused = n - info.folders_active();
            let base = plural(n, "dossier suivi", "dossiers suivis");
            if paused > 0 {
                format!("{base} ({paused} en pause)")
            } else {
                base
            }
        }
    };
    let rules = match info.rules_active {
        0 => "Aucune règle active".to_string(),
        n => plural(n, "règle active", "règles actives"),
    };
    format!("{folders} · {rules}")
}

pub fn folders_menu_title(info: &MenuInfo) -> String {
    format!("Dossiers suivis ({})", info.folders.len())
}

pub fn rules_menu_title(info: &MenuInfo) -> String {
    match info.rules_active {
        0 => "Règles".to_string(),
        n => format!("Règles ({})", plural(n, "active", "actives")),
    }
}

pub fn folder_entry_label(f: &MenuFolder) -> String {
    if f.denied {
        format!("{} (accès refusé)", f.label)
    } else if !f.active {
        format!("{} (en pause)", f.label)
    } else {
        f.label.clone()
    }
}

pub fn toggle_label(enabled: bool) -> &'static str {
    if enabled {
        "Mettre en veille"
    } else {
        "Reprendre la surveillance"
    }
}

pub fn tooltip(info: &MenuInfo) -> String {
    format!("OrganizeMyFiles · {} · {}", status_line(info), counts_line(info))
}

/// « Dernier rangement : il y a 5 min · 12 fichiers » d'après le dernier lot réel (hors aperçu et règle ignorée).
pub fn last_batch_line(history: &[HistoryEntry], now: SystemTime) -> Option<String> {
    let last = history.iter().rev().find(|e| !matches!(e.status, Status::DryRun | Status::Skipped))?;
    let files: HashSet<_> =
        history.iter().filter(|e| e.batch_id == last.batch_id && e.status == Status::Applied).map(|e| &e.original_path).collect();
    let what = match files.len() {
        0 => "aucun fichier rangé".to_string(),
        n => plural(n, "fichier", "fichiers"),
    };
    Some(format!("Dernier rangement : {} · {what}", ago(last.created, now)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::Duration;

    fn folder(id: u64, label: &str, active: bool) -> MenuFolder {
        MenuFolder { id, label: label.into(), active, denied: false }
    }

    fn entry(batch: u64, status: Status, file: &str, secs_ago: u64, now: SystemTime) -> HistoryEntry {
        HistoryEntry {
            batch_id: batch,
            rule_name: "r".into(),
            action_label: "a".into(),
            original_path: PathBuf::from(file),
            result_path: PathBuf::new(),
            undo: None,
            reversible: false,
            status,
            message: String::new(),
            created: now - Duration::from_secs(secs_ago),
        }
    }

    #[test]
    fn counts_use_singular_plural_and_zero() {
        let mut info = MenuInfo { enabled: true, folders: vec![], rules_active: 0, last_batch: None };
        assert_eq!(counts_line(&info), "Aucun dossier suivi · Aucune règle active");
        info.folders = vec![folder(1, "Downloads", true)];
        info.rules_active = 1;
        assert_eq!(counts_line(&info), "1 dossier suivi · 1 règle active");
        info.folders = vec![folder(1, "A", true), folder(2, "B", true), folder(3, "C", false)];
        info.rules_active = 5;
        assert_eq!(counts_line(&info), "3 dossiers suivis (1 en pause) · 5 règles actives");
    }

    #[test]
    fn titles_and_labels() {
        let info =
            MenuInfo { enabled: false, folders: vec![folder(1, "A", true), folder(2, "B", true)], rules_active: 3, last_batch: None };
        assert_eq!(folders_menu_title(&info), "Dossiers suivis (2)");
        assert_eq!(rules_menu_title(&info), "Règles (3 actives)");
        assert_eq!(rules_menu_title(&MenuInfo::default()), "Règles");
        assert_eq!(status_line(&info), "En veille");
        assert_eq!(toggle_label(false), "Reprendre la surveillance");
        assert_eq!(toggle_label(true), "Mettre en veille");
        assert_eq!(folder_entry_label(&folder(1, "Docs", false)), "Docs (en pause)");
        assert_eq!(folder_entry_label(&MenuFolder { denied: true, ..folder(1, "Docs", true) }), "Docs (accès refusé)");
        assert!(tooltip(&info).starts_with("OrganizeMyFiles · En veille · 2 dossiers suivis"));
    }

    #[test]
    fn last_batch_line_counts_files_of_the_latest_real_batch() {
        let now = SystemTime::now();
        let h = vec![
            entry(1, Status::Applied, "/a", 7200, now),
            entry(2, Status::Applied, "/b", 300, now),
            entry(2, Status::Applied, "/c", 299, now),
            entry(2, Status::Failed, "/d", 298, now),
            entry(3, Status::DryRun, "/e", 10, now),
        ];
        assert_eq!(last_batch_line(&h, now).unwrap(), "Dernier rangement : il y a 5 min · 2 fichiers");
        assert_eq!(last_batch_line(&[], now), None);
        assert_eq!(last_batch_line(&[entry(1, Status::DryRun, "/a", 5, now)], now), None);
        assert_eq!(
            last_batch_line(&h.iter().cloned().chain([entry(4, Status::Skipped, "", 1, now)]).collect::<Vec<_>>(), now).unwrap(),
            "Dernier rangement : il y a 5 min · 2 fichiers",
            "une règle ignorée ne remplace pas le dernier vrai rangement"
        );
    }
}
