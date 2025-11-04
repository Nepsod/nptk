use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{LayoutNode, LayoutStyle, StyleNode};
use nptk_core::vgi::Graphics;
use nptk_core::vgi::vello_vg::VelloGraphics;
use nptk_core::widget::Widget;
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;

/// A canvas widget to directly draw to the screen.
///
/// This is a very simplified version of "your own Widget" and you should only use it for simple cases.
///
/// ### Theming
/// The canvas cannot be themed, since it does not draw something on itself.
pub struct Canvas {
    painter: Box<dyn FnMut(&mut dyn Graphics, &AppInfo)>,
}

impl Canvas {
    /// Create a new Canvas widget from a painter function.
    pub fn new(painter: impl FnMut(&mut dyn Graphics, &AppInfo) + 'static) -> Self {
        Self {
            painter: Box::new(painter),
        }
    }

    /// Set a painter function and return itself.
    pub fn with_painter(mut self, painter: impl FnMut(&mut dyn Graphics, &AppInfo) + 'static) -> Self {
        self.painter = Box::new(painter);
        self
    }
}

impl Widget for Canvas {
    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        _: &mut dyn Theme,
        _: &LayoutNode,
        info: &mut AppInfo,
        _: AppContext,
    ) {
        let mut canvas = nptk_core::vg::Scene::new();
        let mut child_graphics = VelloGraphics::new(&mut canvas);
        (self.painter)(&mut child_graphics, info);

        graphics.append(&canvas, None);
    }

    fn layout_style(&self) -> StyleNode {
        StyleNode {
            style: LayoutStyle::default(),
            children: Vec::new(),
        }
    }

    fn update(&mut self, _: &LayoutNode, _: AppContext, _: &mut AppInfo) -> Update {
        Update::DRAW | Update::LAYOUT
    }

    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "Canvas")
    }
}

