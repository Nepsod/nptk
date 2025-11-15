use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{LayoutNode, LayoutStyle, StyleNode};
use nptk_core::signal::MaybeSignal;
use nptk_core::vg::kurbo::{Affine, Vec2};
use nptk_core::vg::peniko::ImageBrush;
pub use nptk_core::vg::peniko::ImageData;
use nptk_core::vgi::Graphics;
use nptk_core::widget::{Widget, WidgetLayoutExt};
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;
use std::ops::Deref;

/// An image widget. Pretty self-explanatory.
///
/// ### Theming
/// The widget itself only draws the underlying image, so theming is useless.
pub struct Image {
    image: MaybeSignal<ImageBrush>,
    style: MaybeSignal<LayoutStyle>,
}

impl Image {
    /// Create an image widget from the given [ImageBrush].
    pub fn new(image: impl Into<MaybeSignal<ImageBrush>>) -> Self {
        Self {
            image: image.into(),
            style: LayoutStyle::default().into(),
        }
    }

    /// Set the image.
    pub fn with_image(mut self, image: impl Into<MaybeSignal<ImageBrush>>) -> Self {
        self.image = image.into();
        self
    }
}

impl WidgetLayoutExt for Image {
    fn set_layout_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.style = layout_style.into();
    }
}

impl Widget for Image {
    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        _: &mut dyn Theme,
        layout_node: &LayoutNode,
        _: &mut AppInfo,
        _: AppContext,
    ) {
        let image = self.image.get();

        // Use as_scene_mut() to get Scene for image drawing
        if let Some(scene) = graphics.as_scene_mut() {
            scene.draw_image(
                image.deref(),
                Affine::translate(Vec2::new(
                    layout_node.layout.location.x as f64,
                    layout_node.layout.location.y as f64,
                )),
            );
        }
    }

    fn layout_style(&self) -> StyleNode {
        StyleNode {
            style: self.style.get().clone(),
            children: Vec::new(),
        }
    }

    fn update(&mut self, _: &LayoutNode, _: AppContext, _: &mut AppInfo) -> Update {
        Update::empty()
    }

    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "Image")
    }
}
