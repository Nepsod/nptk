// SPDX-License-Identifier: LGPL-3.0-only
//! Sidebar widget for navigation and organization.
//!
//! A flexible sidebar widget that accepts customizable content sections,
//! tracks item selection, and provides click callbacks. Supports icons,
//! labels, and expandable sections.

use nptk_widgets::icon::Icon;
use nptk_widgets::text::Text;
use nalgebra::Vector2;
use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{Dimension, Layout, LayoutContext, LayoutNode, LayoutStyle, StyleNode};
use nptk_core::signal::{state::StateSignal, MaybeSignal, Signal};
use nptk_core::vg::kurbo::{Affine, Rect, Shape, Vec2};
use nptk_core::vg::peniko::{Brush, Color, Fill};
use nptk_core::vgi::Graphics;
use nptk_core::widget::{Widget, WidgetLayoutExt};
use nptk_core::window::{ElementState, MouseButton};
use nptk_core::theme::{ColorRole, Palette};
use std::sync::Arc;
use async_trait::async_trait;

// Constants
const HEADER_HEIGHT: f32 = 26.0;
const DEFAULT_FONT_SIZE: f32 = 14.0;
const ICON_SIZE: u32 = 24;
const PATH_ROUNDING: f64 = 0.1;

/// Rendering context for sidebar items and sections.
struct RenderContext {
    item_height: f32,
    icon_size: u32,
    padding: f32,
    selected_id: Option<String>,
    hovered_id: Option<String>,
}

/// A single item in the sidebar.
#[derive(Debug, Clone)]
pub struct SidebarItem {
    /// Unique identifier for the item.
    pub id: String,
    /// Display text for the item.
    pub label: String,
    /// Optional icon name for the item.
    pub icon: Option<String>,
    /// Optional URI/path for navigation.
    pub uri: Option<String>,
    /// Optional user data attached to the item.
    pub data: Option<Arc<dyn std::any::Any + Send + Sync>>,
}

impl SidebarItem {
    /// Create a new sidebar item.
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            uri: None,
            data: None,
        }
    }

    /// Set an optional icon for the item.
    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set an optional URI for the item.
    pub fn with_uri(mut self, uri: impl Into<String>) -> Self {
        self.uri = Some(uri.into());
        self
    }

    /// Set optional user data for the item.
    pub fn with_data(mut self, data: Arc<dyn std::any::Any + Send + Sync>) -> Self {
        self.data = Some(data);
        self
    }
}

/// A section in the sidebar containing multiple items.
#[derive(Debug, Clone)]
pub struct SidebarSection {
    /// Section header title.
    pub title: String,
    /// Optional section icon.
    pub icon: Option<String>,
    /// Items in this section.
    pub items: Vec<SidebarItem>,
    /// Whether section is expanded (if expandable).
    pub expanded: bool,
    /// Whether section can be expanded/collapsed.
    pub expandable: bool,
}

impl SidebarSection {
    /// Create a new sidebar section.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            icon: None,
            items: Vec::new(),
            expanded: true,
            expandable: false,
        }
    }

    /// Set an optional icon for the section.
    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Add an item to the section.
    pub fn with_item(mut self, item: SidebarItem) -> Self {
        self.items.push(item);
        self
    }

    /// Add multiple items to the section.
    pub fn with_items(mut self, items: Vec<SidebarItem>) -> Self {
        self.items.extend(items);
        self
    }

    /// Set whether the section is expandable.
    pub fn with_expandable(mut self, expandable: bool) -> Self {
        self.expandable = expandable;
        self
    }

    /// Set the initial expanded state.
    pub fn with_expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }
}

/// A sidebar widget for navigation and organization.
///
/// ### Theming
/// The sidebar can be styled with:
/// - `color_background` - Background color of the sidebar
/// - `color_text` - Text color for items
/// - `color_hovered` - Background color when hovering over items
/// - `color_selected` - Background color for selected items
pub struct Sidebar {
    /// List of sections in the sidebar.
    sections: Vec<SidebarSection>,
    /// Currently selected item ID.
    selected_id: StateSignal<Option<String>>,
    /// Callback when an item is selected.
    on_item_selected: Option<Arc<dyn Fn(&SidebarItem) -> Update + Send + Sync>>,
    /// Height of each item in pixels.
    item_height: f32,
    /// Size of icons in pixels.
    icon_size: u32,
    /// Padding around items in pixels.
    padding: f32,
    /// Currently hovered item ID.
    hovered_id: Option<String>,
    /// Expanded state for each section (by section index).
    section_expanded: Vec<StateSignal<bool>>,
    /// Layout style.
    layout_style: MaybeSignal<LayoutStyle>,
}

