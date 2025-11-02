use nptk::prelude::*;
use nptk::widgets::tabs_container::{TabItem, TabsContainer, TabPosition};

struct TabsApp;

impl Application for TabsApp {
    type Theme = SystemTheme;
    type State = ();

    fn build(_context: AppContext, _config: Self::State) -> impl Widget {
        // Create tab content with more descriptive text
        let tab1_content = Text::new("Welcome to Tab 1! This content appears in the content area below the tabs. The tab bar is at the top and the content is properly separated.".to_string());
        let tab2_content = Text::new("This is Tab 2's content. Notice how the content changes when you click different tabs, and it's properly contained in the content area.".to_string());
        let tab3_content = Text::new("Tab 3 content is displayed here. This tab has a close button (X) that you can click. The content area is clearly separated from the tab bar.".to_string());
        let tab4_content = Text::new("Tab 4 demonstrates that you can have multiple tabs. Try changing the tab position to Bottom, Left, or Right to see different layouts!".to_string());

        // Create tabs
        let tab1 = TabItem::new("tab1", "Home", tab1_content);
        let tab2 = TabItem::new("tab2", "Settings", tab2_content);
        let tab3 = TabItem::new("tab3", "Help", tab3_content)
            .with_close_callback(|| {
                println!("Help tab close button clicked!");
                Update::empty()
            });
        let tab4 = TabItem::new("tab4", "About", tab4_content);

        // Create tabs container - try different positions!
        TabsContainer::new()
            .with_tab(tab1)
            .with_tab(tab2)
            .with_tab(tab3)
            .with_tab(tab4)
            .with_position(TabPosition::Bottom) // Try: Top, Bottom, Left, Right
            .with_tab_size(40.0)
            .with_layout_style(LayoutStyle {
                size: nalgebra::Vector2::new(Dimension::percent(1.0), Dimension::percent(1.0)),
                ..Default::default()
            })
    }
}

fn main() {
    TabsApp.run(())
}
