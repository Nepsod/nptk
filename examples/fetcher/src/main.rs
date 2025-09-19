use nptk::core::app::context::AppContext;
use nptk::core::app::update::Update;
use nptk::core::app::Application;
use nptk::core::config::{MayConfig, TasksConfig};
use nptk::core::widget::Widget;
use nptk::theme::theme::celeste::CelesteTheme;
use nptk::widgets::fetcher::WidgetFetcher;
use nptk::widgets::text::Text;
use serde::Deserialize;

struct MyApp;

impl Application for MyApp {
    type Theme = CelesteTheme;
    type State = ();

    fn build(_: AppContext, _: Self::State) -> impl Widget {
        WidgetFetcher::new(get_random_quote(), Update::DRAW, |data| {
            if let Some(data) = data {
                Text::new(format!(" \"{}\" \n - {}", data.quote, data.author))
            } else {
                Text::new(" Loading Quote...".to_string())
            }
        })
    }

    fn config(&self) -> MayConfig<Self::Theme> {
        MayConfig {
            tasks: Some(TasksConfig::default()),
            ..Default::default()
        }
    }
}

fn main() {
    MyApp.run(())
}

#[derive(Deserialize)]
struct Quote {
    quote: String,
    author: String,
}

async fn get_random_quote() -> Quote {
    surf::get("https://dummyjson.com/quotes/random")
        .await
        .expect("Failed to fetch quote")
        .body_json::<Quote>()
        .await
        .expect("Failed to parse quote")
}
