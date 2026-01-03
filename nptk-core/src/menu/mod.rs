//! Unified menu system
//!
//! This module provides a unified menu system that works for both menubar menus
//! and context menus, allowing menu items to be shared between different contexts.

pub mod commands;
pub mod manager;
pub mod render;
pub mod templates;
pub mod unified;

// Re-export core types
pub use commands::MenuCommand;
pub use manager::MenuManager;
pub use render::{render_menu, calculate_menu_size, MenuGeometry};
pub use templates::{init_edit_commands, init_view_menu, merge_menus};
pub use unified::{MenuItem, MenuTemplate, MenuContext, ViewMode};

// Legacy context menu system (deprecated, kept for backward compatibility)
pub use legacy::*;

mod legacy {
    // Re-export the old context menu types for backward compatibility
    // These will be deprecated in a future version
    pub use super::legacy_impl::*;
}

mod legacy_impl {
    use std::sync::{Arc, Mutex};
    use vello::kurbo::Point;

    const ITEM_HEIGHT: f64 = 24.0;
    const PADDING: f64 = 4.0;
    const MIN_WIDTH: f64 = 120.0;
    const MAX_WIDTH: f64 = 400.0;

    /// A context menu containing a list of items.
    /// 
    /// # Deprecated
    /// 
    /// This type is deprecated. Use `MenuTemplate` from the unified menu system instead.
    #[deprecated(note = "Use MenuTemplate from unified menu system instead")]
    #[derive(Clone)]
    pub struct ContextMenu {
        /// Legacy flat items; treated as a single group when `groups` is None.
        pub items: Vec<ContextMenuItem>,
        /// Optional grouped items; when present, separators are auto-inserted between groups.
        pub groups: Option<Vec<ContextMenuGroup>>,
    }

    impl ContextMenu {
        pub fn new(items: Vec<ContextMenuItem>) -> Self {
            Self {
                items,
                groups: None,
            }
        }
    }

    /// # Deprecated
    /// 
    /// This type is deprecated. Use `MenuItem` from the unified menu system instead.
    #[deprecated(note = "Use MenuItem from unified menu system instead")]
    #[derive(Clone)]
    pub struct ContextMenuGroup {
        pub items: Vec<ContextMenuItem>,
    }

    /// An item in a context menu.
    /// 
    /// # Deprecated
    /// 
    /// This type is deprecated. Use `MenuItem` from the unified menu system instead.
    #[deprecated(note = "Use MenuItem from unified menu system instead")]
    #[derive(Clone)]
    pub enum ContextMenuItem {
        /// A clickable action item.
        Action {
            label: String,
            action: Arc<dyn Fn() + Send + Sync>,
        },
        /// A visual separator.
        Separator,
        /// A submenu.
        SubMenu {
            label: String,
            items: Vec<ContextMenuItem>,
        },
    }

    /// Result of a menu hit-test click.
    pub enum MenuClickResult {
        Action(Arc<dyn Fn() + Send + Sync>),
        SubMenu(ContextMenu, Point),
        NonActionInside,
    }

    use vello::kurbo::Rect;
    use crate::app::font_ctx::FontContext;
    use crate::text_render::TextRenderContext;

    struct MenuGeometry {
        items: Vec<ContextMenuItem>,
        rect: Rect,
    }

    impl MenuGeometry {
        fn new(
            menu: &ContextMenu,
            position: Point,
            text_render: &mut TextRenderContext,
            font_cx: &mut FontContext,
        ) -> Self {
            let items = flatten_menu_items(menu);
            let (width, height) = calculate_layout_from_items(&items, text_render, font_cx);
            let rect = Rect::new(
                position.x as f64,
                position.y as f64,
                position.x as f64 + width,
                position.y as f64 + height,
            );
            Self { items, rect }
        }

        fn hit_test_index(&self, cursor: Point) -> Option<usize> {
            if !self.rect.contains(cursor) {
                return None;
            }
            let relative_y = cursor.y - self.rect.y0 - PADDING;
            if relative_y < 0.0 {
                return None;
            }
            let idx = (relative_y / ITEM_HEIGHT) as usize;
            if idx < self.items.len() {
                Some(idx)
            } else {
                None
            }
        }

