use nptk::prelude::*;
use std::sync::Arc;
struct ProgressApp;

impl Application for ProgressApp {
    type Theme = SystemTheme;
    type State = ();


    fn build(context: AppContext, _config: Self::State) -> impl Widget {
        // Create signals for progress values
        let progress_value = context.use_signal(StateSignal::new(0.0_f32));
        let indeterminate = context.use_signal(StateSignal::new(false));
        
        // Create buttons to control progress
        let increment_button = {
            let progress_value = progress_value.clone();
            Button::new(Text::new("+10%".to_string()))
                .with_layout_style(LayoutStyle {
                    size: Vector2::new(Dimension::length(80.0), Dimension::length(40.0)),
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

        let decrement_button = {
            let progress_value = progress_value.clone();
            Button::new(Text::new("-10%".to_string()))
                .with_layout_style(LayoutStyle {
                    size: Vector2::new(Dimension::length(80.0), Dimension::length(40.0)),
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

        let toggle_indeterminate_button = {
            let indeterminate = indeterminate.clone();
            Button::new(Text::new("Toggle Indeterminate".to_string()))
                .with_layout_style(LayoutStyle {
                    size: Vector2::new(Dimension::length(180.0), Dimension::length(40.0)),
                    ..Default::default()
                })
                .with_on_pressed(
                    EvalSignal::new(move || {
                        let current = *indeterminate.get();
                        indeterminate.set(!current);
                        Update::DRAW
                    })
                    .hook(&context)
                    .maybe(),
                )
        };

        let reset_button = {
            let progress_value = progress_value.clone();
            let indeterminate = indeterminate.clone();
            Button::new(Text::new("Reset".to_string()))
                .with_layout_style(LayoutStyle {
                    size: Vector2::new(Dimension::length(80.0), Dimension::length(40.0)),
                    ..Default::default()
                })
                .with_on_pressed(
                    EvalSignal::new(move || {
                        progress_value.set(0.0);
                        indeterminate.set(false);
                        Update::DRAW
                    })
                    .hook(&context)
                    .maybe(),
                )
        };

        Container::new(vec![
            // Title
            Box::new(
                Text::new("Progress Widget Demo".to_string())
                    .with_layout_style(LayoutStyle {
                        size: Vector2::new(Dimension::percent(1.0), Dimension::auto()),
                        ..Default::default()
                    }),
            ),
            
            // Progress bar 1: Determinate
            Box::new(
                Text::new("Determinate Progress:".to_string())
                    .with_layout_style(LayoutStyle {
                        size: Vector2::new(Dimension::percent(1.0), Dimension::auto()),
                        ..Default::default()
                    }),
            ),
            Box::new(
                Progress::new(progress_value.clone() as Arc<dyn Signal<f32>>)
                    .with_layout_style(LayoutStyle {
                        size: Vector2::new(Dimension::percent(1.0), Dimension::length(30.0)),
                        ..Default::default()
                    }),
            ),
            
            // Progress bar 2: Indeterminate
            Box::new(
                Text::new("Indeterminate Progress:".to_string())
                    .with_layout_style(LayoutStyle {
                        size: Vector2::new(Dimension::percent(1.0), Dimension::auto()),
                        ..Default::default()
                    }),
            ),
            Box::new(
                Progress::new(0.0)
                    .with_indeterminate(indeterminate.clone() as Arc<dyn Signal<bool>>)
                    .with_layout_style(LayoutStyle {
                        size: Vector2::new(Dimension::percent(1.0), Dimension::length(30.0)),
                        ..Default::default()
                    }),
            ),
            
            // Control buttons
            Box::new(
                Container::new(vec![
                    Box::new(increment_button),
                    Box::new(decrement_button),
                    Box::new(toggle_indeterminate_button),
                    Box::new(reset_button),
                ])
                .with_layout_style(LayoutStyle {
                    size: Vector2::new(Dimension::percent(1.0), Dimension::auto()),
                    flex_direction: FlexDirection::Row,
                    align_items: Some(AlignItems::Center),
                    gap: Vector2::new(
                        LengthPercentage::length(10.0),
                        LengthPercentage::length(0.0),
                    ),
                    ..Default::default()
                }),
            ),
            
            // Instructions
            Box::new(
                Text::new("Use the buttons to control the progress bars. The indeterminate mode shows an animated progress bar.".to_string())
                    .with_layout_style(LayoutStyle {
                        size: Vector2::new(Dimension::percent(1.0), Dimension::auto()),
                        ..Default::default()
                    }),
            ),
        ])
        .with_layout_style(LayoutStyle {
            size: Vector2::new(Dimension::percent(1.0), Dimension::percent(1.0)),
            flex_direction: FlexDirection::Column,
            align_items: Some(AlignItems::Center),
            gap: Vector2::new(
                LengthPercentage::length(0.0),
                LengthPercentage::length(20.0),
            ),
            padding: nptk::core::layout::Rect::<LengthPercentage> {
                left: LengthPercentage::length(20.0),
                right: LengthPercentage::length(20.0),
                top: LengthPercentage::length(40.0),
                bottom: LengthPercentage::length(20.0),
            },
            ..Default::default()
        })
    }
}

fn main() {
    ProgressApp.run(());
}
