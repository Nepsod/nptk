//! Unified menu manager with command routing and state management
//!
//! Provides a centralized system for managing menu state and routing menu commands
//! to their respective actions, regardless of whether they came from menubar,
//! context menu, or global menu.

use crate::app::update::Update;
use crate::menu::commands::MenuCommand;
use crate::menu::unified::MenuTemplate;
use std::collections::HashMap;
use std::sync::Arc;

/// Manages menu command routing and actions
pub struct MenuManager {
    /// Map from MenuCommand to action callbacks
    command_actions: HashMap<u32, Arc<dyn Fn() -> Update + Send + Sync>>,
}

impl MenuManager {
    /// Create a new MenuManager
    pub fn new() -> Self {
        Self {
            command_actions: HashMap::new(),
        }
    }

    /// Register an action for a menu command
    pub fn register_action<F>(&mut self, command: MenuCommand, action: F)
    where
        F: Fn() -> Update + Send + Sync + 'static,
    {
        let id = command.to_u32();
        self.command_actions.insert(id, Arc::new(action));
    }

    /// Register multiple actions at once
    pub fn register_actions<F>(&mut self, actions: Vec<(MenuCommand, F)>)
    where
        F: Fn() -> Update + Send + Sync + 'static,
    {
        for (command, action) in actions {
            self.register_action(command, action);
        }
    }

    /// Handle a menu command and return the Update result
    pub fn handle_command(&self, command: MenuCommand) -> Update {
        let id = command.to_u32();
        if let Some(action) = self.command_actions.get(&id) {
            action()
        } else {
            Update::empty()
        }
    }

    /// Check if a command has a registered action
    pub fn has_action(&self, command: MenuCommand) -> bool {
        let id = command.to_u32();
        self.command_actions.contains_key(&id)
    }

    /// Get the action for a command (for cloning/forwarding to other systems)
    pub fn get_action(&self, command: MenuCommand) -> Option<Arc<dyn Fn() -> Update + Send + Sync>> {
        let id = command.to_u32();
        self.command_actions.get(&id).cloned()
    }

    /// Get all registered command IDs (for debugging/testing)
    pub fn registered_commands(&self) -> Vec<MenuCommand> {
        self.command_actions
            .keys()
            .map(|&id| MenuCommand::from_u32(id))
            .collect()
    }
}

impl Default for MenuManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating menu templates with registered actions
pub struct MenuTemplateBuilder {
    template: MenuTemplate,
    actions: Vec<(MenuCommand, Arc<dyn Fn() -> Update + Send + Sync>)>,
}

impl MenuTemplateBuilder {
    /// Create a new builder
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            template: MenuTemplate::new(id),
            actions: Vec::new(),
        }
    }

    /// Add a menu item with an action
    pub fn add_item<F>(mut self, item: crate::menu::unified::MenuItem, action: Option<F>) -> Self
    where
        F: Fn() -> Update + Send + Sync + 'static,
    {
        if let Some(action) = action {
            self.actions.push((item.id, Arc::new(action)));
        }
        self.template = self.template.add_item(item);
        self
    }

    /// Build the template and register actions in the manager
    pub fn build(self, manager: &mut MenuManager) -> MenuTemplate {
        // Register all actions
        for (command, action) in self.actions {
            let id = command.to_u32();
            manager.command_actions.insert(id, action);
        }
        self.template
    }

    /// Build just the template without registering actions
    pub fn build_template(self) -> MenuTemplate {
        self.template
    }
}
