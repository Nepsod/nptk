use nptk_core::app::context::AppContext;
use nptk_core::app::focus::{FocusId, FocusState, FocusableWidget, FocusProperties, FocusBounds};
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{Dimension, LayoutNode, LayoutStyle, LengthPercentage, StyleNode};
use nptk_core::signal::MaybeSignal;
use nptk_core::text_input::TextBuffer;
use nptk_core::skrifa::instance::Size;
use nptk_core::skrifa::raw::FileRef;
use nptk_core::skrifa::setting::VariationSetting;
use nptk_core::skrifa::MetadataProvider;
use nptk_core::vg::kurbo::{Affine, Line, Rect, RoundedRect, RoundedRectRadii, Stroke};
use nptk_core::vg::peniko::{Brush, Color, Fill};
use nptk_core::vg::{Glyph, Scene};
use std::ops::Deref;
use nptk_core::widget::{Widget, WidgetLayoutExt};
use nptk_core::window::{ElementState, Ime, KeyCode, PhysicalKey};
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;
use nalgebra::Vector2;
use std::time::{Duration, Instant};

/// A password input widget that masks the entered text.
///
/// ### Theming
/// Uses the same theming properties as TextInput:
/// - `color_background` - The background color of the input field.
/// - `color_border` - The border color of the input field.
/// - `color_border_focused` - The border color when focused.
/// - `color_text` - The text color.
/// - `color_cursor` - The cursor color.
/// - `color_selection` - The selection highlight color.
pub struct SecretInput {
    layout_style: MaybeSignal<LayoutStyle>,
    buffer: TextBuffer,
    placeholder: MaybeSignal<String>,
    on_change: MaybeSignal<Update>,
    focus_id: FocusId,
    focus_state: FocusState,
    focus_via_keyboard: bool,
    cursor_blink_timer: Instant,
    cursor_visible: bool,
    mouse_down: bool,
    drag_start_pos: Option<usize>,
    mask_char: char,
    show_password: bool,
}

impl SecretInput {
    /// Create a new secret input widget.
    pub fn new() -> Self {
        Self {
            layout_style: LayoutStyle {
                size: Vector2::<Dimension>::new(Dimension::length(200.0), Dimension::length(30.0)),
                padding: nptk_core::layout::Rect::<LengthPercentage> {
                    left: LengthPercentage::length(8.0),
                    right: LengthPercentage::length(8.0),
                    top: LengthPercentage::length(6.0),
                    bottom: LengthPercentage::length(6.0),
                },
                ..Default::default()
            }
            .into(),
            buffer: TextBuffer::new(),
            placeholder: MaybeSignal::value("".to_string()),
            on_change: MaybeSignal::value(Update::empty()),
            focus_id: FocusId::new(),
            focus_state: FocusState::None,
            focus_via_keyboard: false,
            cursor_blink_timer: Instant::now(),
            cursor_visible: true,
            mouse_down: false,
            drag_start_pos: None,
            mask_char: '•',
            show_password: false,
        }
    }

    /// Set the placeholder text.
    pub fn with_placeholder(mut self, placeholder: impl Into<MaybeSignal<String>>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Set the change callback.
    pub fn with_on_change(mut self, on_change: impl Into<MaybeSignal<Update>>) -> Self {
        self.on_change = on_change.into();
        self
    }

    /// Set the mask character (default is '•').
    pub fn with_mask_char(mut self, mask_char: char) -> Self {
        self.mask_char = mask_char;
        self
    }

    /// Toggle password visibility.
    pub fn toggle_password_visibility(&mut self) {
        self.show_password = !self.show_password;
    }

    /// Set password visibility.
    pub fn set_password_visibility(&mut self, visible: bool) {
        self.show_password = visible;
    }

    /// Get the current password value.
    pub fn password(&self) -> &str {
        self.buffer.text()
    }

    /// Get the masked version of the text for display.
    fn masked_text(&self) -> String {
        if self.show_password {
            self.buffer.text().to_string()
        } else {
            self.mask_char.to_string().repeat(self.buffer.text().chars().count())
        }
    }

    /// Calculate cursor position from mouse coordinates.
    fn cursor_position_from_mouse(&self, mouse_x: f32, layout_node: &LayoutNode, info: &AppInfo) -> usize {
        let font_size = 16.0;
        let font = info.font_context.default_font().clone();

        let font_ref = {
            let file_ref = FileRef::new(font.data.as_ref()).expect("Failed to load font data");
            match file_ref {
                FileRef::Font(font) => Some(font),
                FileRef::Collection(collection) => collection.get(font.index).ok(),
            }
        }
        .expect("Failed to load font reference");

        let location = font_ref.axes().location::<&[VariationSetting; 0]>(&[]);
        let glyph_metrics = font_ref.glyph_metrics(Size::new(font_size), &location);
        let charmap = font_ref.charmap();

        let text_start_x = layout_node.layout.location.x + 8.0; // Padding
        let relative_x = mouse_x - text_start_x;

        if relative_x <= 0.0 {
            return 0;
        }

        // Use consistent mask character width for all positions
        let mask_gid = charmap.map(self.mask_char).unwrap_or_default();
        let mask_advance = glyph_metrics.advance_width(mask_gid).unwrap_or_default();
        
        // Calculate position based on mask character width and actual character count
        let char_count = self.buffer.text().chars().count();
        let click_position = (relative_x / mask_advance).round() as usize;
        
        // Clamp to valid range
        click_position.min(char_count)
    }
}

impl WidgetLayoutExt for SecretInput {
    fn set_layout_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.layout_style = layout_style.into();
    }
}

