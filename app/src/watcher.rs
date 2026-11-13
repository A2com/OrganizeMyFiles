//! Surveillance automatique des dossiers : un fil de travail écoute le système (FSEvents sur
//! macOS), attend que les fichiers « se calment », puis applique les règles seulement aux
//! fichiers concernés. Rien ne tourne dans l'interface ; les résultats reviennent par un canal.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use notify::{EventKind, RecursiveMode, Watcher as _};

use crate::engine::{Folder, HistoryEntry, Rule, Runner};
use crate::system::SystemCommands;

/// Attente après le dernier événement avant de traiter un fichier.
pub const SETTLE: Duration = Duration::from_secs(3);
/// Un fichier produit par l'appli est ignoré pendant ce délai (anti-boucle).
const OWN_OUTPUT_TTL: Duration = Duration::from_secs(15);

/// Ce que le fil de travail doit surveiller : copie des dossiers et règles de l'interface.
#[derive(Clone, Default)]
pub struct WatchConfig {
    pub folders: Vec<Folder>,
    pub rules: Vec<Rule>,
    /// `false` = appli « en veille » : on n'applique rien.
    pub enabled: bool,
    /// Passage complet sur les fichiers déjà présents, au lancement puis à cet intervalle ; `None` = désactivé.
    pub auto_every: Option<Duration>,
}

/// Résultat d'un traitement automatique, à ajouter à l'historique par l'interface.
pub struct WatchResult {
    pub matched: usize,
    pub scanned: usize,
    pub history: Vec<HistoryEntry>,
    pub monthly_delta: u32,
}

enum Msg {
    Event(PathBuf),
    /// La configuration a changé : refaire la liste des dossiers écoutés.
    Reconfigure,
    /// Fichiers que l'appli vient elle-même de déplacer ou restaurer : à ne pas retraiter.
    Ignore(Vec<PathBuf>),
    Stop,
}

pub struct Watcher {
    config: Arc<Mutex<WatchConfig>>,
    tx: Sender<Msg>,
    results: Receiver<WatchResult>,
    #[cfg_attr(not(test), allow(dead_code))]
    watched: Arc<AtomicUsize>,
}

impl Watcher {
    /// Démarre le fil de travail. `wake` est appelé quand un résultat est prêt (réveil de l'interface).
    pub fn start(settle: Duration, wake: impl Fn() + Send + 'static) -> Self {
        let config = Arc::new(Mutex::new(WatchConfig::default()));
        let watched = Arc::new(AtomicUsize::new(0));
        let (tx, rx) = channel();
        let (res_tx, results) = channel();
        let worker = Worker { config: config.clone(), watched: watched.clone(), events: tx.clone(), settle };
        std::thread::spawn(move || worker.run(rx, res_tx, wake));
        Watcher { config, tx, results, watched }
    }

    pub fn set_config(&self, cfg: WatchConfig) {
        if let Ok(mut c) = self.config.lock() {
            *c = cfg;
        }
        let _ = self.tx.send(Msg::Reconfigure);
    }

    pub fn ignore(&self, paths: Vec<PathBuf>) {
        let _ = self.tx.send(Msg::Ignore(paths));
    }

    pub fn try_recv(&self) -> Option<WatchResult> {
        self.results.try_recv().ok()
    }

    /// Nombre de dossiers réellement écoutés en ce moment (sert aux tests : savoir quand l'écoute est prête).
    #[cfg(test)]
    pub fn watched_folders(&self) -> usize {
        self.watched.load(Ordering::Relaxed)
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        let _ = self.tx.send(Msg::Stop);
    }
}

// ------------------------------------------------------------ logique pure (testée)

/// Téléchargement ou copie en cours, ou fichier système sans intérêt : on n'y touche pas.
pub fn is_ignorable(path: &Path) -> bool {
    let name = path.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    if name == ".ds_store" {
        return true;
    }
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    matches!(ext.as_str(), "download" | "crdownload" | "part" | "partial" | "tmp" | "opdownload")
}

/// Fichiers en attente : on ne traite un fichier qu'une fois qu'il n'a plus bougé pendant `settle`.
pub struct Debouncer {
    settle: Duration,
    /// chemin → (dernier événement, taille vue à ce moment-là)
    pending: HashMap<PathBuf, (Instant, Option<u64>)>,
}

impl Debouncer {
    pub fn new(settle: Duration) -> Self {
        Debouncer { settle, pending: HashMap::new() }
    }

