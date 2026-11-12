//! Fonctions pures derrière les « pastilles » de l'écran Règles : extensions de fichiers et
//! noms courts de dossiers. Séparées de l'interface pour être testées.

use unicode_normalization::UnicodeNormalization;

use crate::engine::{nfc, Folder};

const SEPARATORS: [char; 3] = [',', ';', ' '];
const MAX_EXTENSION_LEN: usize = 16;

/// Une extension valide : lettres, chiffres, `_` et `-`, 1 à 16 caractères (sans point).
pub fn is_valid_extension(e: &str) -> bool {
    !e.is_empty() && e.chars().count() <= MAX_EXTENSION_LEN && e.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}

fn normalize_extension(token: &str) -> String {
    nfc(token.trim().trim_start_matches('.')).to_lowercase()
}

/// Découpe la valeur d'une condition (« .png, .JPG ; heic ») en extensions propres, sans doublon.
pub fn split_extensions(value: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for token in value.split(SEPARATORS).map(normalize_extension).filter(|t| !t.is_empty()) {
        if !out.contains(&token) {
            out.push(token);
        }
    }
    out
}

/// Écrit la liste dans le format stocké (« png, jpg »), relu tel quel par le moteur.
pub fn join_extensions(exts: &[String]) -> String {
    exts.join(", ")
}

/// Ajoute à `exts` les extensions saisies dans `draft`. Renvoie (liste modifiée ?, texte refusé).
/// Avec `commit_all` faux, le dernier morceau (sans séparateur après lui) reste en cours de saisie.
pub fn absorb_extensions(exts: &mut Vec<String>, draft: &str, commit_all: bool) -> (bool, String) {
    let (ready, tail) = if commit_all {
        (draft, "")
    } else {
        match draft.rfind(SEPARATORS) {
            Some(i) => (&draft[..i], &draft[i + 1..]),
            None => return (false, draft.to_string()),
        }
    };
    let (mut changed, mut refused) = (false, Vec::new());
    for token in ready.split(SEPARATORS).map(normalize_extension).filter(|t| !t.is_empty()) {
        if !is_valid_extension(&token) {
            refused.push(token);
        } else if !exts.contains(&token) {
            exts.push(token);
            changed = true;
        }
    }
    let mut left = refused.join(" ");
    if !tail.is_empty() {
        if !left.is_empty() {
            left.push(' ');
        }
        left.push_str(tail);
    }
    (changed, left)
}

/// Vrai si la saisie contient un morceau qui ne sera pas accepté comme extension.
pub fn has_invalid_token(draft: &str) -> bool {
    draft.split(SEPARATORS).map(normalize_extension).any(|t| !t.is_empty() && !is_valid_extension(&t))
}

/// Texte sans accents ni majuscules, pour les recherches.
pub fn fold(s: &str) -> String {
    s.nfd().filter(|c| !('\u{300}'..='\u{36f}').contains(c)).collect::<String>().to_lowercase()
}

fn base_name(f: &Folder) -> String {
    f.path.file_name().map(|n| nfc(&n.to_string_lossy())).unwrap_or_else(|| nfc(&f.path.to_string_lossy()))
}

/// Nom court d'un dossier pour une pastille ; le dossier parent est ajouté si deux dossiers portent le même nom.
pub fn folder_label(folders: &[Folder], id: u64) -> String {
    let Some(f) = folders.iter().find(|f| f.id == id) else { return String::new() };
    let name = base_name(f);
    let duplicated = folders.iter().filter(|o| o.id != id && base_name(o) == name).count() > 0;
    match f.path.parent().and_then(|p| p.file_name()).filter(|_| duplicated) {
        Some(parent) => format!("{name} · {}", nfc(&parent.to_string_lossy())),
        None => name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn folder(id: u64, path: &str) -> Folder {
        Folder { id, path: PathBuf::from(path), enabled: true, priority: 0 }
    }

    #[test]
    fn split_cleans_lowercases_and_dedups() {
        assert_eq!(split_extensions(".png, .JPG ; heic  PNG"), vec!["png", "jpg", "heic"]);
        assert!(split_extensions("  , ").is_empty());
        assert_eq!(join_extensions(&split_extensions(".png, .jpg")), "png, jpg");
    }

    #[test]
    fn validation_rejects_odd_characters() {
        assert!(is_valid_extension("tar-gz") && is_valid_extension("mp4"));
        for bad in ["", "a/b", "a.b", "a b", "abcdefghijklmnopq"] {
            assert!(!is_valid_extension(bad), "{bad}");
        }
    }

    #[test]
    fn absorb_commits_on_separator_and_keeps_the_tail() {
        let mut exts = vec!["png".to_string()];
        let (changed, left) = absorb_extensions(&mut exts, "jpg, hei", false);
        assert!(changed);
        assert_eq!(exts, vec!["png", "jpg"]);
        assert_eq!(left, "hei");
        let (changed, left) = absorb_extensions(&mut exts, "heic", true);
        assert!(changed && left.is_empty());
        assert_eq!(exts, vec!["png", "jpg", "heic"]);
    }

    #[test]
    fn absorb_refuses_invalid_tokens_and_ignores_duplicates() {
        let mut exts = vec!["png".to_string()];
        let (changed, left) = absorb_extensions(&mut exts, "a/b, PNG, ok", true);
        assert!(changed);
        assert_eq!(exts, vec!["png", "ok"]);
        assert_eq!(left, "a/b");
        let (changed, _) = absorb_extensions(&mut exts, "png", false);
        assert!(!changed, "sans séparateur, rien n'est validé");
    }

    #[test]
    fn invalid_tokens_are_detected_while_typing() {
        assert!(has_invalid_token("png, a/b"));
        assert!(!has_invalid_token(".png, jpg "));
        assert!(!has_invalid_token(""));
    }

    #[test]
    fn fold_ignores_accents_and_case() {
        assert_eq!(fold("Relevés ÉTÉ"), "releves ete");
        assert_eq!(fold("Rele\u{301}ve\u{301}s"), "releves");
    }

    #[test]
    fn folder_label_adds_parent_only_for_duplicates() {
        let fs = vec![folder(1, "/u/Documents"), folder(2, "/u/Clients/Documents"), folder(3, "/u/Desktop")];
        assert_eq!(folder_label(&fs, 3), "Desktop");
        assert_eq!(folder_label(&fs, 1), "Documents · u");
        assert_eq!(folder_label(&fs, 2), "Documents · Clients");
    }
}
