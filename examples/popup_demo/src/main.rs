use nptk_core::app::context::AppContext;
use nptk_core::app::Application;
use nptk_core::config::MayConfig;
use nptk_core::widget::Widget;
use nptk_theme::theme::celeste::CelesteTheme;
use nptk_widgets::text::Text;

struct PopupDemoApp;

impl Application for PopupDemoApp {
    type Theme = CelesteTheme;
    type State = Self;

    fn config(&self) -> MayConfig<Self::Theme> {
        MayConfig::default()
    }

    fn build(_context: AppContext, _state: Self::State) -> impl Widget {
        Text::new("popup systemtest".to_string())
    }
}

fn main() {
    PopupDemoApp.run(PopupDemoApp);
}