impl Sidebar {
    /// Create a new empty sidebar.
    pub fn new() -> Self {
        Self {
            sections: Vec::new(),
            selected_id: StateSignal::new(None),
            on_item_selected: None,
            item_height: HEADER_HEIGHT,
            icon_size: ICON_SIZE,
            padding: 4.0,
            hovered_id: None,
            section_expanded: Vec::new(),
            layout_style: LayoutStyle {
                size: Vector2::new(
                    Dimension::length(200.0), // Fixed width - typical sidebar width
                    Dimension::percent(1.0), // Full height - responsive vertically
                ),
                min_size: Vector2::new(
                    Dimension::length(150.0), // Minimum width to prevent too narrow sidebar
                    Dimension::auto(), // No minimum height constraint
                ),
                flex_direction: nptk_core::layout::FlexDirection::Column,
                flex_shrink: 0.0, // Prevent sidebar from shrinking below minimum width
                ..Default::default()
            }
            .into(),
        }
    }

    fn apply_with(mut self, f: impl FnOnce(&mut Self)) -> Self {
        f(&mut self);
        self
    }

    /// Add a section to the sidebar.
    pub fn with_section(mut self, section: SidebarSection) -> Self {
        self.section_expanded.push(StateSignal::new(section.expanded));
        self.sections.push(section);
        self
    }

    /// Add multiple sections to the sidebar.
    pub fn with_sections(mut self, sections: Vec<SidebarSection>) -> Self {
        for section in &sections {
            self.section_expanded.push(StateSignal::new(section.expanded));
        }
        self.sections.extend(sections);
        self
    }

    /// Set the initially selected item.
    pub fn with_selected(self, id: impl Into<String>) -> Self {
        self.apply_with(|s| s.selected_id.set(Some(id.into())))
    }

    /// Set the callback when an item is selected.
    pub fn with_on_item_selected<F>(self, callback: F) -> Self
    where
        F: Fn(&SidebarItem) -> Update + Send + Sync + 'static,
    {
        self.apply_with(|s| s.on_item_selected = Some(Arc::new(callback)))
    }

    /// Set the height of each item.
    pub fn with_item_height(self, height: f32) -> Self {
        self.apply_with(|s| s.item_height = height)
    }

    /// Set the icon size.
    pub fn with_icon_size(self, size: u32) -> Self {
        self.apply_with(|s| s.icon_size = size)
    }

    /// Set the padding around items.
    pub fn with_padding(self, padding: f32) -> Self {
        self.apply_with(|s| s.padding = padding)
    }

    /// Get the currently selected item ID.
    pub fn selected_id(&self) -> Option<String> {
        self.selected_id.get().clone()
    }

    /// Set the selected item programmatically.
    pub fn set_selected(&mut self, id: Option<String>) {
        self.selected_id.set(id);
    }

    /// Find an item by its ID across all sections.
    fn find_item_by_id(&self, id: &str) -> Option<&SidebarItem> {
        for section in &self.sections {
            for item in &section.items {
                if item.id == id {
                    return Some(item);
                }
            }
        }
        None
    }

    // Theme helper functions
    fn get_background_color(palette: &Palette) -> Color {
        palette.color(ColorRole::Base)
    }

    fn get_text_color(palette: &Palette) -> Color {
        palette.color(ColorRole::BaseText)
    }

    fn get_selected_color(palette: &Palette) -> Color {
        palette.color(ColorRole::Selection)
    }

    fn get_hovered_color(palette: &Palette) -> Color {
        palette.color(ColorRole::HoverHighlight)
    }

    // Rendering helper functions
    fn render_icon(
        graphics: &mut dyn Graphics,
        info: &mut AppInfo,
        context: AppContext,
        icon_name: &str,
        icon_size: u32,
        x: f32,
        y: f32,
    ) {
        if let Some(scene) = graphics.as_scene_mut() {
            let mut icon_scene = nptk_core::vg::Scene::new();
            let mut icon_gfx = nptk_core::vgi::vello_vg::VelloGraphics::new(&mut icon_scene);
            let icon_size_f = icon_size as f32;
            let mut icon_layout_struct = Layout::default();
            // Set location and size for the icon
            icon_layout_struct.location.x = 0.0;
            icon_layout_struct.location.y = 0.0;
            icon_layout_struct.size.width = icon_size_f;
            icon_layout_struct.size.height = icon_size_f;
            let icon_layout = LayoutNode {
                layout: icon_layout_struct,
                children: vec![],
            };
            let mut icon_widget = Icon::new(icon_name.to_string(), icon_size, None);
            icon_widget.render(&mut icon_gfx, &icon_layout, info, context);
            let icon_transform = Affine::translate(Vec2::new(x as f64, y as f64));
            scene.append(&icon_scene, Some(icon_transform));
        }
    }

