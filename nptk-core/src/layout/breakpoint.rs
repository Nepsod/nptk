// SPDX-License-Identifier: LGPL-3.0-only

/// Breakpoint thresholds for responsive layouts.
///
/// Defines the width thresholds that determine when to switch between
/// different layout modes (small, medium, large).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Breakpoints {
    /// Width threshold for small screens (< small is considered small).
    pub small: f32,
    /// Width threshold for medium screens (small <= width < medium is medium).
    pub medium: f32,
    /// Width threshold for large screens (>= medium is large).
    pub large: f32,
}

impl Default for Breakpoints {
    fn default() -> Self {
        Self {
            small: 600.0,
            medium: 1024.0,
            large: 1440.0,
        }
    }
}

impl Breakpoints {
    /// Create custom breakpoints.
    pub fn new(small: f32, medium: f32, large: f32) -> Self {
        Self { small, medium, large }
    }

    /// Get the breakpoint for a given width.
    pub fn get_breakpoint(&self, width: f32) -> Breakpoint {
        if width < self.small {
            Breakpoint::Small
        } else if width < self.medium {
            Breakpoint::Medium
        } else {
            Breakpoint::Large
        }
    }
}

/// Represents a layout breakpoint size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Breakpoint {
    /// Small screen size (< 600px typically).
    Small,
    /// Medium screen size (600-1024px typically).
    Medium,
    /// Large screen size (> 1024px typically).
    Large,
}
