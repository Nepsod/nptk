//! Conversion helpers between old MenuBarItem and new unified MenuItem system

use nptk_core::menu::unified::{MenuItem, MenuTemplate};
use nptk_core::menu::commands::MenuCommand;
use super::MenuBarItem;

/// Convert MenuBarItem to MenuItem
impl From<MenuBarItem> for MenuItem {
    fn from(item: MenuBarItem) -> Self {
        // Convert String id to MenuCommand - use Custom if not a standard command
        let cmd = MenuCommand::Custom(
            0x1000 + (item.id.as_bytes().iter().map(|&b| b as u32).sum::<u32>() % 0x1000)
        );

        let mut menu_item = MenuItem::new(cmd, item.label)
            .with_enabled(item.enabled)
            .with_shortcut(item.shortcut.unwrap_or_default());

        // Convert submenu
        if !item.submenu.is_empty() {
            let submenu_items: Vec<MenuItem> = item.submenu.into_iter().map(|i| i.into()).collect();
            let submenu_template = MenuTemplate::from_items("submenu", submenu_items);
            menu_item = menu_item.with_submenu(submenu_template);
        }

        // Convert action
        if let Some(action) = item.on_activate {
            menu_item = menu_item.with_action(move || action());
        }

        menu_item
    }
}

/// Convert MenuItem to MenuBarItem (for backwards compatibility)
impl From<MenuItem> for MenuBarItem {
    fn from(item: MenuItem) -> Self {
        let id = match item.id {
            MenuCommand::Custom(c) => format!("custom_{:x}", c),
            cmd => format!("{:?}", cmd).to_lowercase(),
        };

        let mut menu_bar_item = MenuBarItem::new(id, item.label)
            .with_enabled(item.enabled);

        if let Some(shortcut) = item.shortcut {
            menu_bar_item = menu_bar_item.with_shortcut(shortcut);
        }

        // Convert submenu
        if let Some(submenu) = item.submenu {
            let submenu_items: Vec<MenuBarItem> = submenu.items.into_iter().map(|i| i.into()).collect();
            for subitem in submenu_items {
                menu_bar_item = menu_bar_item.with_submenu_item(subitem);
            }
        }

        // Convert action
        if let Some(action) = item.action {
            menu_bar_item = menu_bar_item.with_on_activate(move || action());
        }

        menu_bar_item
    }
}
