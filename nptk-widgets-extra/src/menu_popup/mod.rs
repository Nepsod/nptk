// SPDX-License-Identifier: LGPL-3.0-only

//! Menu popup widget for dropdown menus
//!
//! This widget uses the unified menu system based on MenuTemplate.

mod constants;
mod theme;

pub use constants::*;
pub use theme::*;

use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{Dimension, LayoutNode, LayoutStyle, LengthPercentage, StyleNode};
use nptk_core::menu::unified::MenuTemplate;
use nptk_core::menu::render::{render_menu, calculate_menu_size, MenuGeometry};
use nptk_core::menu::manager::MenuManager;
use nptk_core::signal::MaybeSignal;
use nptk_core::text_render::TextRenderContext;
use nptk_core::vg::kurbo::Point;
use nptk_core::widget::{Widget, WidgetLayoutExt};
use nptk_core::window::{ElementState, MouseButton};
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;
use nptk_core::vgi::Graphics;
use std::sync::Arc;

/// A popup menu widget that displays a menu template
pub struct MenuPopup {
    /// The menu template to display
    template: MenuTemplate,
    /// Menu manager for command routing
    menu_manager: Option<MenuManager>,
    /// Layout style for the popup
    layout_style: MaybeSignal<LayoutStyle>,
    /// Currently hovered item index
    hovered_index: Option<usize>,
    /// Text rendering context
    text_render_context: TextRenderContext,
    /// Callback to execute when the popup should be closed
    on_close: Option<Arc<dyn Fn() -> Update + Send + Sync>>,

    // Submenu support
    /// Currently open child popup
    child_popup: Option<Box<MenuPopup>>,
    /// Index of the item that opened the child popup
    open_item_index: Option<usize>,
}

impl MenuPopup {
    /// Create a new menu popup with a template
    pub fn new(template: MenuTemplate) -> Self {
        Self {
            template,
            menu_manager: None,
            layout_style: LayoutStyle {
                size: nalgebra::Vector2::new(
                    Dimension::length(200.0),
                    Dimension::length(100.0),
                ),
                padding: nptk_core::layout::Rect::<LengthPercentage> {
                    left: LengthPercentage::length(0.0),
                    right: LengthPercentage::length(0.0),
                    top: LengthPercentage::length(ITEM_TOP_PADDING as f32),
                    bottom: LengthPercentage::length(ITEM_TOP_PADDING as f32),
                },
                ..Default::default()
            }
            .into(),
            hovered_index: None,
            text_render_context: TextRenderContext::new(),
            on_close: None,
            child_popup: None,
            open_item_index: None,
        }
    }

    /// Set the menu manager for command routing
    pub fn with_menu_manager(mut self, manager: MenuManager) -> Self {
        self.menu_manager = Some(manager);
        self
    }

    /// Set the layout style
    pub fn with_layout_style(mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) -> Self {
        self.layout_style = layout_style.into();
        self
    }

    /// Set callback for when the popup should be closed
    pub fn with_on_close<F>(mut self, callback: F) -> Self
    where
        F: Fn() -> Update + Send + Sync + 'static,
    {
        self.on_close = Some(Arc::new(callback));
        self
    }

    /// Calculate the size needed for the popup
    /// 
    /// Note: For accurate sizing with actual font context, use the version that takes font contexts.
    /// This method uses temporary contexts and may not match rendered size exactly.
    pub fn calculate_size(&self) -> (f64, f64) {
        // Create temporary contexts for measurement
        // In a real scenario, we'd pass this from the widget tree
        let mut temp_text_render = TextRenderContext::new();
        let mut font_ctx = nptk_core::app::font_ctx::FontContext::new();
        calculate_menu_size(&self.template.items, &mut temp_text_render, &mut font_ctx)
    }

    /// Calculate the size needed for the popup using provided font contexts
    /// 
    /// This is the preferred method as it uses actual font context from the widget tree.
    pub fn calculate_size_with_contexts(&self, text_render: &mut TextRenderContext, font_cx: &mut nptk_core::app::font_ctx::FontContext) -> (f64, f64) {
        calculate_menu_size(&self.template.items, text_render, font_cx)
    }

    /// Get the menu template
    pub fn template(&self) -> &MenuTemplate {
        &self.template
    }
}

impl WidgetLayoutExt for MenuPopup {
    fn set_layout_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.layout_style = layout_style.into();
    }
}

