//! Mises à jour : lit la dernière release GitHub, télécharge le `.dmg`, remplace l'appli et la relance.
//! Rien n'est installé sans clic de l'utilisateur. Tout passe par les outils de macOS (`curl`, `shasum`,
//! `hdiutil`, `ditto`) via `CommandRunner`, donc les étapes se testent sans réseau ni vrai disque image.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::system::CommandRunner;

/// Dernière release publiée (dépôt public).
pub const LATEST_RELEASE_URL: &str = "https://api.github.com/repos/A2com/OrganizeMyFiles/releases/latest";
/// Nom de l'appli dans le `.dmg`.
const APP_NAME: &str = "OrganizeMyFiles.app";
const CHECK_TIMEOUT: Duration = Duration::from_secs(15);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(900);
const TOOL_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    /// Numéro sans le « v » (« 0.2.1 »).
    pub version: String,
    /// Page de la release (notes de version, téléchargement manuel).
    pub page_url: String,
    pub notes: String,
    pub dmg_url: Option<String>,
    /// Empreinte SHA-256 du `.dmg`, si GitHub la fournit.
    pub sha256: Option<String>,
}

/// Lit la réponse de l'API GitHub `releases/latest`.
pub fn parse_release(json: &str) -> Result<Release, String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| format!("Réponse illisible : {e}"))?;
    let tag = v["tag_name"].as_str().ok_or("Réponse sans numéro de version")?;
    let dmg = v["assets"].as_array().and_then(|a| a.iter().find(|x| x["name"].as_str().is_some_and(|n| n.ends_with(".dmg"))));
    Ok(Release {
        version: tag.trim_start_matches('v').to_string(),
        page_url: v["html_url"].as_str().unwrap_or("https://github.com/A2com/OrganizeMyFiles/releases").to_string(),
        notes: v["body"].as_str().unwrap_or_default().trim().to_string(),
        dmg_url: dmg.and_then(|d| d["browser_download_url"].as_str()).map(str::to_string),
        sha256: dmg.and_then(|d| d["digest"].as_str()).and_then(|d| d.strip_prefix("sha256:")).map(|h| h.to_lowercase()),
    })
}

fn version_parts(v: &str) -> Option<Vec<u64>> {
    v.trim().trim_start_matches('v').split('.').map(|p| p.parse().ok()).collect()
}

/// `candidate` est-il strictement plus récent que `current` ? Un numéro illisible n'est jamais « plus récent ».
pub fn is_newer(candidate: &str, current: &str) -> bool {
    match (version_parts(candidate), version_parts(current)) {
        (Some(mut a), Some(mut b)) => {
            let n = a.len().max(b.len());
            a.resize(n, 0);
            b.resize(n, 0);
            a > b
        }
        _ => false,
    }
}

/// `…/X.app/Contents/MacOS/binaire` → `…/X.app` ; `None` si le programme ne tourne pas depuis un `.app`.
pub fn bundle_path(exe: &Path) -> Option<PathBuf> {
    exe.ancestors().find(|p| p.extension().is_some_and(|e| e == "app")).map(Path::to_path_buf)
}

/// Où en est la mise à jour, pour l'interface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    Idle,
    Checking,
    UpToDate,
    Available(Release),
    /// Étape en cours (« Téléchargement… »).
    Installing(String),
    Failed(String),
}

/// Messages envoyés par les fils de travail à l'interface.
pub enum Msg {
    Checked { manual: bool, result: Result<Option<Release>, String> },
    Progress(String),
    Installed(Result<(), String>),
}

/// Interroge GitHub. `Ok(None)` : déjà à jour.
pub fn check(runner: &dyn CommandRunner, current: &str) -> Result<Option<Release>, String> {
    let args = ["-fsSL", "-m", "10", "-H", "Accept: application/vnd.github+json", LATEST_RELEASE_URL].map(String::from);
    let out = runner.run("curl", &args, CHECK_TIMEOUT)?;
    if !out.success {
        return Err("Impossible de joindre GitHub : vérifiez la connexion.".into());
    }
    let release = parse_release(&out.stdout)?;
    Ok(is_newer(&release.version, current).then_some(release))
}