        fn item_rect(&self, index: usize) -> Rect {
            let y = self.rect.y0 + PADDING + (index as f64 * ITEM_HEIGHT);
            Rect::new(self.rect.x0, y, self.rect.x1, y + ITEM_HEIGHT)
        }

        fn submenu_origin(&self, index: usize) -> Point {
            let item_top = self.rect.y0 + PADDING + (index as f64 * ITEM_HEIGHT);
            Point::new(self.rect.x1 + 8.0, item_top)
        }
    }

    fn flatten_menu_items(menu: &ContextMenu) -> Vec<ContextMenuItem> {
        if let Some(groups) = &menu.groups {
            let mut out = Vec::new();
            for (i, group) in groups.iter().enumerate() {
                if i > 0 {
                    out.push(ContextMenuItem::Separator);
                }
                out.extend(group.items.clone());
            }
            out
        } else {
            menu.items.clone()
        }
    }

    /// Manages the state of the active context menu.
    /// 
    /// # Deprecated
    /// 
    /// This type is deprecated. The unified menu system handles context menus differently.
    /// Use `MenuManager` for command routing and manage context menu state separately.
    #[deprecated(note = "Use MenuManager from unified menu system instead")]
    #[derive(Clone, Default)]
    pub struct ContextMenuManager {
        state: Arc<Mutex<ContextMenuState>>,
    }

    #[derive(Default)]
    struct ContextMenuState {
        stack: Vec<(ContextMenu, Point)>,
    }

    impl ContextMenuManager {
        pub fn new() -> Self {
            Self {
                state: Arc::new(Mutex::new(ContextMenuState::default())),
            }
        }

        pub fn show_context_menu(&self, menu: ContextMenu, position: Point) {
            let mut state = self.state.lock().unwrap();
            state.stack.clear();
            state.stack.push((menu, position));
        }

        pub fn push_submenu(&self, menu: ContextMenu, position: Point) {
            let mut state = self.state.lock().unwrap();
            state.stack.push((menu, position));
        }

        pub fn set_stack(&self, stack: Vec<(ContextMenu, Point)>) {
            let mut state = self.state.lock().unwrap();
            state.stack = stack;
        }

        pub fn close_context_menu(&self) {
            let mut state = self.state.lock().unwrap();
            state.stack.clear();
        }

        pub fn get_active_menu(&self) -> Option<(ContextMenu, Point)> {
            let state = self.state.lock().unwrap();
            state.stack.last().cloned()
        }

        pub fn get_menu_stack(&self) -> Vec<(ContextMenu, Point)> {
            let state = self.state.lock().unwrap();
            state.stack.clone()
        }

        pub fn is_open(&self) -> bool {
            !self.state.lock().unwrap().stack.is_empty()
        }
    }

    use crate::vgi::Graphics;
    use nptk_theme::theme::Theme;
    use vello::kurbo::{Affine, Rect as KurboRect};
    use vello::peniko::{Brush, Color};

    use crate::vgi::shape_to_path;
    use nptk_theme::id::WidgetId;
    use nptk_theme::properties::ThemeProperty;
    use vello::kurbo::{RoundedRect, RoundedRectRadii};

