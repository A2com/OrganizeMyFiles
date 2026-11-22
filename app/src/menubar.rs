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

/// Un clic sur l'icône du Dock demande de rouvrir la fenêtre (événement macOS « rouvrir l'application »).
static REOPEN_REQUESTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn request_reopen() {
    REOPEN_REQUESTED.store(true, std::sync::atomic::Ordering::SeqCst);
}

/// Vrai une seule fois par clic sur l'icône du Dock (remet la demande à zéro).
pub fn take_reopen_request() -> bool {
    REOPEN_REQUESTED.swap(false, std::sync::atomic::Ordering::SeqCst)
}

#[cfg(target_os = "macos")]
mod imp {
    use std::sync::mpsc::{channel, Receiver};

    use muda::accelerator::{Accelerator, Code, Modifiers};
    use muda::{ContextMenu, IconMenuItem, IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
    use objc2::rc::Retained;
    use objc2::runtime::{AnyObject, NSObject};
    use objc2::{define_class, msg_send, sel, AllocAnyThread, MainThreadMarker};
    use objc2_app_kit::{
        NSApplication, NSApplicationActivationPolicy, NSAutoresizingMaskOptions, NSControlSize, NSImage, NSImageView, NSMenu, NSMenuItem,
        NSSwitch, NSTextField, NSView,
    };
    use objc2_foundation::{NSAppleEventDescriptor, NSAppleEventManager, NSData, NSPoint, NSRect, NSSize, NSString};
    use tray_icon::{TrayIcon, TrayIconBuilder};

    use super::{menubar_info, MenuCommand, MenuInfo, MenuScreen};

    macro_rules! menu_icon {
        ($name:literal) => {
            include_bytes!(concat!("../../assets/icons/menu/", $name, ".png"))
        };
    }

    define_class!(
        /// Reçoit l'événement « rouvrir l'application » (clic sur l'icône du Dock) : ni winit ni eframe ne le traitent.
        #[unsafe(super(NSObject))]
        #[name = "OMFReopenHandler"]
        struct ReopenHandler;

        impl ReopenHandler {
            #[unsafe(method(handleReopen:withReplyEvent:))]
            fn handle_reopen(&self, _event: &NSAppleEventDescriptor, _reply: &NSAppleEventDescriptor) {
                super::request_reopen();
                if let Some(wake) = REOPEN_WAKE.get() {
                    wake();
                }
            }
        }
    );

    static REOPEN_WAKE: std::sync::OnceLock<std::sync::Arc<dyn Fn() + Send + Sync>> = std::sync::OnceLock::new();

    /// `'aevt'` (événements de base) et `'rapp'` (rouvrir l'application).
    const CORE_EVENT_CLASS: u32 = 0x6165_7674;
    const REOPEN_EVENT_ID: u32 = 0x7261_7070;

    /// Branche le gestionnaire une seule fois. À faire après le démarrage de l'appli : avant, macOS remplacerait le nôtre.
    fn install_reopen_handler(wake: std::sync::Arc<dyn Fn() + Send + Sync>) {
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| {
            let _ = REOPEN_WAKE.set(wake);
            let handler: Retained<ReopenHandler> = unsafe { msg_send![ReopenHandler::alloc(), init] };
            let manager = NSAppleEventManager::sharedAppleEventManager();
            // Appel direct : la méthode typée de objc2-foundation exige une crate supplémentaire (core-services).
            unsafe {
                let _: () = msg_send![
                    &*manager,
                    setEventHandler: &*handler,
                    andSelector: sel!(handleReopen:withReplyEvent:),
                    forEventClass: CORE_EVENT_CLASS,
                    andEventID: REOPEN_EVENT_ID,
                ];
            }
            // Le gestionnaire doit vivre autant que l'appli : le gestionnaire d'événements ne le retient pas.
            std::mem::forget(handler);
        });
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

    /// Ligne « Surveillance » du menu : icône, libellé et vrai interrupteur macOS (NSSwitch).
    struct SwitchRow {
        _view: Retained<NSView>,
        switch: Retained<NSSwitch>,
    }

    impl SwitchRow {
        /// Remplace l'affichage de la ligne `index` du menu par une vue avec un interrupteur. Le clic sur
        /// l'interrupteur réutilise la cible et l'action de l'élément de menu (installées par muda) :
        /// il produit donc le même événement qu'un clic sur la ligne.
        fn install(menu: &Menu, index: isize, label: &str, icon_png: &[u8], enabled: bool) -> Option<Self> {
            let mtm = MainThreadMarker::new()?;
            let ptr = menu.ns_menu() as *const NSMenu;
            // SAFETY : voir `mark_templates`.
            let ns_menu = unsafe { ptr.as_ref() }?;
            let entry: Retained<NSMenuItem> = ns_menu.itemArray().iter().nth(index as usize)?;

            let (width, height) = (280.0, 30.0);
            let rect = |x: f64, y: f64, w: f64, h: f64| NSRect::new(NSPoint::new(x, y), NSSize::new(w, h));
            let view = NSView::initWithFrame(mtm.alloc(), rect(0.0, 0.0, width, height));
            view.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable);

            let data = NSData::with_bytes(icon_png);
            let image = NSImage::initWithData(mtm.alloc(), &data)?;
            image.setTemplate(true);
            let icon = NSImageView::imageViewWithImage(&image, mtm);
            icon.setFrame(rect(14.0, 6.0, 18.0, 18.0));
            view.addSubview(&icon);

            let text = NSTextField::labelWithString(&NSString::from_str(label), mtm);
            text.setFrame(rect(40.0, 5.0, 150.0, 20.0));
            view.addSubview(&text);

            let switch = NSSwitch::initWithFrame(mtm.alloc(), rect(width - 54.0, 4.0, 40.0, 22.0));
            switch.setControlSize(NSControlSize::Small);
            switch.setAutoresizingMask(NSAutoresizingMaskOptions::ViewMinXMargin);
            switch.setState(if enabled { 1 } else { 0 });
            // Même cible et même action que l'élément de menu : le clic déclenche l'événement de menu « toggle ».
            let target: &AnyObject = &entry;
            unsafe {
                switch.setTarget(Some(target));
                switch.setAction(Some(sel!(fireMenuItemAction:)));
            }
            view.addSubview(&switch);

            entry.setView(Some(&view));
            Some(SwitchRow { _view: view, switch })
        }

        fn set_enabled(&self, enabled: bool) {
            self.switch.setState(if enabled { 1 } else { 0 });
        }
    }

