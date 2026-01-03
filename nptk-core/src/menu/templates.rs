//! Menu template system and initialization functions
//!
//! Provides functions for initializing menu templates with context-aware state.

use crate::menu::commands::MenuCommand;
use crate::menu::unified::{MenuContext, MenuTemplate, MenuItem, ViewMode};

/// Initialize Edit menu items based on context.
/// Enables/disables Cut, Copy, Paste, Undo based on selection and clipboard state.
pub fn init_edit_commands(template: &mut MenuTemplate, context: &MenuContext) {
    // Update Cut item
    if let Some(item) = template.find_item_mut(MenuCommand::EditCut) {
        item.enabled = context.can_move && context.selection_count > 0;
    }

    // Update Copy item
    if let Some(item) = template.find_item_mut(MenuCommand::EditCopy) {
        item.enabled = context.can_copy && context.selection_count > 0;
    }

    // Update Paste item
    if let Some(item) = template.find_item_mut(MenuCommand::EditPaste) {
        item.enabled = context.can_paste;
    }

    // Update Paste Link item
    if let Some(item) = template.find_item_mut(MenuCommand::EditPasteLink) {
        item.enabled = context.can_paste;
    }

    // Update Undo item
    if let Some(item) = template.find_item_mut(MenuCommand::EditUndo) {
        item.enabled = context.can_undo;
    }

    // Update Select All item (always enabled unless we're already in a specific state)
    if let Some(item) = template.find_item_mut(MenuCommand::EditSelectAll) {
        // Can be enabled in most contexts
        item.enabled = true;
    }

    // Update Deselect All item
    if let Some(item) = template.find_item_mut(MenuCommand::EditDeselectAll) {
        item.enabled = context.selection_count > 0;
    }
}

/// Initialize View menu items based on context.
/// Updates checkmarks for view modes and enables/disables arrange options.
pub fn init_view_menu(template: &mut MenuTemplate, context: &MenuContext) {
    // Update view mode checkmarks (radio button behavior)
    if let Some(view_mode) = context.view_mode {
        // Uncheck all view items first
        for cmd in &[
            MenuCommand::ViewIcon,
            MenuCommand::ViewSmallIcon,
            MenuCommand::ViewList,
            MenuCommand::ViewDetails,
        ] {
            if let Some(item) = template.find_item_mut(*cmd) {
                item.checked = false;
            }
        }

        // Check the current view mode
        let current_cmd = match view_mode {
            ViewMode::Icon => MenuCommand::ViewIcon,
            ViewMode::SmallIcon => MenuCommand::ViewSmallIcon,
            ViewMode::List => MenuCommand::ViewList,
            ViewMode::Details => MenuCommand::ViewDetails,
        };

        if let Some(item) = template.find_item_mut(current_cmd) {
            item.checked = true;
        }
    }

    // Update Arrange Auto checkmark
    if let Some(item) = template.find_item_mut(MenuCommand::ArrangeAuto) {
        item.checked = context.auto_arrange;
    }

    // Enable/disable Arrange items based on view mode
    // Arrange only makes sense in Icon/SmallIcon view modes
    let arrange_enabled = context.view_mode
        .map(|mode| matches!(mode, ViewMode::Icon | ViewMode::SmallIcon))
        .unwrap_or(false);

    if let Some(item) = template.find_item_mut(MenuCommand::ArrangeAuto) {
        item.enabled = arrange_enabled;
    }

    if let Some(item) = template.find_item_mut(MenuCommand::ArrangeGrid) {
        item.enabled = arrange_enabled;
    }
}

