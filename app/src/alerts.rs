//! Notifications macOS après un rangement.

use crate::engine::{HistoryEntry, Status};

/// Texte de la notification : combien de fichiers rangés, combien d'échecs. `None` = rien à dire.
pub fn summary(history: &[HistoryEntry]) -> Option<String> {
    let applied = history.iter().filter(|e| e.status == Status::Applied).count();
    let failed = history.iter().filter(|e| e.status == Status::Failed).count();
    match (applied, failed) {
        (0, 0) => None,
        (a, 0) => Some(format!("{a} action(s) appliquée(s)")),
        (0, f) => Some(format!("{f} action(s) en échec, voir l'historique")),
        (a, f) => Some(format!("{a} action(s) appliquée(s), {f} en échec")),
    }
}

/// Envoie la notification sans bloquer l'interface. Sans effet hors macOS.
pub fn notify(body: String) {
    #[cfg(target_os = "macos")]
    std::thread::spawn(move || {
        let _ = mac_notification_sys::set_application("com.a2com.organizemyfiles");
        let _ = mac_notification_sys::Notification::new().title("OrganizeMyFiles").message(&body).send();
    });
    #[cfg(not(target_os = "macos"))]
    let _ = body;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::SystemTime;

    fn entry(status: Status) -> HistoryEntry {
        HistoryEntry {
            batch_id: 1,
            rule_name: "r".into(),
            action_label: "a".into(),
            original_path: PathBuf::new(),
            result_path: PathBuf::new(),
            undo: None,
            reversible: false,
            status,
            message: String::new(),
            created: SystemTime::now(),
        }
    }

    #[test]
    fn summary_counts_successes_and_failures() {
        assert_eq!(summary(&[]), None);
        assert_eq!(summary(&[entry(Status::DryRun)]), None);
        assert_eq!(summary(&[entry(Status::Applied), entry(Status::Applied)]).unwrap(), "2 action(s) appliquée(s)");
        assert!(summary(&[entry(Status::Failed)]).unwrap().contains("en échec"));
        assert_eq!(summary(&[entry(Status::Applied), entry(Status::Failed)]).unwrap(), "1 action(s) appliquée(s), 1 en échec");
        assert_eq!(summary(&[entry(Status::Skipped)]), None, "une règle incomplète n'est pas une panne");
    }
}