impl Widget for SecretInput {
    fn render(
        &mut self,
        scene: &mut Scene,
        theme: &mut dyn Theme,
        layout_node: &LayoutNode,
        info: &AppInfo,
        context: AppContext,
    ) {
        // Update focus state
        if let Ok(manager) = info.focus_manager.lock() {
            self.focus_state = manager.get_focus_state(self.focus_id);
        }

        let is_focused = matches!(self.focus_state, FocusState::Focused | FocusState::Gained);

        // Update cursor blink in render method for immediate visual feedback
        if is_focused {
            let now = Instant::now();
            if now.duration_since(self.cursor_blink_timer) > Duration::from_millis(500) {
                self.cursor_visible = !self.cursor_visible;
                self.cursor_blink_timer = now;
                // Request another redraw for the next blink cycle
                context.update().insert(Update::DRAW);
            }
        }

        // Get colors from theme or use defaults (same as TextInput)
        let bg_color = if let Some(style) = theme.of(self.widget_id()) {
            style.get_color("color_background").unwrap_or(Color::WHITE)
        } else {
            Color::WHITE
        };

        let border_color = if let Some(style) = theme.of(self.widget_id()) {
            if is_focused {
                style.get_color("color_border_focused").unwrap_or(Color::from_rgb8(100, 150, 255))
            } else {
                style.get_color("color_border").unwrap_or(Color::from_rgb8(200, 200, 200))
            }
        } else if is_focused {
            Color::from_rgb8(100, 150, 255)
        } else {
            Color::from_rgb8(200, 200, 200)
        };

        let _text_color = if let Some(style) = theme.of(self.widget_id()) {
            style.get_color("color_text").unwrap_or(Color::BLACK)
        } else {
            Color::BLACK
        };

        let input_rect = RoundedRect::from_rect(
            Rect::new(
                layout_node.layout.location.x as f64,
                layout_node.layout.location.y as f64,
                (layout_node.layout.location.x + layout_node.layout.size.width) as f64,
                (layout_node.layout.location.y + layout_node.layout.size.height) as f64,
            ),
            RoundedRectRadii::from_single_radius(4.0),
        );

        // Draw background
        scene.fill(
            Fill::NonZero,
            Affine::default(),
            &Brush::Solid(bg_color),
            None,
            &input_rect,
        );

        // Draw border
        scene.stroke(
            &Stroke::new(if is_focused { 2.0 } else { 1.0 }),
            Affine::default(),
            &Brush::Solid(border_color),
            None,
            &input_rect,
        );

        // Render masked text content or placeholder
        let placeholder_text = self.placeholder.get();
        let display_text = if self.buffer.text().is_empty() {
            placeholder_text.deref()
        } else {
            &self.masked_text()
        };

        let font_size = 16.0;
        let font = info.font_context.default_font().clone();

        let font_ref = {
            let file_ref = FileRef::new(font.data.as_ref()).expect("Failed to load font data");
            match file_ref {
                FileRef::Font(font) => Some(font),
                FileRef::Collection(collection) => collection.get(font.index).ok(),
            }
        }
        .expect("Failed to load font reference");

        let location = font_ref.axes().location::<&[VariationSetting; 0]>(&[]);
        let glyph_metrics = font_ref.glyph_metrics(Size::new(font_size), &location);
        let charmap = font_ref.charmap();

        // Render selection highlight first (behind text)
        if let Some(selection_range) = self.buffer.cursor().selection() {
            let selection_color = if let Some(style) = theme.of(self.widget_id()) {
                style.get_color("color_selection").unwrap_or(Color::from_rgb8(180, 200, 255))
            } else {
                Color::from_rgb8(180, 200, 255)
            };

            // Calculate selection bounds using consistent mask character width
            let mask_gid = charmap.map(self.mask_char).unwrap_or_default();
            let mask_advance = glyph_metrics.advance_width(mask_gid).unwrap_or_default();
            
            let selection_start_x = layout_node.layout.location.x + 8.0 + (mask_advance * selection_range.start as f32);
            let selection_end_x = layout_node.layout.location.x + 8.0 + (mask_advance * selection_range.end as f32);

            // Draw selection background
            scene.fill(
                Fill::NonZero,
                Affine::default(),
                &Brush::Solid(selection_color),
                None,
                &Rect::new(
                    selection_start_x as f64,
                    layout_node.layout.location.y as f64 + 4.0,
                    selection_end_x as f64,
                    layout_node.layout.location.y as f64 + layout_node.layout.size.height as f64 - 4.0,
                ),
            );
        }

        if !display_text.is_empty() {
            let text_color = if self.buffer.text().is_empty() {
                Color::from_rgb8(150, 150, 150) // Placeholder color
            } else {
                _text_color
            };

            let mut pen_x = layout_node.layout.location.x + 8.0; // Padding
            let pen_y = layout_node.layout.location.y + font_size + 6.0; // Padding + baseline

            scene
                .draw_glyphs(&font)
                .font_size(font_size)
                .brush(&Brush::Solid(text_color))
                .normalized_coords(bytemuck::cast_slice(location.coords()))
                .hint(true)
                .draw(
                    &nptk_core::vg::peniko::Style::Fill(Fill::NonZero),
                    display_text.chars().filter_map(|c| {
                        let gid = charmap.map(c).unwrap_or_default();
                        let advance = glyph_metrics.advance_width(gid).unwrap_or_default();
                        let x = pen_x;
                        pen_x += advance;

                        Some(Glyph {
                            id: gid.to_u32(),
                            x,
                            y: pen_y,
                        })
                    }),
                );
        }

        // Render cursor when focused and visible
        if is_focused && self.cursor_visible {
            let cursor_color = if let Some(style) = theme.of(self.widget_id()) {
                style.get_color("color_cursor").unwrap_or(Color::BLACK)
            } else {
                Color::BLACK
            };

            // Calculate cursor position based on actual character count (not visual representation)
            let cursor_pos = self.buffer.cursor().position;
            let mut cursor_x = layout_node.layout.location.x + 8.0;
            
            // Use mask character width for consistent spacing, but count actual characters
            let mask_gid = charmap.map(self.mask_char).unwrap_or_default();
            let mask_advance = glyph_metrics.advance_width(mask_gid).unwrap_or_default();
            
            // Count actual characters up to cursor position, not visual characters
            let actual_char_count = self.buffer.text().chars().take(cursor_pos).count();
            cursor_x += mask_advance * actual_char_count as f32;

            // Draw cursor line
            scene.stroke(
                &Stroke::new(1.0),
                Affine::default(),
                &Brush::Solid(cursor_color),
                None,
                &Line::new(
                    (cursor_x as f64, layout_node.layout.location.y as f64 + 4.0),
                    (cursor_x as f64, layout_node.layout.location.y as f64 + layout_node.layout.size.height as f64 - 4.0),
                ),
            );
        }
    }

