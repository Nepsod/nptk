use nptk::core::app::context::AppContext;
use nptk::core::app::update::Update;
use nptk::core::layout::{Dimension, LayoutStyle, LengthPercentageAuto};
use nptk::core::signal::state::StateSignal;
use nptk::core::signal::eval::EvalSignal;
use nptk::core::signal::Signal;
use nptk::core::widget::{Widget, WidgetLayoutExt};
use nptk::widgets::{Button, Container, Text, Progress};
use nptk::theme::{Theme, celeste::CelesteTheme, dark::DarkTheme};
use nptk::theme::manager::{ThemeManager, ThemeVariant};
use nptk::theme::properties::ThemeProperty;
use nptk::theme::helpers::ThemeHelper;
use nptk::theme::id::WidgetId;
use nptk::core::vg::peniko::Color;
use nalgebra::Vector2;
use std::sync::Arc;

/// A demo application showcasing the improved theming system.
struct ThemeDemoApp {
    theme_manager: Arc<std::sync::RwLock<ThemeManager>>,
    current_variant: StateSignal<ThemeVariant>,
    progress_value: StateSignal<f32>,
}

impl nptk::core::app::Application for ThemeDemoApp {
    type Theme = CelesteTheme; // We'll switch themes at runtime
    type State = ();

    fn build(context: AppContext, _config: Self::State) -> impl Widget {
        let theme_manager = Arc::new(std::sync::RwLock::new(ThemeManager::new()));
        let current_variant = context.use_signal(StateSignal::new(ThemeVariant::Light));
        let progress_value = context.use_signal(StateSignal::new(0.5_f32));

        ThemeDemoApp {
            theme_manager,
            current_variant,
            progress_value,
        }
    }

    fn config(&self) -> nptk::core::config::MayConfig<Self::Theme> {
        nptk::core::config::MayConfig::default()
    }
}

