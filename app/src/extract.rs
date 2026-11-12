//! Lecture du contenu des fichiers pour la condition « Contenu » : texte brut, PDF et images (OCR).
//! Les PDF et les images passent par le petit programme `omf-extract` (PDFKit et Vision, natifs à
//! macOS), livré dans l'appli. Tout reste sur le Mac. Sans ce programme (autre système, build de
//! développement), PDF et images n'ont simplement « pas de contenu ».

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime};

use crate::engine::nfc;
use crate::mechanics::{IMAGE_EXTENSIONS, OCR_IMAGE_DIMENSION_LIMIT, OCR_LIMIT_BYTES, TEXT_LIMIT_BYTES};
use crate::system::{CommandRunner, SystemCommands};

const EXTRACT_TIMEOUT: Duration = Duration::from_secs(45);
const CACHE_CAPACITY: usize = 500;
const HELPER_NAME: &str = "omf-extract";

/// Où chercher le programme de lecture : variable `OMF_EXTRACT_PATH`, puis à côté de l'exécutable
/// (dans l'appli : `Contents/MacOS`), puis un dossier au-dessus (tests : `target/debug/deps`).
pub fn helper_path() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("OMF_EXTRACT_PATH").map(PathBuf::from).filter(|p| p.is_file()) {
        return Some(p);
    }
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    [dir.join(HELPER_NAME), dir.parent()?.join(HELPER_NAME)].into_iter().find(|p| p.is_file())
}

type CacheKey = (PathBuf, u64, Option<SystemTime>);

fn cache() -> &'static Mutex<HashMap<CacheKey, Option<String>>> {
    static CACHE: OnceLock<Mutex<HashMap<CacheKey, Option<String>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Contenu d'un fichier en minuscules, ou `None` s'il n'est pas lisible ou dépasse les limites.
/// Les PDF et images lus par le programme sont mis en cache (chemin, taille, date de modification).
pub fn read_content(path: &Path) -> Option<String> {
    read_content_with(path, &SystemCommands, helper_path().as_deref(), Some(cache()))
}

/// Version testable : exécuteur de commandes et cache fournis par l'appelant.
pub fn read_content_with(
    path: &Path,
    runner: &dyn CommandRunner,
    helper: Option<&Path>,
    cache: Option<&Mutex<HashMap<CacheKey, Option<String>>>>,
) -> Option<String> {
    let md = fs::metadata(path).ok()?;
    let ext = path.extension().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
    let is_pdf = ext == "pdf";
    let is_image = IMAGE_EXTENSIONS.contains(&ext.as_str());
    if !is_pdf && !is_image {
        return read_plain_text(path, md.len());
    }
    let (limit, helper) = (if is_pdf { TEXT_LIMIT_BYTES } else { OCR_LIMIT_BYTES }, helper?);
    if md.len() > limit || (is_image && image_too_large(path)) {
        return None;
    }
    let key: CacheKey = (path.to_path_buf(), md.len(), md.modified().ok());
    if let Some(hit) = cache.and_then(|c| c.lock().ok()).and_then(|c| c.get(&key).cloned()) {
        return hit;
    }
    let text = run_helper(runner, helper, path);
    if let Some(c) = cache {
        if let Ok(mut c) = c.lock() {
            if c.len() >= CACHE_CAPACITY {
                c.clear();
            }
            c.insert(key, text.clone());
        }
    }
    text
}

fn run_helper(runner: &dyn CommandRunner, helper: &Path, path: &Path) -> Option<String> {
    let args = [path.to_string_lossy().into_owned()];
    let out = runner.run(&helper.to_string_lossy(), &args, EXTRACT_TIMEOUT).ok()?;
    let text = nfc(&out.stdout).to_lowercase();
    (out.success && !text.trim().is_empty()).then_some(text)
}

/// Une image au-delà de 12 000 px est refusée avant l'OCR (valeur de l'appli d'origine).
fn image_too_large(path: &Path) -> bool {
    image::image_dimensions(path).is_ok_and(|(w, h)| dimension_limit_exceeded(w, h))
}

