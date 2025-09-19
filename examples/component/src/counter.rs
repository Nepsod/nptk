use nptk::core::app::context::AppContext;
use nptk::core::app::update::Update;
use nptk::core::component::{Component, Composed};
use nptk::core::layout::LayoutStyle;
use nptk::core::reference::Ref;
use nptk::core::signal::eval::EvalSignal;
use nptk::core::signal::{ArcSignal, MaybeSignal, Signal};
use nptk::core::widget::{Widget, WidgetLayoutExt};
use nptk::theme::id::WidgetId;
use nptk::widgets::button::Button;
use nptk::widgets::container::Container;
use nptk::widgets::text::Text;

pub struct Counter {
    counter: ArcSignal<i32>,
    layout: MaybeSignal<LayoutStyle>,
}

impl Counter {
    pub fn new(counter: ArcSignal<i32>) -> Composed<Self> {
        Counter {
            counter,
            layout: LayoutStyle::default().into(),
        }
        .compose()
    }
}

impl Component for Counter {
    fn build(&self, context: AppContext) -> impl Widget + 'static {
        let counter = self.counter.clone();

        Container::new(vec![
            {
                let counter = counter.clone();

                Box::new(
                    Button::new(Text::new("Increase".to_string())).with_on_pressed(
                        EvalSignal::new(move || {
                            counter.set(*counter.get() + 1);
                            Update::DRAW
                        })
                        .hook(&context)
                        .maybe(),
                    ),
                )
            },
            {
                let counter = counter.clone();

                Box::new(
                    Button::new(Text::new("Decrease".to_string())).with_on_pressed(
                        EvalSignal::new(move || {
                            counter.set(*counter.get() - 1);
                            Update::DRAW
                        })
                        .hook(&context)
                        .maybe(),
                    ),
                )
            },
            Box::new(Text::new(
                MaybeSignal::signal(counter).map(|i| Ref::Owned(i.to_string())),
            )),
        ])
        .with_layout_style(self.layout.get().clone())
    }

    fn widget_id(&self) -> WidgetId {
        WidgetId::new("my-example", "Counter")
    }
}

impl WidgetLayoutExt for Counter {
    fn set_layout_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.layout = layout_style.into();
    }
}