impl Widget for ThemeDemoApp {
    fn render(
        &mut self,
        scene: &mut nptk::core::vg::Scene,
        theme: &mut dyn Theme,
        layout: &nptk::core::layout::LayoutNode,
        info: &mut nptk::core::app::info::AppInfo,
        context: AppContext,
    ) {
        // Create a container with all our demo widgets
        let mut container = Container::new(vec![]);
        
        // Title
        let title = Text::new("NPTK Theming System Demo".to_string())
            .with_layout_style(LayoutStyle {
                size: Vector2::new(Dimension::percent(1.0), Dimension::length(40.0)),
                margin: nptk::core::layout::Rect::<LengthPercentageAuto> {
                    left: LengthPercentageAuto::length(10.0),
                    right: LengthPercentageAuto::length(10.0),
                    top: LengthPercentageAuto::length(10.0),
                    bottom: LengthPercentageAuto::length(10.0),
                },
                ..Default::default()
            });
        
        // Theme switching buttons
        let light_theme_button = {
            let current_variant = self.current_variant.clone();
            let theme_manager = self.theme_manager.clone();
            Button::new(Text::new("Light Theme".to_string()))
                .with_layout_style(LayoutStyle {
                    size: Vector2::new(Dimension::length(120.0), Dimension::length(40.0)),
                    margin: nptk::core::layout::Rect::<LengthPercentageAuto> {
                        left: LengthPercentageAuto::length(10.0),
                        right: LengthPercentageAuto::length(5.0),
                        top: LengthPercentageAuto::length(5.0),
                        bottom: LengthPercentageAuto::length(5.0),
                    },
                    ..Default::default()
                })
                .with_on_pressed(
                    EvalSignal::new(move || {
                        if let Ok(mut manager) = theme_manager.write() {
                            manager.switch_theme(&ThemeVariant::Light);
                        }
                        current_variant.set(ThemeVariant::Light);
                        Update::DRAW
                    })
                    .hook(&context)
                    .maybe(),
                )
        };
        
        let dark_theme_button = {
            let current_variant = self.current_variant.clone();
            let theme_manager = self.theme_manager.clone();
            Button::new(Text::new("Dark Theme".to_string()))
                .with_layout_style(LayoutStyle {
                    size: Vector2::new(Dimension::length(120.0), Dimension::length(40.0)),
                    margin: nptk::core::layout::Rect::<LengthPercentageAuto> {
                        left: LengthPercentageAuto::length(5.0),
                        right: LengthPercentageAuto::length(10.0),
                        top: LengthPercentageAuto::length(5.0),
                        bottom: LengthPercentageAuto::length(5.0),
                    },
                    ..Default::default()
                })
                .with_on_pressed(
                    EvalSignal::new(move || {
                        if let Ok(mut manager) = theme_manager.write() {
                            manager.switch_theme(&ThemeVariant::Dark);
                        }
                        current_variant.set(ThemeVariant::Dark);
                        Update::DRAW
                    })
                    .hook(&context)
                    .maybe(),
                )
        };
        
        // Progress control buttons
        let progress_up_button = {
            let progress_value = self.progress_value.clone();
            Button::new(Text::new("+10%".to_string()))
                .with_layout_style(LayoutStyle {
                    size: Vector2::new(Dimension::length(80.0), Dimension::length(40.0)),
                    margin: nptk::core::layout::Rect::<LengthPercentageAuto> {
                        left: LengthPercentageAuto::length(10.0),
                        right: LengthPercentageAuto::length(5.0),
                        top: LengthPercentageAuto::length(5.0),
                        bottom: LengthPercentageAuto::length(5.0),
                    },
                    ..Default::default()
                })
                .with_on_pressed(
                    EvalSignal::new(move || {
                        let current = *progress_value.get();
                        let new_value = (current + 0.1).min(1.0);
                        progress_value.set(new_value);
                        Update::DRAW
                    })
                    .hook(&context)
                    .maybe(),
                )
        };
        
        let progress_down_button = {
            let progress_value = self.progress_value.clone();
            Button::new(Text::new("-10%".to_string()))
                .with_layout_style(LayoutStyle {
                    size: Vector2::new(Dimension::length(80.0), Dimension::length(40.0)),
                    margin: nptk::core::layout::Rect::<LengthPercentageAuto> {
                        left: LengthPercentageAuto::length(5.0),
                        right: LengthPercentageAuto::length(10.0),
                        top: LengthPercentageAuto::length(5.0),
                        bottom: LengthPercentageAuto::length(5.0),
                    },
                    ..Default::default()
                })
                .with_on_pressed(
                    EvalSignal::new(move || {
                        let current = *progress_value.get();
                        let new_value = (current - 0.1).max(0.0);
                        progress_value.set(new_value);
                        Update::DRAW
                    })
                    .hook(&context)
                    .maybe(),
                )
        };
        
        // Progress bar
        let progress_bar = Progress::new(self.progress_value.clone() as Arc<dyn Signal<f32>>)
            .with_layout_style(LayoutStyle {
                size: Vector2::new(Dimension::percent(1.0), Dimension::length(30.0)),
                margin: nptk::core::layout::Rect::<LengthPercentageAuto> {
                    left: LengthPercentageAuto::length(10.0),
                    right: LengthPercentageAuto::length(10.0),
                    top: LengthPercentageAuto::length(10.0),
                    bottom: LengthPercentageAuto::length(10.0),
                },
                ..Default::default()
            });
        
        // Current theme info
        let theme_info = {
            let current_variant = self.current_variant.clone();
            let variant_text = match *current_variant.get() {
                ThemeVariant::Light => "Current Theme: Light",
                ThemeVariant::Dark => "Current Theme: Dark",
                ThemeVariant::Custom(name) => &format!("Current Theme: Custom ({})", name),
            };
            Text::new(variant_text.to_string())
                .with_layout_style(LayoutStyle {
                    size: Vector2::new(Dimension::percent(1.0), Dimension::length(30.0)),
                    margin: nptk::core::layout::Rect::<LengthPercentageAuto> {
                        left: LengthPercentageAuto::length(10.0),
                        right: LengthPercentageAuto::length(10.0),
                        top: LengthPercentageAuto::length(5.0),
                        bottom: LengthPercentageAuto::length(5.0),
                    },
                    ..Default::default()
                })
        };
        
        // Demo text showing theme colors
        let demo_text = Text::new("This text demonstrates theme-aware coloring!".to_string())
            .with_layout_style(LayoutStyle {
                size: Vector2::new(Dimension::percent(1.0), Dimension::length(30.0)),
                margin: nptk::core::layout::Rect::<LengthPercentageAuto> {
                    left: LengthPercentageAuto::length(10.0),
                    right: LengthPercentageAuto::length(10.0),
                    top: LengthPercentageAuto::length(10.0),
                    bottom: LengthPercentageAuto::length(10.0),
                },
                ..Default::default()
            });
        
        // Add all widgets to container
        container = Container::new(vec![
            Box::new(title),
            Box::new(light_theme_button),
            Box::new(dark_theme_button),
            Box::new(progress_up_button),
            Box::new(progress_down_button),
            Box::new(progress_bar),
            Box::new(theme_info),
            Box::new(demo_text),
        ])
        .with_layout_style(LayoutStyle {
            size: Vector2::new(Dimension::percent(1.0), Dimension::percent(1.0)),
            gap: 10.0,
            ..Default::default()
        });
        
        // Render the container
        container.render(scene, theme, layout, info, context);
    }

    fn update(&mut self, layout: &nptk::core::layout::LayoutNode, context: AppContext, info: &mut nptk::core::app::info::AppInfo) -> Update {
        Update::empty()
    }

    fn layout_style(&self) -> nptk::core::layout::StyleNode {
        nptk::core::layout::StyleNode {
            style: LayoutStyle {
                size: Vector2::new(Dimension::percent(1.0), Dimension::percent(1.0)),
                ..Default::default()
            },
            children: vec![],
        }
    }

    fn widget_id(&self) -> WidgetId {
        WidgetId::new("theme-demo", "ThemeDemoApp")
    }
}

fn main() {
    let app = ThemeDemoApp {
        theme_manager: Arc::new(std::sync::RwLock::new(ThemeManager::new())),
        current_variant: StateSignal::new(ThemeVariant::Light),
        progress_value: StateSignal::new(0.5_f32),
    };
    
    app.run(());
}
