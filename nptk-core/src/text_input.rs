use crate::app::info::AppKeyEvent;
use std::ops::Range;
use winit::keyboard::{KeyCode, PhysicalKey};

/// Represents a text cursor position and selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextCursor {
    /// Current cursor position (in characters).
    pub position: usize,
    /// Selection start position (if any).
    pub selection_start: Option<usize>,
}

impl TextCursor {
    /// Create a new cursor at the given position.
    pub fn new(position: usize) -> Self {
        Self {
            position,
            selection_start: None,
        }
    }

    /// Create a cursor with a selection.
    pub fn with_selection(start: usize, end: usize) -> Self {
        Self {
            position: end,
            selection_start: Some(start),
        }
    }

    /// Get the selection range if there is one.
    pub fn selection(&self) -> Option<Range<usize>> {
        self.selection_start.map(|start| {
            if start <= self.position {
                start..self.position
            } else {
                self.position..start
            }
        })
    }

    /// Check if there is a selection.
    pub fn has_selection(&self) -> bool {
        self.selection_start.is_some()
    }

    /// Clear the selection, keeping the cursor position.
    pub fn clear_selection(&mut self) {
        self.selection_start = None;
    }

    /// Move cursor to a new position, clearing selection.
    pub fn move_to(&mut self, position: usize) {
        self.position = position;
        self.selection_start = None;
    }

    /// Move cursor to a new position, extending selection.
    pub fn move_to_with_selection(&mut self, position: usize) {
        if self.selection_start.is_none() {
            self.selection_start = Some(self.position);
        }
        self.position = position;
    }
}

/// Text editing operations that can be performed on a text buffer.
#[derive(Debug, Clone)]
pub enum TextEditOp {
    /// Insert text at the current cursor position.
    Insert(String),
    /// Delete text in the given range.
    Delete(Range<usize>),
    /// Replace text in the given range with new text.
    Replace(Range<usize>, String),
    /// Move cursor to a new position.
    MoveCursor(usize),
    /// Set selection.
    SetSelection(Range<usize>),
    /// Clear selection.
    ClearSelection,
}

/// A text buffer that supports editing operations with undo/redo.
#[derive(Debug, Clone)]
pub struct TextBuffer {
    /// The text content.
    text: String,
    /// Current cursor state.
    pub cursor: TextCursor,
    /// History of operations for undo/redo.
    #[allow(dead_code)]
    history: Vec<TextEditOp>,
    /// Current position in history.
    #[allow(dead_code)]
    history_position: usize,
}

impl TextBuffer {
    /// Create a new empty text buffer.
    pub fn new() -> Self {
        Self {
            text: String::new(),
            cursor: TextCursor::new(0),
            history: Vec::new(),
            history_position: 0,
        }
    }

    /// Create a text buffer with initial text.
    pub fn with_text(text: String) -> Self {
        let cursor_pos = text.len();
        Self {
            text,
            cursor: TextCursor::new(cursor_pos),
            history: Vec::new(),
            history_position: 0,
        }
    }

    /// Get the current text content.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Get the current cursor state.
    pub fn cursor(&self) -> TextCursor {
        self.cursor
    }

    /// Insert text at the current cursor position.
    pub fn insert(&mut self, text: &str) {
        if let Some(selection) = self.cursor.selection() {
            if self.replace_chars(selection.clone(), text).is_some() {
                self.cursor
                    .move_to(selection.start + text.chars().count());
            }
            return;
        }

        let pos = self.cursor.position;
        if self.replace_chars(pos..pos, text).is_some() {
            self.cursor.move_to(pos + text.chars().count());
        }
    }

    /// Delete the character before the cursor (backspace).
    pub fn delete_backward(&mut self) {
        if let Some(selection) = self.cursor.selection() {
            if self.replace_chars(selection.clone(), "").is_some() {
                self.cursor.move_to(selection.start);
            }
            return;
        }

        if self.cursor.position == 0 {
            return;
        }

        let current_pos = self.cursor.position;
        let prev_pos = current_pos - 1;
        if self.replace_chars(prev_pos..current_pos, "").is_some() {
            self.cursor.move_to(prev_pos);
        }
    }

    /// Delete the character after the cursor (delete key).
    pub fn delete_forward(&mut self) {
        if let Some(selection) = self.cursor.selection() {
            if self.replace_chars(selection.clone(), "").is_some() {
                self.cursor.move_to(selection.start);
            }
            return;
        }

        let char_count = self.text.chars().count();
        if self.cursor.position >= char_count {
            return;
        }

        let current_pos = self.cursor.position;
        let target = current_pos + 1;
        let _ = self.replace_chars(current_pos..target, "");
    }

    /// Move cursor left by one character.
    pub fn move_left(&mut self, extend_selection: bool) {
        if self.cursor.position > 0 {
            let new_pos = self.cursor.position - 1;
            if extend_selection {
                self.cursor.move_to_with_selection(new_pos);
            } else {
                self.cursor.move_to(new_pos);
            }
        }
    }