    fn render_text(
        graphics: &mut dyn Graphics,
        info: &mut AppInfo,
        context: AppContext,
        text: &str,
        font_size: f32,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) {
        if width > 0.0 {
            if let Some(scene) = graphics.as_scene_mut() {
                let mut text_scene = nptk_core::vg::Scene::new();
                let mut text_gfx = nptk_core::vgi::vello_vg::VelloGraphics::new(&mut text_scene);
                let text_layout = LayoutNode {
                    layout: nptk_core::layout::Layout::default(),
                    children: vec![],
                };
                let mut text_widget = Text::new(text.to_string())
                    .with_font_size(font_size)
                    .with_layout_style(LayoutStyle {
                        size: Vector2::new(
                            Dimension::length(width.max(0.0)),
                            Dimension::length(height),
                        ),
                        ..Default::default()
                    });
                text_widget.render(&mut text_gfx, &text_layout, info, context);
                let text_transform = Affine::translate(Vec2::new(x as f64, y as f64));
                scene.append(&text_scene, Some(text_transform));
            }
        }
    }

    fn render_background(graphics: &mut dyn Graphics, rect: Rect, color: Color) {
        graphics.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            &Brush::Solid(color),
            None,
            &rect.to_path(PATH_ROUNDING),
        );
    }

    fn get_indicator_icon(expanded: bool) -> &'static str {
        if expanded {
            "pan-down-symbolic"
        } else {
            "pan-end-symbolic"
        }
    }

    /// Render a single sidebar item.
    fn render_item(
        ctx: &RenderContext,
        graphics: &mut dyn Graphics,
        palette: &Palette,
        layout: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
        item: &SidebarItem,
        y_offset: f32,
    ) {
        let item_rect = Rect::new(
            layout.layout.location.x as f64,
            (layout.layout.location.y + y_offset) as f64,
            (layout.layout.location.x + layout.layout.size.width) as f64,
            (layout.layout.location.y + y_offset + ctx.item_height) as f64,
        );

        let is_selected = ctx.selected_id.as_ref() == Some(&item.id);
        let is_hovered = ctx.hovered_id.as_ref() == Some(&item.id);

        // Draw background
        if is_selected {
            let bg_color = Self::get_selected_color(palette);
            Self::render_background(graphics, item_rect, bg_color);
        } else if is_hovered {
            let bg_color = Self::get_hovered_color(palette);
            Self::render_background(graphics, item_rect, bg_color);
        }

        // Calculate positions
        let mut x = layout.layout.location.x + ctx.padding;
        let y = layout.layout.location.y + y_offset + ctx.item_height / 3.5;

        // Render icon if provided
        if let Some(ref icon_name) = item.icon {
            if !icon_name.is_empty() {
                let icon_size_f = ctx.icon_size as f32;
                let icon_y = y - icon_size_f / 2.0;
                Self::render_icon(
                    graphics,
                    info,
                    context.clone(),
                    icon_name,
                    ctx.icon_size,
                    x as f32,
                    icon_y,
                );
                x += icon_size_f + ctx.padding;
            } else {
                log::warn!("Sidebar item '{}' has empty icon name", item.id);
            }
        }

        // Render label
        let text_width = layout.layout.size.width - (x - layout.layout.location.x) - ctx.padding * 2.0;
        Self::render_text(
            graphics,
            info,
            context,
            &item.label,
            DEFAULT_FONT_SIZE,
            x as f32,
            (layout.layout.location.y + y_offset) as f32,
            text_width,
            ctx.item_height,
        );
    }

    /// Render a section header (for expandable sections).
    fn render_section_header(
        ctx: &RenderContext,
        graphics: &mut dyn Graphics,
        palette: &Palette,
        layout: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
        section: &SidebarSection,
        y_offset: f32,
        expanded: bool,
    ) {
        let header_rect = Rect::new(
            layout.layout.location.x as f64,
            (layout.layout.location.y + y_offset) as f64,
            (layout.layout.location.x + layout.layout.size.width) as f64,
            (layout.layout.location.y + y_offset + HEADER_HEIGHT) as f64,
        );

        // Draw header background
        let bg_color = Self::get_background_color(palette);
        Self::render_background(graphics, header_rect, bg_color);

        // Render section title and icon
        let mut x = layout.layout.location.x + ctx.padding;
        let y = layout.layout.location.y + y_offset + HEADER_HEIGHT / 2.0;

        // Render section icon if provided
        if let Some(ref icon_name) = section.icon {
            let icon_size_f = ctx.icon_size as f32;
            let icon_y = y - icon_size_f / 2.0;
            Self::render_icon(
                graphics,
                info,
                context.clone(),
                icon_name,
                ctx.icon_size,
                x as f32,
                icon_y,
            );
            x += icon_size_f + ctx.padding;
        }

        // Render expand/collapse indicator
        let indicator_icon = Self::get_indicator_icon(expanded);
        let indicator_size = ctx.icon_size as f32;
        let indicator_x = layout.layout.location.x + layout.layout.size.width
            - indicator_size
            - ctx.padding;
        let indicator_y = y - indicator_size / 2.0;

        Self::render_icon(
            graphics,
            info,
            context.clone(),
            indicator_icon,
            ctx.icon_size,
            indicator_x as f32,
            indicator_y,
        );

        // Render section title
        let text_width = layout.layout.size.width - (x - layout.layout.location.x) - indicator_size - ctx.padding * 2.0;
        Self::render_text(
            graphics,
            info,
            context,
            &section.title,
            DEFAULT_FONT_SIZE,
            x as f32,
            (layout.layout.location.y + y_offset) as f32,
            text_width,
            HEADER_HEIGHT,
        );
    }

    /// Render a section (expandable or flat) - static version to avoid borrow issues.
    fn render_section_static(
        ctx: &RenderContext,
        graphics: &mut dyn Graphics,
        palette: &Palette,
        layout: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
        section: &SidebarSection,
        y_offset: &mut f32,
        section_expanded: bool,
    ) {
        if section.expandable {
            // Render section header
            Self::render_section_header(
                ctx,
                graphics,
                palette,
                layout,
                info,
                context.clone(),
                section,
                *y_offset,
                section_expanded,
            );
            *y_offset += HEADER_HEIGHT; // Header height

            // Render items if expanded
            if section_expanded {
                for item in &section.items {
                    Self::render_item(
                        ctx,
                        graphics,
                        palette,
                        layout,
                        info,
                        context.clone(),
                        item,
                        *y_offset,
                    );
                    *y_offset += ctx.item_height;
                }
            }
        } else {
            // Render flat section - just render items directly
            for item in &section.items {
                Self::render_item(
                    ctx,
                    graphics,
                    palette,
                    layout,
                    info,
                    context.clone(),
                    item,
                    *y_offset,
                );
                *y_offset += ctx.item_height;
            }
        }
    }

    // Update helper functions
    fn update_hover_state(
        &self,
        layout: &LayoutNode,
        mouse_x: f32,
        mouse_y: f32,
    ) -> Option<String> {
        let mut y_offset = 0.0f32;

        for (section_index, section) in self.sections.iter().enumerate() {
            if section.expandable {
                y_offset += HEADER_HEIGHT; // Header height

                // Check items if expanded
                let expanded = *self.section_expanded[section_index].get();
                if expanded {
                    for item in &section.items {
                        let item_rect = Rect::new(
                            layout.layout.location.x as f64,
                            (layout.layout.location.y + y_offset) as f64,
                            (layout.layout.location.x + layout.layout.size.width) as f64,
                            (layout.layout.location.y + y_offset + self.item_height) as f64,
                        );

                        if item_rect.contains((mouse_x as f64, mouse_y as f64)) {
                            return Some(item.id.clone());
                        }
                        y_offset += self.item_height;
                    }
                }
            } else {
                // Flat section - check items
                for item in &section.items {
                    let item_rect = Rect::new(
                        layout.layout.location.x as f64,
                        (layout.layout.location.y + y_offset) as f64,
                        (layout.layout.location.x + layout.layout.size.width) as f64,
                        (layout.layout.location.y + y_offset + self.item_height) as f64,
                    );

                    if item_rect.contains((mouse_x as f64, mouse_y as f64)) {
                        return Some(item.id.clone());
                    }
                    y_offset += self.item_height;
                }
            }
        }

        None
    }

    fn handle_section_header_clicks(
        &self,
        layout: &LayoutNode,
        mouse_x: f32,
        mouse_y: f32,
        info: &AppInfo,
    ) -> Option<usize> {
        let mut y_offset = 0.0f32;

        for (section_index, section) in self.sections.iter().enumerate() {
            if section.expandable {
                // Check if mouse is over section header
                let header_rect = Rect::new(
                    layout.layout.location.x as f64,
                    (layout.layout.location.y + y_offset) as f64,
                    (layout.layout.location.x + layout.layout.size.width) as f64,
                    (layout.layout.location.y + y_offset + HEADER_HEIGHT) as f64,
                );

                // Handle clicks on section headers
                for (_, button, state) in &info.buttons {
                    if *button == MouseButton::Left && *state == ElementState::Pressed {
                        if header_rect.contains((mouse_x as f64, mouse_y as f64)) {
                            return Some(section_index);
                        }
                    }
                }

                y_offset += HEADER_HEIGHT; // Header height
            }
        }

        None
    }

    fn handle_item_clicks(&mut self) -> Update {
        let mut update = Update::empty();

        // Handle item clicks
        if let Some(ref hovered_id) = self.hovered_id {
            // Find and select the item
            if let Some(item) = self.find_item_by_id(hovered_id) {
                // Update selection
                self.selected_id.set(Some(hovered_id.clone()));
                update |= Update::DRAW;

                // Emit callback
                if let Some(ref callback) = self.on_item_selected {
                    update |= callback(item);
                }
            }
        }

        update
    }
}

