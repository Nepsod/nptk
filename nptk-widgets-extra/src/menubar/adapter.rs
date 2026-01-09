// SPDX-License-Identifier: LGPL-3.0-only
//! Global menu adapter for converting unified MenuTemplate to DBus format

use nptk_core::menu::unified::{MenuTemplate, MenuItem};
use nptk_core::menu::commands::MenuCommand;
use nptk_core::menu::manager::MenuManager;
use super::dbus::{MenuSnapshot, RemoteMenuNode};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use nptk_core::app::update::Update;

/// Convert MenuTemplate to MenuSnapshot for DBus transmission
pub fn menu_template_to_snapshot(
    templates: &[MenuTemplate],
    manager: &MenuManager,
) -> (MenuSnapshot, HashMap<i32, Arc<dyn Fn() -> Update + Send + Sync>>, u64) {
    struct SnapshotBuilder {
        next_id: i32,
        nodes: Vec<RemoteMenuNode>,
        actions: HashMap<i32, Arc<dyn Fn() -> Update + Send + Sync>>,
        hasher: std::collections::hash_map::DefaultHasher,
    }

    impl SnapshotBuilder {
        fn new() -> Self {
            use std::hash::Hasher;
            Self {
                next_id: 1,
                nodes: Vec::new(),
                actions: HashMap::new(),
                hasher: std::collections::hash_map::DefaultHasher::new(),
            }
        }

        fn convert_template(&mut self, template: &MenuTemplate) -> RemoteMenuNode {
            use std::hash::Hasher;
            let id = self.next_id;
            self.next_id += 1;

            // Hash template properties for signature
            template.id.hash(&mut self.hasher);

            // Use template id as label, or first item's label if available
            let label = if !template.items.is_empty() {
                template.items[0].label.clone()
            } else {
                template.id.clone()
            };

            // Convert submenu items (without manager for backward compat)
            let children = self.convert_items(&template.items);
            
            RemoteMenuNode {
                id,
                label,
                enabled: true,
                is_separator: false,
                shortcut: None,
                children,
            }
        }
        
        fn convert_template_with_manager(&mut self, template: &MenuTemplate, manager: &MenuManager) -> RemoteMenuNode {
            use std::hash::Hasher;
            let id = self.next_id;
            self.next_id += 1;

            // Hash template properties for signature
            template.id.hash(&mut self.hasher);

            // Use template id as the label for the top-level menu item
            // The template id should be the menu name (e.g., "File", "Edit")
            let label = template.id.clone();

            // Convert submenu items with manager
            let children = self.convert_items_with_manager(&template.items, manager);

            RemoteMenuNode {
                id,
                label,
                enabled: true,
                is_separator: false,
                shortcut: None,
                children,
            }
        }

        fn convert_items(&mut self, items: &[MenuItem]) -> Vec<RemoteMenuNode> {
            use std::hash::Hasher;
            items
                .iter()
                .map(|item| self.convert_item(item, &MenuManager::new())) // Dummy manager for backward compat
                .collect()
        }
        
        fn convert_items_with_manager(&mut self, items: &[MenuItem], manager: &MenuManager) -> Vec<RemoteMenuNode> {
            use std::hash::Hasher;
            items
                .iter()
                .map(|item| self.convert_item(item, manager))
                .collect()
        }

        fn convert_item(&mut self, item: &MenuItem, manager: &MenuManager) -> RemoteMenuNode {
            use std::hash::Hasher;
            
            let is_separator = item.is_separator();
            item.label.hash(&mut self.hasher);
            item.enabled.hash(&mut self.hasher);
            is_separator.hash(&mut self.hasher);
            item.shortcut.hash(&mut self.hasher);

            let id = self.next_id;
            self.next_id += 1;

            let children = if let Some(ref submenu) = item.submenu {
                self.hasher.write_usize(submenu.items.len());
                self.convert_items_with_manager(&submenu.items, manager)
            } else {
                Vec::new()
            };

            // Register action if item has one and no submenu
            if children.is_empty() && !is_separator {
                // Prefer item's direct action, otherwise use MenuManager
                if let Some(ref action) = item.action {
                    self.actions.insert(id, action.clone());
                } else if let Some(manager_action) = manager.get_action(item.id) {
                    self.actions.insert(id, manager_action);
                }
            }

            RemoteMenuNode {
                id,
                label: item.label.clone(),
                enabled: item.enabled && !is_separator,
                is_separator,
                shortcut: item.shortcut.clone(),
                children,
            }
        }

        fn build(mut self, templates: &[MenuTemplate], manager: &MenuManager) -> (MenuSnapshot, HashMap<i32, Arc<dyn Fn() -> Update + Send + Sync>>, u64) {
            self.nodes = templates
                .iter()
                .map(|template| self.convert_template_with_manager(template, manager))
                .collect();

            let signature = self.hasher.finish();
            (
                MenuSnapshot {
                    entries: self.nodes,
                },
                self.actions,
                signature,
            )
        }
    }

    SnapshotBuilder::new().build(templates, manager)
}

/// Helper to convert MenuTemplate items to RemoteMenuNode format
pub fn convert_menu_item_to_remote(
    item: &MenuItem,
    next_id: &mut i32,
    manager: &MenuManager,
    actions: &mut HashMap<i32, Arc<dyn Fn() -> Update + Send + Sync>>,
) -> RemoteMenuNode {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;

    let mut hasher = DefaultHasher::new();
    let is_separator = item.is_separator();
    item.label.hash(&mut hasher);
    item.enabled.hash(&mut hasher);
    is_separator.hash(&mut hasher);
    item.shortcut.hash(&mut hasher);

    let id = *next_id;
    *next_id += 1;

    let children = if let Some(ref submenu) = item.submenu {
        submenu.items
            .iter()
            .map(|subitem| convert_menu_item_to_remote(subitem, next_id, manager, actions))
            .collect()
    } else {
        Vec::new()
    };

    // Register action
    if children.is_empty() && !is_separator {
        if manager.has_action(item.id) {
            let cmd = item.id;
            let action = Arc::new(move || MenuManager::new().handle_command(cmd));
            actions.insert(id, action);
        } else if let Some(ref action) = item.action {
            actions.insert(id, action.clone());
        }
    }

    RemoteMenuNode {
        id,
        label: item.label.clone(),
        enabled: item.enabled && !is_separator,
        is_separator,
        shortcut: item.shortcut.clone(),
        children,
    }
}
