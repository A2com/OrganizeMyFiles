//! Journal local de diagnostic : démarrage, lots de rangement, erreurs et mises à jour.
//! Fichier `~/Library/Logs/OrganizeMyFiles/journal.log` (600), rotation simple à 1 Mo (3 archives).
//! Rien n'est envoyé sur le réseau ; le fichier reste sur le Mac.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const MAX_BYTES: u64 = 1_000_000;
const ARCHIVES: u32 = 3;
/// Longueur maximale d'un message : une erreur système peut être très bavarde.
const MAX_MESSAGE: usize = 500;

pub fn journal_path() -> PathBuf {
    let home = PathBuf::from(crate::engine::home_dir());
    if cfg!(target_os = "macos") {
        home.join("Library/Logs/OrganizeMyFiles/journal.log")
    } else if let Ok(state) = std::env::var("XDG_STATE_HOME") {
        PathBuf::from(state).join("OrganizeMyFiles/journal.log")
    } else {
        home.join(".local/state/OrganizeMyFiles/journal.log")
    }
}

pub fn info(message: impl AsRef<str>) {
    append(&journal_path(), "INFO", message.as_ref());
}

pub fn warn(message: impl AsRef<str>) {
    append(&journal_path(), "WARN", message.as_ref());
}

/// Écrit une ligne horodatée. Sans effet si le fichier ne peut pas être écrit : le journal ne doit
/// jamais faire échouer une action de l'utilisateur.
fn append(path: &Path, level: &str, message: &str) {
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    rotate(path);
    let clean: String = message.chars().take(MAX_MESSAGE).collect();
    let line = format!("{} [{level}] {}\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), clean.replace('\n', " "));
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // Chemins de fichiers personnels : lisible par le seul propriétaire.
        options.mode(0o600);
    }
    if let Ok(mut file) = options.open(path) {
        let _ = file.write_all(line.as_bytes());
    }
}

/// `journal.log` → `journal.log.1` → `journal.log.2` … quand le fichier dépasse la taille limite.
fn rotate(path: &Path) {
    let too_big = fs::metadata(path).map(|m| m.len() > MAX_BYTES).unwrap_or(false);
    if !too_big {
        return;
    }
    for index in (1..ARCHIVES).rev() {
        let from = archive(path, index);
        let to = archive(path, index + 1);
        if from.exists() {
            let _ = fs::rename(&from, &to);
        }
    }
    let _ = fs::rename(path, archive(path, 1));
}

fn archive(path: &Path, index: u32) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(format!(".{index}"));
    PathBuf::from(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(tag: &str) -> PathBuf {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("omf-journal-{tag}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_line_holds_date_level_and_message() {
        let dir = temp_dir("ligne");
        let path = dir.join("journal.log");
        append(&path, "WARN", "lot 3 : 2 échecs\nsur 5 fichiers");
        let text = fs::read_to_string(&path).unwrap();
        let (date, rest) = text.split_once(" [WARN] ").expect("niveau présent");
        assert_eq!(date.len(), 19, "horodatage AAAA-MM-JJ HH:MM:SS : {date}");
        assert_eq!(rest.trim_end(), "lot 3 : 2 échecs sur 5 fichiers", "les sauts de ligne sont aplatis");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_long_message_is_cut() {
        let dir = temp_dir("coupe");
        let path = dir.join("journal.log");
        append(&path, "INFO", &"x".repeat(MAX_MESSAGE + 50));
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.len() < MAX_MESSAGE + 80);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_big_file_is_rotated_and_archives_are_kept() {
        let dir = temp_dir("rotation");
        let path = dir.join("journal.log");
        fs::write(&path, "x".repeat(MAX_BYTES as usize + 1)).unwrap();
        append(&path, "INFO", "après rotation");
        assert!(path.exists() && fs::metadata(&path).unwrap().len() < 100);
        assert!(archive(&path, 1).exists(), "l'ancien journal devient .1");
        // Une seconde rotation décale l'archive précédente.
        fs::write(&path, "x".repeat(MAX_BYTES as usize + 1)).unwrap();
        append(&path, "INFO", "encore");
        assert!(archive(&path, 2).exists());
        assert!(archive(&path, 1).exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn the_file_is_readable_only_by_its_owner() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp_dir("droits");
        let path = dir.join("journal.log");
        append(&path, "INFO", "bonjour");
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "droits {mode:o}");
        let _ = fs::remove_dir_all(&dir);
    }
}