    /// Move cursor right by one character.
    pub fn move_right(&mut self, extend_selection: bool) {
        let char_count = self.text.chars().count();
        if self.cursor.position < char_count {
            let new_pos = self.cursor.position + 1;
            if extend_selection {
                self.cursor.move_to_with_selection(new_pos);
            } else {
                self.cursor.move_to(new_pos);
            }
        }
    }

    /// Move cursor to the beginning of the text.
    pub fn move_to_start(&mut self, extend_selection: bool) {
        if extend_selection {
            self.cursor.move_to_with_selection(0);
        } else {
            self.cursor.move_to(0);
        }
    }

    /// Move cursor to the end of the text.
    pub fn move_to_end(&mut self, extend_selection: bool) {
        let char_count = self.text.chars().count();
        if extend_selection {
            self.cursor.move_to_with_selection(char_count);
        } else {
            self.cursor.move_to(char_count);
        }
    }

    /// Get the selected text (if any).
    pub fn selected_text(&self) -> Option<String> {
        self.cursor.selection().map(|range| {
            self.text
                .chars()
                .skip(range.start)
                .take(range.end - range.start)
                .collect()
        })
    }

    /// Set the entire text content.
    pub fn set_text(&mut self, text: String) {
        self.text = text;
        let char_count = self.text.chars().count();
        self.cursor.move_to(char_count.min(self.cursor.position));
    }

    /// Clear all text.
    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor.move_to(0);
    }

    /// Convert a character-based range to a byte-based range.
    fn char_range_to_byte_range(&self, range: &Range<usize>) -> Option<Range<usize>> {
        let start_byte = self.char_index_to_byte(range.start)?;
        let end_byte = self.char_index_to_byte(range.end)?;
        Some(start_byte..end_byte)
    }

    fn char_index_to_byte(&self, idx: usize) -> Option<usize> {
        let mut iter = self.text.char_indices().map(|(i, _)| i);
        if idx == self.text.chars().count() {
            Some(self.text.len())
        } else {
            iter.nth(idx)
        }
    }

    fn replace_chars(&mut self, range: Range<usize>, with: &str) -> Option<()> {
        let byte_range = self.char_range_to_byte_range(&range)?;
        self.text.replace_range(byte_range, with);
        Some(())
    }
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Processes keyboard events and converts them to text editing operations.
pub struct TextInputProcessor;

impl TextInputProcessor {
    /// Process a keyboard event and return text editing operations.
    pub fn process_key_event(event: &AppKeyEvent, buffer: &TextBuffer) -> Vec<TextEditOp> {
        let mut ops = Vec::new();

        if event.state != winit::event::ElementState::Pressed {
            return ops;
        }

        match event.physical_key {
            PhysicalKey::Code(KeyCode::Backspace) => {
                ops.push(TextEditOp::Delete(if buffer.cursor().has_selection() {
                    buffer.cursor().selection().unwrap()
                } else if buffer.cursor().position > 0 {
                    (buffer.cursor().position - 1)..buffer.cursor().position
                } else {
                    return ops; // Nothing to delete
                }));
            },

            PhysicalKey::Code(KeyCode::Delete) => {
                ops.push(TextEditOp::Delete(if buffer.cursor().has_selection() {
                    buffer.cursor().selection().unwrap()
                } else {
                    let char_count = buffer.text().chars().count();
                    if buffer.cursor().position < char_count {
                        buffer.cursor().position..(buffer.cursor().position + 1)
                    } else {
                        return ops; // Nothing to delete
                    }
                }));
            },

            PhysicalKey::Code(KeyCode::ArrowLeft) => {
                // TODO: Handle Shift modifier for selection
                ops.push(TextEditOp::MoveCursor(if buffer.cursor().position > 0 {
                    buffer.cursor().position - 1
                } else {
                    0
                }));
            },

            PhysicalKey::Code(KeyCode::ArrowRight) => {
                // TODO: Handle Shift modifier for selection
                let char_count = buffer.text().chars().count();
                ops.push(TextEditOp::MoveCursor(
                    if buffer.cursor().position < char_count {
                        buffer.cursor().position + 1
                    } else {
                        char_count
                    },
                ));
            },

            PhysicalKey::Code(KeyCode::Home) => {
                // TODO: Handle Shift modifier for selection
                ops.push(TextEditOp::MoveCursor(0));
            },

            PhysicalKey::Code(KeyCode::End) => {
                // TODO: Handle Shift modifier for selection
                let char_count = buffer.text().chars().count();
                ops.push(TextEditOp::MoveCursor(char_count));
            },

            _ => {
                // Handle character input
                if let Some(text) = &event.text {
                    if !text.chars().any(|c| c.is_control()) {
                        ops.push(TextEditOp::Insert(text.clone()));
                    }
                }
            },
        }

        ops
    }
}
