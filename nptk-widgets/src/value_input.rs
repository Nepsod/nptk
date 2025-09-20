use std::time::Instant;

use nptk_core::app::context::AppContext;
use nptk_core::app::focus::{FocusId, FocusState, FocusProperties, FocusBounds, FocusableWidget};
use nptk_core::app::{info::AppInfo, font_ctx::FontContext};
use nptk_core::app::update::Update;
use nptk_core::layout::{LayoutNode, LayoutStyle, StyleNode};
use nptk_core::signal::{MaybeSignal, Signal, state::StateSignal};
use nptk_core::text_input::TextBuffer;
use nptk_core::skrifa::instance::Size;
use nptk_core::skrifa::raw::FileRef;
use nptk_core::skrifa::setting::VariationSetting;
use nptk_core::skrifa::MetadataProvider;
use nptk_core::vg::kurbo::{Affine, Rect, RoundedRect, RoundedRectRadii, Stroke};
use nptk_core::vg::peniko::{Brush, Color, Fill};
use nptk_core::vg::{Glyph, Scene};
use std::ops::Deref;
use nptk_core::widget::{Widget, WidgetLayoutExt};
use nptk_core::window::{ElementState, KeyCode, PhysicalKey, MouseButton, Ime};
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;

/// A numeric input widget with validation and constraints.
pub struct ValueInput {
    /// Current numeric value
    value: StateSignal<f64>,
    /// Minimum allowed value
    min_value: Option<f64>,
    /// Maximum allowed value
    max_value: Option<f64>,
    /// Step size for increment/decrement
    step: f64,
    /// Number of decimal places to display
    decimal_places: usize,
    /// Placeholder text when empty
    placeholder: StateSignal<String>,
    /// Layout style for the widget
    layout_style: MaybeSignal<LayoutStyle>,
    /// Text buffer for editing
    buffer: TextBuffer,
    /// Focus management
    focus_id: FocusId,
    focus_state: FocusState,
    focus_via_keyboard: bool,
    /// Cursor blinking
    cursor_blink_timer: Instant,
    cursor_visible: bool,
    /// Mouse interaction
    mouse_down: bool,
    drag_start_pos: Option<usize>,
    /// Validation state
    is_valid: bool,
    /// Whether to allow negative values
    allow_negative: bool,
}

impl ValueInput {
    /// Create a new ValueInput widget with default settings.
    pub fn new() -> Self {
        Self {
            value: StateSignal::new(0.0),
            min_value: None,
            max_value: None,
            step: 1.0,
            decimal_places: 0,
            placeholder: StateSignal::new("Enter number...".to_string()),
            layout_style: MaybeSignal::value(LayoutStyle::default()),
            buffer: TextBuffer::new(),
            focus_id: FocusId::new(),
            focus_state: FocusState::None,
            focus_via_keyboard: false,
            cursor_blink_timer: Instant::now(),
            cursor_visible: true,
            mouse_down: false,
            drag_start_pos: None,
            is_valid: true,
            allow_negative: false,
        }
    }

    /// Set the current value.
    pub fn with_value(mut self, value: impl Into<MaybeSignal<f64>>) -> Self {
        let signal = value.into();
        let initial_value = signal.get();
        self.value.set(*initial_value);
        self.sync_text_from_value();
        self
    }

    /// Set minimum value constraint.
    pub fn with_min(mut self, min: f64) -> Self {
        self.min_value = Some(min);
        self
    }

    /// Set maximum value constraint.
    pub fn with_max(mut self, max: f64) -> Self {
        self.max_value = Some(max);
        self
    }

    /// Set step size for increment/decrement.
    pub fn with_step(mut self, step: f64) -> Self {
        self.step = step;
        self
    }

    /// Set number of decimal places to display.
    pub fn with_decimal_places(mut self, places: usize) -> Self {
        self.decimal_places = places;
        self
    }

    /// Set placeholder text.
    pub fn with_placeholder(self, placeholder: impl Into<MaybeSignal<String>>) -> Self {
        let signal = placeholder.into();
        self.placeholder.set(signal.get().clone());
        self
    }

    /// Allow negative values.
    pub fn with_negative(mut self, allow: bool) -> Self {
        self.allow_negative = allow;
        self
    }

    /// Get the current value signal.
    pub fn value(&self) -> &StateSignal<f64> {
        &self.value
    }