fn run_ok(runner: &dyn CommandRunner, program: &str, args: &[&str], timeout: Duration, what: &str) -> Result<String, String> {
    let args: Vec<String> = args.iter().map(|a| (*a).to_string()).collect();
    let out = runner.run(program, &args, timeout).map_err(|e| format!("{what} : {e}"))?;
    if out.success {
        Ok(out.stdout)
    } else {
        Err(format!("{what} a échoué. {}", out.stderr).trim().to_string())
    }
}

/// Télécharge, vérifie, puis remplace `bundle` par la nouvelle version. L'ancienne appli reste intacte en cas d'échec.
/// `progress` reçoit une phrase courte à chaque étape.
pub fn install(runner: &dyn CommandRunner, release: &Release, bundle: &Path, work: &Path, progress: &dyn Fn(&str)) -> Result<(), String> {
    let url = release.dmg_url.as_deref().ok_or("Cette version n'a pas de fichier .dmg à télécharger.")?;
    fs::create_dir_all(work).map_err(|e| format!("Dossier temporaire impossible : {e}"))?;
    let dmg = work.join("update.dmg");
    let mount = work.join("mnt");
    let result = install_steps(runner, release, url, bundle, &dmg, &mount, progress);
    // Nettoyage dans tous les cas : démontage puis suppression des fichiers temporaires.
    let _ = runner.run("hdiutil", &["detach".into(), "-force".into(), mount.to_string_lossy().into_owned()], TOOL_TIMEOUT);
    let _ = fs::remove_dir_all(work);
    result
}

fn install_steps(
    runner: &dyn CommandRunner,
    release: &Release,
    url: &str,
    bundle: &Path,
    dmg: &Path,
    mount: &Path,
    progress: &dyn Fn(&str),
) -> Result<(), String> {
    let (dmg_s, mount_s, bundle_s) = (dmg.to_string_lossy(), mount.to_string_lossy(), bundle.to_string_lossy());

    progress("Téléchargement…");
    run_ok(runner, "curl", &["-fsSL", "-m", "900", "-o", &dmg_s, url], DOWNLOAD_TIMEOUT, "Le téléchargement")?;

    if let Some(expected) = &release.sha256 {
        progress("Vérification…");
        let out = run_ok(runner, "shasum", &["-a", "256", &dmg_s], TOOL_TIMEOUT, "La vérification")?;
        let actual = out.split_whitespace().next().unwrap_or_default().to_lowercase();
        if &actual != expected {
            return Err("Le fichier téléchargé ne correspond pas à l'empreinte attendue : installation refusée.".into());
        }
    }

    progress("Installation…");
    fs::create_dir_all(mount).map_err(|e| format!("Dossier temporaire impossible : {e}"))?;
    run_ok(
        runner,
        "hdiutil",
        &["attach", "-nobrowse", "-readonly", "-mountpoint", &mount_s, &dmg_s],
        TOOL_TIMEOUT,
        "L'ouverture du disque image",
    )?;
    let source = mount.join(APP_NAME);
    let (new, old) = (sibling(bundle, "new"), sibling(bundle, "old"));
    let _ = fs::remove_dir_all(&new);
    let _ = fs::remove_dir_all(&old);
    run_ok(runner, "ditto", &[&source.to_string_lossy(), &new.to_string_lossy()], TOOL_TIMEOUT, "La copie").map_err(|e| {
        format!("{e} Si l'appli est dans Applications, glissez la nouvelle version à la main depuis la page de téléchargement.")
    })?;

    // Échange en deux renommages : si le second échoue, l'ancienne appli est remise en place.
    fs::rename(bundle, &old).map_err(|e| format!("Impossible de remplacer {bundle_s} : {e}"))?;
    if let Err(e) = fs::rename(&new, bundle) {
        let _ = fs::rename(&old, bundle);
        return Err(format!("Impossible de mettre en place la nouvelle version : {e}"));
    }
    let _ = fs::remove_dir_all(&old);
    Ok(())
}

