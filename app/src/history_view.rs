//! Mise en forme de l'historique : regroupement par jour puis par lot, filtres, résumés.
//! Fonctions pures (sans interface), pour être testées.

use std::collections::HashSet;
use std::time::{Duration, SystemTime};

use crate::engine::{date_str, nfc, HistoryEntry, Status};
use crate::tags::fold;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StatusFilter {
    All,
    Applied,
    Failed,
    Undone,
}

pub const FILTERS: [StatusFilter; 4] = [StatusFilter::All, StatusFilter::Applied, StatusFilter::Failed, StatusFilter::Undone];

impl StatusFilter {
    pub fn label(self) -> &'static str {
        match self {
            StatusFilter::All => "Tous",
            StatusFilter::Applied => "Appliqués",
            StatusFilter::Failed => "Échecs",
            StatusFilter::Undone => "Annulés",
        }
    }

    pub fn matches(self, s: Status) -> bool {
        match self {
            StatusFilter::All => true,
            StatusFilter::Applied => s == Status::Applied,
            StatusFilter::Failed => matches!(s, Status::Failed | Status::BlockedByConflict),
            StatusFilter::Undone => s == Status::Undone,
        }
    }
}

/// Un lot d'actions : indices dans l'historique, dans l'ordre chronologique.
pub struct BatchView {
    pub batch_id: u64,
    pub created: SystemTime,
    pub entries: Vec<usize>,
}

/// Les lots d'un même jour, du plus récent au plus ancien.
pub struct DayView {
    pub label: String,
    pub batches: Vec<BatchView>,
}

/// « il y a 5 min », « hier »… à partir d'un instant passé.
pub fn ago(t: SystemTime, now: SystemTime) -> String {
    let secs = now.duration_since(t).map(|d| d.as_secs()).unwrap_or(0);
    match secs {
        0..=44 => "à l'instant".to_string(),
        45..=3599 => format!("il y a {} min", (secs + 30) / 60),
        3600..=86_399 => format!("il y a {} h", secs / 3600),
        86_400..=172_799 => "hier".to_string(),
        _ => format!("il y a {} j", secs / 86_400),
    }
}

/// « Aujourd'hui », « Hier », sinon la date.
pub fn day_label(created: SystemTime, now: SystemTime) -> String {
    let day = date_str(created);
    if day == date_str(now) {
        "Aujourd'hui".into()
    } else if now.checked_sub(Duration::from_secs(86_400)).is_some_and(|y| day == date_str(y)) {
        "Hier".into()
    } else {
        day
    }
}

/// Nom du fichier traité (forme composée, sans dossier). Une règle ignorée affiche son propre nom :
/// aucune entrée fichier n'existe dans ce cas.
pub fn file_name(e: &HistoryEntry) -> String {
    if e.status == Status::Skipped {
        return nfc(&e.rule_name);
    }
    e.original_path.file_name().map(|n| nfc(&n.to_string_lossy())).unwrap_or_else(|| nfc(&e.original_path.to_string_lossy()))
}

/// Libellé court de l'action pour la pastille d'une ligne (« Déplacer vers un dossier » → « Déplacer »).
pub fn short_action(label: &str) -> String {
    if label.contains("corbeille") {
        return "Corbeille".into();
    }
    label.split_whitespace().next().unwrap_or(label).to_string()
}

/// Nom du dossier d'arrivée, ou « — » quand il n'y en a pas (échec, suppression, aperçu).
pub fn destination_label(e: &HistoryEntry) -> String {
    if !matches!(e.status, Status::Applied | Status::Undone) || e.result_path.as_os_str().is_empty() {
        return "—".into();
    }
    e.result_path.parent().and_then(|p| p.file_name()).map(|n| nfc(&n.to_string_lossy())).unwrap_or_else(|| "—".into())
}

fn matches_query(e: &HistoryEntry, query: &str) -> bool {
    query.is_empty() || [file_name(e), destination_label(e), e.rule_name.clone()].iter().any(|t| fold(t).contains(query))
}