    /// Sync text buffer from current value.
    fn sync_text_from_value(&mut self) {
        let value = self.value.get();
        let text = if self.decimal_places == 0 {
            format!("{:.0}", *value)
        } else {
            format!("{:.prec$}", *value, prec = self.decimal_places)
        };
        self.buffer.set_text(text);
        // Position cursor at the end of the text
        let text_len = self.buffer.text().len();
        self.buffer.cursor.position = text_len;
        self.buffer.cursor.selection_start = None;
    }

    /// Parse and validate text input, update value if valid.
    fn validate_and_update(&mut self) -> bool {
        let text = self.buffer.text().trim();
        
        if text.is_empty() {
            self.is_valid = true;
            return true;
        }

        match text.parse::<f64>() {
            Ok(mut parsed_value) => {
                // Check negative constraint
                if !self.allow_negative && parsed_value < 0.0 {
                    self.is_valid = false;
                    return false;
                }

                // Apply min/max constraints
                if let Some(min) = self.min_value {
                    if parsed_value < min {
                        parsed_value = min;
                    }
                }
                if let Some(max) = self.max_value {
                    if parsed_value > max {
                        parsed_value = max;
                    }
                }

                self.value.set(parsed_value);
                self.is_valid = true;
                true
            }
            Err(_) => {
                self.is_valid = false;
                false
            }
        }
    }

    /// Increment value by step.
    pub fn increment(&mut self) {
        let current = *self.value.get();
        let new_value = current + self.step;
        
        let constrained_value = if let Some(max) = self.max_value {
            new_value.min(max)
        } else {
            new_value
        };

        self.value.set(constrained_value);
        self.sync_text_from_value();
    }

    /// Decrement value by step.
    pub fn decrement(&mut self) {
        let current = *self.value.get();
        let new_value = current - self.step;
        
        let constrained_value = if let Some(min) = self.min_value {
            new_value.max(min)
        } else if !self.allow_negative {
            new_value.max(0.0)
        } else {
            new_value
        };

        self.value.set(constrained_value);
        self.sync_text_from_value();
    }

    /// Calculate cursor position from mouse coordinates.
    fn cursor_position_from_mouse(&self, mouse_x: f64, widget_left: f64, font_ctx: &FontContext) -> usize {
        let text = self.buffer.text();
        if text.is_empty() {
            return 0;
        }

        let font_size = 16.0;
        let font = font_ctx.default_font().clone();

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

        let relative_x = mouse_x - widget_left - 8.0; // Account for padding
        let mut current_x = 0.0;
        
        for (i, ch) in text.chars().enumerate() {
            let gid = charmap.map(ch).unwrap_or_default();
            let advance = glyph_metrics.advance_width(gid).unwrap_or_default();
            
            if relative_x <= current_x + (advance as f64) / 2.0 {
                return i;
            }
            current_x += advance as f64;
        }
        
        text.len()
    }
}

impl Default for ValueInput {
    fn default() -> Self {
        Self::new()
    }
}

impl WidgetLayoutExt for ValueInput {
    fn set_layout_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.layout_style = layout_style.into();
    }
}

impl Widget for ValueInput {
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
        
        // Get colors from theme or use defaults with more visible fallbacks
        let background_color = if let Some(style) = theme.of(self.widget_id()) {
            if is_focused {
                style.get_color("color_background_focused").unwrap_or(Color::WHITE)
            } else {
                style.get_color("color_background").unwrap_or(Color::from_rgb8(240, 240, 240))
            }
        } else if is_focused {
            Color::WHITE
        } else {
            Color::from_rgb8(240, 240, 240) // Slightly darker for visibility
        };

        let border_color = if let Some(style) = theme.of(self.widget_id()) {
            if !self.is_valid {
                style.get_color("color_border_error").unwrap_or(Color::from_rgb8(255, 0, 0))
            } else if is_focused {
                style.get_color("color_border_focused").unwrap_or(Color::from_rgb8(0, 120, 255))
            } else {
                style.get_color("color_border").unwrap_or(Color::from_rgb8(120, 120, 120))
            }
        } else if !self.is_valid {
            Color::from_rgb8(255, 0, 0)
        } else if is_focused {
            Color::from_rgb8(0, 120, 255)
        } else {
            Color::from_rgb8(120, 120, 120) // Darker border for visibility
        };

        let text_color = if let Some(style) = theme.of(self.widget_id()) {
            style.get_color("color_text").unwrap_or(Color::BLACK)
        } else {
            Color::BLACK
        };

        // Draw background and border
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
            &Brush::Solid(background_color),
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

