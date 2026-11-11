//! Commandes système lancées par les actions « Ouvrir une application »,
//! « Importer dans une médiathèque » et « Lancer un raccourci ».
//! Passe par `CommandRunner` pour pouvoir tester sans lancer de vraies applis.

use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub struct CommandOutput {
    pub success: bool,
    pub stderr: String,
}

pub trait CommandRunner {
    /// Lance `program args…` et attend au plus `timeout` ; le processus est tué au dépassement.
    fn run(&self, program: &str, args: &[String], timeout: Duration) -> Result<CommandOutput, String>;
}

/// Vrai lancement de processus (macOS uniquement).
pub struct SystemCommands;

impl CommandRunner for SystemCommands {
    fn run(&self, program: &str, args: &[String], timeout: Duration) -> Result<CommandOutput, String> {
        if !cfg!(target_os = "macos") {
            return Err("Disponible seulement sur macOS".into());
        }
        let mut child = Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Impossible de lancer « {program} » : {e}"))?;
        let mut pipe = child.stderr.take();
        let reader = std::thread::spawn(move || {
            let mut text = String::new();
            if let Some(p) = pipe.as_mut() {
                let _ = p.read_to_string(&mut text);
            }
            text
        });
        let start = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(s)) => break s,
                Ok(None) if start.elapsed() >= timeout => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("Délai dépassé ({} s), commande arrêtée", timeout.as_secs()));
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(50)),
                Err(e) => return Err(e.to_string()),
            }
        };
        let stderr = reader.join().unwrap_or_default().trim().to_string();
        Ok(CommandOutput { success: status.success(), stderr })
    }
}

pub fn open_application_args(app: &str, file: &str) -> Vec<String> {
    vec!["-a".into(), app.into(), file.into()]
}

pub fn run_shortcut_args(name: &str, file: &str) -> Vec<String> {
    vec!["run".into(), name.into(), "--input-path".into(), file.into()]
}

/// Arguments d'`osascript` pour importer `file` ; le chemin passe en argument du
/// script (pas collé dans son texte), donc aucun risque de guillemet mal échappé.
pub fn import_args(library: &str, file: &str) -> Result<Vec<String>, String> {
    let line = match library {
        "photos" => "tell application \"Photos\" to import {POSIX file (item 1 of argv)}",
        "music" => "tell application \"Music\" to add (POSIX file (item 1 of argv))",
        "tv" => "tell application \"TV\" to add (POSIX file (item 1 of argv))",
        other => return Err(format!("Médiathèque inconnue « {other} » (photos, music ou tv)")),
    };
    Ok(["-e", "on run argv", "-e", line, "-e", "end run", file].iter().map(|s| s.to_string()).collect())
}

/// Message d'aide quand macOS refuse l'accès à l'appli cible (Apple Events).
pub fn explain_import_error(stderr: &str) -> String {
    if stderr.contains("-1743") || stderr.contains("Not authorized") {
        "macOS a refusé l'accès : autorisez Forel dans Réglages Système › Confidentialité et sécurité › Automatisation".into()
    } else {
        stderr.to_string()
    }
}

#[cfg(test)]
pub mod fake {
    use super::*;
    use std::cell::RefCell;

    /// Enregistre les commandes demandées et répond avec un résultat prévu.
    pub struct FakeCommands {
        pub calls: RefCell<Vec<(String, Vec<String>)>>,
        pub reply: Result<bool, String>,
        pub stderr: String,
    }

    impl FakeCommands {
        pub fn ok() -> Self {
            Self { calls: RefCell::new(vec![]), reply: Ok(true), stderr: String::new() }
        }
    }

    impl CommandRunner for FakeCommands {
        fn run(&self, program: &str, args: &[String], _t: Duration) -> Result<CommandOutput, String> {
            self.calls.borrow_mut().push((program.to_string(), args.to_vec()));
            match &self.reply {
                Ok(success) => Ok(CommandOutput { success: *success, stderr: self.stderr.clone() }),
                Err(e) => Err(e.clone()),
            }
        }
    }
}

#[cfg(test)]
impl CommandRunner for std::rc::Rc<fake::FakeCommands> {
    fn run(&self, program: &str, args: &[String], t: Duration) -> Result<CommandOutput, String> {
        (**self).run(program, args, t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_rejects_unknown_library() {
        assert!(import_args("books", "/a").is_err());
        assert_eq!(import_args("photos", "/a b").unwrap().last().unwrap(), "/a b");
    }

    #[test]
    fn permission_error_gets_help_text() {
        assert!(explain_import_error("execution error: Not authorized to send Apple events (-1743)").contains("Automatisation"));
        assert_eq!(explain_import_error("boom"), "boom");
    }
}
