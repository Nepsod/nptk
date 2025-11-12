use crate::app::context::AppContext;
use crate::app::info::AppInfo;
use crate::app::update::Update;
use crate::layout::{LayoutNode, LayoutStyle, StyleNode};
use crate::signal::MaybeSignal;
use crate::vgi::Graphics;
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;

/// A boxed widget.
pub type BoxedWidget = Box<dyn Widget>;

/// The base trait for all widgets.
///
/// # Widget Rendering Lifecycle
///
/// Widgets follow a three-phase rendering lifecycle:
///
/// 1. **Main Render**: [`render()`](Widget::render) - Draw the widget's main content
/// 2. **Children Render**: Children are rendered recursively
/// 3. **Postfix Render**: [`render_postfix()`](Widget::render_postfix) - Draw overlays on top
///
/// This ensures that overlays (popups, tooltips, dropdowns) always appear on top of other content
/// through natural render ordering, without needing a separate overlay management system.
///
/// # Postfix Rendering Pattern
///
/// The postfix rendering pattern  provides a simple way to implement overlays, popups,
/// and other content that should appear "on top" of everything else.
///
/// ## How It Works
///
/// ```text
/// Widget Tree:          Render Order:
///
/// Container             1. Container.render()      (background)
/// ├─ Text "Hello"       2. Text.render()           (text)
/// └─ MenuButton         3. MenuButton.render()     (button)
///    └─ Popup           4. Text.render_postfix()   (nothing)
///                       5. MenuButton.render_postfix()  (popup on top!)
///                       6. Container.render_postfix()   (nothing)
/// ```
///
/// ## Basic Example: Tooltip Widget
///
/// ```rust,no_run
/// use nptk_core::widget::Widget;
/// use nptk_core::vgi::Graphics;
/// use nptk_core::layout::{LayoutNode, StyleNode};
/// use nptk_core::app::{AppContext, AppInfo};
/// use nptk_theme::theme::Theme;
/// use nptk_core::app::update::Update;
/// use nptk_theme::id::WidgetId;
///
/// struct TooltipWidget {
///     child: Box<dyn Widget>,
///     tooltip_text: String,
///     is_hovered: bool,
/// }
///
/// impl Widget for TooltipWidget {
///     fn render(&mut self, graphics: &mut dyn Graphics, theme: &mut dyn Theme,
///               layout: &LayoutNode, info: &mut AppInfo, context: AppContext) {
///         // Render the main widget content (the child)
///         if !layout.children.is_empty() {
///             self.child.render(graphics, theme, &layout.children[0], info, context);
///         }
///     }
///
///     fn render_postfix(&mut self, graphics: &mut dyn Graphics, theme: &mut dyn Theme,
///                       layout: &LayoutNode, info: &mut AppInfo, context: AppContext) {
///         // Render tooltip on top if hovered
///         if self.is_hovered {
///             // Draw tooltip box below the widget
///             // This will appear on top of everything else!
///             // ... tooltip rendering code ...
///         }
///     }
///
///     fn update(&mut self, layout: &LayoutNode, context: AppContext, info: &mut AppInfo) -> Update {
///         // Check if mouse is hovering over widget
///         if let Some(cursor_pos) = info.cursor_pos {
///             self.is_hovered = /* check if cursor is inside bounds */
/// #           false;
///         }
/// #       Update::empty()
///     }
///
///     fn layout_style(&self) -> StyleNode {
/// #       StyleNode { style: Default::default(), children: vec![] }
///         // ... layout code ...
///     }
///
///     fn widget_id(&self) -> WidgetId {
///         WidgetId::new("myapp", "TooltipWidget")
///     }
/// }
/// ```
///
/// ## Advanced Example: Dropdown Menu
///
/// ```rust,no_run
/// use nptk_core::widget::Widget;
/// use nptk_core::vgi::Graphics;
/// use nptk_core::layout::{LayoutNode, StyleNode, Layout};
/// use nptk_core::app::{AppContext, AppInfo};
/// use nptk_theme::theme::Theme;
/// use nptk_core::app::update::Update;
/// use nptk_theme::id::WidgetId;
///
/// struct DropdownWidget {
///     button: Box<dyn Widget>,
///     items: Vec<String>,
///     is_open: bool,
///     selected_index: Option<usize>,
/// }
///
/// impl Widget for DropdownWidget {
///     fn render(&mut self, graphics: &mut dyn Graphics, theme: &mut dyn Theme,
///               layout: &LayoutNode, info: &mut AppInfo, context: AppContext) {
///         // Render just the button showing selected item
///         if !layout.children.is_empty() {
///             self.button.render(graphics, theme, &layout.children[0], info, context);
///         }
///     }
///
///     fn render_postfix(&mut self, graphics: &mut dyn Graphics, theme: &mut dyn Theme,
///                       layout: &LayoutNode, info: &mut AppInfo, context: AppContext) {
///         // Render dropdown list on top when open
///         if self.is_open {
///             // Calculate position below the button
///             let dropdown_x = layout.layout.location.x as f64;
///             let dropdown_y = (layout.layout.location.y + layout.layout.size.height) as f64;
///
///             // Create layout for dropdown list
///             let mut dropdown_layout = LayoutNode {
///                 layout: Layout::default(),
///                 children: Vec::new(),
///             };
///             dropdown_layout.layout.location.x = dropdown_x as f32;
///             dropdown_layout.layout.location.y = dropdown_y as f32;
///
///             // Render each item in the list
///             // This appears on top of ALL other content!
///             for (i, item) in self.items.iter().enumerate() {
///                 // ... render item ...
///             }
///         }
///     }
///
///     fn update(&mut self, layout: &LayoutNode, context: AppContext, info: &mut AppInfo) -> Update {
///         let mut update = Update::empty();
///
///         // Toggle dropdown on button click
///         // Handle item selection
///         // Handle click-outside-to-close
///         // ... update logic ...
///
///         update
///     }
///
///     fn layout_style(&self) -> StyleNode {
/// #       StyleNode { style: Default::default(), children: vec![] }
///         // ... layout code ...
///     }
///
///     fn widget_id(&self) -> WidgetId {
///         WidgetId::new("myapp", "DropdownWidget")
///     }
/// }
/// ```
///
/// ## Container Widgets Must Propagate
///
/// If your widget has children, you **must** propagate `render_postfix()` to them:
///
/// ```rust,no_run
/// use nptk_core::widget::{Widget, BoxedWidget};
/// use nptk_core::vgi::Graphics;
/// use nptk_core::layout::{LayoutNode, StyleNode};
/// use nptk_core::app::{AppContext, AppInfo};
/// use nptk_theme::theme::Theme;
/// use nptk_core::app::update::Update;
/// use nptk_theme::id::WidgetId;
///
/// struct MyContainer {
///     children: Vec<BoxedWidget>,
/// }
///
/// impl Widget for MyContainer {
///     fn render(&mut self, graphics: &mut dyn Graphics, theme: &mut dyn Theme,
///               layout: &LayoutNode, info: &mut AppInfo, context: AppContext) {
///         // Render all children
///         for (i, child) in self.children.iter_mut().enumerate() {
///             child.render(graphics, theme, &layout.children[i], info, context.clone());
///         }
///     }
///
///     fn render_postfix(&mut self, graphics: &mut dyn Graphics, theme: &mut dyn Theme,
///                       layout: &LayoutNode, info: &mut AppInfo, context: AppContext) {
///         // IMPORTANT: Call render_postfix on all children!
///         // This ensures their overlays appear on top
///         for (i, child) in self.children.iter_mut().enumerate() {
///             child.render_postfix(graphics, theme, &layout.children[i], info, context.clone());
///         }
///     }
///
/// #   fn update(&mut self, _layout: &LayoutNode, _context: AppContext, _info: &mut AppInfo) -> Update {
/// #       Update::empty()
/// #   }
/// #   fn layout_style(&self) -> StyleNode {
/// #       StyleNode { style: Default::default(), children: vec![] }
/// #   }
/// #   fn widget_id(&self) -> WidgetId {
/// #       WidgetId::new("myapp", "MyContainer")
/// #   }
///     // ... other methods ...
/// }
/// ```
///
/// ## Real-World Examples
///
/// See these widgets for complete working examples:
/// - [`MenuButton`](nptk_widgets::menu_button::MenuButton) - Dropdown menu button
/// - [`MenuBar`](nptk_widgets::menubar::MenuBar) - Horizontal menu bar with popups
/// - [`Container`](nptk_widgets::container::Container) - Shows how to propagate postfix rendering
///
/// ## Benefits
///
/// - ✅ **Simple**: No separate overlay management system needed
/// - ✅ **Natural**: Z-ordering follows render order automatically
/// - ✅ **Integrated**: Full access to AppInfo, fonts, events, etc.
pub trait Widget {
    /// Render the widget to the canvas.
    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        theme: &mut dyn Theme,
        layout_node: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    );

    /// Render content that should appear after children (overlays, popups).
    ///
    /// This method is called after the widget's main content and all its children
    /// have been rendered, allowing the widget to draw content that should appear
    /// "on top" of everything else (e.g., dropdown menus, tooltips, context menus).
    ///
    /// The default implementation does nothing. Override this method if your widget
    /// needs to render overlays or popups.
    ///
    /// # Render Order
    ///
    /// 1. Widget's [`render()`](Widget::render) method is called
    /// 2. All children's `render()` methods are called recursively
    /// 3. Widget's `render_postfix()` method is called ← **Overlays go here!**
    /// 4. All children's `render_postfix()` methods are called recursively
    ///
    /// This ensures overlays always appear on top of the widget's main content and
    /// all its children, but still allow children to have their own overlays.
    ///
    /// # Important for Container Widgets
    ///
    /// If your widget has children, you **must** call `render_postfix()` on each child,
    /// otherwise their overlays won't be rendered. See the trait-level documentation
    /// for examples.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// fn render_postfix(&mut self, graphics: &mut dyn Graphics, theme: &mut dyn Theme,
    ///                    layout_node: &LayoutNode, info: &mut AppInfo,
    ///                    context: AppContext) {
    ///     if self.is_menu_open {
    ///         // Render popup menu on top of everything
    ///         self.popup.render(graphics, theme, &popup_layout, info, context);
    ///     }
    /// }
    /// ```
    ///
    /// # See Also
    ///
    /// - Trait-level documentation for detailed examples
    /// - [`MenuButton`](nptk_widgets::menu_button::MenuButton) for a complete implementation
    fn render_postfix(
        &mut self,
        _graphics: &mut dyn Graphics,
        _theme: &mut dyn Theme,
        _layout_node: &LayoutNode,
        _info: &mut AppInfo,
        _context: AppContext,
    ) {
        // Default: do nothing
    }

    /// Return the layout style node for layout computation.
    fn layout_style(&self) -> StyleNode;

    /// Update the widget state with given info and layout. Returns if the app should be updated.
    fn update(&mut self, layout: &LayoutNode, context: AppContext, info: &mut AppInfo) -> Update;

    /// Return the widget id.
    fn widget_id(&self) -> WidgetId;
}

