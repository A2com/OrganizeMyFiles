//! Lancement à l'ouverture de session (macOS) : un fichier LaunchAgent dans
//! `~/Library/LaunchAgents`, lu par le système à chaque connexion.

use std::path::PathBuf;

use crate::engine::home_dir;

const LABEL: &str = "com.a2com.organizemyfiles";

fn plist_path() -> PathBuf {
    PathBuf::from(home_dir()).join("Library/LaunchAgents").join(format!("{LABEL}.plist"))
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Contenu du LaunchAgent : lance `exe --arriere-plan` à l'ouverture de session.
pub fn plist_content(exe: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
        <string>--arriere-plan</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>
"#,
        xml_escape(exe)
    )
}

/// Active ou désactive le lancement à l'ouverture de session.
pub fn set_enabled(enabled: bool) -> Result<(), String> {
    if !cfg!(target_os = "macos") {
        return Err("Disponible seulement sur macOS".into());
    }
    let path = plist_path();
    if enabled {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, plist_content(&exe.to_string_lossy())).map_err(|e| e.to_string())
    } else {
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plist_launches_the_app_in_background_at_login() {
        let p = plist_content("/Applications/A & B/app");
        assert!(p.contains("<string>/Applications/A &amp; B/app</string>"));
        assert!(p.contains("--arriere-plan"));
        assert!(p.contains("<key>RunAtLoad</key>\n    <true/>"));
        assert!(p.contains(LABEL));
    }
}
