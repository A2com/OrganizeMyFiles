//! Commandes système lancées par les actions « Ouvrir une application » et « Lancer un raccourci ».
//! Passe par `CommandRunner` pour pouvoir tester sans lancer de vraies applis.

use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub struct CommandOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

/// Taille maximale de sortie conservée (le reste est ignoré).
const MAX_OUTPUT_BYTES: usize = 2 * 1024 * 1024;

fn read_capped(pipe: Option<impl Read>) -> String {
    let mut bytes = Vec::new();
    if let Some(p) = pipe {
        let _ = p.take(MAX_OUTPUT_BYTES as u64).read_to_end(&mut bytes);
    }
    String::from_utf8_lossy(&bytes).into_owned()
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
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Impossible de lancer « {program} » : {e}"))?;
        let (out_pipe, err_pipe) = (child.stdout.take(), child.stderr.take());
        let out_reader = std::thread::spawn(move || read_capped(out_pipe));
        let err_reader = std::thread::spawn(move || read_capped(err_pipe));
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
        let stdout = out_reader.join().unwrap_or_default();
        let stderr = err_reader.join().unwrap_or_default().trim().to_string();
        Ok(CommandOutput { success: status.success(), stdout, stderr })
    }
}

/// Montre le fichier dans le Finder (`open -R`), sans bloquer l'interface. Sans effet hors macOS.
pub fn reveal_in_finder(path: &std::path::Path) {
    #[cfg(target_os = "macos")]
    {
        let target = path.to_string_lossy().into_owned();
        std::thread::spawn(move || {
            let _ = SystemCommands.run("open", &["-R".to_string(), target], Duration::from_secs(10));
        });
    }
    #[cfg(not(target_os = "macos"))]
    let _ = path;
}

/// Ouvre Réglages Système sur « Accès complet au disque ». Sans effet hors macOS.
pub fn open_privacy_settings() {
    #[cfg(target_os = "macos")]
    std::thread::spawn(|| {
        let url = "x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles".to_string();
        let _ = SystemCommands.run("open", &[url], Duration::from_secs(10));
    });
}

pub fn open_application_args(app: &str, file: &str) -> Vec<String> {
    vec!["-a".into(), app.into(), file.into()]
}

pub fn run_shortcut_args(name: &str, file: &str) -> Vec<String> {
    vec!["run".into(), name.into(), "--input-path".into(), file.into()]
}

#[cfg(test)]
pub mod fake {
    use super::*;
    use std::cell::RefCell;

    /// Enregistre les commandes demandées et répond avec un résultat prévu.
    pub struct FakeCommands {
        pub calls: RefCell<Vec<(String, Vec<String>)>>,
        pub reply: Result<bool, String>,
        pub stdout: String,
        pub stderr: String,
    }

    impl FakeCommands {
        pub fn ok() -> Self {
            Self { calls: RefCell::new(vec![]), reply: Ok(true), stdout: String::new(), stderr: String::new() }
        }
    }

    impl CommandRunner for FakeCommands {
        fn run(&self, program: &str, args: &[String], _t: Duration) -> Result<CommandOutput, String> {
            self.calls.borrow_mut().push((program.to_string(), args.to_vec()));
            match &self.reply {
                Ok(success) => Ok(CommandOutput { success: *success, stdout: self.stdout.clone(), stderr: self.stderr.clone() }),
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
}
