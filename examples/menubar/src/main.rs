use nptk::prelude::*;

struct MenuBarApp;

impl Application for MenuBarApp {
    type Theme = SystemTheme;
    type State = ();

    fn build(_context: AppContext, _config: Self::State) -> impl Widget {
        let _status_text = StateSignal::new("Welcome! Use the menu bar above.".to_string());

        // Create comprehensive menu items
        let file_menu = MenuBarItem::new("file", "File")
            .with_submenu_item(
                MenuBarItem::new("new", "New Document")
                    .with_shortcut("Ctrl+N")
                    .with_on_activate(|| Update::empty())
            )
            .with_submenu_item(
                MenuBarItem::new("new_window", "New Window")
                    .with_shortcut("Ctrl+Shift+N")
                    .with_on_activate(|| Update::empty())
            )
            .with_submenu_item(
                MenuBarItem::new("separator1", "---")
                    .with_enabled(false) // Separator
            )
            .with_submenu_item(
                MenuBarItem::new("open", "Open...")
                    .with_shortcut("Ctrl+O")
                    .with_on_activate(|| Update::empty())
            )
            .with_submenu_item(
                MenuBarItem::new("open_recent", "Open Recent")
                    .with_submenu_item(
                        MenuBarItem::new("recent1", "document1.txt")
                            .with_on_activate(|| Update::empty())
                    )
                    .with_submenu_item(
                        MenuBarItem::new("recent2", "project.rs")
                            .with_on_activate(|| Update::empty())
                    )
                    .with_submenu_item(
                        MenuBarItem::new("recent3", "notes.md")
                            .with_on_activate(|| Update::empty())
                    )
            )
            .with_submenu_item(
                MenuBarItem::new("separator2", "---")
                    .with_enabled(false) // Separator
            )
            .with_submenu_item(
                MenuBarItem::new("save", "Save")
                    .with_shortcut("Ctrl+S")
                    .with_on_activate(|| Update::empty())
            )
            .with_submenu_item(
                MenuBarItem::new("save_as", "Save As...")
                    .with_shortcut("Ctrl+Shift+S")
                    .with_on_activate(|| Update::empty())
            )
            .with_submenu_item(
                MenuBarItem::new("save_all", "Save All")
                    .with_shortcut("Ctrl+Alt+S")
                    .with_on_activate(|| Update::empty())
            )
            .with_submenu_item(
                MenuBarItem::new("separator3", "---")
                    .with_enabled(false) // Separator
            )
            .with_submenu_item(
                MenuBarItem::new("close", "Close")
                    .with_shortcut("Ctrl+W")
                    .with_on_activate(|| Update::empty())
            )
            .with_submenu_item(
                MenuBarItem::new("quit", "Quit")
                    .with_shortcut("Ctrl+Q")
                    .with_on_activate(|| Update::empty())
            );

        let edit_menu = MenuBarItem::new("edit", "Edit")
            .with_submenu_item(
                MenuBarItem::new("undo", "Undo")
                    .with_shortcut("Ctrl+Z")
                    .with_on_activate(|| Update::empty())
            )
            .with_submenu_item(
                MenuBarItem::new("redo", "Redo")
                    .with_shortcut("Ctrl+Y")
                    .with_on_activate(|| Update::empty())
            )
            .with_submenu_item(
                MenuBarItem::new("separator4", "---")
                    .with_enabled(false) // Separator
            )
            .with_submenu_item(
                MenuBarItem::new("cut", "Cut")
                    .with_shortcut("Ctrl+X")
                    .with_on_activate(|| Update::empty())
            )
            .with_submenu_item(
                MenuBarItem::new("copy", "Copy")
                    .with_shortcut("Ctrl+C")
                    .with_on_activate(|| Update::empty())
            )
            .with_submenu_item(
                MenuBarItem::new("paste", "Paste")
                    .with_shortcut("Ctrl+V")
                    .with_on_activate(|| Update::empty())
            )
            .with_submenu_item(
                MenuBarItem::new("paste_special", "Paste Special")
                    .with_shortcut("Ctrl+Shift+V")
                    .with_on_activate(|| Update::empty())
            )
            .with_submenu_item(
                MenuBarItem::new("separator5", "---")
                    .with_enabled(false) // Separator
            )
            .with_submenu_item(
                MenuBarItem::new("select_all", "Select All")
                    .with_shortcut("Ctrl+A")
                    .with_on_activate(|| Update::empty())
            )
            .with_submenu_item(
                MenuBarItem::new("find", "Find...")
                    .with_shortcut("Ctrl+F")
                    .with_on_activate(|| Update::empty())
            )
            .with_submenu_item(
                MenuBarItem::new("replace", "Replace...")
                    .with_shortcut("Ctrl+H")
                    .with_on_activate(|| Update::empty())
            );

        let view_menu = MenuBarItem::new("view", "View")
            .with_submenu_item(
                MenuBarItem::new("toolbar", "Show Toolbar")
                    .with_on_activate(|| Update::empty())
            )
            .with_submenu_item(
                MenuBarItem::new("statusbar", "Show Status Bar")
                    .with_on_activate(|| Update::empty())
            )
            .with_submenu_item(
                MenuBarItem::new("sidebar", "Show Sidebar")
                    .with_shortcut("Ctrl+B")
                    .with_on_activate(|| Update::empty())
            )
            .with_submenu_item(
                MenuBarItem::new("separator6", "---")
                    .with_enabled(false) // Separator
            )
            .with_submenu_item(
                MenuBarItem::new("fullscreen", "Enter Fullscreen")
                    .with_shortcut("F11")
                    .with_on_activate(|| Update::empty())
            )
            .with_submenu_item(
                MenuBarItem::new("zoom", "Zoom")
                    .with_submenu_item(
                        MenuBarItem::new("zoom_in", "Zoom In")
                            .with_shortcut("Ctrl++")
                            .with_on_activate(|| Update::empty())
                    )
                    .with_submenu_item(
                        MenuBarItem::new("zoom_out", "Zoom Out")
                            .with_shortcut("Ctrl+-")
                            .with_on_activate(|| Update::empty())
                    )
                    .with_submenu_item(
                        MenuBarItem::new("zoom_reset", "Reset Zoom")
                            .with_shortcut("Ctrl+0")
                            .with_on_activate(|| Update::empty())
                    )
            );

        let tools_menu = MenuBarItem::new("tools", "Tools")
            .with_submenu_item(
                MenuBarItem::new("preferences", "Preferences...")
                    .with_shortcut("Ctrl+,")
                    .with_on_activate(|| Update::empty())
            )
            .with_submenu_item(
                MenuBarItem::new("extensions", "Extensions")
                    .with_shortcut("Ctrl+Shift+X")
                    .with_on_activate(|| Update::empty())
            )
            .with_submenu_item(
                MenuBarItem::new("terminal", "Terminal")
                    .with_shortcut("Ctrl+`")
                    .with_on_activate(|| Update::empty())
            )
            .with_submenu_item(
                MenuBarItem::new("separator7", "---")
                    .with_enabled(false) // Separator
            )
            .with_submenu_item(
                MenuBarItem::new("developer", "Developer Tools")
                    .with_submenu_item(
                        MenuBarItem::new("console", "Developer Console")
                            .with_shortcut("F12")
                            .with_on_activate(|| Update::empty())
                    )
                    .with_submenu_item(
                        MenuBarItem::new("inspector", "Element Inspector")
                            .with_shortcut("Ctrl+Shift+I")
                            .with_on_activate(|| Update::empty())
                    )
            );

        let help_menu = MenuBarItem::new("help", "Help")
            .with_submenu_item(
                MenuBarItem::new("welcome", "Welcome Guide")
                    .with_on_activate(|| Update::empty())
            )
            .with_submenu_item(
                MenuBarItem::new("documentation", "Documentation")
                    .with_shortcut("F1")
                    .with_on_activate(|| Update::empty())
            )
            .with_submenu_item(
                MenuBarItem::new("shortcuts", "Keyboard Shortcuts")
                    .with_shortcut("Ctrl+/")
                    .with_on_activate(|| Update::empty())
            )
            .with_submenu_item(
                MenuBarItem::new("separator8", "---")
                    .with_enabled(false) // Separator
            )
            .with_submenu_item(
                MenuBarItem::new("report_bug", "Report Issue...")
                    .with_on_activate(|| Update::empty())
            )
            .with_submenu_item(
                MenuBarItem::new("check_updates", "Check for Updates...")
                    .with_on_activate(|| Update::empty())
            )
            .with_submenu_item(
                MenuBarItem::new("separator9", "---")
                    .with_enabled(false) // Separator
            )
            .with_submenu_item(
                MenuBarItem::new("about", "About NPTK")
                    .with_on_activate(|| Update::empty())
            );

        // Create the menu bar with all menus
        let menu_bar = MenuBar::new()
            .with_item(file_menu)
            .with_item(edit_menu)
            .with_item(view_menu)
            .with_item(tools_menu)
            .with_item(help_menu);
            // Global menu integration will be added later

        // Create content container with padding
        let content_container = Container::new(vec![
            Box::new(Text::new("NPTK MenuBar Demo".to_string())),
            Box::new(Text::new("".to_string())),
            Box::new(Text::new("Features demonstrated:".to_string())),
            Box::new(Text::new("• Auto-sized menu items based on text length".to_string())),
            Box::new(Text::new("• Hierarchical menu structure with submenus".to_string())),
            Box::new(Text::new("• Keyboard shortcuts (shown in menu items)".to_string())),
            Box::new(Text::new("• Hover and selection states".to_string())),
            Box::new(Text::new("• Menu separators and disabled items".to_string())),
            Box::new(Text::new("".to_string())),
            Box::new(Text::new("Try clicking on File, Edit, View, Tools, or Help!".to_string())),
            Box::new(Text::new("Press F10 to toggle menu bar visibility".to_string())),
        ])
        .with_layout_style(LayoutStyle {
            flex_direction: FlexDirection::Column,
            align_items: Some(AlignItems::Center),
            padding: nptk_core::layout::Rect {
                left: LengthPercentage::length(20.0),
                right: LengthPercentage::length(20.0),
                top: LengthPercentage::length(20.0),
                bottom: LengthPercentage::length(20.0),
            },
            gap: Vector2::new(
                LengthPercentage::length(10.0),
                LengthPercentage::length(10.0),
            ),
            ..Default::default()
        });

        // Create the main container
        Container::new(vec![
            Box::new(menu_bar),
            Box::new(content_container),
        ])
        .with_layout_style(LayoutStyle {
            size: Vector2::new(
                Dimension::length(800.0),
                Dimension::length(600.0),
            ),
            flex_direction: FlexDirection::Column,
            align_items: Some(AlignItems::Stretch), // Stretch to fill width
            padding: nptk_core::layout::Rect {
                left: LengthPercentage::length(0.0), // No padding to let menu bar fill
                right: LengthPercentage::length(0.0),
                top: LengthPercentage::length(0.0),
                bottom: LengthPercentage::length(0.0),
            },
            gap: Vector2::new(
                LengthPercentage::length(0.0), // No gap between menu and content
                LengthPercentage::length(0.0),
            ),
            ..Default::default()
        })
    }
}

fn main() {
    // Print environment variable information
    println!("Menu Bar Demo");
    println!("=============");
    println!("Set the following environment variables to configure the theme:");
    println!("  NPTK_THEME=light     # Use light theme");
    println!("  NPTK_THEME=dark      # Use dark theme");
    println!();
    
    if let Ok(theme_env) = std::env::var("NPTK_THEME") {
        println!("Current NPTK_THEME: {}", theme_env);
    } else {
        println!("NPTK_THEME not set, using default theme");
    }
    
    println!();
    println!("Starting application...");
    
    // Demonstrate theme configuration
    let config = ThemeConfig::from_env_or_default();
    println!("Theme configuration loaded:");
    println!("  Default theme: {:?}", config.default_theme);
    println!("  Fallback theme: {:?}", config.fallback_theme);
    
    println!();
    println!("Running GUI application...");
    
    // Run the application
    let app = MenuBarApp;
    app.run(());
}