/// `X.app` → `X.app.new` / `X.app.old`, dans le même dossier (le renommage reste sur le même disque).
fn sibling(bundle: &Path, suffix: &str) -> PathBuf {
    let mut name = bundle.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    name.push(format!(".{suffix}"));
    bundle.with_file_name(name)
}

/// Relance l'appli après quelques instants (le temps que celle-ci se ferme). Sans effet si le lancement échoue.
pub fn relaunch(bundle: &Path) {
    let _ = std::process::Command::new("sh")
        .args(["-c", "sleep 1; open \"$1\"", "sh"])
        .arg(bundle)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::fake::FakeCommands;
    use crate::system::CommandOutput;
    use std::cell::RefCell;

    const RELEASE_JSON: &str = r#"{
        "tag_name": "v0.2.1",
        "html_url": "https://github.com/A2com/OrganizeMyFiles/releases/tag/v0.2.1",
        "body": "  Correctifs.  ",
        "assets": [
            {"name": "notes.txt", "browser_download_url": "https://x/notes.txt"},
            {"name": "OrganizeMyFiles-0.2.1.dmg", "browser_download_url": "https://x/a.dmg", "digest": "sha256:ABCDEF"}
        ]
    }"#;

    fn temp(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        std::env::temp_dir().join(format!("omf-update-test-{tag}-{}-{nanos}", std::process::id()))
    }

    /// Exécuteur scénarisé : réponses par programme, `ditto` crée vraiment la nouvelle appli.
    struct Script {
        calls: RefCell<Vec<String>>,
        sha: String,
        fail_on: Option<&'static str>,
    }

    impl CommandRunner for Script {
        fn run(&self, program: &str, args: &[String], _t: Duration) -> Result<CommandOutput, String> {
            self.calls.borrow_mut().push(program.to_string());
            let ok = |stdout: String| Ok(CommandOutput { success: true, stdout, stderr: String::new() });
            if self.fail_on == Some(program) {
                return Ok(CommandOutput { success: false, stdout: String::new(), stderr: "boom".into() });
            }
            match program {
                "shasum" => ok(format!("{}  update.dmg\n", self.sha)),
                "ditto" => {
                    let dest = Path::new(&args[1]);
                    fs::create_dir_all(dest.join("Contents")).unwrap();
                    fs::write(dest.join("Contents/version"), "nouvelle").unwrap();
                    ok(String::new())
                }
                _ => ok(String::new()),
            }
        }
    }

    fn fake_app(dir: &Path) -> PathBuf {
        let app = dir.join("OrganizeMyFiles.app");
        fs::create_dir_all(app.join("Contents")).unwrap();
        fs::write(app.join("Contents/version"), "ancienne").unwrap();
        app
    }

    fn release(sha: Option<&str>) -> Release {
        Release {
            version: "0.2.1".into(),
            page_url: "p".into(),
            notes: String::new(),
            dmg_url: Some("https://x/a.dmg".into()),
            sha256: sha.map(str::to_string),
        }
    }

    #[test]
    fn the_github_answer_is_read() {
        let r = parse_release(RELEASE_JSON).unwrap();
        assert_eq!(r.version, "0.2.1");
        assert_eq!(r.notes, "Correctifs.");
        assert_eq!(r.dmg_url.as_deref(), Some("https://x/a.dmg"));
        assert_eq!(r.sha256.as_deref(), Some("abcdef"));
        assert!(parse_release("pas du json").is_err());
        assert!(parse_release("{}").is_err());
    }

    #[test]
    fn a_release_without_dmg_or_digest_still_parses() {
        let r = parse_release(r#"{"tag_name":"v1.0.0","assets":[]}"#).unwrap();
        assert_eq!(r.dmg_url, None);
        assert_eq!(r.sha256, None);
    }

    #[test]
    fn versions_compare_numerically() {
        assert!(is_newer("v0.2.1", "0.2.0"));
        assert!(is_newer("0.10.0", "0.9.9"));
        assert!(is_newer("1.0", "0.9.9"));
        assert!(!is_newer("0.2.0", "0.2.0"));
        assert!(!is_newer("0.2.0", "0.2"));
        assert!(!is_newer("0.1.9", "0.2.0"));
        assert!(!is_newer("n'importe quoi", "0.2.0"));
    }

    #[test]
    fn the_bundle_is_found_from_the_executable() {
        let exe = Path::new("/Applications/OrganizeMyFiles.app/Contents/MacOS/OrganizeMyFiles");
        assert_eq!(bundle_path(exe), Some(PathBuf::from("/Applications/OrganizeMyFiles.app")));
        assert_eq!(bundle_path(Path::new("/tmp/target/release/OrganizeMyFiles")), None);
    }

    #[test]
    fn check_reports_a_newer_release_only() {
        let mut fake = FakeCommands::ok();
        fake.stdout = RELEASE_JSON.into();
        assert_eq!(check(&fake, "0.2.0").unwrap().map(|r| r.version), Some("0.2.1".into()));
        assert_eq!(check(&fake, "0.2.1").unwrap(), None);
        assert_eq!(fake.calls.borrow()[0].0, "curl");
    }

    #[test]
    fn check_explains_a_network_failure() {
        let mut fake = FakeCommands::ok();
        fake.reply = Ok(false);
        assert!(check(&fake, "0.2.0").unwrap_err().contains("connexion"));
    }

    #[test]
    fn install_replaces_the_app_and_cleans_up() {
        let dir = temp("ok");
        let bundle = fake_app(&dir);
        let work = dir.join("work");
        let runner = Script { calls: RefCell::new(vec![]), sha: "abcdef".into(), fail_on: None };
        install(&runner, &release(Some("abcdef")), &bundle, &work, &|_| {}).unwrap();
        assert_eq!(fs::read_to_string(bundle.join("Contents/version")).unwrap(), "nouvelle");
        assert!(!sibling(&bundle, "old").exists() && !sibling(&bundle, "new").exists());
        assert!(!work.exists(), "le dossier temporaire est supprimé");
        assert_eq!(*runner.calls.borrow(), ["curl", "shasum", "hdiutil", "ditto", "hdiutil"]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_wrong_checksum_refuses_the_installation() {
        let dir = temp("sha");
        let bundle = fake_app(&dir);
        let runner = Script { calls: RefCell::new(vec![]), sha: "autre".into(), fail_on: None };
        let err = install(&runner, &release(Some("abcdef")), &bundle, &dir.join("work"), &|_| {}).unwrap_err();
        assert!(err.contains("empreinte"));
        assert_eq!(fs::read_to_string(bundle.join("Contents/version")).unwrap(), "ancienne");
        assert!(!runner.calls.borrow().contains(&"ditto".to_string()), "rien n'est copié");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_failed_copy_keeps_the_old_app() {
        let dir = temp("ditto");
        let bundle = fake_app(&dir);
        let runner = Script { calls: RefCell::new(vec![]), sha: String::new(), fail_on: Some("ditto") };
        let err = install(&runner, &release(None), &bundle, &dir.join("work"), &|_| {}).unwrap_err();
        assert!(err.contains("La copie a échoué") && err.contains("glissez"));
        assert_eq!(fs::read_to_string(bundle.join("Contents/version")).unwrap(), "ancienne");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_failed_download_stops_everything() {
        let dir = temp("curl");
        let bundle = fake_app(&dir);
        let runner = Script { calls: RefCell::new(vec![]), sha: String::new(), fail_on: Some("curl") };
        let err = install(&runner, &release(None), &bundle, &dir.join("work"), &|_| {}).unwrap_err();
        assert!(err.contains("téléchargement"));
        assert_eq!(runner.calls.borrow()[0], "curl");
        assert_eq!(fs::read_to_string(bundle.join("Contents/version")).unwrap(), "ancienne");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_release_without_dmg_cannot_be_installed() {
        let mut r = release(None);
        r.dmg_url = None;
        let fake = FakeCommands::ok();
        assert!(install(&fake, &r, Path::new("/x/A.app"), &temp("nodmg"), &|_| {}).is_err());
        assert!(fake.calls.borrow().is_empty());
    }
}
