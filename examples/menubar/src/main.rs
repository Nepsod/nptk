use nptk::prelude::*;
use nptk_core::menu::{MenuTemplate, MenuItem, MenuCommand, MenuManager};

struct MenuBarApp;

impl Application for MenuBarApp {
    type State = ();

    fn build(_context: AppContext, _config: Self::State) -> impl Widget {
        let _status_text = StateSignal::new("Welcome! Use the menu bar above.".to_string());

        // Create menu manager for command routing
        let menu_manager = MenuManager::new();

        // Create File menu template
        let file_menu = MenuTemplate::new("File")
            .add_item(MenuItem::new(MenuCommand::FileNew, "New Document")
                .with_shortcut("Ctrl+N")
                .with_action(|| {
                    println!("New Document");
                    Update::empty()
                }))
            .add_item(MenuItem::new(MenuCommand::Custom(0x1001), "New Window")
                .with_shortcut("Ctrl+Shift+N")
                .with_action(|| {
                    println!("New Window");
                    Update::empty()
                }))
            .add_item(MenuItem::separator())
            .add_item(MenuItem::new(MenuCommand::FileOpen, "Open...")
                .with_shortcut("Ctrl+O")
                .with_action(|| {
                    println!("Open...");
                    Update::empty()
                }))
            .add_item(MenuItem::new(MenuCommand::Custom(0x1002), "Open Recent")
                .with_submenu(MenuTemplate::new("open_recent")
                    .add_item(MenuItem::new(MenuCommand::Custom(0x1101), "document1.txt")
                        .with_action(|| {
                            println!("Open: document1.txt");
                            Update::empty()
                        }))
                    .add_item(MenuItem::new(MenuCommand::Custom(0x1102), "project.rs")
                        .with_action(|| {
                            println!("Open: project.rs");
                            Update::empty()
                        }))
                    .add_item(MenuItem::new(MenuCommand::Custom(0x1103), "notes.md")
                        .with_action(|| {
                            println!("Open: notes.md");
                            Update::empty()
                        }))))
            .add_item(MenuItem::separator())
            .add_item(MenuItem::new(MenuCommand::FileSave, "Save")
                .with_shortcut("Ctrl+S")
                .with_action(|| {
                    println!("Save");
                    Update::empty()
                }))
            .add_item(MenuItem::new(MenuCommand::FileSaveAs, "Save As...")
                .with_shortcut("Ctrl+Shift+S")
                .with_action(|| {
                    println!("Save As...");
                    Update::empty()
                }))
            .add_item(MenuItem::new(MenuCommand::Custom(0x1003), "Save All")
                .with_shortcut("Ctrl+Alt+S")
                .with_action(|| {
                    println!("Save All");
                    Update::empty()
                }))
            .add_item(MenuItem::separator())
            .add_item(MenuItem::new(MenuCommand::FileClose, "Close")
                .with_shortcut("Ctrl+W")
                .with_action(|| {
                    println!("Close");
                    Update::empty()
                }))
            .add_item(MenuItem::new(MenuCommand::FileExit, "Quit")
                .with_shortcut("Ctrl+Q")
                .with_action(|| {
                    println!("Quit");
                    Update::empty()
                }));

        // Create Edit menu template
        let edit_menu = MenuTemplate::new("Edit")
            .add_item(MenuItem::new(MenuCommand::EditUndo, "Undo")
                .with_shortcut("Ctrl+Z")
                .with_action(|| {
                    println!("Undo");
                    Update::empty()
                }))
            .add_item(MenuItem::new(MenuCommand::EditRedo, "Redo")
                .with_shortcut("Ctrl+Y")
                .with_action(|| {
                    println!("Redo");
                    Update::empty()
                }))
            .add_item(MenuItem::separator())
            .add_item(MenuItem::new(MenuCommand::EditCut, "Cut")
                .with_shortcut("Ctrl+X")
                .with_action(|| {
                    println!("Cut");
                    Update::empty()
                }))
            .add_item(MenuItem::new(MenuCommand::EditCopy, "Copy")
                .with_shortcut("Ctrl+C")
                .with_action(|| {
                    println!("Copy");
                    Update::empty()
                }))
            .add_item(MenuItem::new(MenuCommand::EditPaste, "Paste")
                .with_shortcut("Ctrl+V")
                .with_action(|| {
                    println!("Paste");
                    Update::empty()
                }))
            .add_item(MenuItem::new(MenuCommand::Custom(0x2001), "Paste Special")
                .with_shortcut("Ctrl+Shift+V")
                .with_action(|| {
                    println!("Paste Special");
                    Update::empty()
                }))
            .add_item(MenuItem::separator())
            .add_item(MenuItem::new(MenuCommand::EditSelectAll, "Select All")
                .with_shortcut("Ctrl+A")
                .with_action(|| {
                    println!("Select All");
                    Update::empty()
                }))
            .add_item(MenuItem::new(MenuCommand::Custom(0x2002), "Find...")
                .with_shortcut("Ctrl+F")
                .with_action(|| {
                    println!("Find...");
                    Update::empty()
                }))
            .add_item(MenuItem::new(MenuCommand::Custom(0x2003), "Replace...")
                .with_shortcut("Ctrl+H")
                .with_action(|| {
                    println!("Replace...");
                    Update::empty()
                }));

        // Create View menu template
        let view_menu = MenuTemplate::new("View")
            .add_item(MenuItem::new(MenuCommand::Custom(0x3001), "Show Toolbar")
                .with_action(|| {
                    println!("Toggle Toolbar");
                    Update::empty()
                }))
            .add_item(MenuItem::new(MenuCommand::Custom(0x3002), "Show Status Bar")
                .with_action(|| {
                    println!("Toggle Status Bar");
                    Update::empty()
                }))
            .add_item(MenuItem::new(MenuCommand::Custom(0x3003), "Show Sidebar")
                .with_shortcut("Ctrl+B")
                .with_action(|| {
                    println!("Toggle Sidebar");
                    Update::empty()
                }))
            .add_item(MenuItem::separator())
            .add_item(MenuItem::new(MenuCommand::Custom(0x3004), "Enter Fullscreen")
                .with_shortcut("F11")
                .with_action(|| {
                    println!("Toggle Fullscreen");
                    Update::empty()
                }))
            .add_item(MenuItem::new(MenuCommand::Custom(0x3005), "Zoom")
                .with_submenu(MenuTemplate::new("zoom")
                    .add_item(MenuItem::new(MenuCommand::Custom(0x3101), "Zoom In")
                        .with_shortcut("Ctrl++")
                        .with_action(|| {
                            println!("Zoom In");
                            Update::empty()
                        }))
                    .add_item(MenuItem::new(MenuCommand::Custom(0x3102), "Zoom Out")
                        .with_shortcut("Ctrl+-")
                        .with_action(|| {
                            println!("Zoom Out");
                            Update::empty()
                        }))
                    .add_item(MenuItem::new(MenuCommand::Custom(0x3103), "Reset Zoom")
                        .with_shortcut("Ctrl+0")
                        .with_action(|| {
                            println!("Reset Zoom");
                            Update::empty()
                        }))));

        // Create Tools menu template
        let tools_menu = MenuTemplate::new("Tools")
            .add_item(MenuItem::new(MenuCommand::Custom(0x4001), "Preferences...")
                .with_shortcut("Ctrl+,")
                .with_action(|| {
                    println!("Preferences...");
                    Update::empty()
                }))
            .add_item(MenuItem::new(MenuCommand::Custom(0x4002), "Extensions")
                .with_shortcut("Ctrl+Shift+X")
                .with_action(|| {
                    println!("Extensions");
                    Update::empty()
                }))
            .add_item(MenuItem::new(MenuCommand::Custom(0x4003), "Terminal")
                .with_shortcut("Ctrl+`")
                .with_action(|| {
                    println!("Terminal");
                    Update::empty()
                }))
            .add_item(MenuItem::separator())
            .add_item(MenuItem::new(MenuCommand::Custom(0x4004), "Developer Tools")
                .with_submenu(MenuTemplate::new("developer")
                    .add_item(MenuItem::new(MenuCommand::Custom(0x4101), "Developer Console")
                        .with_shortcut("F12")
                        .with_action(|| {
                            println!("Developer Console");
                            Update::empty()
                        }))
                    .add_item(MenuItem::new(MenuCommand::Custom(0x4102), "Element Inspector")
                        .with_shortcut("Ctrl+Shift+I")
                        .with_action(|| {
                            println!("Element Inspector");
                            Update::empty()
                        }))));

        // Create Help menu template
        let help_menu = MenuTemplate::new("Help")
            .add_item(MenuItem::new(MenuCommand::Custom(0x5001), "Welcome Guide")
                .with_action(|| {
                    println!("Welcome Guide");
                    Update::empty()
                }))
            .add_item(MenuItem::new(MenuCommand::Custom(0x5002), "Documentation")
                .with_shortcut("F1")
                .with_action(|| {
                    println!("Documentation");
                    Update::empty()
                }))
            .add_item(MenuItem::new(MenuCommand::Custom(0x5003), "Keyboard Shortcuts")
                .with_shortcut("Ctrl+/")
                .with_action(|| {
                    println!("Keyboard Shortcuts");
                    Update::empty()
                }))
            .add_item(MenuItem::separator())
            .add_item(MenuItem::new(MenuCommand::Custom(0x5004), "Report Issue...")
                .with_action(|| {
                    println!("Report Issue...");
                    Update::empty()
                }))
            .add_item(MenuItem::new(MenuCommand::Custom(0x5005), "Check for Updates...")
                .with_action(|| {
                    println!("Check for Updates...");
                    Update::empty()
                }))
            .add_item(MenuItem::separator())
            .add_item(MenuItem::new(MenuCommand::Custom(0x5006), "About NPTK")
                .with_action(|| {
                    println!("About NPTK");
                    Update::empty()
                }));

        // Create the menu bar with all menus
        // The template IDs are used as the menubar labels, and the template items are shown in the dropdown
        let menu_bar = MenuBar::new()
            .with_template(file_menu)
            .with_template(edit_menu)
            .with_template(view_menu)
            .with_template(tools_menu)
            .with_template(help_menu)
            .with_menu_manager(menu_manager);
        // let menu_bar = menu_bar.without_global_menu(); // Need to disable the global menu?

        // Create content container with padding
        let content_container = Container::new(vec![
            Box::new(Text::new("NPTK MenuBar Demo".to_string())),
            Box::new(Text::new("".to_string())),
            Box::new(Text::new("Features demonstrated:".to_string())),
            Box::new(Text::new(
                "• Auto-sized menu items based on text length".to_string(),
            )),
            Box::new(Text::new(
                "• Hierarchical menu structure with submenus".to_string(),
            )),
            Box::new(Text::new(
                "• Keyboard shortcuts (shown in menu items)".to_string(),
            )),
            Box::new(Text::new("• Hover and selection states".to_string())),
            Box::new(Text::new(
                "• Menu separators and disabled items".to_string(),
            )),
            Box::new(Text::new("".to_string())),
            Box::new(Text::new(
                "Try clicking on File, Edit, View, Tools, or Help!".to_string(),
            )),
            Box::new(Text::new(
                "Press F10 to toggle menu bar visibility".to_string(),
            )),
        ])
        .with_layout_style(LayoutStyle {
            flex_direction: FlexDirection::Column,
            ..Default::default()
        });

        // Create the main container
        Container::new(vec![Box::new(menu_bar), Box::new(content_container)]).with_layout_style(
            LayoutStyle {
                size: Vector2::new(Dimension::length(800.0), Dimension::length(600.0)),
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
            },
        )
    }
}

fn main() {
    // Initialize logging for DBus bridge diagnostics
    // if std::env::var("RUST_LOG").is_err() {
    //     std::env::set_var("RUST_LOG", "info");
    // }
    // let _ = env_logger::try_init();
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
