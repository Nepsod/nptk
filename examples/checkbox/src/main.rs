use nptk::prelude::*;

struct MyApp;

impl Application for MyApp {
    type Theme = SystemTheme;
    type State = ();

    fn build(context: AppContext, _: Self::State) -> impl Widget {
        let checkbox1_state = context.use_signal(StateSignal::new(CheckboxState::Unchecked));
        let checkbox2_state = context.use_signal(StateSignal::new(CheckboxState::Checked));
        let checkbox3_state = context.use_signal(StateSignal::new(CheckboxState::Indeterminate));
        let toggle_state = context.use_signal(StateSignal::new(false));

        Container::new(vec![
            Box::new(Text::new(
                "Three-State Checkbox with State Locking".to_string(),
            )),
            // Simple checkbox - only checked/unchecked (default behavior)
            Box::new(Text::new(
                "Simple Checkbox (checked/unchecked only):".to_string(),
            )),
            Box::new(Checkbox::new(checkbox1_state.clone().maybe())),
            Box::new(Text::new(
                checkbox1_state.map(|val| Ref::Owned(format!("State: {:?}", *val))),
            )),
            // Three-state checkbox - with indeterminate state enabled
            Box::new(Text::new(
                "Three-State Checkbox (with indeterminate):".to_string(),
            )),
            Box::new(Checkbox::new(checkbox2_state.clone().maybe()).with_indeterminate_state()),
            Box::new(Text::new(
                checkbox2_state.map(|val| Ref::Owned(format!("State: {:?}", *val))),
            )),
            // Three-state checkbox with indeterminate state locked
            Box::new(Text::new(
                "Three-State Checkbox (indeterminate locked):".to_string(),
            )),
            Box::new(
                Checkbox::new(checkbox3_state.clone().maybe())
                    .with_indeterminate_state()
                    .with_locked_state(CheckboxState::Indeterminate),
            ),
            Box::new(Text::new(
                checkbox3_state.map(|val| Ref::Owned(format!("State: {:?}", *val))),
            )),
            // Toggle button
            Box::new(Text::new("".to_string())), // Spacer
            Box::new(Text::new("Toggle Button:".to_string())),
            Box::new(
                Toggle::new(toggle_state.clone().maybe()).with_on_toggle(|is_on| {
                    println!("Toggle is now: {}", if is_on { "ON" } else { "OFF" });
                }),
            ),
            Box::new(Text::new(toggle_state.map(|val| {
                Ref::Owned(format!("State: {}", if *val { "ON" } else { "OFF" }))
            }))),
            // Instructions
            Box::new(Text::new("".to_string())), // Spacer
            Box::new(Text::new("Instructions:".to_string())),
            Box::new(Text::new(
                "• Simple checkbox: Unchecked ↔ Checked".to_string(),
            )),
            Box::new(Text::new(
                "• Three-state checkbox: Unchecked → Checked → Indeterminate → Unchecked"
                    .to_string(),
            )),
            Box::new(Text::new(
                "• Locked checkboxes won't change when in locked state".to_string(),
            )),
            Box::new(Text::new(
                "• Use .with_indeterminate_state() for master checkboxes".to_string(),
            )),
        ])
        .with_layout_style(LayoutStyle {
            size: Vector2::<Dimension>::new(Dimension::percent(1.0), Dimension::percent(1.0)),
            flex_direction: FlexDirection::Column,
            align_items: Some(AlignItems::FlexStart),
            padding: Rect {
                left: LengthPercentage::length(10.0),
                right: LengthPercentage::length(10.0),
                top: LengthPercentage::length(10.0),
                bottom: LengthPercentage::length(10.0),
            },
            ..Default::default()
        })
    }
}

fn main() {
    MyApp.run(())
}