    fn layout_style(&self) -> StyleNode {
        StyleNode {
            style: self.layout_style.get().clone(),
            children: Vec::new(),
        }
    }

    fn update(&mut self, layout: &LayoutNode, _context: AppContext, info: &AppInfo) -> Update {
        let mut update = Update::empty();
        let old_focus_state = self.focus_state;

        // Register with focus manager
        if let Ok(mut manager) = info.focus_manager.lock() {
            let focusable_widget = FocusableWidget {
                id: self.focus_id,
                properties: FocusProperties {
                    tab_focusable: true,
                    click_focusable: true,
                    tab_index: 0,
                    accepts_keyboard: true,
                },
                bounds: FocusBounds {
                    x: layout.layout.location.x,
                    y: layout.layout.location.y,
                    width: layout.layout.size.width,
                    height: layout.layout.size.height,
                },
            };
            manager.register_widget(focusable_widget);
            
            // Update focus state
            let new_focus_state = manager.get_focus_state(self.focus_id);
            
            if matches!(new_focus_state, FocusState::Gained) && !matches!(old_focus_state, FocusState::Focused) {
                self.focus_via_keyboard = manager.was_last_focus_via_keyboard();
            } else if matches!(new_focus_state, FocusState::Lost | FocusState::None) {
                self.focus_via_keyboard = false;
            }
            
            self.focus_state = new_focus_state;
        }

        // Process input when focused (same logic as TextInput)
        if matches!(self.focus_state, FocusState::Focused | FocusState::Gained) {
            let mut text_changed = false;
            
            // Process IME events for text composition
            for ime_event in &info.ime_events {
                match ime_event {
                    Ime::Commit(text) => {
                        self.buffer.insert(text);
                        text_changed = true;
                        self.cursor_blink_timer = Instant::now();
                        self.cursor_visible = true;
                    }
                    Ime::Preedit(_, _) => {
                        // TODO: Handle text composition preview
                    }
                    Ime::Enabled => {}
                    Ime::Disabled => {}
                }
            }
            
            // Process keyboard events
            for (_, key_event) in &info.keys {
                if key_event.state == ElementState::Pressed {
                    if info.modifiers.control_key() {
                        match key_event.physical_key {
                            PhysicalKey::Code(KeyCode::KeyA) => {
                                if !self.buffer.text().is_empty() {
                                    let text_len = self.buffer.text().chars().count();
                                    self.buffer.cursor.selection_start = Some(0);
                                    self.buffer.cursor.position = text_len;
                                    update |= Update::DRAW;
                                }
                            }
                            PhysicalKey::Code(KeyCode::KeyC) => {
                                if let Some(selected_text) = self.buffer.selected_text() {
                                    println!("Copy password: [HIDDEN]");
                                }
                            }
                            PhysicalKey::Code(KeyCode::KeyX) => {
                                if let Some(_) = self.buffer.selected_text() {
                                    println!("Cut password: [HIDDEN]");
                                    self.buffer.delete_backward();
                                    text_changed = true;
                                }
                            }
                            PhysicalKey::Code(KeyCode::KeyV) => {
                                println!("Paste to password field requested");
                            }
                            _ => {}
                        }
                    } else {
                        match key_event.physical_key {
                            PhysicalKey::Code(KeyCode::Backspace) => {
                                self.buffer.delete_backward();
                                text_changed = true;
                                self.cursor_blink_timer = Instant::now();
                                self.cursor_visible = true;
                            }
                            PhysicalKey::Code(KeyCode::Delete) => {
                                self.buffer.delete_forward();
                                text_changed = true;
                                self.cursor_blink_timer = Instant::now();
                                self.cursor_visible = true;
                            }
                            PhysicalKey::Code(KeyCode::ArrowLeft) => {
                                self.buffer.move_left(info.modifiers.shift_key());
                                self.cursor_blink_timer = Instant::now();
                                self.cursor_visible = true;
                                update |= Update::DRAW;
                            }
                            PhysicalKey::Code(KeyCode::ArrowRight) => {
                                self.buffer.move_right(info.modifiers.shift_key());
                                self.cursor_blink_timer = Instant::now();
                                self.cursor_visible = true;
                                update |= Update::DRAW;
                            }
                            PhysicalKey::Code(KeyCode::Home) => {
                                self.buffer.move_to_start(info.modifiers.shift_key());
                                self.cursor_blink_timer = Instant::now();
                                self.cursor_visible = true;
                                update |= Update::DRAW;
                            }
                            PhysicalKey::Code(KeyCode::End) => {
                                self.buffer.move_to_end(info.modifiers.shift_key());
                                self.cursor_blink_timer = Instant::now();
                                self.cursor_visible = true;
                                update |= Update::DRAW;
                            }
                            _ => {
                                if let Some(text) = &key_event.text {
                                    // Filter out control characters and empty strings
                                    let clean_text: String = text.chars()
                                        .filter(|c| !c.is_control() && *c != '\0')
                                        .collect();
                                    
                                    if !clean_text.is_empty() {
                                        self.buffer.insert(&clean_text);
                                        text_changed = true;
                                        self.cursor_blink_timer = Instant::now();
                                        self.cursor_visible = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if text_changed {
                update |= *self.on_change.get();
                update |= Update::DRAW;
            }
        }

        // Handle mouse selection (same logic as TextInput but with masked positioning)
        if let Some(cursor_pos) = info.cursor_pos {
            let in_bounds = cursor_pos.x as f32 >= layout.layout.location.x
                && cursor_pos.x as f32 <= layout.layout.location.x + layout.layout.size.width
                && cursor_pos.y as f32 >= layout.layout.location.y
                && cursor_pos.y as f32 <= layout.layout.location.y + layout.layout.size.height;

            if in_bounds {
                for (_, button, state) in &info.buttons {
                    if *button == nptk_core::window::MouseButton::Left {
                        match state {
                            nptk_core::window::ElementState::Pressed => {
                                if matches!(self.focus_state, FocusState::Focused | FocusState::Gained) {
                                    let click_pos = self.cursor_position_from_mouse(cursor_pos.x as f32, layout, info);
                                    self.buffer.cursor.move_to(click_pos);
                                    self.mouse_down = true;
                                    self.drag_start_pos = Some(click_pos);
                                    self.cursor_blink_timer = Instant::now();
                                    self.cursor_visible = true;
                                    update |= Update::DRAW;
                                }
                            }
                            nptk_core::window::ElementState::Released => {
                                self.mouse_down = false;
                                self.drag_start_pos = None;
                            }
                        }
                    }
                }

                if self.mouse_down && matches!(self.focus_state, FocusState::Focused | FocusState::Gained) {
                    if let Some(start_pos) = self.drag_start_pos {
                        let current_pos = self.cursor_position_from_mouse(cursor_pos.x as f32, layout, info);
                        
                        if current_pos != self.buffer.cursor().position {
                            self.buffer.cursor.selection_start = Some(start_pos);
                            self.buffer.cursor.position = current_pos;
                            update |= Update::DRAW;
                        }
                    }
                }
            } else if self.mouse_down && matches!(self.focus_state, FocusState::Focused | FocusState::Gained) {
                // Extended selection outside bounds
                if let Some(start_pos) = self.drag_start_pos {
                    let text_len = self.buffer.text().chars().count();
                    let widget_left = layout.layout.location.x;
                    let widget_right = layout.layout.location.x + layout.layout.size.width;
                    
                    let current_pos = if (cursor_pos.x as f32) < widget_left {
                        0
                    } else if (cursor_pos.x as f32) > widget_right {
                        text_len
                    } else {
                        self.cursor_position_from_mouse(cursor_pos.x as f32, layout, info)
                    };
                    
                    if current_pos != self.buffer.cursor().position {
                        self.buffer.cursor.selection_start = Some(start_pos);
                        self.buffer.cursor.position = current_pos;
                        update |= Update::DRAW;
                    }
                }
            }
        } else if self.mouse_down && matches!(self.focus_state, FocusState::Focused | FocusState::Gained) {
            // Mouse left window entirely - extend to end
            if let Some(start_pos) = self.drag_start_pos {
                let text_len = self.buffer.text().chars().count();
                if text_len != self.buffer.cursor().position {
                    self.buffer.cursor.selection_start = Some(start_pos);
                    self.buffer.cursor.position = text_len;
                    update |= Update::DRAW;
                }
            }
        }

        // Handle global mouse release
        for (_, button, state) in &info.buttons {
            if *button == nptk_core::window::MouseButton::Left && *state == nptk_core::window::ElementState::Released {
                self.mouse_down = false;
                self.drag_start_pos = None;
            }
        }

        // Update on focus state change
        if old_focus_state != self.focus_state {
            update |= Update::DRAW;
            
            if matches!(self.focus_state, FocusState::Gained) {
                self.cursor_blink_timer = Instant::now();
                self.cursor_visible = true;
            }
        }

        update
    }

    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "SecretInput")
    }
}

impl Default for SecretInput {
    fn default() -> Self {
        Self::new()
    }
}