    pub struct MenuBar {
        tray: TrayIcon,
        menu: Menu,
        status: IconMenuItem,
        counts: IconMenuItem,
        last: IconMenuItem,
        toggle: IconMenuItem,
        switch: Option<SwitchRow>,
        folders: Submenu,
        rules: IconMenuItem,
        commands: Receiver<MenuCommand>,
        shown: Option<MenuInfo>,
    }

    impl MenuBar {
        /// `wake` réveille l'interface à chaque clic dans le menu.
        pub fn new(wake: impl Fn() + Send + Sync + 'static) -> Option<Self> {
            let wake: std::sync::Arc<dyn Fn() + Send + Sync> = std::sync::Arc::new(wake);
            install_reopen_handler(wake.clone());
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
            let switch = SwitchRow::install(&menu, 8, "Surveillance", menu_icon!("veille"), true);
            let bar = MenuBar { tray, menu, status, counts, last, toggle, switch, folders, rules, commands, shown: None };
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
            match &self.switch {
                Some(row) => row.set_enabled(info.enabled),
                None => self.toggle.set_text(menubar_info::toggle_label(info.enabled)),
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dock_click_is_reported_once() {
        assert!(!take_reopen_request(), "rien à signaler au départ");
        request_reopen();
        assert!(take_reopen_request());
        assert!(!take_reopen_request(), "la demande est consommée");
    }
}
