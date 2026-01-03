use nptk::prelude::*;

struct MyApp;

impl Application for MyApp {
    type State = ();

    fn build(_: AppContext, _: Self::State) -> impl Widget {
        Text::new("Hello World! Unicode: ≠, ←, ↓, →, ≤\n\nMultilingual Test:\n中文 (Chinese)\n한국어 (Korean)\nالعربية (Arabic)\nहिन्दी (Hindi)\nবাংলা (Bengali)\nதமிழ் (Tamil)\nമലയാളം (Malayalam)\nગુજરાતી (Gujarati)\nਪੰਜਾਬੀ (Punjabi)\nاردু (Urdu)\nفارسی (Persian)\nעברית (Hebrew)\nРусский (Russian)\nΕλληνικά (Greek)\n日本語 (Japanese)".to_string())
    }
}

fn main() {
    println!("DEBUG: Starting hello_world application");
    MyApp.run(())
}