    /// Renders the context menu.
    /// Returns the bounds of the rendered menu for hit testing.
    /// 
    /// # Deprecated
    /// 
    /// This function is deprecated. Use `render_menu` from the unified menu system instead.
    #[deprecated(note = "Use render_menu from unified menu system instead")]
    pub fn render_context_menu(
        graphics: &mut dyn Graphics,
        menu: &ContextMenu,
        position: Point,
        theme: &mut dyn Theme,
        text_render: &mut TextRenderContext,
        font_cx: &mut FontContext,
        cursor_pos: Option<Point>,
    ) -> Rect {
        let geometry = MenuGeometry::new(menu, position, text_render, font_cx);
        let rect = geometry.rect;

        let menu_id = WidgetId::new("nptk-widgets", "MenuPopup");

        // Colors
        let bg_color = theme
            .get_property(menu_id.clone(), &ThemeProperty::ColorBackground)
            .unwrap_or(Color::from_rgb8(255, 255, 255));
        let border_color = theme
            .get_property(menu_id.clone(), &ThemeProperty::ColorBorder)
            .unwrap_or(Color::from_rgb8(200, 200, 200));
        let text_color = theme
            .get_property(menu_id.clone(), &ThemeProperty::ColorText)
            .unwrap_or(Color::from_rgb8(0, 0, 0));
        let hovered_color = theme
            .get_property(menu_id.clone(), &ThemeProperty::ColorMenuHovered)
            .unwrap_or(Color::from_rgb8(230, 230, 230));

        // Shadow
        let shadow_rect = RoundedRect::new(
            rect.x0 + 2.0,
            rect.y0 + 2.0,
            rect.x1 + 2.0,
            rect.y1 + 2.0,
            RoundedRectRadii::new(4.0, 4.0, 4.0, 4.0),
        );
        graphics.fill(
            vello::peniko::Fill::NonZero,
            Affine::IDENTITY,
            &Brush::Solid(Color::new([0.0, 0.0, 0.0, 0.2])),
            None,
            &shape_to_path(&shadow_rect),
        );

        // Main background
        let rounded_rect = RoundedRect::new(
            rect.x0,
            rect.y0,
            rect.x1,
            rect.y1,
            RoundedRectRadii::new(4.0, 4.0, 4.0, 4.0),
        );
        graphics.fill(
            vello::peniko::Fill::NonZero,
            Affine::IDENTITY,
            &Brush::Solid(bg_color),
            None,
            &shape_to_path(&rounded_rect),
        );
        graphics.stroke(
            &vello::kurbo::Stroke::new(1.0),
            Affine::IDENTITY,
            &Brush::Solid(border_color),
            None,
            &shape_to_path(&rounded_rect),
        );

        // Draw items
        let mut current_y = rect.y0 + PADDING;

        // Determine hovered item index
        let hovered_index = cursor_pos.and_then(|cursor| geometry.hit_test_index(cursor));

        for (i, item) in geometry.items.iter().enumerate() {
            let item_rect = geometry.item_rect(i);

            // Draw hover background
            if Some(i) == hovered_index {
                match item {
                    ContextMenuItem::Separator => {},
                    _ => {
                        let item_rounded = RoundedRect::new(
                            item_rect.x0 + 2.0,
                            item_rect.y0,
                            item_rect.x1 - 2.0,
                            item_rect.y1,
                            RoundedRectRadii::new(2.0, 2.0, 2.0, 2.0),
                        );
                        graphics.fill(
                            vello::peniko::Fill::NonZero,
                            Affine::IDENTITY,
                            &Brush::Solid(hovered_color),
                            None,
                            &shape_to_path(&item_rounded),
                        );
                    },
                }
            }

            match item {
                ContextMenuItem::Action { label, .. } => {
                    text_render.render_text(
                        font_cx,
                        graphics,
                        label,
                        None,
                        14.0,
                        Brush::Solid(text_color),
                        Affine::translate((rect.x0 + 10.0, current_y + 4.0)),
                        true,
                        Some((rect.width() - 20.0) as f32),
                    );
                },
                ContextMenuItem::Separator => {
                    let sep_y = current_y + ITEM_HEIGHT / 2.0;
                    let line =
                        vello::kurbo::Line::new((rect.x0 + 8.0, sep_y), (rect.x1 - 8.0, sep_y));
                    graphics.stroke(
                        &vello::kurbo::Stroke::new(1.0),
                        Affine::IDENTITY,
                        &Brush::Solid(Color::from_rgb8(200, 200, 200)),
                        None,
                        &shape_to_path(&line),
                    );
                },
                ContextMenuItem::SubMenu { label, .. } => {
                    text_render.render_text(
                        font_cx,
                        graphics,
                        label,
                        None,
                        14.0,
                        Brush::Solid(text_color),
                        Affine::translate((rect.x0 + 10.0, current_y + 4.0)),
                        true,
                        Some((rect.width() - 30.0) as f32),
                    );

                    let arrow_x = rect.x1 - 12.0;
                    let arrow_y = current_y + (ITEM_HEIGHT / 2.0);
                    let arrow_size = 3.0;
                    let arrow_stroke = vello::kurbo::Stroke::new(1.0);

                    graphics.stroke(
                        &arrow_stroke,
                        Affine::IDENTITY,
                        &Brush::Solid(Color::from_rgb8(100, 100, 100)),
                        None,
                        &shape_to_path(&vello::kurbo::Line::new(
                            Point::new(arrow_x - arrow_size, arrow_y - arrow_size),
                            Point::new(arrow_x, arrow_y),
                        )),
                    );
                    graphics.stroke(
                        &arrow_stroke,
                        Affine::IDENTITY,
                        &Brush::Solid(Color::from_rgb8(100, 100, 100)),
                        None,
                        &shape_to_path(&vello::kurbo::Line::new(
                            Point::new(arrow_x, arrow_y),
                            Point::new(arrow_x - arrow_size, arrow_y + arrow_size),
                        )),
                    );
                },
            }
            current_y += ITEM_HEIGHT;
        }

        rect
    }

