//! Icône et menu de l'appli dans la barre des menus de macOS, et gestion de l'icône du Dock.
//! Ailleurs que sur macOS : `MenuBar::new` renvoie `None` et rien n'est affiché.

#[cfg(not(target_os = "macos"))]
use crate::menubar_info::MenuInfo;
#[cfg(target_os = "macos")]
use crate::menubar_info::{self, MenuInfo};

/// Écran de l'appli qu'une entrée du menu peut ouvrir.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub enum MenuScreen {
    Dashboard,
    Folders,
    Rules,
    History,
    Settings,
}

/// Ce que l'utilisateur a demandé dans le menu (construit seulement sur macOS).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub enum MenuCommand {
    Open(MenuScreen),
    Organize,
    Preview,
    ToggleEnabled,
    RevealFolder(u64),
    Quit,
}

#[cfg(target_os = "macos")]
mod imp {
    use std::sync::mpsc::{channel, Receiver};

    use muda::accelerator::{Accelerator, Code, Modifiers};
    use muda::{ContextMenu, IconMenuItem, IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSMenu};
    use tray_icon::{TrayIcon, TrayIconBuilder};

    use super::{menubar_info, MenuCommand, MenuInfo, MenuScreen};

    macro_rules! menu_icon {
        ($name:literal) => {
            include_bytes!(concat!("../../assets/icons/menu/", $name, ".png"))
        };
    }

    const TRAY_ICON: &[u8] = include_bytes!("../../assets/icons/tray-template-44.png");
    const TRAY_ICON_VEILLE: &[u8] = include_bytes!("../../assets/icons/tray-template-veille-44.png");

    fn muda_icon(png: &[u8]) -> Option<muda::Icon> {
        let img = image::load_from_memory(png).ok()?.to_rgba8();
        let (w, h) = (img.width(), img.height());
        muda::Icon::from_rgba(img.into_raw(), w, h).ok()
    }

    fn tray_icon(png: &[u8]) -> Option<tray_icon::Icon> {
        let img = image::load_from_memory(png).ok()?.to_rgba8();
        let (w, h) = (img.width(), img.height());
        tray_icon::Icon::from_rgba(img.into_raw(), w, h).ok()
    }

    fn item(id: &str, text: &str, enabled: bool, icon: &[u8], accel: Option<Accelerator>) -> IconMenuItem {
        IconMenuItem::with_id(id, text, enabled, muda_icon(icon), accel)
    }

    fn cmd(key: Code) -> Option<Accelerator> {
        Some(Accelerator::new(Some(Modifiers::SUPER), key))
    }

    /// Marque toutes les images du menu comme « template » : macOS les teinte selon le mode clair ou sombre.
    fn mark_templates(menu: &Menu) {
        fn walk(menu: &NSMenu) {
            for entry in menu.itemArray().iter() {
                if let Some(image) = entry.image() {
                    image.setTemplate(true);
                }
                if let Some(sub) = entry.submenu() {
                    walk(&sub);
                }
            }
        }
        let ptr = menu.ns_menu() as *const NSMenu;
        // SAFETY : muda renvoie un pointeur vers le NSMenu qu'il possède, valable tant que `menu` vit.
        if let Some(ns_menu) = unsafe { ptr.as_ref() } {
            walk(ns_menu);
        }
    }

    pub struct MenuBar {
        tray: TrayIcon,
        menu: Menu,
        status: IconMenuItem,
        counts: IconMenuItem,
        last: IconMenuItem,
        toggle: IconMenuItem,
        folders: Submenu,
        rules: IconMenuItem,
        commands: Receiver<MenuCommand>,
        shown: Option<MenuInfo>,
    }

