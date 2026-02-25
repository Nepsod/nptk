use nptk_core::widget::{BoxedWidget, Widget};
use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{LayoutNode, StyleNode};
use nptk_core::vgi::{Graphics, BatchedGraphics};
use nptk_core::vg::kurbo::Affine;
use nptk_core::vg::Scene;
use async_trait::async_trait;
use nptk_core::layout::{AvailableSpace, Size};

/// A widget that caches its rendering output into a vector scene fragment.
///
/// `CachedWidget` intercepts the drawing operations of its child and encodes them
/// into a `vello::Scene`. On subsequent frames where the widget is not dirty,
/// the cached scene is appended to the main graphics context with a translation,
/// bypassing the need to re-encode the primitives.
///
/// This provides a significant performance boost for complex, static subtrees.
pub struct CachedWidget {
    child: BoxedWidget,
    cached_scene: Option<Scene>,
    is_dirty: bool,
    last_layout_size: Option<Size<f32>>,
}

impl CachedWidget {
    /// Create a new `CachedWidget` wrapping the given child.
    pub fn new(child: impl Widget + 'static) -> Self {
        Self {
            child: Box::new(child),
            cached_scene: None,
            is_dirty: true,
            last_layout_size: None,
        }
    }

    fn offset_layout_tree(node: &mut LayoutNode, dx: f32, dy: f32) {
        node.layout.location.x += dx;
        node.layout.location.y += dy;
        for child in &mut node.children {
            Self::offset_layout_tree(child, dx, dy);
        }
    }
}

#[async_trait(?Send)]
impl Widget for CachedWidget {
    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        layout_node: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        if self.is_dirty || self.cached_scene.is_none() {
            let mut new_scene = Scene::new();
            
            // To ensure the scene can be appended with any transform without coordinate distortion,
            // we must trick the child into rendering itself at the layout origin (0, 0).
            let mut origin_layout = layout_node.clone();
            Self::offset_layout_tree(
                &mut origin_layout,
                -layout_node.layout.location.x,
                -layout_node.layout.location.y,
            );
            
            {
                let mut child_graphics = BatchedGraphics::new(&mut new_scene);
                self.child.render(&mut child_graphics, &origin_layout, info, context.clone());
                child_graphics.finish();
            }
            
            self.cached_scene = Some(new_scene);
            self.is_dirty = false;
        }

        if let Some(scene) = &self.cached_scene {
            let transform = Affine::translate((
                layout_node.layout.location.x as f64,
                layout_node.layout.location.y as f64,
            ));
            graphics.append(scene, Some(transform));
        }
    }

    fn render_postfix(
        &mut self,
        graphics: &mut dyn Graphics,
        layout_node: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        // We do not cache postfix overlays, as they frequently depend on dynamic states (like popups).
        self.child.render_postfix(graphics, layout_node, info, context);
    }

    fn layout_style(&self, context: &nptk_core::layout::LayoutContext) -> StyleNode {
        self.child.layout_style(context)
    }

    fn measure(&self, constraints: Size<AvailableSpace>) -> Option<Size<f32>> {
        self.child.measure(constraints)
    }

    async fn update(
        &mut self,
        layout: &LayoutNode,
        context: AppContext,
        info: &mut AppInfo,
    ) -> Update {
        let update = self.child.update(layout, context, info).await;
        
        // Invalidate cache if drawing or layout changes were requested by the child subtree
        if update.contains(Update::DRAW) || update.contains(Update::LAYOUT) {
            self.is_dirty = true;
        }

        let new_size = Size { 
            width: layout.layout.size.width, 
            height: layout.layout.size.height 
        };
        if let Some(last) = self.last_layout_size {
            // Invalidate cache if the parent implicitly assigned a new geometry size to this subtree
            if last.width != new_size.width || last.height != new_size.height {
                self.is_dirty = true;
            }
        }
        self.last_layout_size = Some(new_size);

        update
    }

    fn context_menu(&self) -> Option<nptk_core::menu::MenuTemplate> {
        self.child.context_menu()
    }

    fn tooltip(&self) -> Option<String> {
        self.child.tooltip()
    }

    fn set_tooltip(&mut self, tooltip: Option<String>) {
        self.child.set_tooltip(tooltip);
    }

    fn is_visible(&self) -> bool {
        self.child.is_visible()
    }
}