/// Regroupe l'historique : jours, puis lots, le plus récent d'abord. `query` est déjà passé par `fold`.
pub fn group(history: &[HistoryEntry], now: SystemTime, filter: StatusFilter, query: &str) -> Vec<DayView> {
    let mut days: Vec<DayView> = Vec::new();
    let mut last_key: Option<(String, u64)> = None;
    for (i, e) in history.iter().enumerate().rev() {
        if !filter.matches(e.status) || !matches_query(e, query) {
            continue;
        }
        let key = (date_str(e.created), e.batch_id);
        if last_key.as_ref() != Some(&key) {
            let label = day_label(e.created, now);
            if days.last().is_none_or(|d| d.label != label) {
                days.push(DayView { label, batches: Vec::new() });
            }
            if let Some(day) = days.last_mut() {
                day.batches.push(BatchView { batch_id: e.batch_id, created: e.created, entries: Vec::new() });
            }
            last_key = Some(key);
        }
        if let Some(batch) = days.last_mut().and_then(|d| d.batches.last_mut()) {
            batch.entries.push(i);
        }
    }
    for batch in days.iter_mut().flat_map(|d| d.batches.iter_mut()) {
        batch.entries.reverse();
    }
    days
}

#[derive(Debug, PartialEq, Eq)]
pub struct BatchSummary {
    pub files: usize,
    pub applied: usize,
    pub failed: usize,
    pub undone: usize,
    pub skipped: usize,
    pub undoable: usize,
}

pub fn batch_summary(history: &[HistoryEntry], entries: &[usize]) -> BatchSummary {
    let mut s = BatchSummary { files: 0, applied: 0, failed: 0, undone: 0, skipped: 0, undoable: 0 };
    let mut seen = HashSet::new();
    for &i in entries {
        let e = &history[i];
        if e.status != Status::Skipped && seen.insert(&e.original_path) {
            s.files += 1;
        }
        match e.status {
            Status::Applied => s.applied += 1,
            Status::Failed | Status::BlockedByConflict => s.failed += 1,
            Status::Undone => s.undone += 1,
            Status::Skipped => s.skipped += 1,
            Status::DryRun => {}
        }
        if is_undoable(e) {
            s.undoable += 1;
        }
    }
    s
}

pub fn is_undoable(e: &HistoryEntry) -> bool {
    e.reversible && e.status == Status::Applied
}

/// Nombre d'actions du dernier lot refusées par macOS (permissions) : de quoi afficher le bandeau d'aide.
pub fn latest_permission_failures(history: &[HistoryEntry]) -> usize {
    let Some(last) = history.last().map(|e| e.batch_id) else { return 0 };
    history
        .iter()
        .rev()
        .take_while(|e| e.batch_id == last)
        .filter(|e| matches!(e.status, Status::Failed | Status::BlockedByConflict) && crate::engine::is_permission_error(&e.message))
        .count()
}