    impl MenuBar {
        /// `wake` réveille l'interface à chaque clic dans le menu.
        pub fn new(wake: impl Fn() + Send + Sync + 'static) -> Option<Self> {
            let title = MenuItem::with_id("title", "OrganizeMyFiles", false, None);
            let status = item("status", "Surveillance active", false, menu_icon!("actif"), None);
            let counts = item("counts", "—", false, menu_icon!("dossiers"), None);
            let last = item("last", "Aucun rangement pour l'instant", false, menu_icon!("historique"), None);
            let open = item("open", "Ouvrir OrganizeMyFiles", true, menu_icon!("ouvrir"), cmd(Code::KeyO));
            let organize = item("organize", "Organiser maintenant", true, menu_icon!("organiser"), cmd(Code::KeyR));
            let preview = item("preview", "Aperçu (simulation)", true, menu_icon!("apercu"), cmd(Code::KeyP));
            let toggle = item("toggle", "Mettre en veille", true, menu_icon!("veille"), None);
            let folders = Submenu::with_id("folders", "Dossiers suivis", true);
            folders.set_icon(muda_icon(menu_icon!("dossiers")));
            let rules = item("rules", "Règles", true, menu_icon!("regles"), None);
            let history = item("history", "Historique", true, menu_icon!("historique"), None);
            let settings = item("settings", "Réglages…", true, menu_icon!("reglages"), cmd(Code::Comma));
            let quit = item("quit", "Quitter OrganizeMyFiles", true, menu_icon!("quitter"), cmd(Code::KeyQ));

            let menu = Menu::new();
            let sep = || PredefinedMenuItem::separator();
            let (s1, s2, s3) = (sep(), sep(), sep());
            let all: [&dyn IsMenuItem; 16] = [
                &title, &status, &counts, &last, &s1, &open, &organize, &preview, &toggle, &s2, &folders, &rules, &history, &settings, &s3,
                &quit,
            ];
            menu.append_items(&all).ok()?;

            let tray = TrayIconBuilder::new()
                .with_menu(Box::new(menu.clone()))
                .with_icon(tray_icon(TRAY_ICON)?)
                .with_icon_as_template(true)
                .with_tooltip("OrganizeMyFiles")
                .build()
                .ok()?;

            let (tx, commands) = channel();
            MenuEvent::set_event_handler(Some(move |ev: MenuEvent| {
                let id = ev.id.0.as_str();
                let command = match id {
                    "open" => MenuCommand::Open(MenuScreen::Dashboard),
                    "manage-folders" => MenuCommand::Open(MenuScreen::Folders),
                    "rules" => MenuCommand::Open(MenuScreen::Rules),
                    "history" => MenuCommand::Open(MenuScreen::History),
                    "settings" => MenuCommand::Open(MenuScreen::Settings),
                    "organize" => MenuCommand::Organize,
                    "preview" => MenuCommand::Preview,
                    "toggle" => MenuCommand::ToggleEnabled,
                    "quit" => MenuCommand::Quit,
                    other => match other.strip_prefix("folder:").and_then(|n| n.parse().ok()) {
                        Some(folder_id) => MenuCommand::RevealFolder(folder_id),
                        None => return,
                    },
                };
                let _ = tx.send(command);
                wake();
            }));
            let bar = MenuBar { tray, menu, status, counts, last, toggle, folders, rules, commands, shown: None };
            mark_templates(&bar.menu);
            Some(bar)
        }

        pub fn poll(&self) -> Vec<MenuCommand> {
            self.commands.try_iter().collect()
        }

        /// Met le menu à jour : seulement ce qui a changé depuis la dernière fois.
        pub fn update(&mut self, info: &MenuInfo) {
            if self.shown.as_ref() == Some(info) {
                return;
            }
            let previous = self.shown.replace(info.clone());
            self.status.set_text(menubar_info::status_line(info));
            self.status.set_icon(muda_icon(if info.enabled { menu_icon!("actif") } else { menu_icon!("pause") }));
            self.counts.set_text(menubar_info::counts_line(info));
            if let Some(text) = &info.last_batch {
                self.last.set_text(text);
            }
            self.toggle.set_text(menubar_info::toggle_label(info.enabled));
            self.rules.set_text(menubar_info::rules_menu_title(info));
            self.folders.set_text(menubar_info::folders_menu_title(info));
            if previous.as_ref().map(|p| &p.folders) != Some(&info.folders) {
                self.rebuild_folders(info);
            }
            if previous.as_ref().map(|p| p.enabled) != Some(info.enabled) {
                if let Some(icon) = tray_icon(if info.enabled { TRAY_ICON } else { TRAY_ICON_VEILLE }) {
                    let _ = self.tray.set_icon_with_as_template(Some(icon), true);
                }
            }
            let _ = self.tray.set_tooltip(Some(menubar_info::tooltip(info)));
            mark_templates(&self.menu);
        }

        fn rebuild_folders(&self, info: &MenuInfo) {
            while self.folders.remove_at(0).is_some() {}
            if info.folders.is_empty() {
                let none = item("no-folder", "Aucun dossier suivi", false, menu_icon!("dossier"), None);
                let _ = self.folders.append(&none);
            }
            for f in &info.folders {
                let entry = item(&format!("folder:{}", f.id), &menubar_info::folder_entry_label(f), true, menu_icon!("dossier"), None);
                let _ = self.folders.append(&entry);
            }
            let manage = item("manage-folders", "Gérer les dossiers…", true, menu_icon!("dossiers"), None);
            let _ = self.folders.append(&PredefinedMenuItem::separator());
            let _ = self.folders.append(&manage);
        }
    }

    /// Affiche ou masque l'icône de l'appli dans le Dock (politique d'activation de macOS).
    /// Fenêtre fermée : l'appli devient une appli de barre des menus (`false`) ; rouverte : `true`.
    pub fn set_dock_visible(visible: bool) {
        let Some(mtm) = MainThreadMarker::new() else { return };
        let app = NSApplication::sharedApplication(mtm);
        let policy = if visible { NSApplicationActivationPolicy::Regular } else { NSApplicationActivationPolicy::Accessory };
        app.setActivationPolicy(policy);
        if visible {
            #[allow(deprecated)]
            app.activateIgnoringOtherApps(true);
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    use super::{MenuCommand, MenuInfo};

    pub struct MenuBar;

    impl MenuBar {
        pub fn new(_wake: impl Fn() + Send + Sync + 'static) -> Option<Self> {
            None
        }
        pub fn poll(&self) -> Vec<MenuCommand> {
            Vec::new()
        }
        pub fn update(&mut self, _info: &MenuInfo) {}
    }

    pub fn set_dock_visible(_visible: bool) {}
}

pub use imp::{set_dock_visible, MenuBar};
