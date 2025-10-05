use nptk::core::app::context::AppContext;
use nptk::core::app::Application;
use nptk::core::config::MayConfig;
use nptk::core::layout::{AlignItems, Dimension, FlexDirection, LayoutStyle};
use nptk::core::signal::state::StateSignal;
use nptk::core::signal::{MaybeSignal, Signal};
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
use nptk::widgets::checkbox::{Checkbox, CheckboxState};
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

struct MyApp;

impl Application for MyApp {
    type Theme = ConfigurableTheme;
    type State = ();

    fn build(context: AppContext, _: Self::State) -> impl Widget {
        let checkbox1_state = context.use_signal(StateSignal::new(CheckboxState::Unchecked));
        let checkbox2_state = context.use_signal(StateSignal::new(CheckboxState::Checked));
        let checkbox3_state = context.use_signal(StateSignal::new(CheckboxState::Indeterminate));

        Container::new(vec![
            Box::new(Text::new("Three-State Checkbox with State Locking".to_string())),
            
            // Normal checkbox - no locking
            Box::new(Text::new("Normal Checkbox (no locks):".to_string())),
            Box::new(Checkbox::new(MaybeSignal::signal(checkbox1_state.clone()))),
            Box::new(Text::new(checkbox1_state.map(|val| nptk::core::reference::Ref::Owned(format!("State: {:?}", *val))))),
            
            // Checkbox with checked state locked
            Box::new(Text::new("Checked State Locked:".to_string())),
            Box::new(Checkbox::new(MaybeSignal::signal(checkbox2_state.clone())).with_locked_state(CheckboxState::Checked)),
            Box::new(Text::new(checkbox2_state.map(|val| nptk::core::reference::Ref::Owned(format!("State: {:?}", *val))))),
            
            // Checkbox with indeterminate state locked
            Box::new(Text::new("Indeterminate State Locked:".to_string())),
            Box::new(Checkbox::new(MaybeSignal::signal(checkbox3_state.clone())).with_locked_state(CheckboxState::Indeterminate)),
            Box::new(Text::new(checkbox3_state.map(|val| nptk::core::reference::Ref::Owned(format!("State: {:?}", *val))))),
            
            // Instructions
            Box::new(Text::new("".to_string())), // Spacer
            Box::new(Text::new("Instructions:".to_string())),
            Box::new(Text::new("• Normal checkbox cycles through all states".to_string())),
            Box::new(Text::new("• Locked checkboxes won't change when in locked state".to_string())),
            Box::new(Text::new("• Try clicking each checkbox to see the difference".to_string())),
        ])
        .with_layout_style(LayoutStyle {
            size: Vector2::<Dimension>::new(Dimension::percent(1.0), Dimension::percent(1.0)),
            flex_direction: FlexDirection::Column,
            align_items: Some(AlignItems::FlexStart),
            padding: nptk::core::layout::Rect {
                left: nptk::core::layout::LengthPercentage::length(10.0),
                right: nptk::core::layout::LengthPercentage::length(10.0),
                top: nptk::core::layout::LengthPercentage::length(10.0),
                bottom: nptk::core::layout::LengthPercentage::length(10.0),
            },
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
    MyApp.run(())
}