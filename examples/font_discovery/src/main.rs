use nptk::core::app::context::AppContext;
use nptk::core::app::Application;
use nptk::core::config::MayConfig;
use nptk::core::widget::Widget;
use nptk::theme::theme::celeste::CelesteTheme;
use nptk::widgets::text::Text;

struct FontDiscoveryApp;

impl Application for FontDiscoveryApp {
    type Theme = CelesteTheme;
    type State = ();

    fn build(_: AppContext, _: Self::State) -> impl Widget {
        // Create font context with enhanced dynamic discovery
        let font_context = nptk::core::app::font_ctx::FontContext::new_with_discovery();
        let font_families = font_context.get_font_families();
        let available_fonts = font_families.len();
        
        // Get the actual default font name
        let default_font_name = font_context.get_default_font_name();
        
        let info_text = format!(
            "Font Discovery Demo\n\n\
            Available font families: {}\n\
            Default font: {}\n\n\
            Top 10 discovered fonts:\n{}",
            available_fonts,
            default_font_name,
            font_families.iter().take(10).enumerate()
                .map(|(i, family)| format!("{}. {}", i + 1, family))
                .collect::<Vec<_>>()
                .join("\n")
        );

        Text::new(info_text)
    }

    fn config(&self) -> MayConfig<Self::Theme> {
        MayConfig::default()
    }
}

fn main() {
    FontDiscoveryApp.run(())
}
