use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{LayoutContext, LayoutNode, StyleNode};
use nptk_core::signal::MaybeSignal;
use nptk_core::vgi::Graphics;
use nptk_core::widget::{BoxedWidget, Widget};
use nptk_core::window::{ElementState, MouseButton};
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;
use async_trait::async_trait;

/// A widget to detect gestures like pressing or releasing the left mouse button.
/// It can also contain a child widget.
///
/// The [GestureDetector] has different callbacks that are called on different events:
/// - `on_press` is called when the left mouse button is pressed.
/// - `on_release` is called when the left mouse button is released.
/// - `on_hover` is called when the mouse cursor hovers over the widget.
///
/// ### Theming
/// The [GestureDetector] should not be themed and does not draw anything on itself.
/// It just contains the given child widget.
pub struct GestureDetector {
    child: BoxedWidget,
    callbacks: GestureCallbacks,
}

/// Bundled set of gesture callbacks used by [`GestureDetector`].
#[derive(Clone)]
pub struct GestureCallbacks {
    press: MaybeSignal<Update>,
    release: MaybeSignal<Update>,
    hover: MaybeSignal<Update>,
}

#[allow(missing_docs)] // TODO: Add docs?
impl GestureCallbacks {
    fn new() -> Self {
        Self::default()
    }

    pub fn with_press(mut self, update: impl Into<MaybeSignal<Update>>) -> Self {
        self.press = update.into();
        self
    }

    pub fn with_release(mut self, update: impl Into<MaybeSignal<Update>>) -> Self {
        self.release = update.into();
        self
    }

    pub fn with_hover(mut self, update: impl Into<MaybeSignal<Update>>) -> Self {
        self.hover = update.into();
        self
    }

    pub fn set_press(&mut self, update: impl Into<MaybeSignal<Update>>) {
        self.press = update.into();
    }

    pub fn set_release(&mut self, update: impl Into<MaybeSignal<Update>>) {
        self.release = update.into();
    }

    pub fn set_hover(&mut self, update: impl Into<MaybeSignal<Update>>) {
        self.hover = update.into();
    }

    fn trigger_press(&self) -> Update {
        *self.press.get()
    }

    fn trigger_release(&self) -> Update {
        *self.release.get()
    }

    fn trigger_hover(&self) -> Update {
        *self.hover.get()
    }
}

impl Default for GestureCallbacks {
    fn default() -> Self {
        Self {
            press: MaybeSignal::value(Update::empty()),
            release: MaybeSignal::value(Update::empty()),
            hover: MaybeSignal::value(Update::empty()),
        }
    }
}

impl GestureDetector {
    /// Creates a new [GestureDetector] with the given child widget.
    pub fn new(child: impl Widget + 'static) -> Self {
        Self {
            child: Box::new(child),
            callbacks: GestureCallbacks::new(),
        }
    }

    fn apply_with(mut self, f: impl FnOnce(&mut Self)) -> Self {
        f(&mut self);
        self
    }

    /// Sets the child widget of the [GestureDetector] and returns self.
    pub fn with_child(self, child: impl Widget + 'static) -> Self {
        self.apply_with(|this| this.child = Box::new(child))
    }

    /// Sets the `on_press` callback of the [GestureDetector] and returns self.
    pub fn with_on_press(self, on_press: impl Into<MaybeSignal<Update>>) -> Self {
        self.apply_with(move |this| this.callbacks.set_press(on_press))
    }

    /// Sets the `on_release` callback of the [GestureDetector] and returns self.
    pub fn with_on_release(self, on_release: impl Into<MaybeSignal<Update>>) -> Self {
        self.apply_with(move |this| this.callbacks.set_release(on_release))
    }

    /// Sets the `on_hover` callback of the [GestureDetector] and returns self.
    pub fn with_on_hover(self, on_hover: impl Into<MaybeSignal<Update>>) -> Self {
        self.apply_with(move |this| this.callbacks.set_hover(on_hover))
    }

    /// Replace all callbacks with a bundled configuration.
    pub fn with_callbacks(self, callbacks: GestureCallbacks) -> Self {
        self.apply_with(|this| this.callbacks = callbacks)
    }

    /// Call the `on_hover` callback of the [GestureDetector].
    pub fn on_hover(&mut self) -> Update {
        self.callbacks.trigger_hover()
    }

    /// Call the `on_press` callback of the [GestureDetector].
    pub fn on_press(&mut self) -> Update {
        self.callbacks.trigger_press()
    }

    /// Call the `on_release` callback of the [GestureDetector].
    pub fn on_release(&mut self) -> Update {
        self.callbacks.trigger_release()
    }

    fn cursor_in_bounds(layout: &LayoutNode, cursor_x: f64, cursor_y: f64) -> bool {
        cursor_x as f32 >= layout.layout.location.x
            && cursor_x as f32 <= layout.layout.location.x + layout.layout.size.width
            && cursor_y as f32 >= layout.layout.location.y
            && cursor_y as f32 <= layout.layout.location.y + layout.layout.size.height
    }

    fn process_mouse_buttons(&mut self, info: &AppInfo) -> Update {
        let mut update = Update::empty();
        for (_, btn, el) in &info.buttons {
            if *btn == MouseButton::Left {
                match el {
                    ElementState::Pressed => {
                        update |= self.on_press();
                    },
                    ElementState::Released => {
                        update |= self.on_release();
                    },
                }
            }
        }
        update
    }
}

#[async_trait(?Send)]
impl Widget for GestureDetector {
    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        layout_node: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        self.child
            .render(graphics, layout_node, info, context)
    }

    fn layout_style(&self, context: &LayoutContext) -> StyleNode {
        self.child.layout_style(context)
    }

    async fn update(&mut self, layout: &LayoutNode, context: AppContext, info: &mut AppInfo) -> Update {
        let mut update = Update::empty();

        if let Some(cursor) = info.cursor_pos {
            if Self::cursor_in_bounds(layout, cursor.x, cursor.y) {
                update |= self.on_hover();
                update |= self.process_mouse_buttons(info);
            }
        }

        update |= self.child.update(layout, context, info).await;

        update
    }

    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "GestureDetector")
    }
}
