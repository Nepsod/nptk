use nptk::core::app::context::AppContext;
use nptk::core::app::update::Update;
use nptk::core::app::Application;
use nptk::core::config::MayConfig;
use nptk::core::layout::{AlignItems, Dimension, FlexDirection, LayoutStyle};
use nptk::core::reference::Ref;
use nptk::core::signal::eval::EvalSignal;
use nptk::core::signal::state::StateSignal;
use nptk::core::signal::Signal;
use nptk::core::widget::{Widget, WidgetLayoutExt};
use nptk::math::Vector2;
use nptk::theme::theme::Theme;
use nptk::theme::theme::dark::DarkTheme;
use nptk::theme::theme::celeste::CelesteTheme;
use nptk::theme::config::{ThemeConfig, ThemeSource};
use nptk::theme::id::WidgetId;
use nptk::theme::style::{DefaultStyles, Style};
use nptk::theme::globals::Globals;
use nptk::core::vg::peniko::Color;
use nptk::widgets::button::Button;
use nptk::widgets::container::Container;
use nptk::widgets::text::Text;

/// A wrapper theme that can switch between different themes
#[derive(Clone)]
pub enum ConfigurableTheme {
    Light(CelesteTheme),
    Dark(DarkTheme),
}

impl Theme for ConfigurableTheme {
    fn of(&self, id: WidgetId) -> Option<Style> {
        match self {
            ConfigurableTheme::Light(theme) => theme.of(id),
            ConfigurableTheme::Dark(theme) => theme.of(id),
        }
    }

    fn defaults(&self) -> DefaultStyles {
        match self {
            ConfigurableTheme::Light(theme) => theme.defaults(),
            ConfigurableTheme::Dark(theme) => theme.defaults(),
        }
    }

    fn window_background(&self) -> Color {
        match self {
            ConfigurableTheme::Light(theme) => theme.window_background(),
            ConfigurableTheme::Dark(theme) => theme.window_background(),
        }
    }

    fn globals(&self) -> &Globals {
        match self {
            ConfigurableTheme::Light(theme) => theme.globals(),
            ConfigurableTheme::Dark(theme) => theme.globals(),
        }
    }

    fn globals_mut(&mut self) -> &mut Globals {
        match self {
            ConfigurableTheme::Light(theme) => theme.globals_mut(),
            ConfigurableTheme::Dark(theme) => theme.globals_mut(),
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Default for ConfigurableTheme {
    fn default() -> Self {
        ConfigurableTheme::Dark(DarkTheme::new())
    }
}

impl ConfigurableTheme {
    pub fn from_config(config: &ThemeConfig) -> Self {
        match &config.default_theme {
            ThemeSource::Light => ConfigurableTheme::Light(CelesteTheme::light()),
            ThemeSource::Dark => ConfigurableTheme::Dark(DarkTheme::new()),
            _ => ConfigurableTheme::Dark(DarkTheme::new()), // Default fallback
        }
    }
}
struct FocusApp;

impl Application for FocusApp {
    type Theme = ConfigurableTheme;
    type State = ();

    fn build(context: AppContext, _: Self::State) -> impl Widget {
        let clicked_message = context.use_signal(StateSignal::new("Click a button or use Tab + Space/Enter".to_string()));
        Container::new(vec![
            Box::new(Text::new("Focus Navigation Demo".to_string())),
            Box::new(Text::new("Use Tab to navigate between buttons".to_string())),
            Box::new({
                let clicked_message = clicked_message.clone();
                Button::new(Text::new("Button 1".to_string()))
                    .with_on_pressed(
                        EvalSignal::new(move || {
                            clicked_message.set("Button 1 clicked!".to_string());
                            Update::DRAW
                        })
                        .hook(&context)
                        .maybe(),
                    )
            }),
            Box::new({
                let clicked_message = clicked_message.clone();
                Button::new(Text::new("Button 2".to_string()))
                    .with_on_pressed(
                        EvalSignal::new(move || {
                            clicked_message.set("Button 2 clicked!".to_string());
                            Update::DRAW
                        })
                        .hook(&context)
                        .maybe(),
                    )
            }),
            Box::new({
                let clicked_message = clicked_message.clone();
                Button::new(Text::new("Button 3".to_string()))
                    .with_on_pressed(
                        EvalSignal::new(move || {
                            clicked_message.set("Button 3 clicked!".to_string());
                            Update::DRAW
                        })
                        .hook(&context)
                        .maybe(),
                    )
            }),
            Box::new(Text::new("Press Tab to see focus navigation in action!".to_string())),
            Box::new(Text::new(clicked_message.map(|msg| Ref::Owned(msg.clone())))),
        ])
        .with_layout_style(LayoutStyle {
            size: Vector2::<Dimension>::new(Dimension::percent(1.0), Dimension::percent(1.0)),
            flex_direction: FlexDirection::Column,
            align_items: Some(AlignItems::Center),
            gap: Vector2::new(
                nptk::core::layout::LengthPercentage::length(0.0),
                nptk::core::layout::LengthPercentage::length(20.0),
            ),
            ..Default::default()
        })
    }

    fn config(&self) -> MayConfig<Self::Theme> {
        // Load theme configuration and create the appropriate theme
        let config = ThemeConfig::from_env_or_default();
        let theme = ConfigurableTheme::from_config(&config);
        
        MayConfig {
            theme,
            ..Default::default()
        }
    }
}

fn main() {
    FocusApp.run(())
}