    pub fn push(&mut self, path: PathBuf, now: Instant, size: Option<u64>) {
        self.pending.insert(path, (now, size));
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    pub fn clear(&mut self) {
        self.pending.clear();
    }

    /// Chemins calmes depuis `settle`. `size_of` donne la taille actuelle : si elle a changé
    /// depuis le dernier événement, le fichier grossit encore et on repart pour une attente.
    pub fn take_ready(&mut self, now: Instant, size_of: impl Fn(&Path) -> Option<u64>) -> Vec<PathBuf> {
        let due: Vec<PathBuf> =
            self.pending.iter().filter(|(_, (t, _))| now.duration_since(*t) >= self.settle).map(|(p, _)| p.clone()).collect();
        let mut ready = Vec::new();
        for p in due {
            let (_, seen) = self.pending[&p];
            let current = size_of(&p);
            if current != seen {
                self.pending.insert(p, (now, current));
            } else {
                self.pending.remove(&p);
                ready.push(p);
            }
        }
        ready.sort();
        ready
    }
}

/// Mémoire des fichiers produits par l'appli (anti-boucle).
pub struct OwnOutputs {
    ttl: Duration,
    seen: HashMap<PathBuf, Instant>,
}

impl OwnOutputs {
    pub fn new(ttl: Duration) -> Self {
        OwnOutputs { ttl, seen: HashMap::new() }
    }

    pub fn add(&mut self, path: PathBuf, now: Instant) {
        self.seen.insert(path, now);
    }

    pub fn contains(&mut self, path: &Path, now: Instant) -> bool {
        self.seen.retain(|_, t| now.duration_since(*t) < self.ttl);
        self.seen.contains_key(path)
    }
}

// ----------------------------------------------------------------------- fil de travail

struct Worker {
    config: Arc<Mutex<WatchConfig>>,
    watched: Arc<AtomicUsize>,
    events: Sender<Msg>,
    settle: Duration,
}

impl Worker {
    fn run(self, rx: Receiver<Msg>, results: Sender<WatchResult>, wake: impl Fn()) {
        let mut debouncer = Debouncer::new(self.settle);
        let mut own = OwnOutputs::new(OWN_OUTPUT_TTL);
        let mut last_full: Option<Instant> = None;
        let mut last_every: Option<Duration> = None;
        // Garder l'écouteur en vie : le lâcher arrête l'écoute (aussi à la sortie de la boucle).
        let mut _fs_watcher: Option<notify::RecommendedWatcher> = None;
        loop {
            match rx.recv_timeout(Duration::from_millis(200)) {
                Ok(Msg::Stop) | Err(RecvTimeoutError::Disconnected) => return,
                Ok(Msg::Reconfigure) => _fs_watcher = self.rebuild_watcher(),
                Ok(Msg::Ignore(paths)) => {
                    let now = Instant::now();
                    for p in paths {
                        own.add(p, now);
                    }
                }
                Ok(Msg::Event(path)) => {
                    let now = Instant::now();
                    if !is_ignorable(&path) && !own.contains(&path, now) {
                        debouncer.push(path.clone(), now, file_size(&path));
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
            }
            let (enabled, auto_every) = self.config.lock().map(|c| (c.enabled, c.auto_every)).unwrap_or_default();
            if auto_every != last_every {
                last_every = auto_every;
                last_full = None; // nouvel intervalle (ou option activée) : passage immédiat
            }
            if let (true, Some(every)) = (enabled, auto_every) {
                let now = Instant::now();
                if last_full.is_none_or(|t| now.duration_since(t) >= every) {
                    last_full = Some(now);
                    let config = self.config.lock().map(|c| c.clone()).unwrap_or_default();
                    let outcome = process(&config, None);
                    for e in &outcome.history {
                        own.add(e.original_path.clone(), now);
                        own.add(e.result_path.clone(), now);
                    }
                    if !outcome.history.is_empty() {
                        let _ = results.send(outcome);
                        wake();
                    }
                }
            }
            if debouncer.is_empty() {
                continue;
            }
            let config = self.config.lock().map(|c| c.clone()).unwrap_or_default();
            if !config.enabled {
                debouncer.clear();
                continue;
            }
            let now = Instant::now();
            let ready: Vec<PathBuf> =
                debouncer.take_ready(now, file_size).into_iter().filter(|p| p.exists() && !own.contains(p, now)).collect();
            if ready.is_empty() {
                continue;
            }
            let outcome = process(&config, Some(ready));
            for e in &outcome.history {
                own.add(e.original_path.clone(), now);
                own.add(e.result_path.clone(), now);
            }
            if !outcome.history.is_empty() {
                let _ = results.send(outcome);
                wake();
            }
        }
    }

    /// Écoute les dossiers actifs ; récursif seulement si une règle descend dans les sous-dossiers.
    fn rebuild_watcher(&self) -> Option<notify::RecommendedWatcher> {
        let config = self.config.lock().map(|c| c.clone()).unwrap_or_default();
        let tx = self.events.clone();
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if let Ok(ev) = res {
                if matches!(ev.kind, EventKind::Create(_) | EventKind::Modify(_)) {
                    for p in ev.paths {
                        let _ = tx.send(Msg::Event(p));
                    }
                }
            }
        })
        .ok()?;
        let mut count = 0;
        for folder in config.folders.iter().filter(|f| f.enabled) {
            let deep = config.rules.iter().any(|r| r.folder_ids.contains(&folder.id) && r.enabled && r.recursion_depth > 0);
            let mode = if deep { RecursiveMode::Recursive } else { RecursiveMode::NonRecursive };
            if watcher.watch(&folder.path, mode).is_ok() {
                count += 1;
            }
        }
        self.watched.store(count, Ordering::Relaxed);
        Some(watcher)
    }
}

fn file_size(p: &Path) -> Option<u64> {
    std::fs::metadata(p).ok().map(|m| m.len())
}

/// Applique les règles aux seuls fichiers donnés, ou à tout le contenu des dossiers suivis si `files` est `None`.
fn process(config: &WatchConfig, files: Option<Vec<PathBuf>>) -> WatchResult {
    let mut runner = Runner {
        monthly_used: 0,
        batch_id: 0,
        dry_run: false,
        commands: Box::new(SystemCommands),
        only_paths: files.map(|f| f.into_iter().collect::<HashSet<_>>()),
    };
    let outcome = runner.run(&config.folders, &config.rules, SystemTime::now());
    WatchResult { matched: outcome.matched, scanned: outcome.scanned, history: outcome.history, monthly_delta: runner.monthly_used }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Action, ActionKind, Condition, Field, Op, Status};

