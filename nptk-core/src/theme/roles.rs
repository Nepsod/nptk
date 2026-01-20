// SPDX-License-Identifier: LGPL-3.0-only

//! Theme role definitions.
//!
//! This module defines all the role enums used in the theme system.

use std::fmt;

/// Color roles for theme colors.
///
/// These roles represent semantic color purposes that widgets can use
/// to get appropriate colors from the theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorRole {
    // Window roles
    ActiveWindowBorder1,
    ActiveWindowBorder2,
    ActiveWindowTitle,
    ActiveWindowTitleShadow,
    ActiveWindowTitleStripes,
    InactiveWindowBorder1,
    InactiveWindowBorder2,
    InactiveWindowTitle,
    InactiveWindowTitleShadow,
    InactiveWindowTitleStripes,
    MovingWindowBorder1,
    MovingWindowBorder2,
    MovingWindowTitle,
    MovingWindowTitleShadow,
    MovingWindowTitleStripes,
    HighlightWindowBorder1,
    HighlightWindowBorder2,
    HighlightWindowTitle,
    HighlightWindowTitleShadow,
    HighlightWindowTitleStripes,

    // Widget roles
    Window,
    WindowText,
    Button,
    ButtonText,
    Base,
    BaseText,
    Selection,
    SelectionText,
    InactiveSelection,
    InactiveSelectionText,
    HoverHighlight,
    DisabledTextFront,
    DisabledTextBack,
    PlaceholderText,

    // Menu roles
    MenuBase,
    MenuBaseText,
    MenuSelection,
    MenuSelectionText,
    MenuStripe,

    // Link roles
    Link,
    ActiveLink,
    VisitedLink,

    // Syntax highlighting roles
    SyntaxComment,
    SyntaxKeyword,
    SyntaxControlKeyword,
    SyntaxString,
    SyntaxNumber,
    SyntaxType,
    SyntaxIdentifier,
    SyntaxFunction,
    SyntaxVariable,
    SyntaxCustomType,
    SyntaxNamespace,
    SyntaxMember,
    SyntaxParameter,
    SyntaxPreprocessorStatement,
    SyntaxPreprocessorValue,
    SyntaxPunctuation,
    SyntaxOperator,

    // Terminal colors
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
    ColorSchemeBackground,
    ColorSchemeForeground,

    // Other roles
    Accent,
    DesktopBackground,
    FocusOutline,
    TextCursor,
    ThreedHighlight,
    ThreedShadow1,
    ThreedShadow2,
    RubberBandFill,
    RubberBandBorder,
    Gutter,
    GutterBorder,
    Ruler,
    RulerBorder,
    RulerActiveText,
    RulerInactiveText,
    Tooltip,
    TooltipText,
    Tray,
    TrayText,
    OverlayBackground,
    OverlayText,
    HighlightSearching,
    HighlightSearchingText,
}

