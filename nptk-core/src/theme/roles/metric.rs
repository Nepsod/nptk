// SPDX-License-Identifier: LGPL-3.0-only

//! Metric roles for integer theme properties.

/// Metric roles for integer theme properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetricRole {
    BorderThickness,
    BorderRadius,
    TitleHeight,
    TitleButtonWidth,
    TitleButtonHeight,
    TitleButtonInactiveAlpha,
}

crate::impl_role_string_conversion!(MetricRole, {
    BorderThickness => "BorderThickness",
    BorderRadius => "BorderRadius",
    TitleHeight => "TitleHeight",
    TitleButtonWidth => "TitleButtonWidth",
    TitleButtonHeight => "TitleButtonHeight",
    TitleButtonInactiveAlpha => "TitleButtonInactiveAlpha",
});

impl MetricRole {
    /// Get the default value for a metric role.
    pub fn default_value(&self) -> i32 {
        match self {
            MetricRole::BorderThickness => 4,
            MetricRole::BorderRadius => 0,
            MetricRole::TitleHeight => 19,
            MetricRole::TitleButtonWidth => 15,
            MetricRole::TitleButtonHeight => 15,
            MetricRole::TitleButtonInactiveAlpha => 255,
        }
    }
}