impl Default for Sidebar {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait(?Send)]
impl Widget for Sidebar {

    fn layout_style(&self, context: &LayoutContext) -> StyleNode {
        StyleNode {
            style: self.layout_style.get().clone(),
            children: vec![],
            measure_func: None,
        }
    }

    async fn update(
        &mut self,
        layout: &LayoutNode,
        context: AppContext,
        info: &mut AppInfo,
    ) -> Update {
        let mut update = Update::empty();

        // Update mouse hover state
        let (mouse_x, mouse_y) = if let Some(cursor) = info.cursor_pos {
            (cursor.x as f32, cursor.y as f32)
        } else {
            return update;
        };

        // Check for section header clicks
        if let Some(section_index) = self.handle_section_header_clicks(layout, mouse_x, mouse_y, info) {
            if section_index < self.section_expanded.len() {
                let current_state = *self.section_expanded[section_index].get();
                self.section_expanded[section_index].set(!current_state);
                update |= Update::DRAW;
            }
        }

        // Update hover state
        let new_hovered_id = self.update_hover_state(layout, mouse_x, mouse_y);
        if new_hovered_id != self.hovered_id {
            self.hovered_id = new_hovered_id;
            update |= Update::DRAW;
        }

        // Handle item clicks
        for (_, button, state) in &info.buttons {
            if *button == MouseButton::Left && *state == ElementState::Pressed {
                update |= self.handle_item_clicks();
                break;
            }
        }

        update
    }

    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        layout: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        let palette = context.palette();
        