impl ColorRole {
    /// Get the string representation of the color role (for TOML keys).
    pub fn as_str(&self) -> &'static str {
        match self {
            ColorRole::ActiveWindowBorder1 => "ActiveWindowBorder1",
            ColorRole::ActiveWindowBorder2 => "ActiveWindowBorder2",
            ColorRole::ActiveWindowTitle => "ActiveWindowTitle",
            ColorRole::ActiveWindowTitleShadow => "ActiveWindowTitleShadow",
            ColorRole::ActiveWindowTitleStripes => "ActiveWindowTitleStripes",
            ColorRole::InactiveWindowBorder1 => "InactiveWindowBorder1",
            ColorRole::InactiveWindowBorder2 => "InactiveWindowBorder2",
            ColorRole::InactiveWindowTitle => "InactiveWindowTitle",
            ColorRole::InactiveWindowTitleShadow => "InactiveWindowTitleShadow",
            ColorRole::InactiveWindowTitleStripes => "InactiveWindowTitleStripes",
            ColorRole::MovingWindowBorder1 => "MovingWindowBorder1",
            ColorRole::MovingWindowBorder2 => "MovingWindowBorder2",
            ColorRole::MovingWindowTitle => "MovingWindowTitle",
            ColorRole::MovingWindowTitleShadow => "MovingWindowTitleShadow",
            ColorRole::MovingWindowTitleStripes => "MovingWindowTitleStripes",
            ColorRole::HighlightWindowBorder1 => "HighlightWindowBorder1",
            ColorRole::HighlightWindowBorder2 => "HighlightWindowBorder2",
            ColorRole::HighlightWindowTitle => "HighlightWindowTitle",
            ColorRole::HighlightWindowTitleShadow => "HighlightWindowTitleShadow",
            ColorRole::HighlightWindowTitleStripes => "HighlightWindowTitleStripes",
            ColorRole::Window => "Window",
            ColorRole::WindowText => "WindowText",
            ColorRole::Button => "Button",
            ColorRole::ButtonText => "ButtonText",
            ColorRole::Base => "Base",
            ColorRole::BaseText => "BaseText",
            ColorRole::Selection => "Selection",
            ColorRole::SelectionText => "SelectionText",
            ColorRole::InactiveSelection => "InactiveSelection",
            ColorRole::InactiveSelectionText => "InactiveSelectionText",
            ColorRole::HoverHighlight => "HoverHighlight",
            ColorRole::DisabledTextFront => "DisabledTextFront",
            ColorRole::DisabledTextBack => "DisabledTextBack",
            ColorRole::PlaceholderText => "PlaceholderText",
            ColorRole::MenuBase => "MenuBase",
            ColorRole::MenuBaseText => "MenuBaseText",
            ColorRole::MenuSelection => "MenuSelection",
            ColorRole::MenuSelectionText => "MenuSelectionText",
            ColorRole::MenuStripe => "MenuStripe",
            ColorRole::Link => "Link",
            ColorRole::ActiveLink => "ActiveLink",
            ColorRole::VisitedLink => "VisitedLink",
            ColorRole::SyntaxComment => "SyntaxComment",
            ColorRole::SyntaxKeyword => "SyntaxKeyword",
            ColorRole::SyntaxControlKeyword => "SyntaxControlKeyword",
            ColorRole::SyntaxString => "SyntaxString",
            ColorRole::SyntaxNumber => "SyntaxNumber",
            ColorRole::SyntaxType => "SyntaxType",
            ColorRole::SyntaxIdentifier => "SyntaxIdentifier",
            ColorRole::SyntaxFunction => "SyntaxFunction",
            ColorRole::SyntaxVariable => "SyntaxVariable",
            ColorRole::SyntaxCustomType => "SyntaxCustomType",
            ColorRole::SyntaxNamespace => "SyntaxNamespace",
            ColorRole::SyntaxMember => "SyntaxMember",
            ColorRole::SyntaxParameter => "SyntaxParameter",
            ColorRole::SyntaxPreprocessorStatement => "SyntaxPreprocessorStatement",
            ColorRole::SyntaxPreprocessorValue => "SyntaxPreprocessorValue",
            ColorRole::SyntaxPunctuation => "SyntaxPunctuation",
            ColorRole::SyntaxOperator => "SyntaxOperator",
            ColorRole::Black => "Black",
            ColorRole::Red => "Red",
            ColorRole::Green => "Green",
            ColorRole::Yellow => "Yellow",
            ColorRole::Blue => "Blue",
            ColorRole::Magenta => "Magenta",
            ColorRole::Cyan => "Cyan",
            ColorRole::White => "White",
            ColorRole::BrightBlack => "BrightBlack",
            ColorRole::BrightRed => "BrightRed",
            ColorRole::BrightGreen => "BrightGreen",
            ColorRole::BrightYellow => "BrightYellow",
            ColorRole::BrightBlue => "BrightBlue",
            ColorRole::BrightMagenta => "BrightMagenta",
            ColorRole::BrightCyan => "BrightCyan",
            ColorRole::BrightWhite => "BrightWhite",
            ColorRole::ColorSchemeBackground => "ColorSchemeBackground",
            ColorRole::ColorSchemeForeground => "ColorSchemeForeground",
            ColorRole::Accent => "Accent",
            ColorRole::DesktopBackground => "DesktopBackground",
            ColorRole::FocusOutline => "FocusOutline",
            ColorRole::TextCursor => "TextCursor",
            ColorRole::ThreedHighlight => "ThreedHighlight",
            ColorRole::ThreedShadow1 => "ThreedShadow1",
            ColorRole::ThreedShadow2 => "ThreedShadow2",
            ColorRole::RubberBandFill => "RubberBandFill",
            ColorRole::RubberBandBorder => "RubberBandBorder",
            ColorRole::Gutter => "Gutter",
            ColorRole::GutterBorder => "GutterBorder",
            ColorRole::Ruler => "Ruler",
            ColorRole::RulerBorder => "RulerBorder",
            ColorRole::RulerActiveText => "RulerActiveText",
            ColorRole::RulerInactiveText => "RulerInactiveText",
            ColorRole::Tooltip => "Tooltip",
            ColorRole::TooltipText => "TooltipText",
            ColorRole::Tray => "Tray",
            ColorRole::TrayText => "TrayText",
            ColorRole::OverlayBackground => "OverlayBackground",
            ColorRole::OverlayText => "OverlayText",
            ColorRole::HighlightSearching => "HighlightSearching",
            ColorRole::HighlightSearchingText => "HighlightSearchingText",
        }
    }

    /// Parse a color role from a string (for TOML parsing).
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "ActiveWindowBorder1" => Some(ColorRole::ActiveWindowBorder1),
            "ActiveWindowBorder2" => Some(ColorRole::ActiveWindowBorder2),
            "ActiveWindowTitle" => Some(ColorRole::ActiveWindowTitle),
            "ActiveWindowTitleShadow" => Some(ColorRole::ActiveWindowTitleShadow),
            "ActiveWindowTitleStripes" => Some(ColorRole::ActiveWindowTitleStripes),
            "InactiveWindowBorder1" => Some(ColorRole::InactiveWindowBorder1),
            "InactiveWindowBorder2" => Some(ColorRole::InactiveWindowBorder2),
            "InactiveWindowTitle" => Some(ColorRole::InactiveWindowTitle),
            "InactiveWindowTitleShadow" => Some(ColorRole::InactiveWindowTitleShadow),
            "InactiveWindowTitleStripes" => Some(ColorRole::InactiveWindowTitleStripes),
            "MovingWindowBorder1" => Some(ColorRole::MovingWindowBorder1),
            "MovingWindowBorder2" => Some(ColorRole::MovingWindowBorder2),
            "MovingWindowTitle" => Some(ColorRole::MovingWindowTitle),
            "MovingWindowTitleShadow" => Some(ColorRole::MovingWindowTitleShadow),
            "MovingWindowTitleStripes" => Some(ColorRole::MovingWindowTitleStripes),
            "HighlightWindowBorder1" => Some(ColorRole::HighlightWindowBorder1),
            "HighlightWindowBorder2" => Some(ColorRole::HighlightWindowBorder2),
            "HighlightWindowTitle" => Some(ColorRole::HighlightWindowTitle),
            "HighlightWindowTitleShadow" => Some(ColorRole::HighlightWindowTitleShadow),
            "HighlightWindowTitleStripes" => Some(ColorRole::HighlightWindowTitleStripes),
            "Window" => Some(ColorRole::Window),
            "WindowText" => Some(ColorRole::WindowText),
            "Button" => Some(ColorRole::Button),
            "ButtonText" => Some(ColorRole::ButtonText),
            "Base" => Some(ColorRole::Base),
            "BaseText" => Some(ColorRole::BaseText),
            "Selection" => Some(ColorRole::Selection),
            "SelectionText" => Some(ColorRole::SelectionText),
            "InactiveSelection" => Some(ColorRole::InactiveSelection),
            "InactiveSelectionText" => Some(ColorRole::InactiveSelectionText),
            "HoverHighlight" => Some(ColorRole::HoverHighlight),
            "DisabledTextFront" => Some(ColorRole::DisabledTextFront),
            "DisabledTextBack" => Some(ColorRole::DisabledTextBack),
            "PlaceholderText" => Some(ColorRole::PlaceholderText),
            "MenuBase" => Some(ColorRole::MenuBase),
            "MenuBaseText" => Some(ColorRole::MenuBaseText),
            "MenuSelection" => Some(ColorRole::MenuSelection),
            "MenuSelectionText" => Some(ColorRole::MenuSelectionText),
            "MenuStripe" => Some(ColorRole::MenuStripe),
            "Link" => Some(ColorRole::Link),
            "ActiveLink" => Some(ColorRole::ActiveLink),
            "VisitedLink" => Some(ColorRole::VisitedLink),
            "SyntaxComment" => Some(ColorRole::SyntaxComment),
            "SyntaxKeyword" => Some(ColorRole::SyntaxKeyword),
            "SyntaxControlKeyword" => Some(ColorRole::SyntaxControlKeyword),
            "SyntaxString" => Some(ColorRole::SyntaxString),
            "SyntaxNumber" => Some(ColorRole::SyntaxNumber),
            "SyntaxType" => Some(ColorRole::SyntaxType),
            "SyntaxIdentifier" => Some(ColorRole::SyntaxIdentifier),
            "SyntaxFunction" => Some(ColorRole::SyntaxFunction),
            "SyntaxVariable" => Some(ColorRole::SyntaxVariable),
            "SyntaxCustomType" => Some(ColorRole::SyntaxCustomType),
            "SyntaxNamespace" => Some(ColorRole::SyntaxNamespace),
            "SyntaxMember" => Some(ColorRole::SyntaxMember),
            "SyntaxParameter" => Some(ColorRole::SyntaxParameter),
            "SyntaxPreprocessorStatement" => Some(ColorRole::SyntaxPreprocessorStatement),
            "SyntaxPreprocessorValue" => Some(ColorRole::SyntaxPreprocessorValue),
            "SyntaxPunctuation" => Some(ColorRole::SyntaxPunctuation),
            "SyntaxOperator" => Some(ColorRole::SyntaxOperator),
            "Black" => Some(ColorRole::Black),
            "Red" => Some(ColorRole::Red),
            "Green" => Some(ColorRole::Green),
            "Yellow" => Some(ColorRole::Yellow),
            "Blue" => Some(ColorRole::Blue),
            "Magenta" => Some(ColorRole::Magenta),
            "Cyan" => Some(ColorRole::Cyan),
            "White" => Some(ColorRole::White),
            "BrightBlack" => Some(ColorRole::BrightBlack),
            "BrightRed" => Some(ColorRole::BrightRed),
            "BrightGreen" => Some(ColorRole::BrightGreen),
            "BrightYellow" => Some(ColorRole::BrightYellow),
            "BrightBlue" => Some(ColorRole::BrightBlue),
            "BrightMagenta" => Some(ColorRole::BrightMagenta),
            "BrightCyan" => Some(ColorRole::BrightCyan),
            "BrightWhite" => Some(ColorRole::BrightWhite),
            "ColorSchemeBackground" => Some(ColorRole::ColorSchemeBackground),
            "ColorSchemeForeground" => Some(ColorRole::ColorSchemeForeground),
            "Accent" => Some(ColorRole::Accent),
            "DesktopBackground" => Some(ColorRole::DesktopBackground),
            "FocusOutline" => Some(ColorRole::FocusOutline),
            "TextCursor" => Some(ColorRole::TextCursor),
            "ThreedHighlight" => Some(ColorRole::ThreedHighlight),
            "ThreedShadow1" => Some(ColorRole::ThreedShadow1),
            "ThreedShadow2" => Some(ColorRole::ThreedShadow2),
            "RubberBandFill" => Some(ColorRole::RubberBandFill),
            "RubberBandBorder" => Some(ColorRole::RubberBandBorder),
            "Gutter" => Some(ColorRole::Gutter),
            "GutterBorder" => Some(ColorRole::GutterBorder),
            "Ruler" => Some(ColorRole::Ruler),
            "RulerBorder" => Some(ColorRole::RulerBorder),
            "RulerActiveText" => Some(ColorRole::RulerActiveText),
            "RulerInactiveText" => Some(ColorRole::RulerInactiveText),
            "Tooltip" => Some(ColorRole::Tooltip),
            "TooltipText" => Some(ColorRole::TooltipText),
            "Tray" => Some(ColorRole::Tray),
            "TrayText" => Some(ColorRole::TrayText),
            "OverlayBackground" => Some(ColorRole::OverlayBackground),
            "OverlayText" => Some(ColorRole::OverlayText),
            "HighlightSearching" => Some(ColorRole::HighlightSearching),
            "HighlightSearchingText" => Some(ColorRole::HighlightSearchingText),
            _ => None,
        }
    }
}