    #[test]
    fn partial_downloads_and_system_files_are_ignored() {
        for n in ["a.crdownload", "b.PART", "c.download", "d.tmp", ".DS_Store"] {
            assert!(is_ignorable(Path::new(n)), "{n}");
        }
        for n in ["photo.png", "notes.txt", "archive.zip"] {
            assert!(!is_ignorable(Path::new(n)), "{n}");
        }
    }

    #[test]
    fn debouncer_waits_for_quiet_and_for_a_stable_size() {
        let t0 = Instant::now();
        let mut d = Debouncer::new(Duration::from_secs(3));
        let p = PathBuf::from("/x/a.png");
        d.push(p.clone(), t0, Some(10));
        assert!(d.take_ready(t0 + Duration::from_secs(1), |_| Some(10)).is_empty());
        // La taille a bougé : le fichier grossit encore, on réarme l'attente.
        assert!(d.take_ready(t0 + Duration::from_secs(4), |_| Some(20)).is_empty());
        assert!(d.take_ready(t0 + Duration::from_secs(5), |_| Some(20)).is_empty());
        assert_eq!(d.take_ready(t0 + Duration::from_secs(8), |_| Some(20)), vec![p]);
        assert!(d.is_empty());
    }

    #[test]
    fn own_outputs_are_remembered_then_forgotten() {
        let t0 = Instant::now();
        let mut own = OwnOutputs::new(Duration::from_secs(15));
        own.add(PathBuf::from("/x/a"), t0);
        assert!(own.contains(Path::new("/x/a"), t0 + Duration::from_secs(5)));
        assert!(!own.contains(Path::new("/x/a"), t0 + Duration::from_secs(20)));
    }