/// Règle d'origine (extractImageOCR) : largeur > 12 000 ou hauteur ≥ 12 001 refusée.
fn dimension_limit_exceeded(width: u32, height: u32) -> bool {
    width > OCR_IMAGE_DIMENSION_LIMIT || height > OCR_IMAGE_DIMENSION_LIMIT + 1
}

/// Texte brut : on lit le fichier s'il ne ressemble pas à du binaire.
fn read_plain_text(path: &Path, size: u64) -> Option<String> {
    if size > TEXT_LIMIT_BYTES {
        return None;
    }
    let bytes = fs::read(path).ok()?;
    let text = String::from_utf8_lossy(&bytes).to_string();
    let controls = text.chars().filter(|c| c.is_control() && *c != '\n' && *c != '\r' && *c != '\t').count();
    (controls <= text.len() / 8).then(|| nfc(&text).to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::fake::FakeCommands;

    fn temp_file(name: &str, bytes: &[u8]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("organizemyfiles-extract-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let p = dir.join(name);
        fs::write(&p, bytes).unwrap();
        p
    }

    fn fresh_cache() -> Mutex<HashMap<CacheKey, Option<String>>> {
        Mutex::new(HashMap::new())
    }

    #[test]
    fn plain_text_is_read_lowercase_and_composed() {
        let p = temp_file("note.txt", "Facture Relevé de e\u{301}te\u{301}".as_bytes());
        assert_eq!(read_content_with(&p, &FakeCommands::ok(), None, None).unwrap(), "facture relevé de été");
    }

    #[test]
    fn binary_files_have_no_content() {
        let p = temp_file("blob.bin", &[0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
        assert_eq!(read_content_with(&p, &FakeCommands::ok(), None, None), None);
    }

    #[test]
    fn pdf_and_images_go_through_the_helper() {
        let pdf = temp_file("doc.pdf", b"%PDF-1.4 fake");
        let mut fake = FakeCommands::ok();
        fake.stdout = "Facture MARS 2026\n".into();
        let helper = PathBuf::from("/fake/omf-extract");
        assert_eq!(read_content_with(&pdf, &fake, Some(&helper), None).unwrap(), "facture mars 2026\n");
        let calls = fake.calls.borrow();
        assert_eq!(calls[0].0, "/fake/omf-extract");
        assert_eq!(calls[0].1, vec![pdf.to_string_lossy().into_owned()]);
    }

    #[test]
    fn no_helper_means_no_content_not_an_error() {
        let pdf = temp_file("nohelper.pdf", b"%PDF-1.4");
        assert_eq!(read_content_with(&pdf, &FakeCommands::ok(), None, None), None);
    }

    #[test]
    fn helper_failure_or_empty_output_gives_no_content() {
        let pdf = temp_file("fail.pdf", b"%PDF-1.4");
        let helper = PathBuf::from("/fake/omf-extract");
        let mut failing = FakeCommands::ok();
        failing.reply = Ok(false);
        failing.stdout = "partial".into();
        assert_eq!(read_content_with(&pdf, &failing, Some(&helper), None), None);
        let mut timeout = FakeCommands::ok();
        timeout.reply = Err("Délai dépassé (45 s), commande arrêtée".into());
        assert_eq!(read_content_with(&pdf, &timeout, Some(&helper), None), None);
        assert_eq!(read_content_with(&pdf, &FakeCommands::ok(), Some(&helper), None), None, "sortie vide");
    }

    #[test]
    fn second_read_comes_from_the_cache() {
        let pdf = temp_file("cached.pdf", b"%PDF-1.4 cached");
        let cache = fresh_cache();
        let helper = PathBuf::from("/fake/omf-extract");
        let mut fake = FakeCommands::ok();
        fake.stdout = "texte".into();
        assert_eq!(read_content_with(&pdf, &fake, Some(&helper), Some(&cache)).unwrap(), "texte");
        assert_eq!(read_content_with(&pdf, &fake, Some(&helper), Some(&cache)).unwrap(), "texte");
        assert_eq!(fake.calls.borrow().len(), 1, "le programme n'est lancé qu'une fois");
        // Le fichier change : nouvelle lecture.
        fs::write(&pdf, b"%PDF-1.4 cached but longer now").unwrap();
        read_content_with(&pdf, &fake, Some(&helper), Some(&cache));
        assert_eq!(fake.calls.borrow().len(), 2);
    }

    #[test]
    fn dimension_limit_matches_the_original_rule() {
        assert_eq!(OCR_IMAGE_DIMENSION_LIMIT, 12000);
        assert!(!dimension_limit_exceeded(12000, 12001));
        assert!(dimension_limit_exceeded(12001, 1));
        assert!(dimension_limit_exceeded(1, 12002));
    }

    #[test]
    fn oversized_image_is_refused_before_ocr() {
        // Vraie image de 12 001 × 1 px : refusée sans lancer le programme.
        let p = temp_file("huge.png", b"");
        image::RgbaImage::new(12_001, 1).save(&p).unwrap();
        assert!(image_too_large(&p));
        let helper = PathBuf::from("/fake/omf-extract");
        let mut fake = FakeCommands::ok();
        fake.stdout = "texte".into();
        assert_eq!(read_content_with(&p, &fake, Some(&helper), None), None);
        assert!(fake.calls.borrow().is_empty());
    }

    #[cfg(target_os = "macos")]
    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
    }

    /// Vrai programme (macOS) : texte d'un PDF, OCR d'une photo et d'un PDF scanné.
    /// Ignoré si `omf-extract` n'est pas construit (`scripts/construire-omf-extract.sh`).
    #[cfg(target_os = "macos")]
    #[test]
    fn real_helper_reads_pdf_image_and_scanned_pdf() {
        let Some(helper) = helper_path() else {
            eprintln!("omf-extract absent : test ignoré");
            return;
        };
        for (file, needle) in [("facture-texte.pdf", "facture mars 2026"), ("recu.png", "total 42,50 eur"), ("scan.pdf", "boulangerie")] {
            let text = read_content_with(&fixture(file), &SystemCommands, Some(&helper), None).unwrap_or_default();
            assert!(text.contains(needle), "{file} : « {needle} » introuvable dans « {text} »");
        }
    }

    /// Une règle « Contenu contient total » range la photo du reçu et le PDF scanné, pas le PDF de facture.
    #[cfg(target_os = "macos")]
    #[test]
    fn contents_rule_sorts_real_pdf_and_receipt() {
        use crate::engine::{Action, ActionKind, Condition, Field, Folder, Op, Rule, Runner};
        if helper_path().is_none() {
            eprintln!("omf-extract absent : test ignoré");
            return;
        }
        let base = std::env::temp_dir().join(format!("organizemyfiles-contents-e2e-{}", std::process::id()));
        let (watched, dest) = (base.join("in"), base.join("out"));
        fs::create_dir_all(&watched).unwrap();
        for f in ["facture-texte.pdf", "recu.png", "scan.pdf"] {
            fs::copy(fixture(f), watched.join(f)).unwrap();
        }
        let mut act = Action::new(ActionKind::Move);
        act.params.insert("destination".into(), dest.to_string_lossy().into_owned());
        let rule = Rule {
            id: 1,
            folder_ids: vec![1],
            name: "totaux".into(),
            enabled: true,
            match_all: true,
            recursion_depth: 0,
            priority: 0,
            conditions: vec![Condition { field: Field::Contents, op: Op::Contains, value: "total".into() }],
            actions: vec![act],
        };
        let folders = vec![Folder { id: 1, path: watched.clone(), enabled: true, priority: 0 }];
        let mut runner = Runner { monthly_used: 0, batch_id: 1, dry_run: false, commands: Box::new(SystemCommands), only_paths: None };
        let out = runner.run(&folders, &[rule], SystemTime::now());
        assert_eq!(out.matched, 2);
        assert!(dest.join("recu.png").exists() && dest.join("scan.pdf").exists());
        assert!(watched.join("facture-texte.pdf").exists(), "le PDF de facture ne contient pas « total »");
    }
}