impl fmt::Display for ColorRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Alignment roles for text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlignmentRole {
    TitleAlignment,
}

impl AlignmentRole {
    /// Get the string representation of the alignment role.
    pub fn as_str(&self) -> &'static str {
        match self {
            AlignmentRole::TitleAlignment => "TitleAlignment",
        }
    }

    /// Parse an alignment role from a string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "TitleAlignment" => Some(AlignmentRole::TitleAlignment),
            _ => None,
        }
    }
}

/// Text alignment values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlignment {
    Left,
    Center,
    Right,
}

impl TextAlignment {
    /// Parse text alignment from a string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "left" | "centerleft" => Some(TextAlignment::Left),
            "center" => Some(TextAlignment::Center),
            "right" | "centerright" => Some(TextAlignment::Right),
            _ => None,
        }
    }
}

/// Flag roles for boolean theme properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlagRole {
    BoldTextAsBright,
    TitleButtonsIconOnly,
}

impl FlagRole {
    /// Get the string representation of the flag role.
    pub fn as_str(&self) -> &'static str {
        match self {
            FlagRole::BoldTextAsBright => "BoldTextAsBright",
            FlagRole::TitleButtonsIconOnly => "TitleButtonsIconOnly",
        }
    }

    /// Parse a flag role from a string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "BoldTextAsBright" => Some(FlagRole::BoldTextAsBright),
            "TitleButtonsIconOnly" => Some(FlagRole::TitleButtonsIconOnly),
            _ => None,
        }
    }
}

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

