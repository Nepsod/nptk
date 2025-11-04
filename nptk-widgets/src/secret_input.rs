use nptk_core::app::context::AppContext;
use nptk_core::app::focus::{FocusId, FocusState, FocusableWidget, FocusProperties, FocusBounds};
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{Dimension, LayoutNode, LayoutStyle, LengthPercentage, StyleNode};
use nptk_core::signal::MaybeSignal;
use nptk_core::text_input::TextBuffer;
use nptk_core::vg::kurbo::{Affine, Circle, Line, Rect, RoundedRect, RoundedRectRadii, Shape, Stroke};
use nptk_core::vg::peniko::{Brush, Color, Fill};
use nptk_core::vgi::Graphics;
use nptk_core::text_render::TextRenderContext;
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
    last_click_time: Instant,
    last_click_pos: Option<usize>,
    mask_char: char,
    show_password: bool,
    toggle_button_hovered: bool,
    toggle_button_pressed: bool,
    text_render_context: TextRenderContext,
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
            last_click_time: Instant::now(),
            last_click_pos: None,
            mask_char: '•',
            show_password: false,
            toggle_button_hovered: false,
            toggle_button_pressed: false,
            text_render_context: TextRenderContext::new(),
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

    /// Get the bounds of the toggle button.
    fn get_toggle_button_bounds(&self, layout: &LayoutNode) -> Rect {
        let button_size = 20.0;
        let margin = 4.0;
        
        Rect::new(
            (layout.layout.location.x + layout.layout.size.width - button_size - margin) as f64,
            (layout.layout.location.y + (layout.layout.size.height - button_size) / 2.0) as f64,
            (layout.layout.location.x + layout.layout.size.width - margin) as f64,
            (layout.layout.location.y + (layout.layout.size.height + button_size) / 2.0) as f64,
        )
    }

    /// Check if a point is within the toggle button bounds.
    fn is_in_toggle_button(&self, x: f64, y: f64, layout: &LayoutNode) -> bool {
        let bounds = self.get_toggle_button_bounds(layout);
        x >= bounds.x0 && x <= bounds.x1 && y >= bounds.y0 && y <= bounds.y1
    }

    /// Get the masked version of the text for display.
    fn masked_text(&self) -> String {
        if self.show_password {
            self.buffer.text().to_string()
        } else {
            self.mask_char.to_string().repeat(self.buffer.text().chars().count())
        }
    }

    /// Calculate the actual width of text using Parley's font metrics
    fn calculate_text_width(&self, text: &str, font_size: f32, info: &mut AppInfo) -> f32 {
        if text.is_empty() {
            return 0.0;
        }
        
        // For secret input, we need to use the masked text (dots) for cursor positioning
        // when the password is hidden, but use actual text when it's shown
        let display_text = if self.show_password {
            text.to_string()
        } else {
            // Use dots for each character in the actual text
            "•".repeat(text.chars().count())
        };
        
        // Use TextRenderContext to get accurate measurements from Parley
        // This handles all Unicode characters, emojis, and different scripts properly
        self.text_render_context.measure_text_width(&mut info.font_context, &display_text, font_size)
    }

    /// Calculate the X position of the cursor based on its character position.
    fn cursor_x_position(&self, cursor_pos: usize, layout_node: &LayoutNode, info: &mut AppInfo) -> f32 {
        let font_size = 16.0;
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
    fn cursor_position_from_mouse(&self, mouse_x: f32, layout_node: &LayoutNode, info: &mut AppInfo) -> usize {
        let font_size = 16.0;
        let text_start_x = layout_node.layout.location.x + 8.0; // Padding
        let relative_x = mouse_x - text_start_x;

        if relative_x <= 0.0 {
            return 0;
        }

        let actual_text = self.buffer.text();
        
        // Find the character position by calculating cumulative text widths
        let mut current_width = 0.0;
        let mut char_position = 0;
        
        for (i, c) in actual_text.chars().enumerate() {
            let char_text = c.to_string();
            let char_width = self.calculate_text_width(&char_text, font_size, info);
            
            if relative_x <= current_width + char_width / 2.0 {
                return i;
            }
            
            current_width += char_width;
            char_position = i + 1;
        }
        
        char_position
    }

    /// Find word boundaries around a given position.
    fn find_word_boundaries(&self, pos: usize) -> (usize, usize) {
        let text = self.buffer.text();
        if text.is_empty() {
            return (0, 0);
        }

        let chars: Vec<char> = text.chars().collect();
        let pos = pos.min(chars.len());

        // If we're at the end of the text, return empty boundaries
        if pos >= chars.len() {
            return (pos, pos);
        }

        // If the character at the current position is not alphanumeric (space, punctuation, etc.)
        // then we're in empty space between words
        if !chars[pos].is_alphanumeric() {
            return (pos, pos);
        }

        // Find start of word (go backwards until we hit a non-word character)
        let mut start = pos;
        while start > 0 && chars[start - 1].is_alphanumeric() {
            start -= 1;
        }

        // Find end of word (go forwards until we hit a non-word character)
        let mut end = pos;
        while end < chars.len() && chars[end].is_alphanumeric() {
            end += 1;
        }

        (start, end)
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
                // For security reasons, when password is hidden, only allow "select all"
                // Word selection could reveal information about password structure
                if !self.show_password {
                    // Password is hidden - always select all text
                    let text_len = text.chars().count();
                    self.buffer.cursor.selection_start = Some(0);
                    self.buffer.cursor.position = text_len;
                } else {
                    // Password is visible - allow normal word selection
                    let (start, end) = self.find_word_boundaries(click_pos);
                    
                    if start == end {
                        // Clicked in empty space - select all text
                        let text_len = text.chars().count();
                        self.buffer.cursor.selection_start = Some(0);
                        self.buffer.cursor.position = text_len;
                    } else {
                        // Clicked on a word - select that word
                        self.buffer.cursor.selection_start = Some(start);
                        self.buffer.cursor.position = end;
                    }
                }
            }
            
            self.cursor_blink_timer = Instant::now();
            self.cursor_visible = true;
            return true;
        }

        false
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
        let bg_color = theme.get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorBackground)
            .unwrap_or_else(|| Color::from_rgb8(255, 255, 255));

        let border_color = if is_focused {
            theme.get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorBorderFocused)
                .unwrap_or_else(|| Color::from_rgb8(100, 150, 255))
        } else {
            theme.get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorBorder)
                .unwrap_or_else(|| Color::from_rgb8(200, 200, 200)) // Light gray border
        };

        let _text_color = theme.get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorText)
            .unwrap_or_else(|| Color::from_rgb8(0, 0, 0));

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
            &Brush::Solid(bg_color),
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

        // Render masked text content or placeholder
        let placeholder_text = self.placeholder.get();
        let display_text = if self.buffer.text().is_empty() {
            placeholder_text.deref()
        } else {
            &self.masked_text()
        };

        let font_size = 16.0;
        

        // Render selection highlight first (behind text)
        if let Some(selection_range) = self.buffer.cursor().selection() {
            // Use a very visible selection color
            let selection_color = theme.get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorSelection)
                .unwrap_or_else(|| Color::from_rgb8(255, 100, 100)); // Bright red for maximum visibility

            // Calculate selection bounds using the same method as cursor positioning
            let selection_start_x = self.cursor_x_position(selection_range.start, layout_node, info);
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
                        layout_node.layout.location.y as f64 + layout_node.layout.size.height as f64 - 4.0,
                    ).to_path(0.1),
                );
            }
        }

        if !display_text.is_empty() {
            let text_color = if self.buffer.text().is_empty() {
                Color::from_rgb8(150, 150, 150) // Placeholder color
            } else {
                _text_color
            };

            // Use Parley for proper text rendering
            let transform = Affine::translate((
                layout_node.layout.location.x as f64 + 8.0, // Padding
                layout_node.layout.location.y as f64 + 4.5, // Position text within the input field
            ));
            
            self.text_render_context.render_text(
                &mut info.font_context,
                graphics,
                &display_text,
                None, // No specific font, use default (same as TextInput)
                font_size,
                Brush::Solid(text_color),
                transform,
                true, // hint
            );
        }

        // Render cursor when focused and visible
        if is_focused && self.cursor_visible {
            let cursor_color = theme.get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorCursor)
                .unwrap_or_else(|| Color::BLACK);

            // Calculate cursor position using the same method as mouse positioning
            let cursor_pos = self.buffer.cursor().position;
            let cursor_x = self.cursor_x_position(cursor_pos, layout_node, info);

            // Draw cursor line
                graphics.stroke(
                &Stroke::new(1.0),
                Affine::default(),
                &Brush::Solid(cursor_color),
                None,
                &Line::new(
                    (cursor_x as f64, layout_node.layout.location.y as f64 + 4.0),
                    (cursor_x as f64, layout_node.layout.location.y as f64 + layout_node.layout.size.height as f64 - 4.0),
                ).to_path(0.1),
            );
        }

        // Draw toggle button (eye icon)
        let toggle_bounds = self.get_toggle_button_bounds(layout_node);
        let toggle_center_x = (toggle_bounds.x0 + toggle_bounds.x1) / 2.0;
        let toggle_center_y = (toggle_bounds.y0 + toggle_bounds.y1) / 2.0;
        
        // Toggle button background
        let toggle_bg_color = if self.toggle_button_pressed {
            Color::from_rgb8(200, 200, 200)
        } else if self.toggle_button_hovered {
            Color::from_rgb8(240, 240, 240)
        } else {
            Color::from_rgb8(250, 250, 250)
        };
        
        graphics.fill(
            Fill::NonZero,
            Affine::default(),
            &Brush::Solid(toggle_bg_color),
            None,
            &RoundedRect::from_rect(toggle_bounds, RoundedRectRadii::from_single_radius(4.0)).to_path(0.1),
        );

        // Draw eye icon (simplified)
        let eye_color = if self.show_password {
            Color::from_rgb8(0, 120, 255) // Blue when showing password
        } else {
            Color::from_rgb8(100, 100, 100) // Gray when hiding password
        };

        // Draw eye outline (circle)
        let eye_radius = 6.0;
        let eye_circle = Circle::new((toggle_center_x, toggle_center_y), eye_radius);
        graphics.stroke(
            &Stroke::new(1.5),
            Affine::default(),
            &Brush::Solid(eye_color),
            None,
            &eye_circle.to_path(0.1),
        );

        // Draw eye pupil (inner circle)
        if self.show_password {
            let pupil_radius = 2.5;
            let pupil_circle = Circle::new((toggle_center_x, toggle_center_y), pupil_radius);
            graphics.fill(
                Fill::NonZero,
                Affine::default(),
                &Brush::Solid(eye_color),
                None,
                &pupil_circle.to_path(0.1),
            );
        } else {
            // Draw slash through eye when hidden
            let slash_length = 8.0;
            graphics.stroke(
                &Stroke::new(1.5),
                Affine::default(),
                &Brush::Solid(eye_color),
                None,
                &Line::new(
                    (toggle_center_x - slash_length / 2.0, toggle_center_y - slash_length / 2.0),
                    (toggle_center_x + slash_length / 2.0, toggle_center_y + slash_length / 2.0),
                ).to_path(0.1),
            );
        }
    }

    fn layout_style(&self) -> StyleNode {
        StyleNode {
            style: self.layout_style.get().clone(),
            children: Vec::new(),
        }
    }

    fn update(&mut self, layout: &LayoutNode, context: AppContext, info: &mut AppInfo) -> Update {
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
                                if let Some(_selected_text) = self.buffer.selected_text() {
                                    // Copy password (hidden for security)
                                }
                            }
                            PhysicalKey::Code(KeyCode::KeyX) => {
                                if let Some(_) = self.buffer.selected_text() {
                                    // Cut password (hidden for security)
                                    self.buffer.delete_backward();
                                    text_changed = true;
                                }
                            }
                            PhysicalKey::Code(KeyCode::KeyV) => {
                                // Paste to password field
                                // TODO: Implement clipboard paste
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
                            PhysicalKey::Code(KeyCode::ArrowUp) => {
                                // Move cursor to beginning of text (like most toolkits)
                                self.buffer.move_to_start(info.modifiers.shift_key());
                                self.cursor_blink_timer = Instant::now();
                                self.cursor_visible = true;
                                update |= Update::DRAW;
                            }
                            PhysicalKey::Code(KeyCode::ArrowDown) => {
                                // Move cursor to end of text (like most toolkits)
                                self.buffer.move_to_end(info.modifiers.shift_key());
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
        let cursor_pos = info.cursor_pos;
        let button_events: Vec<_> = info.buttons.iter().collect();
        
        // Process mouse events in a separate scope to avoid borrowing conflicts
        {
            if let Some(cursor_pos) = cursor_pos {
            let in_bounds = cursor_pos.x as f32 >= layout.layout.location.x
                && cursor_pos.x as f32 <= layout.layout.location.x + layout.layout.size.width
                && cursor_pos.y as f32 >= layout.layout.location.y
                && cursor_pos.y as f32 <= layout.layout.location.y + layout.layout.size.height;

            // Check if mouse is over toggle button
            let in_toggle_button = self.is_in_toggle_button(cursor_pos.x, cursor_pos.y, layout);
            
            // Update toggle button hover state
            let old_toggle_hovered = self.toggle_button_hovered;
            self.toggle_button_hovered = in_toggle_button;
            if old_toggle_hovered != self.toggle_button_hovered {
                update |= Update::DRAW;
            }

            // Handle mouse button events
            for (_, button, state) in button_events {
                if *button == nptk_core::window::MouseButton::Left {
                    match state {
                        nptk_core::window::ElementState::Pressed => {
                            if in_bounds {
                                if in_toggle_button {
                                    // Handle toggle button press
                                    self.toggle_button_pressed = true;
                                    update |= Update::DRAW;
                                } else {
                                    // Set focus first
                                    context.set_focus(Some(self.focus_id));
                                    
                                    // Handle text area click
                                    let click_pos = self.cursor_position_from_mouse_simple(cursor_pos.x as f32, layout);
                                    
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
                                        self.cursor_blink_timer = Instant::now();
                                        self.cursor_visible = true;
                                        update |= Update::DRAW;
                                    }
                                }
                            }
                        }
                        nptk_core::window::ElementState::Released => {
                            if self.toggle_button_pressed && in_bounds && in_toggle_button {
                                // Toggle password visibility
                                self.toggle_password_visibility();
                                update |= Update::DRAW;
                            }
                            self.toggle_button_pressed = false;
                            self.mouse_down = false;
                            self.drag_start_pos = None;
                        }
                    }
                }
            }

            // Handle mouse drag for selection (works both in and out of bounds)
            if self.mouse_down {
                if let Some(start_pos) = self.drag_start_pos {
                    let current_pos = if in_bounds {
                        self.cursor_position_from_mouse(cursor_pos.x as f32, layout, info)
                    } else {
                        // Mouse is outside bounds - extend selection to beginning or end
                        let text_len = self.buffer.text().chars().count();
                        let widget_left = layout.layout.location.x;
                        let widget_right = layout.layout.location.x + layout.layout.size.width;
                        
                        if (cursor_pos.x as f32) < widget_left {
                            0
                        } else if (cursor_pos.x as f32) > widget_right {
                            text_len
                        } else {
                            // This shouldn't happen if in_bounds is false, but just in case
                            self.cursor_position_from_mouse(cursor_pos.x as f32, layout, info)
                        }
                    };
                    
                    if current_pos != self.buffer.cursor().position {
                        self.buffer.cursor.selection_start = Some(start_pos);
                        self.buffer.cursor.position = current_pos;
                        update |= Update::DRAW;
                    }
                }
            }
        } else {
            // Mouse left widget area - reset toggle button states
            if self.toggle_button_hovered {
                self.toggle_button_hovered = false;
                update |= Update::DRAW;
            }
            
            if self.mouse_down && matches!(self.focus_state, FocusState::Focused | FocusState::Gained) {
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
        }

        // Handle global mouse release
        for (_, button, state) in &info.buttons {
            if *button == nptk_core::window::MouseButton::Left && *state == nptk_core::window::ElementState::Released {
                self.mouse_down = false;
                self.drag_start_pos = None;
                self.toggle_button_pressed = false;
            }
        }
        } // End of mouse handling scope

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
