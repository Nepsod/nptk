use nptk::core::app::context::AppContext;
use nptk::core::app::Application;
use nptk::core::config::MayConfig;
use nptk::core::widget::Widget;
use nptk::theme::theme::celeste::CelesteTheme;
use nptk::widgets::text::Text;

struct MyApp;

impl Application for MyApp {
    type Theme = CelesteTheme;
    type State = ();

    fn build(_: AppContext, _: Self::State) -> impl Widget {
        Text::new("Hello World! Unicode: ≠, ←, ↓, →, ≤\n\nMultilingual Test:\n中文 (Chinese)\n한국어 (Korean)\nالعربية (Arabic)\nहिन्दी (Hindi)\nবাংলা (Bengali)\nதமிழ் (Tamil)\nമലയാളം (Malayalam)\nગુજરાતી (Gujarati)\nਪੰਜਾਬੀ (Punjabi)\nاردو (Urdu)\nفارسی (Persian)\nעברית (Hebrew)\nРусский (Russian)\nΕλληνικά (Greek)\n日本語 (Japanese)".to_string())
    }

    fn config(&self) -> MayConfig<Self::Theme> {
        MayConfig::default()
    }
}

fn main() {
    println!("DEBUG: Starting hello_world application");
    MyApp.run(())
}