        // Render text content or placeholder
        let placeholder_text = self.placeholder.get();
        let display_text = if self.buffer.text().is_empty() {
            placeholder_text.deref()
        } else {
            self.buffer.text()
        };

        let font_size = 16.0; // TODO: Make this configurable
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

        // Render text selection highlight if focused and has selection
        if is_focused && self.buffer.cursor.has_selection() && !self.buffer.text().is_empty() {
            let selection_color = if let Some(style) = theme.of(self.widget_id()) {
                style.get_color("color_selection").unwrap_or(Color::from_rgb8(0, 120, 215))
            } else {
                Color::from_rgb8(0, 120, 215)
            };

            let selection_range = self.buffer.cursor.selection().unwrap();
            let (start, end) = (selection_range.start, selection_range.end);
            let text = self.buffer.text();
            
            // Calculate selection bounds properly
            let mut selection_start_x = layout_node.layout.location.x as f64 + 8.0;
            let mut selection_end_x = layout_node.layout.location.x as f64 + 8.0;
            
            for (i, ch) in text.chars().enumerate() {
                let gid = charmap.map(ch).unwrap_or_default();
                let advance = glyph_metrics.advance_width(gid).unwrap_or_default();
                
                if i == start {
                    selection_start_x = selection_start_x;
                }
                if i < start {
                    selection_start_x += advance as f64;
                }
                if i < end {
                    selection_end_x += advance as f64;
                }
            }
            
            // Draw selection rectangle
            let selection_rect = Rect::new(
                selection_start_x,
                layout_node.layout.location.y as f64 + 4.0,
                selection_end_x,
                layout_node.layout.location.y as f64 + layout_node.layout.size.height as f64 - 4.0,
            );
            
            scene.fill(
                Fill::NonZero,
                Affine::default(),
                &Brush::Solid(selection_color),
                None,
                &selection_rect,
            );
        }

        if !display_text.is_empty() {
            // Determine text color (placeholder vs regular text)
            let display_color = if self.buffer.text().is_empty() {
                if let Some(style) = theme.of(self.widget_id()) {
                    style.get_color("color_placeholder").unwrap_or(Color::from_rgb8(150, 150, 150))
                } else {
                    Color::from_rgb8(150, 150, 150)
                }
            } else {
                text_color
            };

            // Calculate text position (with padding)
            let mut pen_x = layout_node.layout.location.x + 8.0; // Left padding
            let pen_y = layout_node.layout.location.y + font_size + 6.0; // Padding + baseline

            scene
                .draw_glyphs(&font)
                .font_size(font_size)
                .brush(&Brush::Solid(display_color))
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

        // Update cursor blink in render method for immediate visual feedback
        if is_focused {
            let now = Instant::now();
            if now.duration_since(self.cursor_blink_timer) > std::time::Duration::from_millis(500) {
                self.cursor_visible = !self.cursor_visible;
                self.cursor_blink_timer = now;
                // Request another redraw for the next blink cycle
                context.update().insert(Update::DRAW);
            }

            if self.cursor_visible {
                let cursor_color = if let Some(style) = theme.of(self.widget_id()) {
                    style.get_color("color_cursor").unwrap_or(Color::BLACK)
                } else {
                    Color::BLACK
                };
                
                // Calculate cursor position based on text
                let cursor_pos = self.buffer.cursor().position;
                let mut cursor_x = layout_node.layout.location.x + 8.0; // Left padding
                
                // Calculate cursor position based on character width
                for (i, ch) in self.buffer.text().chars().enumerate() {
                    if i >= cursor_pos {
                        break;
                    }
                    let gid = charmap.map(ch).unwrap_or_default();
                    let advance = glyph_metrics.advance_width(gid).unwrap_or_default();
                    cursor_x += advance;
                }
                
                let cursor_y = layout_node.layout.location.y + 4.0;
                let cursor_height = layout_node.layout.size.height - 8.0;

                scene.stroke(
                    &Stroke::new(1.0),
                    Affine::default(),
                    &Brush::Solid(cursor_color),
                    None,
                    &Rect::new(
                        cursor_x as f64,
                        cursor_y as f64,
                        cursor_x as f64,
                        (cursor_y + cursor_height) as f64,
                    ),
                );
            }
        }
    }