/// An extension trait for widgets with a single child widget.
pub trait WidgetChildExt {
    /// Sets the child widget of the widget.
    fn set_child(&mut self, child: impl Widget + 'static);

    /// Sets the child widget of the widget and returns self.
    fn with_child(mut self, child: impl Widget + 'static) -> Self
    where
        Self: Sized,
    {
        self.set_child(child);
        self
    }
}

/// An extension trait for widgets with multiple child widgets.
pub trait WidgetChildrenExt {
    /// Sets the child widgets of the widget.
    fn set_children(&mut self, children: Vec<BoxedWidget>);

    /// Sets the child widgets of the widget and returns self.
    fn with_children(mut self, children: Vec<BoxedWidget>) -> Self
    where
        Self: Sized,
    {
        self.set_children(children);
        self
    }

    /// Adds a child widget to the widget.
    fn add_child(&mut self, child: impl Widget + 'static);

    /// Adds a child widget to the widget and returns self.
    fn with_child(mut self, child: impl Widget + 'static) -> Self
    where
        Self: Sized,
    {
        self.add_child(child);
        self
    }
}

/// An extension trait for widgets with a layout style.
pub trait WidgetLayoutExt {
    /// Sets the layout style of the widget.
    fn set_layout_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>);

    /// Sets the layout style of the widget and returns self.
    fn with_layout_style(mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) -> Self
    where
        Self: Sized,
    {
        self.set_layout_style(layout_style);
        self
    }
}
