use nptk::prelude::*;
use nptk::widgets::{Button, ScrollContainer, Text};
use nptk_widgets_extra::menubar::MenuBar;
use nptk_widgets_extra::menu_popup::MenuItem;
use async_trait::async_trait;
use std::time::Duration;

struct AsyncDemoApp;

struct AppState {
    counter: usize,
}

#[async_trait]
impl AsyncApplication for AsyncDemoApp {
    type State = AppState;

    async fn initialize() -> Self::State {
        println!("Initializing async app...");
        // Simulate async work
        crate::tasks::sleep(Duration::from_millis(100)).await;
        println!("Async initialization complete.");
        AppState { counter: 0 }
    }

    fn config(&self) -> MayConfig {
        MayConfig::new("Async Demo", 800.0, 600.0)
    }

    fn build(context: AppContext, state: Self::State) -> impl Widget {
        let mut col = Column::new();
        
        let mut menubar = MenuBar::new();
        // Add menu items logic here if needed, keeping it simple for verification
        
        let scroll = ScrollContainer::new(
            Text::new("Content inside async ScrollContainer")
        );

        col = col.push(menubar);
        col = col.push(scroll);
        col = col.push(Text::new(format!("Counter: {}", state.counter)));

        col
    }
}

fn main() {
    AsyncDemoApp.run();
}
