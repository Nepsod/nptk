use crate::plugin::MyPlugin;
use nptk::core::app::context::AppContext;
use nptk::core::app::Application;
use nptk::core::plugin::PluginManager;
use nptk::core::widget::Widget;
use nptk::widgets::text::Text;

pub mod plugin;

struct MyApp;

impl Application for MyApp {
    type State = ();

    fn build(_: AppContext, _: Self::State) -> impl Widget {
        Text::new("Drop a file!".to_string())
    }

    fn plugins(&self) -> PluginManager {
        let mut plugins = PluginManager::new();
        plugins.register(MyPlugin);
        plugins
    }
}

fn main() {
    MyApp.run(())
}
