//! Icône et menu dans la barre des menus de macOS. Ailleurs : pas d'icône (`MenuBar::new` renvoie `None`).

/// Ce que l'utilisateur a demandé dans le menu (construit seulement sur macOS).
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuCommand {
    Open,
    Organize,
    ToggleEnabled,
    Quit,
}

#[cfg(target_os = "macos")]
mod imp {
    use super::MenuCommand;
    use std::sync::mpsc::{channel, Receiver};
    use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
    use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

    pub struct MenuBar {
        _tray: TrayIcon,
        enabled_item: CheckMenuItem,
        commands: Receiver<MenuCommand>,
    }

    impl MenuBar {
        /// `icon_png` : pictogramme noir sur fond transparent (« template », teinté par macOS) ; `wake` réveille l'interface à chaque clic dans le menu.
        pub fn new(icon_png: &[u8], enabled: bool, wake: impl Fn() + Send + Sync + 'static) -> Option<Self> {
            let open = MenuItem::new("Ouvrir OrganizeMyFiles", true, None);
            let organize = MenuItem::new("Organiser maintenant", true, None);
            let enabled_item = CheckMenuItem::new("Surveillance active", true, enabled, None);
            let quit = MenuItem::new("Quitter", true, None);
            let menu = Menu::new();
            menu.append_items(&[&open, &organize, &enabled_item, &PredefinedMenuItem::separator(), &quit]).ok()?;

            let img = image::load_from_memory(icon_png).ok()?.to_rgba8();
            let (w, h) = (img.width(), img.height());
            let icon = Icon::from_rgba(img.into_raw(), w, h).ok()?;
            let tray = TrayIconBuilder::new()
                .with_menu(Box::new(menu))
                .with_icon(icon)
                .with_icon_as_template(true)
                .with_tooltip("OrganizeMyFiles")
                .build()
                .ok()?;

            let (tx, commands) = channel();
            let ids = (open.id().clone(), organize.id().clone(), enabled_item.id().clone(), quit.id().clone());
            MenuEvent::set_event_handler(Some(move |ev: MenuEvent| {
                let cmd = if ev.id == ids.0 {
                    MenuCommand::Open
                } else if ev.id == ids.1 {
                    MenuCommand::Organize
                } else if ev.id == ids.2 {
                    MenuCommand::ToggleEnabled
                } else if ev.id == ids.3 {
                    MenuCommand::Quit
                } else {
                    return;
                };
                let _ = tx.send(cmd);
                wake();
            }));
            Some(MenuBar { _tray: tray, enabled_item, commands })
        }

        pub fn poll(&self) -> Vec<MenuCommand> {
            self.commands.try_iter().collect()
        }

        pub fn set_enabled(&self, enabled: bool) {
            if self.enabled_item.is_checked() != enabled {
                self.enabled_item.set_checked(enabled);
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    use super::MenuCommand;

    pub struct MenuBar;

    impl MenuBar {
        pub fn new(_icon_png: &[u8], _enabled: bool, _wake: impl Fn() + Send + Sync + 'static) -> Option<Self> {
            None
        }
        pub fn poll(&self) -> Vec<MenuCommand> {
            Vec::new()
        }
        pub fn set_enabled(&self, _enabled: bool) {}
    }
}

pub use imp::MenuBar;
