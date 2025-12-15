use std::time::Instant;

use nptk_core::app::context::AppContext;
use nptk_core::app::focus::{FocusBounds, FocusId, FocusProperties, FocusState, FocusableWidget};
use nptk_core::app::update::Update;
use nptk_core::app::{font_ctx::FontContext, info::AppInfo};
use nptk_core::layout::{LayoutNode, LayoutStyle, StyleNode};
use nptk_core::signal::{state::StateSignal, MaybeSignal, Signal};
use nptk_core::text_input::TextBuffer;
use nptk_core::text_render::TextRenderContext;
use nptk_core::vg::kurbo::{Affine, Rect, RoundedRect, RoundedRectRadii, Shape, Stroke};
use nptk_core::vg::peniko::{Brush, Color, Fill};
use nptk_core::vgi::Graphics;
use nptk_core::widget::{Widget, WidgetLayoutExt};
use nptk_core::window::{ElementState, Ime, KeyCode, MouseButton, PhysicalKey};
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;
use std::ops::Deref;

/// A numeric input widget with validation and constraints.
///
/// ### Theming
/// Styling the value input requires the following properties:
/// - `color_background` - The background color of the input field.
/// - `color_background_focused` - The background color when focused.
/// - `color_border` - The border color of the input field.
/// - `color_border_focused` - The border color when focused.
/// - `color_border_error` - The border color when the value is invalid.
/// - `color_text` - The text color.
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
    /// Text rendering context
    text_render_context: TextRenderContext,
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
    last_click_time: Instant,
    last_click_pos: Option<usize>,
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
            text_render_context: TextRenderContext::new(),
            focus_id: FocusId::new(),
            focus_state: FocusState::None,
            focus_via_keyboard: false,
            cursor_blink_timer: Instant::now(),
            cursor_visible: true,
            mouse_down: false,
            drag_start_pos: None,
            last_click_time: Instant::now(),
            last_click_pos: None,
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
            },
            Err(_) => {
                self.is_valid = false;
                false
            },
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

    /// Calculate the actual width of text using Parley's font metrics
    fn calculate_text_width(&self, text: &str, font_size: f32, info: &mut AppInfo) -> f32 {
        if text.is_empty() {
            return 0.0;
        }

        // Use TextRenderContext to get accurate measurements from Parley
        // This handles all Unicode characters, emojis, and different scripts properly
        self.text_render_context
            .measure_text_width(&mut info.font_context, text, None, font_size)
    }

    /// Calculate the X position of the cursor based on its character position.
    fn cursor_x_position(
        &self,
        cursor_pos: usize,
        layout_node: &LayoutNode,
        info: &mut AppInfo,
    ) -> f32 {
        let font_size = 16.0f32; // Match TextInput font size
        let text_start_x = layout_node.layout.location.x + 8.0; // Padding
        let text = self.buffer.text();

        if cursor_pos == 0 || text.is_empty() {
            return text_start_x;
        }

        // Calculate actual width of text up to cursor position
        let text_up_to_cursor: String = text.chars().take(cursor_pos).collect();
        let actual_width = self.calculate_text_width(&text_up_to_cursor, font_size, info);

        text_start_x + actual_width
    }

    /// Check if this is a double-click and handle accordingly.
    fn handle_double_click(&mut self, click_pos: usize, _layout_node: &LayoutNode) -> bool {
        let now = Instant::now();
        let time_since_last_click = now.duration_since(self.last_click_time);
        let is_double_click = time_since_last_click.as_millis() < 500; // 500ms double-click window
        let is_same_position = self.last_click_pos == Some(click_pos);

        self.last_click_time = now;
        self.last_click_pos = Some(click_pos);

        if is_double_click && is_same_position {
            let text = self.buffer.text();

            if text.is_empty() {
                // Double-click on empty field - do nothing (no text to select)
                return false;
            } else {
                // For numeric input, double-click should select the entire number
                // This is more useful than word selection for numeric values
                let text_len = text.chars().count();
                self.buffer.cursor.selection_start = Some(0);
                self.buffer.cursor.position = text_len;
            }

            self.cursor_blink_timer = Instant::now();
            self.cursor_visible = true;
            return true;
        }

        false
    }

    /// Calculate cursor position from mouse coordinates (simple version without font context).
    fn cursor_position_from_mouse_simple(&self, mouse_x: f32, layout_node: &LayoutNode) -> usize {
        let font_size = 16.0;
        let text_start_x = layout_node.layout.location.x + 8.0; // Padding
        let relative_x = mouse_x - text_start_x;

        if relative_x <= 0.0 {
            return 0;
        }

        let text = self.buffer.text();
        if text.is_empty() {
            return 0;
        }

        // Simple character-based positioning (approximate)
        let char_width = font_size * 0.6; // Approximate character width
        let char_pos = (relative_x / char_width) as usize;

        // Clamp to text length
        let text_len = text.chars().count();
        char_pos.min(text_len)
    }

    /// Calculate cursor position from mouse coordinates.
    fn cursor_position_from_mouse(
        &self,
        mouse_x: f64,
        widget_left: f64,
        _font_ctx: &FontContext,
    ) -> usize {
        let text = self.buffer.text();
        if text.is_empty() {
            return 0;
        }

        let _font_size = 16.0;
        // Use approximate character width for text measurement
        // TODO: Implement proper text measurement when needed

        // For now, use a simple approximation based on character count
        // TODO: Implement proper glyph-based cursor positioning
        let relative_x = mouse_x - widget_left - 8.0; // Account for padding

        // Improved approximation using character analysis
        let avg_char_width = 16.0 * 0.6; // Use fixed font size for approximation
        let mut current_x = 0.0;

        for (i, c) in text.chars().enumerate() {
            let char_width = match c {
                'i' | 'l' | 'I' | '1' | '|' => avg_char_width * 0.5,
                'm' | 'M' | 'W' | 'w' => avg_char_width * 1.2,
                ' ' => avg_char_width * 0.8,
                '.' | ',' => avg_char_width * 0.4, // Punctuation
                '0'..='9' => avg_char_width * 0.8, // Numbers are typically narrower
                _ => avg_char_width,
            };

            if relative_x <= current_x + char_width / 2.0 {
                return i;
            }
            current_x += char_width;
        }

        // If we get here, the cursor is at the end
        text.chars().count()
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
        graphics: &mut dyn Graphics,
        theme: &mut dyn Theme,
        layout_node: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        // Update focus state
        if let Ok(manager) = info.focus_manager.lock() {
            self.focus_state = manager.get_focus_state(self.focus_id);
        }

        let is_focused = matches!(self.focus_state, FocusState::Focused | FocusState::Gained);

        // Get colors from theme with proper fallbacks
        let background_color = if is_focused {
            theme
                .get_property(
                    self.widget_id(),
                    &nptk_theme::properties::ThemeProperty::ColorBackgroundFocused,
                )
                .unwrap_or_else(|| Color::from_rgb8(100, 150, 255))
        } else {
            theme
                .get_property(
                    self.widget_id(),
                    &nptk_theme::properties::ThemeProperty::ColorBackground,
                )
                .unwrap_or_else(|| Color::from_rgb8(255, 255, 255))
        };

        let border_color = if !self.is_valid {
            theme
                .get_property(
                    self.widget_id(),
                    &nptk_theme::properties::ThemeProperty::ColorBorderError,
                )
                .unwrap_or_else(|| Color::from_rgb8(255, 0, 0)) // Red for error
        } else if is_focused {
            theme
                .get_property(
                    self.widget_id(),
                    &nptk_theme::properties::ThemeProperty::ColorBorderFocused,
                )
                .unwrap_or_else(|| Color::from_rgb8(100, 150, 255))
        } else {
            theme
                .get_property(
                    self.widget_id(),
                    &nptk_theme::properties::ThemeProperty::ColorBorder,
                )
                .unwrap_or_else(|| Color::from_rgb8(200, 200, 200)) // Light gray border
        };

        let text_color = theme
            .get_property(
                self.widget_id(),
                &nptk_theme::properties::ThemeProperty::ColorText,
            )
            .unwrap_or_else(|| Color::from_rgb8(0, 0, 0));

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
        graphics.fill(
            Fill::NonZero,
            Affine::default(),
            &Brush::Solid(background_color),
            None,
            &input_rect.to_path(0.1),
        );

        // Draw border
        graphics.stroke(
            &Stroke::new(if is_focused { 2.0 } else { 1.0 }),
            Affine::default(),
            &Brush::Solid(border_color),
            None,
            &input_rect.to_path(0.1),
        );

        // Render text content or placeholder
        let placeholder_text = self.placeholder.get();
        let display_text = if self.buffer.text().is_empty() {
            placeholder_text.deref()
        } else {
            self.buffer.text()
        };

        let font_size = 16.0f32; // Match TextInput font size
                                 // Use approximate character width for text measurement
                                 // TODO: Implement proper text measurement when needed

        // TODO: Fix the FileRef lifetime issue
        // let location = font_ref.axes().location::<&[VariationSetting; 0]>(&[]);
        // let glyph_metrics = font_ref.glyph_metrics(Size::new(font_size), &location);
        // let charmap = font_ref.charmap();

        // Render text selection highlight if focused and has selection (same as TextInput)
        if let Some(selection_range) = self.buffer.cursor().selection() {
            // Use a very visible selection color
            let selection_color = theme
                .get_property(
                    self.widget_id(),
                    &nptk_theme::properties::ThemeProperty::ColorSelection,
                )
                .unwrap_or_else(|| Color::from_rgb8(255, 100, 100)); // Bright red for maximum visibility

            // Calculate selection bounds using the same method as cursor positioning
            let selection_start_x =
                self.cursor_x_position(selection_range.start, layout_node, info);
            let selection_end_x = self.cursor_x_position(selection_range.end, layout_node, info);

            // Only draw selection if there's actually a range (start != end)
            if selection_range.start != selection_range.end {
                // Draw selection background
                graphics.fill(
                    Fill::NonZero,
                    Affine::default(),
                    &Brush::Solid(selection_color),
                    None,
                    &Rect::new(
                        selection_start_x as f64,
                        layout_node.layout.location.y as f64 + 4.0,
                        selection_end_x as f64,
                        layout_node.layout.location.y as f64
                            + layout_node.layout.size.height as f64
                            - 4.0,
                    )
                    .to_path(0.1),
                );
            }
        }

        if !display_text.is_empty() {
            // Use the TextRenderContext for proper text rendering
            let text_color = if self.buffer.text().is_empty() {
                theme
                    .get_property(
                        self.widget_id(),
                        &nptk_theme::properties::ThemeProperty::ColorPlaceholder,
                    )
                    .unwrap_or_else(|| Color::from_rgb8(150, 150, 150))
            } else {
                text_color
            };

            // Render text using TextRenderContext (same positioning as TextInput)
            let transform = nptk_core::vg::kurbo::Affine::translate((
                layout_node.layout.location.x as f64 + 8.0, // Left padding
                layout_node.layout.location.y as f64 + 4.5, // Position text within the input field (same as TextInput)
            ));

            self.text_render_context.render_text(
                &mut info.font_context,
                graphics,
                &display_text,
                None,      // No specific font, use default
                font_size, // Use the font_size variable
                Brush::Solid(text_color),
                transform,
                true, // Hinting
                None, // Value input handles its own scrolling/clipping
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
                let cursor_color = theme
                    .get_property(
                        self.widget_id(),
                        &nptk_theme::properties::ThemeProperty::ColorCursor,
                    )
                    .unwrap_or_else(|| Color::BLACK);

                // Calculate cursor position using the same method as TextInput
                let cursor_pos = self.buffer.cursor().position;
                let cursor_x = self.cursor_x_position(cursor_pos, layout_node, info);

                let cursor_y = layout_node.layout.location.y + 4.0;
                let cursor_height = layout_node.layout.size.height - 8.0;

                graphics.stroke(
                    &Stroke::new(1.0),
                    Affine::default(),
                    &Brush::Solid(cursor_color),
                    None,
                    &Rect::new(
                        cursor_x as f64,
                        cursor_y as f64,
                        cursor_x as f64,
                        (cursor_y + cursor_height) as f64,
                    )
                    .to_path(0.1),
                );
            }
        }
    }

    fn update(&mut self, layout: &LayoutNode, context: AppContext, info: &mut AppInfo) -> Update {
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
                    let filtered_text: String = text
                        .chars()
                        .filter(|ch| {
                            ch.is_ascii_digit() || *ch == '.' || (self.allow_negative && *ch == '-')
                        })
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
                        },
                        PhysicalKey::Code(KeyCode::Delete) => {
                            self.buffer.delete_forward();
                            text_changed = true;
                            self.cursor_blink_timer = Instant::now();
                            self.cursor_visible = true;
                        },
                        PhysicalKey::Code(KeyCode::ArrowLeft) => {
                            self.buffer.move_left(shift_pressed);
                            self.cursor_blink_timer = Instant::now();
                            self.cursor_visible = true;
                            update |= Update::DRAW;
                        },
                        PhysicalKey::Code(KeyCode::ArrowRight) => {
                            self.buffer.move_right(shift_pressed);
                            self.cursor_blink_timer = Instant::now();
                            self.cursor_visible = true;
                            update |= Update::DRAW;
                        },
                        PhysicalKey::Code(KeyCode::ArrowUp) => {
                            if !shift_pressed {
                                self.increment();
                                text_changed = true;
                                update |= Update::DRAW;
                            }
                        },
                        PhysicalKey::Code(KeyCode::ArrowDown) => {
                            if !shift_pressed {
                                self.decrement();
                                text_changed = true;
                                update |= Update::DRAW;
                            }
                        },
                        PhysicalKey::Code(KeyCode::Home) => {
                            self.buffer.move_to_start(shift_pressed);
                            self.cursor_blink_timer = Instant::now();
                            self.cursor_visible = true;
                            update |= Update::DRAW;
                        },
                        PhysicalKey::Code(KeyCode::End) => {
                            self.buffer.move_to_end(shift_pressed);
                            self.cursor_blink_timer = Instant::now();
                            self.cursor_visible = true;
                            update |= Update::DRAW;
                        },
                        PhysicalKey::Code(KeyCode::KeyA) if ctrl_pressed => {
                            // Select all text
                            let text_len = self.buffer.text().len();
                            self.buffer.cursor.selection_start = Some(0);
                            self.buffer.cursor.position = text_len;
                            self.cursor_blink_timer = Instant::now();
                            self.cursor_visible = true;
                            update |= Update::DRAW;
                        },
                        PhysicalKey::Code(KeyCode::KeyC) if ctrl_pressed => {
                            // TODO: Copy to clipboard
                            if let Some(selected) = self.buffer.selected_text() {
                                println!("Copy: {}", selected);
                            }
                        },
                        PhysicalKey::Code(KeyCode::KeyV) if ctrl_pressed => {
                            // TODO: Paste from clipboard
                            println!("Paste requested");
                        },
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
                        },
                        PhysicalKey::Code(KeyCode::Enter) => {
                            // Validate and finalize input
                            self.validate_and_update();
                            self.sync_text_from_value();
                            update |= Update::DRAW;
                        },
                        _ => {
                            // Character input is handled via IME events above
                            if let Some(text) = &key_event.text {
                                let clean_text: String = text
                                    .chars()
                                    .filter(|c| {
                                        c.is_ascii_digit()
                                            || (*c == '.' && self.decimal_places > 0)
                                            || (*c == '-' && self.allow_negative)
                                    })
                                    .collect();

                                if !clean_text.is_empty() {
                                    self.buffer.insert(&clean_text);
                                    text_changed = true;
                                    self.cursor_blink_timer = Instant::now();
                                    self.cursor_visible = true;
                                }
                            }
                        },
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
                                if matches!(
                                    self.focus_state,
                                    FocusState::Focused | FocusState::Gained
                                ) {
                                    // Set focus first
                                    context.set_focus(Some(self.focus_id));

                                    // Handle mouse click in bounds
                                    let click_pos = self.cursor_position_from_mouse_simple(
                                        cursor_pos.x as f32,
                                        layout,
                                    );

                                    // Check for double-click first
                                    if self.handle_double_click(click_pos, layout) {
                                        // Double-click handled - selection already set, don't modify cursor position or drag
                                        self.mouse_down = true;
                                        // Don't set drag_start_pos for double-click to avoid interfering with selection
                                        update |= Update::DRAW;
                                    } else {
                                        // Single click - clear selection and set cursor position
                                        self.buffer.cursor.selection_start = None;
                                        self.buffer.cursor.position = click_pos;
                                        self.mouse_down = true;
                                        self.drag_start_pos = Some(click_pos);

                                        // Reset cursor blink
                                        self.cursor_blink_timer = Instant::now();
                                        self.cursor_visible = true;
                                        update |= Update::DRAW;
                                    }
                                }
                            },
                            ElementState::Released => {
                                // Always handle mouse release, regardless of bounds
                                self.mouse_down = false;
                                self.drag_start_pos = None;
                            },
                        }
                    }
                }

                // Handle mouse drag for selection (when in bounds and dragging)
                if self.mouse_down
                    && matches!(self.focus_state, FocusState::Focused | FocusState::Gained)
                {
                    if let Some(start_pos) = self.drag_start_pos {
                        let current_pos =
                            self.cursor_position_from_mouse_simple(cursor_pos.x as f32, layout);

                        if current_pos != self.buffer.cursor.position {
                            // Update selection
                            self.buffer.cursor.selection_start = Some(start_pos);
                            self.buffer.cursor.position = current_pos;
                            update |= Update::DRAW;
                        }
                    }
                }
            } else if self.mouse_down
                && matches!(self.focus_state, FocusState::Focused | FocusState::Gained)
            {
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
                            &info.font_context,
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
        } else if self.mouse_down
            && matches!(self.focus_state, FocusState::Focused | FocusState::Gained)
        {
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