    fn calculate_layout_from_items(
        items: &[ContextMenuItem],
        text_render: &mut TextRenderContext,
        font_cx: &mut FontContext,
    ) -> (f64, f64) {
        let mut max_text_width: f64 = 0.0;
        for item in items {
            if let ContextMenuItem::Action { label, .. } | ContextMenuItem::SubMenu { label, .. } = item
            {
                let (text_width, _) = text_render.measure_text_layout(font_cx, label, None, 14.0, None);
                max_text_width = max_text_width.max(text_width as f64);
            }
        }
        let estimated = (max_text_width + 40.0).max(MIN_WIDTH);
        let width = estimated.min(MAX_WIDTH);
        let height = items.len() as f64 * ITEM_HEIGHT + PADDING * 2.0;
        (width, height)
    }

    /// Get the menu rectangle for layout calculation.
    /// 
    /// # Deprecated
    /// 
    /// This function is deprecated. Use `calculate_menu_size` from the unified menu system instead.
    #[deprecated(note = "Use calculate_menu_size from unified menu system instead")]
    pub fn get_menu_rect(
        menu: &ContextMenu,
        position: Point,
        text_render: &mut TextRenderContext,
        font_cx: &mut FontContext,
    ) -> Rect {
        MenuGeometry::new(menu, position, text_render, font_cx).rect
    }

    /// Handle a click on a menu.
    /// 
    /// # Deprecated
    /// 
    /// This function is deprecated. Use `MenuManager::handle_command` from the unified menu system instead.
    #[deprecated(note = "Use MenuManager::handle_command from unified menu system instead")]
    pub fn handle_click(
        menu: &ContextMenu,
        position: Point,
        cursor: Point,
        text_render: &mut TextRenderContext,
        font_cx: &mut FontContext,
    ) -> Option<MenuClickResult> {
        let geometry = MenuGeometry::new(menu, position, text_render, font_cx);
        let index = geometry.hit_test_index(cursor)?;

        let submenu_origin = geometry.submenu_origin(index);

        match &geometry.items[index] {
            ContextMenuItem::Action { action, .. } => Some(MenuClickResult::Action(action.clone())),
            ContextMenuItem::SubMenu { items, .. } => Some(MenuClickResult::SubMenu(
                ContextMenu {
                    items: items.clone(),
                    groups: None,
                },
                submenu_origin,
            )),
            _ => Some(MenuClickResult::NonActionInside),
        }
    }

    /// Hover helper: if cursor is over a submenu item, return its submenu and origin.
    /// 
    /// # Deprecated
    /// 
    /// This function is deprecated. Submenu handling is now part of the unified renderer.
    #[deprecated(note = "Submenu handling is now part of the unified renderer")]
    pub fn hover_submenu(
        menu: &ContextMenu,
        position: Point,
        cursor: Point,
        text_render: &mut TextRenderContext,
        font_cx: &mut FontContext,
    ) -> Option<(ContextMenu, Point)> {
        let geometry = MenuGeometry::new(menu, position, text_render, font_cx);
        let index = geometry.hit_test_index(cursor)?;

        if let ContextMenuItem::SubMenu { items, .. } = &geometry.items[index] {
            let submenu_origin = geometry.submenu_origin(index);
            return Some((
                ContextMenu {
                    items: items.clone(),
                    groups: None,
                },
                submenu_origin,
            ));
        }
        None
    }
}
