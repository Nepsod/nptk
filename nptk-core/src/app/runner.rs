use crate::app::context::AppContext;
use crate::app::font_ctx::FontContext;
use crate::app::handler::AppHandler;
use crate::app::update::UpdateManager;
use crate::config::MayConfig;
use crate::plugin::PluginManager;
use crate::widget::Widget;
use nptk_theme::theme::Theme;
use peniko::Font;
use winit::dpi::{LogicalPosition, LogicalSize, Position, Size};
use winit::event_loop::EventLoopBuilder;
use winit::window::WindowAttributes;

/// The core Application structure.
pub struct MayRunner<T: Theme> {
    config: MayConfig<T>,
    font_ctx: FontContext,
}

impl<T: Theme> MayRunner<T> {
    /// Create a new App with the given [MayConfig].
    pub fn new(config: MayConfig<T>) -> Self {
        Self::initialize_task_runner(&config);
        let font_ctx = Self::create_font_context(&config);
        
        Self {
            config,
            font_ctx,
        }
    }

    /// Initialize the task runner if configured.
    fn initialize_task_runner(config: &MayConfig<T>) {
        if let Some(task_config) = &config.tasks {
            log::info!("initializing task runner");
            crate::tasks::init(*task_config);
        }
    }

    /// Create a font context based on the configuration.
    fn create_font_context(config: &MayConfig<T>) -> FontContext {
        if config.render.lazy_font_loading {
            FontContext::new()
        } else {
            FontContext::new_with_system_fonts()
        }
    }

    /// Loads a new font into the font context.
    ///
    /// See [FontContext::load] for more.
    pub fn with_font(mut self, name: impl ToString, font: Font) -> Self {
        self.font_ctx.load(name, font);
        self
    }

    /// Set the font context. Can be used to configure fonts.
    pub fn with_font_context(mut self, font_ctx: FontContext) -> Self {
        self.font_ctx = font_ctx;
        self
    }

    /// Run the application with given widget and state.
    pub fn run<S, W, F>(mut self, state: S, builder: F, mut plugins: PluginManager<T>)
    where
        W: Widget,
        F: Fn(AppContext, S) -> W,
    {
        let mut event_loop = Self::create_event_loop();
        let mut attrs = Self::build_window_attributes(&self.config);
        Self::apply_optional_window_attributes(&self.config, &mut attrs);

        log::info!("Launching Application...");
        let update = UpdateManager::new();
        plugins.run(|pl| pl.init(&mut event_loop, &update, &mut attrs, &mut self.config));

        Self::run_app_handler(
            event_loop,
            attrs,
            self.config,
            builder,
            state,
            self.font_ctx,
            update,
            plugins,
        );
    }

    /// Create and build the event loop.
    fn create_event_loop() -> winit::event_loop::EventLoop<()> {
        EventLoopBuilder::default()
            .build()
            .expect("Failed to create event loop")
    }

    /// Build window attributes from configuration.
    fn build_window_attributes(config: &MayConfig<T>) -> WindowAttributes {
        WindowAttributes::default()
            .with_inner_size(LogicalSize::new(
                config.window.size.x,
                config.window.size.y,
            ))
            .with_resizable(config.window.resizable)
            .with_enabled_buttons(config.window.buttons)
            .with_title(config.window.title.clone())
            .with_maximized(config.window.maximized)
            .with_visible(config.window.visible)
            .with_transparent(config.window.transparent)
            .with_blur(config.window.blur)
            .with_decorations(config.window.decorations)
            .with_window_icon(config.window.icon.clone())
            .with_content_protected(config.window.content_protected)
            .with_window_level(config.window.level)
            .with_active(config.window.active)
            .with_cursor(config.window.cursor.clone())
    }

    /// Apply optional window attributes that require manual setting.
    ///
    /// These attributes don't have builder methods that accept `Option` values,
    /// so they must be set directly on the attributes struct.
    fn apply_optional_window_attributes(config: &MayConfig<T>, attrs: &mut WindowAttributes) {
        Self::set_optional_size(&mut attrs.max_inner_size, &config.window.max_size);
        Self::set_optional_size(&mut attrs.min_inner_size, &config.window.min_size);
        Self::set_optional_position(&mut attrs.position, &config.window.position);
        Self::set_optional_size(&mut attrs.resize_increments, &config.window.resize_increments);
    }

    /// Set an optional size attribute.
    fn set_optional_size(target: &mut Option<Size>, source: &Option<nalgebra::Vector2<f64>>) {
        *target = source.map(|v| Size::Logical(LogicalSize::new(v.x, v.y)));
    }

    /// Set an optional position attribute.
    fn set_optional_position(target: &mut Option<Position>, source: &Option<nalgebra::Point2<f64>>) {
        *target = source.map(|v| Position::Logical(LogicalPosition::new(v.x, v.y)));
    }

    /// Run the application handler with the event loop.
    fn run_app_handler<S, W, F>(
        event_loop: winit::event_loop::EventLoop<()>,
        attrs: WindowAttributes,
        config: MayConfig<T>,
        builder: F,
        state: S,
        font_ctx: FontContext,
        update: UpdateManager,
        plugins: PluginManager<T>,
    ) where
        W: Widget,
        F: Fn(AppContext, S) -> W,
    {
        event_loop
            .run_app(&mut AppHandler::new(
                attrs,
                config,
                builder,
                state,
                font_ctx,
                update,
                plugins,
            ))
            .expect("Failed to run event loop");
    }
}