        // Draw background
        let bg_color = palette.color(ColorRole::Base);

        let sidebar_rect = Rect::new(
            layout.layout.location.x as f64,
            layout.layout.location.y as f64,
            (layout.layout.location.x + layout.layout.size.width) as f64,
            (layout.layout.location.y + layout.layout.size.height) as f64,
        );

        graphics.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            &Brush::Solid(bg_color),
            None,
            &sidebar_rect.to_path(PATH_ROUNDING),
        );

        // Render sections and items
        let mut y_offset = 0.0f32;
        // Collect expanded states and sections first to avoid borrow conflicts
        let expanded_states: Vec<bool> = self.section_expanded.iter().map(|s| *s.get()).collect();
        let sections: Vec<&SidebarSection> = self.sections.iter().collect();
        
        let render_ctx = RenderContext {
            item_height: self.item_height,
            icon_size: self.icon_size,
            padding: self.padding,
            selected_id: self.selected_id.get().clone(),
            hovered_id: self.hovered_id.clone(),
        };

        for (section_index, section) in sections.iter().enumerate() {
            let section_expanded = if section.expandable && section_index < expanded_states.len() {
                expanded_states[section_index]
            } else {
                true // For non-expandable sections, always "expanded"
            };
            Self::render_section_static(
                &render_ctx,
                graphics,
                palette,
                layout,
                info,
                context.clone(),
                section,
                &mut y_offset,
                section_expanded,
            );
        }
    }
}