impl Widget for MenuPopup {
    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "MenuPopup")
    }

    fn layout_style(&self) -> StyleNode {
        // Calculate size using temporary contexts (layout_style is called before we have font context)
        // Actual rendering will use the app's font context for more accurate sizing
        let (width, height) = self.calculate_size();
        let mut style = self.layout_style.get().clone();

        style.size = nalgebra::Vector2::new(
            Dimension::length(width as f32),
            Dimension::length(height as f32),
        );

        StyleNode {
            style,
            children: Vec::new(),
        }
    }

    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        theme: &mut dyn Theme,
        layout: &LayoutNode,
        info: &mut AppInfo,
        _context: AppContext,
    ) {
        let popup_position = Point::new(
            layout.layout.location.x as f64,
            layout.layout.location.y as f64,
        );

        let cursor_pos = info.cursor_pos.map(|p| Point::new(p.x, p.y));

        render_menu(
            graphics,
            &self.template,
            popup_position,
            theme,
            &mut self.text_render_context,
            &mut info.font_context,
            cursor_pos,
            self.hovered_index,
        );

        // Render child popup if open
        if let Some(ref mut child) = self.child_popup {
            if let Some(open_index) = self.open_item_index {
                if let Some(_submenu_template) = self.template.items.get(open_index)
                    .and_then(|item| item.submenu.as_ref())
                {
                    // Calculate submenu position using geometry helper
                    let geometry = MenuGeometry::new(
                        &self.template,
                        popup_position,
                        &mut self.text_render_context,
                        &mut info.font_context,
                    );
                    let child_position = geometry.submenu_origin(open_index);
                    
                    let (child_width, child_height) = child.calculate_size_with_contexts(
                        &mut self.text_render_context,
                        &mut info.font_context,
                    );
                    
                    let mut child_layout_struct = nptk_core::layout::Layout::default();
                    child_layout_struct.location.x = child_position.x as f32;
                    child_layout_struct.location.y = child_position.y as f32;
                    child_layout_struct.size.width = child_width as f32;
                    child_layout_struct.size.height = child_height as f32;
                    let child_layout = LayoutNode {
                        layout: child_layout_struct,
                        children: Vec::new(),
                    };
                    child.render(graphics, theme, &child_layout, info, _context);
                }
            }
        }
    }

    fn update(&mut self, layout: &LayoutNode, context: AppContext, info: &mut AppInfo) -> Update {
        let mut update = Update::empty();

        let popup_position = Point::new(
            layout.layout.location.x as f64,
            layout.layout.location.y as f64,
        );
        let cursor_pos = info.cursor_pos.map(|p| Point::new(p.x, p.y));

        // Calculate hover state
        let geometry = MenuGeometry::new(
            &self.template,
            popup_position,
            &mut self.text_render_context,
            &mut info.font_context,
        );

        let old_hovered = self.hovered_index;
        self.hovered_index = cursor_pos.and_then(|cursor| geometry.hit_test_index(cursor));

        if old_hovered != self.hovered_index {
            update |= Update::DRAW;

            // Trigger action callbacks for hover state changes
            if let Some(old_idx) = old_hovered {
                if let Some(old_item) = self.template.items.get(old_idx) {
                    context.action_callbacks.trigger_leave(old_item.id);
                }
            }
            if let Some(new_idx) = self.hovered_index {
                if let Some(new_item) = self.template.items.get(new_idx) {
                    context.action_callbacks.trigger_enter(new_item.id);
                }
            }

            // Handle submenu opening/closing
            if let Some(hovered) = self.hovered_index {
                if self.open_item_index != Some(hovered) {
                    if let Some(item) = self.template.items.get(hovered) {
                        if item.has_submenu() {
                            // Open new submenu
                            if let Some(submenu_template) = item.submenu.clone() {
                                let child = MenuPopup::new(submenu_template);
                                // Note: MenuManager doesn't implement Clone, so we can't share it
                                // In practice, MenuManager should be shared via Arc or similar
                                // Actions are stored in MenuItem.action, so child popup doesn't need manager
                                self.child_popup = Some(Box::new(child));
                                self.open_item_index = Some(hovered);
                                update |= Update::DRAW;
                            }
                        } else {
                            // Close submenu if hovering over leaf item
                            if self.child_popup.is_some() {
                                self.child_popup = None;
                                self.open_item_index = None;
                                update |= Update::DRAW;
                            }
                        }
                    }
                }
            }
        }

        // Update child popup
        if let Some(ref mut child) = self.child_popup {
            if let Some(open_index) = self.open_item_index {
                // Calculate submenu position using geometry helper for consistency
                let geometry = MenuGeometry::new(
                    &self.template,
                    popup_position,
                    &mut self.text_render_context,
                    &mut info.font_context,
                );
                let child_position = geometry.submenu_origin(open_index);
                
                let (child_width, child_height) = child.calculate_size_with_contexts(
                    &mut self.text_render_context,
                    &mut info.font_context,
                );
                
                let mut child_layout_struct = nptk_core::layout::Layout::default();
                child_layout_struct.location.x = child_position.x as f32;
                child_layout_struct.location.y = child_position.y as f32;
                child_layout_struct.size.width = child_width as f32;
                child_layout_struct.size.height = child_height as f32;
                let child_layout = LayoutNode {
                    layout: child_layout_struct,
                    children: Vec::new(),
                };
                update |= child.update(&child_layout, context.clone(), info);
            }
        }

        // Handle mouse clicks
        for (_, button, state) in &info.buttons {
            if *button == MouseButton::Left && *state == ElementState::Pressed {
                if let Some(hovered) = self.hovered_index {
                    if let Some(item) = self.template.items.get(hovered) {
                        if item.enabled && !item.is_separator() {
                            if item.has_submenu() {
                                // Already handled by hover
                                update |= Update::DRAW;
                            } else {
                                // Execute command
                                if let Some(ref manager) = self.menu_manager {
                                    update |= manager.handle_command(item.id);
                                } else if let Some(ref action) = item.action {
                                    update |= action();
                                }

                                // Close popup
                                if let Some(ref on_close) = self.on_close {
                                    update |= on_close();
                                } else {
                                    update |= Update::DRAW;
                                }
                            }
                        }
                    }
                } else {
                    // Click outside - close popup
                    if let Some(ref on_close) = self.on_close {
                        update |= on_close();
                    } else {
                        update |= Update::DRAW;
                    }
                }
            }
        }

        update
    }
}
