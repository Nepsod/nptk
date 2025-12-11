use nptk::prelude::*;
use nptk::widgets::tabs_container::{TabItem, TabPosition, TabsContainer};

struct TabsApp;

impl Application for TabsApp {
    type Theme = SystemTheme;
    type State = ();

    fn build(context: AppContext, _config: Self::State) -> impl Widget {
        // First container - static tabs
        let static_tab1_content = Text::new(
            "Welcome to Tab 1! This content appears in the content area below the tabs."
                .to_string(),
        );
        let static_tab2_content = Text::new("This is Tab 2's content. Notice how the content changes when you click different tabs.".to_string());
        let static_tab3_content = Text::new(
            "Tab 3 content is displayed here. This tab has a close button (X) that you can click."
                .to_string(),
        );
        let static_tab4_content =
            Text::new("Tab 4 demonstrates that you can have multiple tabs.".to_string());

        let static_tab1 = TabItem::new("static_tab1", "Home", static_tab1_content);
        let static_tab2 = TabItem::new("static_tab2", "Settings", static_tab2_content);
        let static_tab3 = TabItem::new("static_tab3", "Help", static_tab3_content)
            .with_close_callback(|| {
                println!("Help tab close button clicked!");
                Update::empty()
            });
        let static_tab4 = TabItem::new("static_tab4", "About", static_tab4_content);

        let static_container = TabsContainer::new()
            .with_tab(static_tab1)
            .with_tab(static_tab2)
            .with_tab(static_tab3)
            .with_tab(static_tab4)
            .with_position(TabPosition::Bottom)
            .with_tab_size(40.0)
            .with_layout_style(LayoutStyle {
                size: nalgebra::Vector2::new(Dimension::percent(1.0), Dimension::percent(1.0)),
                ..Default::default()
            });

        // Second container - dynamic tabs with some initial tabs
        let dynamic_tab1 = TabItem::new(
            "dyn_tab1",
            "Dynamic Tab 1",
            Text::new("This is a dynamic tab!".to_string()),
        );
        let dynamic_tab2 = TabItem::new(
            "dyn_tab2",
            "Dynamic Tab 2",
            Text::new("Another dynamic tab.".to_string()),
        );

        let dynamic_container =
            TabsContainer::new_dynamic(&context, vec![dynamic_tab1, dynamic_tab2])
                .with_position(TabPosition::Top)
                .with_tab_size(40.0);

        // Get the shared tabs state to add new tabs (thread-safe)
        let tabs_shared = dynamic_container
            .get_tabs_shared()
            .expect("Dynamic container should have shared state");
        let tab_counter = std::sync::Arc::new(std::sync::Mutex::new(2usize));

        let dynamic_container = dynamic_container
            .with_action_button({
                let tab_counter = tab_counter.clone();
                let tabs_shared = tabs_shared.clone();
                move || {
                    let counter = {
                        let mut c = tab_counter.lock().unwrap();
                        *c += 1;
                        *c
                    };
                    let tab_id = format!("dyn_tab_{}", counter);
                    let tab_label = format!("Dynamic Tab {}", counter);

                    // Add new tab data to the shared state (thread-safe)
                    if let Ok(mut tabs) = tabs_shared.lock() {
                        tabs.push(nptk::widgets::tabs_container::TabData {
                            id: tab_id.clone(),
                            label: tab_label.clone(),
                            enabled: true,
                        });
                    }

                    println!("Action button clicked! Added: {}", tab_label);
                    Update::EVAL | Update::LAYOUT | Update::DRAW
                }
            })
            .with_layout_style(LayoutStyle {
                size: Vector2::new(Dimension::percent(1.0), Dimension::percent(1.0)),
                ..Default::default()
            });

        // Create a container for the dynamic tabs
        let dynamic_section =
            Container::new(vec![Box::new(dynamic_container)]).with_layout_style(LayoutStyle {
                size: Vector2::new(Dimension::percent(1.0), Dimension::percent(1.0)),
                flex_direction: FlexDirection::Column,
                gap: Vector2::new(
                    LengthPercentage::length(0.0),
                    LengthPercentage::length(10.0),
                ),
                padding: nptk::core::layout::Rect::<LengthPercentage> {
                    left: LengthPercentage::length(10.0),
                    right: LengthPercentage::length(10.0),
                    top: LengthPercentage::length(10.0),
                    bottom: LengthPercentage::length(10.0),
                },
                ..Default::default()
            });

        // Main container with both tab containers side by side
        Container::new(vec![Box::new(static_container), Box::new(dynamic_section)])
            .with_layout_style(LayoutStyle {
                size: Vector2::new(Dimension::percent(1.0), Dimension::percent(1.0)),
                flex_direction: FlexDirection::Row,
                gap: Vector2::new(
                    LengthPercentage::length(10.0),
                    LengthPercentage::length(0.0),
                ),
                padding: nptk::core::layout::Rect::<LengthPercentage> {
                    left: LengthPercentage::length(10.0),
                    right: LengthPercentage::length(10.0),
                    top: LengthPercentage::length(10.0),
                    bottom: LengthPercentage::length(10.0),
                },
                ..Default::default()
            })
    }
}

fn main() {
    let _ = env_logger::try_init();
    TabsApp.run(())
}