    fn test_dir(tag: &str) -> PathBuf {
        let nanos = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("organizemyfiles-watch-{tag}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.canonicalize().unwrap()
    }

    /// Règle : « extension = .png » → déplacer vers `dest`.
    fn png_config(watched: &Path, dest: &Path, enabled: bool) -> WatchConfig {
        let mut act = Action::new(ActionKind::Move);
        act.params.insert("destination".into(), dest.to_string_lossy().into_owned());
        WatchConfig {
            folders: vec![Folder { id: 1, path: watched.to_path_buf(), enabled: true, priority: 0 }],
            rules: vec![Rule {
                id: 2,
                folder_ids: vec![1],
                name: "png".into(),
                enabled: true,
                match_all: true,
                recursion_depth: 0,
                priority: 0,
                conditions: vec![Condition { field: Field::Extension, op: Op::Equals, value: ".png".into() }],
                actions: vec![act],
            }],
            enabled,
            auto_every: None,
        }
    }

    fn wait_for(cond: impl Fn() -> bool) -> bool {
        for _ in 0..80 {
            if cond() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        false
    }

    #[test]
    fn a_dropped_file_is_organised_by_itself() {
        let (watched, dest) = (test_dir("in"), test_dir("out"));
        let w = Watcher::start(Duration::from_millis(300), || {});
        w.set_config(png_config(&watched, &dest, true));
        assert!(wait_for(|| w.watched_folders() == 1));
        std::fs::write(watched.join("shot.png"), b"x").unwrap();
        std::fs::write(watched.join("notes.txt"), b"x").unwrap();
        assert!(wait_for(|| dest.join("shot.png").exists()), "le fichier n'a pas été rangé");
        let result = loop {
            if let Some(r) = w.try_recv() {
                break r;
            }
            std::thread::sleep(Duration::from_millis(50));
        };
        assert_eq!(result.history.len(), 1);
        assert_eq!(result.history[0].status, Status::Applied);
        assert!(watched.join("notes.txt").exists(), "un fichier hors règle ne doit pas bouger");
    }

    #[test]
    fn existing_files_are_organised_when_auto_is_on() {
        let (watched, dest) = (test_dir("in-auto"), test_dir("out-auto"));
        std::fs::write(watched.join("deja.png"), b"x").unwrap();
        let w = Watcher::start(Duration::from_millis(300), || {});
        let mut config = png_config(&watched, &dest, true);
        config.auto_every = Some(Duration::from_secs(3600));
        w.set_config(config);
        assert!(wait_for(|| dest.join("deja.png").exists()), "le fichier déjà présent n'a pas été rangé");
    }

    #[test]
    fn existing_files_stay_put_when_auto_is_off() {
        let (watched, dest) = (test_dir("in-noauto"), test_dir("out-noauto"));
        std::fs::write(watched.join("deja.png"), b"x").unwrap();
        // macOS (FSEvents) peut rejouer la création d'un fichier écrit juste avant le démarrage : on laisse passer ce délai.
        std::thread::sleep(Duration::from_millis(2500));
        let w = Watcher::start(Duration::from_millis(300), || {});
        w.set_config(png_config(&watched, &dest, true));
        assert!(wait_for(|| w.watched_folders() == 1));
        std::thread::sleep(Duration::from_millis(1200));
        assert!(watched.join("deja.png").exists());
    }

    #[test]
    fn partial_download_is_left_alone() {
        let (watched, dest) = (test_dir("in2"), test_dir("out2"));
        let w = Watcher::start(Duration::from_millis(300), || {});
        w.set_config(png_config(&watched, &dest, true));
        assert!(wait_for(|| w.watched_folders() == 1));
        std::fs::write(watched.join("shot.png.crdownload"), b"x").unwrap();
        std::thread::sleep(Duration::from_millis(1200));
        assert!(watched.join("shot.png.crdownload").exists());
        assert!(w.try_recv().is_none());
    }

    #[test]
    fn nothing_happens_while_the_app_is_asleep() {
        let (watched, dest) = (test_dir("in3"), test_dir("out3"));
        let w = Watcher::start(Duration::from_millis(300), || {});
        w.set_config(png_config(&watched, &dest, false));
        assert!(wait_for(|| w.watched_folders() == 1));
        std::fs::write(watched.join("shot.png"), b"x").unwrap();
        std::thread::sleep(Duration::from_millis(1200));
        assert!(watched.join("shot.png").exists());
        assert!(!dest.join("shot.png").exists());
    }

    #[test]
    fn ignored_paths_are_not_reprocessed() {
        let (watched, dest) = (test_dir("in4"), test_dir("out4"));
        let w = Watcher::start(Duration::from_millis(300), || {});
        w.set_config(png_config(&watched, &dest, true));
        assert!(wait_for(|| w.watched_folders() == 1));
        let p = watched.join("restored.png");
        w.ignore(vec![p.clone()]);
        std::thread::sleep(Duration::from_millis(200));
        std::fs::write(&p, b"x").unwrap();
        std::thread::sleep(Duration::from_millis(1200));
        assert!(p.exists(), "un fichier restauré par l'appli ne doit pas être re-rangé");
    }
}