/// Merge menu items from source template into destination template.
/// Items are inserted at the specified position (or appended if None).
pub fn merge_menus(
    dst: &mut MenuTemplate,
    src: &MenuTemplate,
    insert_position: Option<usize>,
    add_separator: bool,
) {
    let mut items_to_insert = src.items.clone();

    // Add separator before inserting if requested and not at the beginning
    if add_separator {
        let insert_pos = insert_position.unwrap_or(dst.items.len());
        if insert_pos > 0 && !dst.items.is_empty() {
            // Check if last item is already a separator
            let last_is_separator = dst.items
                .last()
                .map(|item| item.is_separator())
                .unwrap_or(false);

            if !last_is_separator {
                items_to_insert.insert(0, MenuItem::separator());
            }
        }
    }

    if let Some(pos) = insert_position {
        if pos <= dst.items.len() {
            dst.items.splice(pos..pos, items_to_insert);
        } else {
            dst.items.extend(items_to_insert);
        }
    } else {
        dst.items.extend(items_to_insert);
    }
}

/// Create a standard Edit menu template
pub fn create_edit_menu() -> MenuTemplate {
    MenuTemplate::from_items(
        "edit_menu",
        vec![
            MenuItem::new(MenuCommand::EditUndo, "&Undo"),
            MenuItem::separator(),
            MenuItem::new(MenuCommand::EditCut, "Cu&t").with_shortcut("Ctrl+X"),
            MenuItem::new(MenuCommand::EditCopy, "&Copy").with_shortcut("Ctrl+C"),
            MenuItem::new(MenuCommand::EditPaste, "&Paste").with_shortcut("Ctrl+V"),
            MenuItem::new(MenuCommand::EditPasteLink, "Paste &Shortcut"),
            MenuItem::separator(),
            MenuItem::new(MenuCommand::EditSelectAll, "Select &All").with_shortcut("Ctrl+A"),
            MenuItem::new(MenuCommand::EditDeselectAll, "&Deselect All"),
        ],
    )
}

/// Create a standard View menu template
pub fn create_view_menu() -> MenuTemplate {
    MenuTemplate::from_items(
        "view_menu",
        vec![
            MenuItem::new(MenuCommand::ViewIcon, "Lar&ge Icons"),
            MenuItem::new(MenuCommand::ViewSmallIcon, "S&mall Icons"),
            MenuItem::new(MenuCommand::ViewList, "&List"),
            MenuItem::new(MenuCommand::ViewDetails, "&Details"),
            MenuItem::separator(),
            MenuItem::new(MenuCommand::ArrangeAuto, "&Auto Arrange")
                .with_submenu(MenuTemplate::from_items(
                    "arrange",
                    vec![MenuItem::new(MenuCommand::ArrangeAuto, "&Auto Arrange")],
                )),
            MenuItem::new(MenuCommand::ArrangeGrid, "Lin&e up Icons"),
            MenuItem::separator(),
            MenuItem::new(MenuCommand::ViewOptions, "&Options..."),
        ],
    )
}

/// Create a standard File menu template
pub fn create_file_menu() -> MenuTemplate {
    MenuTemplate::from_items(
        "file_menu",
        vec![
            MenuItem::new(MenuCommand::FileNew, "&New").with_shortcut("Ctrl+N"),
            MenuItem::new(MenuCommand::FileOpen, "&Open...").with_shortcut("Ctrl+O"),
            MenuItem::separator(),
            MenuItem::new(MenuCommand::FileSave, "&Save").with_shortcut("Ctrl+S"),
            MenuItem::new(MenuCommand::FileSaveAs, "Save &As..."),
            MenuItem::separator(),
            MenuItem::new(MenuCommand::FileClose, "&Close"),
            MenuItem::separator(),
            MenuItem::new(MenuCommand::FileDelete, "&Delete"),
            MenuItem::new(MenuCommand::FileRename, "Rena&me"),
            MenuItem::new(MenuCommand::FileLink, "Create &Shortcut"),
            MenuItem::separator(),
            MenuItem::new(MenuCommand::FileProperties, "P&roperties"),
            MenuItem::separator(),
            MenuItem::new(MenuCommand::FileExit, "E&xit"),
        ],
    )
}
