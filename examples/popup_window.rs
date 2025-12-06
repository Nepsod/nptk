use nptk::prelude::*;
use nptk::core::widget::WidgetChildrenExt;
use nptk::core::layout::{LengthPercentage, Rect, FlexDirection, LayoutStyle};
use nptk::math::Vector2;

use std::sync::{Arc, Mutex};

#[derive(Default)]
struct PopupExampleState {
    counter: usize,
}

struct PopupExample {
    state: Arc<Mutex<PopupExampleState>>,
}

impl Application for PopupExample {
    type Theme = DarkTheme;
    type State = ();

    fn build(context: AppContext, _state: Self::State) -> impl Widget {
        let popup_manager = context.popup_manager.clone();
        
        Container::new(vec![])
            .with_layout_style(LayoutStyle {
                flex_direction: FlexDirection::Column,
                padding: Rect { 
                    left: LengthPercentage::length(50.0), 
                    right: LengthPercentage::length(50.0), 
                    top: LengthPercentage::length(50.0), 
                    bottom: LengthPercentage::length(50.0) 
                },
                gap: Vector2::new(LengthPercentage::length(20.0), LengthPercentage::length(20.0)),
                ..Default::default()
            })
            .with_child(
                Text::new("Main Window".to_string()).with_font_size(24.0)
            )
            .with_child(
                Button::new(Text::new("Open Popup".to_string()))
                    .with_on_pressed(MaybeSignal::signal(Box::new(EvalSignal::new(move || {
                        println!("Opening popup...");
                        let popup_content = Container::new(vec![])
                            .with_layout_style(LayoutStyle {
                                flex_direction: FlexDirection::Column,
                                flex_grow: 1.0,
                                padding: Rect { 
                                    left: LengthPercentage::length(20.0), 
                                    right: LengthPercentage::length(20.0), 
                                    top: LengthPercentage::length(20.0), 
                                    bottom: LengthPercentage::length(20.0) 
                                },
                                ..Default::default()
                            })
                            .with_child(Text::new("I am a popup window!".to_string()).with_font_size(20.0));
                        
                        popup_manager.create_popup(
                            Box::new(popup_content),
                            "Popup Window",
                            (300, 200)
                        );
                        Update::empty()
                    }))))
            )
    }
}

fn main() {
    PopupExample {
        state: Arc::new(Mutex::new(PopupExampleState::default())),
    }
    .run(());
}