/// Ordre d'annulation d'un lot : l'action la plus récente d'abord (un déplacement suivi d'un
/// renommage se défait en annulant le renommage avant le déplacement).
pub fn undo_order(history: &[HistoryEntry], entries: &[usize]) -> Vec<usize> {
    entries.iter().rev().copied().filter(|&i| is_undoable(&history[i])).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const DAY: u64 = 86_400;

    fn at(secs: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(secs)
    }

    fn entry(batch: u64, secs: u64, status: Status, from: &str, to: &str) -> HistoryEntry {
        HistoryEntry {
            batch_id: batch,
            rule_name: "Tri".into(),
            action_label: "Déplacer vers un dossier".into(),
            original_path: PathBuf::from(from),
            result_path: PathBuf::from(to),
            undo: None,
            reversible: status == Status::Applied,
            status,
            message: String::new(),
            created: at(secs),
        }
    }

    fn sample() -> Vec<HistoryEntry> {
        let t0 = 100 * DAY;
        vec![
            entry(1, t0 + 10, Status::Applied, "/in/vieux.pdf", "/out/Archives/vieux.pdf"),
            entry(2, t0 + DAY + 10, Status::Applied, "/in/facture.pdf", "/out/Compta/facture.pdf"),
            entry(2, t0 + DAY + 11, Status::Failed, "/in/Relevé.pdf", "/in/Relevé.pdf"),
            entry(3, t0 + 2 * DAY + 5, Status::Undone, "/in/photo.png", "/out/Images/photo.png"),
        ]
    }

    #[test]
    fn ago_formats_relative_time() {
        let now = SystemTime::now();
        assert_eq!(ago(now - Duration::from_secs(10), now), "à l'instant");
        assert_eq!(ago(now - Duration::from_secs(5 * 60), now), "il y a 5 min");
        assert_eq!(ago(now - Duration::from_secs(3 * 3600), now), "il y a 3 h");
        assert_eq!(ago(now - Duration::from_secs(30 * 3600), now), "hier");
        assert_eq!(ago(now - Duration::from_secs(5 * 86_400), now), "il y a 5 j");
    }

    #[test]
    fn groups_by_day_then_batch_newest_first() {
        let now = at(102 * DAY + 100);
        let days = group(&sample(), now, StatusFilter::All, "");
        let labels: Vec<&str> = days.iter().map(|d| d.label.as_str()).collect();
        assert_eq!(labels, vec!["Aujourd'hui", "Hier", "1970-04-11"]);
        assert_eq!(days[1].batches[0].batch_id, 2);
        assert_eq!(days[1].batches[0].entries, vec![1, 2], "ordre chronologique dans le lot");
    }

    #[test]
    fn filters_by_status_and_by_folded_query() {
        let now = at(102 * DAY + 100);
        let h = sample();
        let failed = group(&h, now, StatusFilter::Failed, "");
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].batches[0].entries, vec![2]);
        let found = group(&h, now, StatusFilter::All, &fold("RELEVÉ"));
        assert_eq!(found.iter().flat_map(|d| &d.batches).flat_map(|b| &b.entries).count(), 1);
        assert!(group(&h, now, StatusFilter::All, "zzz").is_empty());
    }

    #[test]
    fn summary_counts_files_and_statuses() {
        let h = sample();
        let s = batch_summary(&h, &[1, 2]);
        assert_eq!(s, BatchSummary { files: 2, applied: 1, failed: 1, undone: 0, skipped: 0, undoable: 1 });
    }

    #[test]
    fn skipped_rule_is_named_and_never_counted_as_failure() {
        let now = at(102 * DAY + 100);
        let mut h = sample();
        let mut e = entry(4, 102 * DAY + 20, Status::Skipped, "", "");
        e.rule_name = "Brouillon".into();
        e.action_label = "Règle incomplète".into();
        e.message = "Règle incomplète — Choisissez le dossier de destination.".into();
        h.push(e);

        assert_eq!(file_name(&h[4]), "Brouillon");
        assert_eq!(destination_label(&h[4]), "—");
        let s = batch_summary(&h, &[4]);
        assert_eq!(s, BatchSummary { files: 0, applied: 0, failed: 0, undone: 0, skipped: 1, undoable: 0 });
        assert!(group(&h, now, StatusFilter::Failed, "").iter().flat_map(|d| &d.batches).flat_map(|b| &b.entries).all(|&i| i != 4));
        let all = group(&h, now, StatusFilter::All, "");
        assert_eq!(all[0].batches[0].entries, vec![4]);
    }

    #[test]
    fn labels_show_file_name_and_destination_folder_only() {
        let h = sample();
        assert_eq!(file_name(&h[0]), "vieux.pdf");
        assert_eq!(destination_label(&h[0]), "Archives");
        assert_eq!(destination_label(&h[2]), "—", "un échec n'a pas de destination");
    }

    #[test]
    fn permission_failures_only_count_the_latest_batch() {
        let mut h = sample();
        h[0].status = Status::Failed;
        h[0].message = "Operation not permitted (os error 1)".into();
        assert_eq!(latest_permission_failures(&h), 0, "un ancien lot ne compte pas");
        h[3].status = Status::Failed;
        h[3].message = "Permission denied (os error 13)".into();
        assert_eq!(latest_permission_failures(&h), 1);
    }

    #[test]
    fn short_action_keeps_the_verb() {
        assert_eq!(short_action("Déplacer vers un dossier"), "Déplacer");
        assert_eq!(short_action("Mettre à la corbeille"), "Corbeille");
        assert_eq!(short_action("Compresser (ZIP)"), "Compresser");
    }

    #[test]
    fn undo_order_is_newest_first_and_skips_non_undoable() {
        let h = sample();
        assert_eq!(undo_order(&h, &[0, 1, 2]), vec![1, 0]);
    }
}
