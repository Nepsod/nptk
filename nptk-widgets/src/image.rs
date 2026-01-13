use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{LayoutNode, LayoutStyle, StyleNode};
use nptk_core::signal::MaybeSignal;
use nptk_core::vg::kurbo::{Affine, Vec2};
use nptk_core::vg::peniko::{Blob, ImageAlphaType, ImageBrush, ImageFormat};
pub use nptk_core::vg::peniko::ImageData;
use nptk_core::vgi::Graphics;
use nptk_core::widget::{Widget, WidgetLayoutExt};
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;
use async_trait::async_trait;

/// An image widget that renders a brush-backed bitmap inside the layout rect.
///
/// The widget itself only draws the underlying image, so it ignores theme data.
pub struct Image {
    state: ImageState,
}

impl Image {
    /// Create an image widget from the given [`ImageBrush`] (or signal of brushes).
    pub fn new(image: impl Into<MaybeSignal<ImageBrush>>) -> Self {
        Self::from_brush(image)
    }

    /// Create an image from any brush-like signal.
    pub fn from_brush(image: impl Into<MaybeSignal<ImageBrush>>) -> Self {
        Self {
            state: ImageState::new(image.into()),
        }
    }

    /// Create an image directly from [`ImageData`].
    pub fn from_data(data: ImageData) -> Self {
        Self::from_brush(ImageBrush::new(data))
    }

    /// Convenience helper to build an image from raw RGBA pixels.
    pub fn from_rgba(size: (u32, u32), pixels: impl Into<Vec<u8>>) -> Self {
        let (width, height) = size;
        let data = ImageData {
            data: Blob::from(pixels.into()),
            format: ImageFormat::Rgba8,
            alpha_type: ImageAlphaType::Alpha,
            width,
            height,
        };
        Self::from_data(data)
    }

    /// Replace the backing brush/signal.
    pub fn with_image(self, image: impl Into<MaybeSignal<ImageBrush>>) -> Self {
        self.apply_with(|this| this.state.set_image(image))
    }

    /// Override the widget's layout style.
    pub fn with_layout_style(self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) -> Self {
        self.apply_with(|this| this.state.set_style(layout_style))
    }

    fn apply_with(mut self, f: impl FnOnce(&mut Self)) -> Self {
        f(&mut self);
        self
    }

    fn current_brush(&self) -> ImageBrush {
        self.state.brush()
    }
}

impl WidgetLayoutExt for Image {
    fn set_layout_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.state.set_style(layout_style);
    }
}

#[async_trait(?Send)]
impl Widget for Image {
    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        _: &mut dyn Theme,
        layout_node: &LayoutNode,
        _: &mut AppInfo,
        _: AppContext,
    ) {
        let brush = self.current_brush();

        let Some(scene) = graphics.as_scene_mut() else {
            // TODO: provide a CPU fallback rendering path when no scene is available.
            return;
        };

        scene.draw_image(
            &brush,
            Affine::translate(Vec2::new(
                layout_node.layout.location.x as f64,
                layout_node.layout.location.y as f64,
            )),
        );
    }

    fn layout_style(&self) -> StyleNode {
        StyleNode {
            style: self.state.layout_style(),
            children: Vec::new(),
        }
    }

    async fn update(&mut self, _layout: &LayoutNode, _context: AppContext, _info: &mut AppInfo) -> Update {
        Update::empty()
    }

    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "Image")
    }
}

struct ImageState {
    image: MaybeSignal<ImageBrush>,
    style: MaybeSignal<LayoutStyle>,
}

impl ImageState {
    fn new(image: MaybeSignal<ImageBrush>) -> Self {
        Self {
            image,
            style: LayoutStyle::default().into(),
        }
    }

    fn set_image(&mut self, image: impl Into<MaybeSignal<ImageBrush>>) {
        self.image = image.into();
    }

    fn set_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.style = layout_style.into();
    }

    fn layout_style(&self) -> LayoutStyle {
        self.style.get().clone()
    }

    fn brush(&self) -> ImageBrush {
        self.image.get().clone()
    }
}
