use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{LayoutContext, LayoutNode, LayoutStyle, StyleNode};
use nptk_core::vgi::vello_vg::VelloGraphics;
use nptk_core::vgi::Graphics;
use nptk_core::widget::Widget;
use async_trait::async_trait;

/// A canvas widget to directly draw to the screen.
///
/// This is a very simplified version of "your own Widget" and you should only use it for simple cases.
///
/// ### Theming
/// The canvas cannot be themed, since it does not draw something on itself.
pub struct Canvas {
    painter: Box<dyn FnMut(&mut dyn Graphics, &LayoutNode, &mut AppInfo, AppContext) + Send + Sync>,
}

impl Canvas {
    /// Create a new Canvas widget from a painter function.
    pub fn new(painter: impl FnMut(&mut dyn Graphics, &LayoutNode, &mut AppInfo, AppContext) + Send + Sync + 'static) -> Self {
        Self {
            painter: Box::new(painter),
        }
    }

    /// Set the painter function.
    pub fn with_painter(
        mut self,
        painter: impl FnMut(&mut dyn Graphics, &LayoutNode, &mut AppInfo, AppContext) + Send + Sync + 'static,
    ) -> Self {
        self.painter = Box::new(painter);
        self
    }
}

#[async_trait(?Send)]
impl Widget for Canvas
{
    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        layout_node: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        let mut canvas = nptk_core::vg::Scene::new();
        let mut child_graphics = VelloGraphics::new(&mut canvas);
        (self.painter)(&mut child_graphics, layout_node, info, context);

        graphics.append(&canvas, None);
    }

    fn layout_style(&self, context: &LayoutContext) -> StyleNode {
        StyleNode {
            style: LayoutStyle::default(),
            children: Vec::new(),
            measure_func: None,
        }
    }

    async fn update(&mut self, _layout: &LayoutNode, _context: AppContext, _info: &mut AppInfo) -> Update {
        Update::DRAW | Update::LAYOUT
    }
}