    fn update(&mut self, layout: &LayoutNode, context: AppContext, info: &AppInfo) -> Update {
        let mut update = Update::empty();

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
            if new_focus_state != self.focus_state {
                self.focus_state = new_focus_state;
                
                if matches!(self.focus_state, FocusState::Gained) {
                    self.focus_via_keyboard = manager.was_last_focus_via_keyboard();
                    self.cursor_blink_timer = Instant::now();
                    self.cursor_visible = true;
                }
                
                update |= Update::DRAW;
            }
        }

        // Handle keyboard input when focused
        let is_focused = matches!(self.focus_state, FocusState::Focused | FocusState::Gained);
        if is_focused {
            let mut text_changed = false;

            // Process IME events for text composition
            for ime_event in &info.ime_events {
                if let Ime::Commit(text) = ime_event {
                    // Filter input for numeric values
                    let filtered_text: String = text.chars()
                        .filter(|ch| ch.is_ascii_digit() || *ch == '.' || (self.allow_negative && *ch == '-'))
                        .collect();
                    if !filtered_text.is_empty() {
                        self.buffer.insert(&filtered_text);
                        text_changed = true;
                    }
                    self.cursor_blink_timer = Instant::now();
                    self.cursor_visible = true;
                }
            }

            // Process keyboard events
            for (_device_id, key_event) in &info.keys {
                if key_event.state == ElementState::Pressed {
                    let shift_pressed = info.modifiers.shift_key();
                    let ctrl_pressed = info.modifiers.control_key();
                    
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
                            self.buffer.move_left(shift_pressed);
                            self.cursor_blink_timer = Instant::now();
                            self.cursor_visible = true;
                            update |= Update::DRAW;
                        }
                        PhysicalKey::Code(KeyCode::ArrowRight) => {
                            self.buffer.move_right(shift_pressed);
                            self.cursor_blink_timer = Instant::now();
                            self.cursor_visible = true;
                            update |= Update::DRAW;
                        }
                        PhysicalKey::Code(KeyCode::ArrowUp) => {
                            if !shift_pressed {
                                self.increment();
                                text_changed = true;
                                update |= Update::DRAW;
                            }
                        }
                        PhysicalKey::Code(KeyCode::ArrowDown) => {
                            if !shift_pressed {
                                self.decrement();
                                text_changed = true;
                                update |= Update::DRAW;
                            }
                        }
                        PhysicalKey::Code(KeyCode::Home) => {
                            self.buffer.move_to_start(shift_pressed);
                            self.cursor_blink_timer = Instant::now();
                            self.cursor_visible = true;
                            update |= Update::DRAW;
                        }
                        PhysicalKey::Code(KeyCode::End) => {
                            self.buffer.move_to_end(shift_pressed);
                            self.cursor_blink_timer = Instant::now();
                            self.cursor_visible = true;
                            update |= Update::DRAW;
                        }
                        PhysicalKey::Code(KeyCode::KeyA) if ctrl_pressed => {
                            // Select all text
                            let text_len = self.buffer.text().len();
                            self.buffer.cursor.selection_start = Some(0);
                            self.buffer.cursor.position = text_len;
                            self.cursor_blink_timer = Instant::now();
                            self.cursor_visible = true;
                            update |= Update::DRAW;
                        }
                        PhysicalKey::Code(KeyCode::KeyC) if ctrl_pressed => {
                            // TODO: Copy to clipboard
                            if let Some(selected) = self.buffer.selected_text() {
                                println!("Copy: {}", selected);
                            }
                        }
                        PhysicalKey::Code(KeyCode::KeyV) if ctrl_pressed => {
                            // TODO: Paste from clipboard
                            println!("Paste requested");
                        }
                        PhysicalKey::Code(KeyCode::KeyX) if ctrl_pressed => {
                            // TODO: Cut to clipboard
                            if let Some(selected) = self.buffer.selected_text() {
                                println!("Cut: {}", selected);
                                // Delete selection by inserting empty string
                                if self.buffer.cursor.has_selection() {
                                    self.buffer.insert("");
                                    text_changed = true;
                                }
                            }
                        }
                        PhysicalKey::Code(KeyCode::Enter) => {
                            // Validate and finalize input
                            self.validate_and_update();
                            self.sync_text_from_value();
                            update |= Update::DRAW;
                        }
                        _ => {
                            // Character input is handled via IME events above
                            if let Some(text) = &key_event.text {
                                let clean_text: String = text.chars()
                                    .filter(|c| {
                                        c.is_ascii_digit() || 
                                        (*c == '.' && self.decimal_places > 0) ||
                                        (*c == '-' && self.allow_negative)
                                    })
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

            if text_changed {
                self.validate_and_update();
                update |= Update::DRAW;
            }
        }

        // Handle mouse input
        if let Some(cursor_pos) = info.cursor_pos {
            let in_bounds = cursor_pos.x >= layout.layout.location.x as f64
                && cursor_pos.x <= (layout.layout.location.x + layout.layout.size.width) as f64
                && cursor_pos.y >= layout.layout.location.y as f64
                && cursor_pos.y <= (layout.layout.location.y + layout.layout.size.height) as f64;

            // Handle mouse button events (only when in bounds)
            if in_bounds {
                for (_device_id, button, state) in &info.buttons {
                    if *button == MouseButton::Left {
                        match state {
                            ElementState::Pressed => {
                                if matches!(self.focus_state, FocusState::Focused | FocusState::Gained) {
                                    // Set focus and position cursor
                                    context.set_focus(Some(self.focus_id));
                                    self.mouse_down = true;
                                    
                                    // Calculate cursor position from mouse
                                    let click_pos = self.cursor_position_from_mouse(
                                        cursor_pos.x,
                                        layout.layout.location.x as f64,
                                        &info.font_context
                                    );
                                    self.buffer.cursor.position = click_pos;
                                    self.buffer.cursor.selection_start = None;
                                    self.drag_start_pos = Some(click_pos);
                                    
                                    self.cursor_blink_timer = Instant::now();
                                    self.cursor_visible = true;
                                    update |= Update::DRAW;
                                }
                            }
                            ElementState::Released => {
                                // Always handle mouse release, regardless of bounds
                                self.mouse_down = false;
                                self.drag_start_pos = None;
                            }
                        }
                    }
                }

                // Handle mouse drag for selection (when in bounds and dragging)
                if self.mouse_down && matches!(self.focus_state, FocusState::Focused | FocusState::Gained) {
                    if let Some(start_pos) = self.drag_start_pos {
                        let current_pos = self.cursor_position_from_mouse(
                            cursor_pos.x,
                            layout.layout.location.x as f64,
                            &info.font_context
                        );
                        
                        if current_pos != self.buffer.cursor.position {
                            // Update selection
                            self.buffer.cursor.selection_start = Some(start_pos);
                            self.buffer.cursor.position = current_pos;
                            update |= Update::DRAW;
                        }
                    }
                }
            } else if self.mouse_down && matches!(self.focus_state, FocusState::Focused | FocusState::Gained) {
                // Mouse is outside bounds but we're still dragging - extend selection
                if let Some(start_pos) = self.drag_start_pos {
                    let text_len = self.buffer.text().chars().count();
                    let widget_left = layout.layout.location.x as f64;
                    let widget_right = widget_left + layout.layout.size.width as f64;
                    
                    let current_pos = if cursor_pos.x < widget_left {
                        // Dragging to the left of widget - select to beginning
                        0
                    } else if cursor_pos.x > widget_right {
                        // Dragging to the right of widget - select to end
                        text_len
                    } else {
                        // This shouldn't happen since we're in the else branch, but handle it
                        self.cursor_position_from_mouse(
                            cursor_pos.x,
                            layout.layout.location.x as f64,
                            &info.font_context
                        )
                    };
                    
                    if current_pos != self.buffer.cursor.position {
                        // Update selection
                        self.buffer.cursor.selection_start = Some(start_pos);
                        self.buffer.cursor.position = current_pos;
                        update |= Update::DRAW;
                    }
                }
            }
        } else if self.mouse_down && matches!(self.focus_state, FocusState::Focused | FocusState::Gained) {
            // Mouse cursor left the window entirely but we're still dragging
            // Continue selection to the end of text (most common behavior)
            if let Some(start_pos) = self.drag_start_pos {
                let text_len = self.buffer.text().chars().count();
                if text_len != self.buffer.cursor.position {
                    self.buffer.cursor.selection_start = Some(start_pos);
                    self.buffer.cursor.position = text_len;
                    update |= Update::DRAW;
                }
            }
        }

        // Also handle global mouse release events (in case mouse was released outside widget)
        for (_device_id, button, state) in &info.buttons {
            if *button == MouseButton::Left && *state == ElementState::Released {
                self.mouse_down = false;
                self.drag_start_pos = None;
            }
        }

        update
    }

    fn layout_style(&self) -> StyleNode {
        StyleNode {
            style: self.layout_style.get().clone(),
            children: Vec::new(),
        }
    }

    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "ValueInput")
    }
}