impl MetricRole {
    /// Get the string representation of the metric role.
    pub fn as_str(&self) -> &'static str {
        match self {
            MetricRole::BorderThickness => "BorderThickness",
            MetricRole::BorderRadius => "BorderRadius",
            MetricRole::TitleHeight => "TitleHeight",
            MetricRole::TitleButtonWidth => "TitleButtonWidth",
            MetricRole::TitleButtonHeight => "TitleButtonHeight",
            MetricRole::TitleButtonInactiveAlpha => "TitleButtonInactiveAlpha",
        }
    }

    /// Parse a metric role from a string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "BorderThickness" => Some(MetricRole::BorderThickness),
            "BorderRadius" => Some(MetricRole::BorderRadius),
            "TitleHeight" => Some(MetricRole::TitleHeight),
            "TitleButtonWidth" => Some(MetricRole::TitleButtonWidth),
            "TitleButtonHeight" => Some(MetricRole::TitleButtonHeight),
            "TitleButtonInactiveAlpha" => Some(MetricRole::TitleButtonInactiveAlpha),
            _ => None,
        }
    }

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

/// Path roles for file path theme properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PathRole {
    TitleButtonIcons,
    // Shadow paths are TODO for future kurbo-based implementation
    // ActiveWindowShadow,
    // InactiveWindowShadow,
    // TaskbarShadow,
    // MenuShadow,
    // TooltipShadow,
    // OverlayRectShadow,
}

impl PathRole {
    /// Get the string representation of the path role.
    pub fn as_str(&self) -> &'static str {
        match self {
            PathRole::TitleButtonIcons => "TitleButtonIcons",
        }
    }

    /// Parse a path role from a string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "TitleButtonIcons" => Some(PathRole::TitleButtonIcons),
            _ => None,
        }
    }

    /// Get the default value for a path role.
    pub fn default_value(&self) -> &'static str {
        match self {
            PathRole::TitleButtonIcons => "/res/icons/16x16/",
        }
    }
}

/// Window theme provider types (for future window manager integration).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowThemeProvider {
    Classic,
    RedmondGlass,
    RedmondPlastic,
}

impl WindowThemeProvider {
    /// Parse window theme provider from a string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Classic" => Some(WindowThemeProvider::Classic),
            "RedmondGlass" => Some(WindowThemeProvider::RedmondGlass),
            "RedmondPlastic" => Some(WindowThemeProvider::RedmondPlastic),
            _ => None,
        }
    }
}
