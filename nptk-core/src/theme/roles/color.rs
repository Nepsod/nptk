// SPDX-License-Identifier: LGPL-3.0-only

//! Color roles for theme colors.
//!
//! These roles represent semantic color purposes that widgets can use
//! to get appropriate colors from the theme.

use std::fmt;

/// Color roles for theme colors.
///
/// These roles represent semantic color purposes that widgets can use
/// to get appropriate colors from the theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorRole {
    // Window roles
    /// Active window primary border color.
    ActiveWindowBorder1,
    /// Active window secondary border color.
    ActiveWindowBorder2,
    /// Active window title bar background color.
    ActiveWindowTitle,
    /// Active window title bar shadow color.
    ActiveWindowTitleShadow,
    /// Active window title bar stripes color.
    ActiveWindowTitleStripes,
    /// Inactive window primary border color.
    InactiveWindowBorder1,
    /// Inactive window secondary border color.
    InactiveWindowBorder2,
    /// Inactive window title bar background color.
    InactiveWindowTitle,
    /// Inactive window title bar shadow color.
    InactiveWindowTitleShadow,
    /// Inactive window title bar stripes color.
    InactiveWindowTitleStripes,
    /// Moving window primary border color.
    MovingWindowBorder1,
    /// Moving window secondary border color.
    MovingWindowBorder2,
    /// Moving window title bar background color.
    MovingWindowTitle,
    /// Moving window title bar shadow color.
    MovingWindowTitleShadow,
    /// Moving window title bar stripes color.
    MovingWindowTitleStripes,
    /// Highlighted window primary border color.
    HighlightWindowBorder1,
    /// Highlighted window secondary border color.
    HighlightWindowBorder2,
    /// Highlighted window title bar background color.
    HighlightWindowTitle,
    /// Highlighted window title bar shadow color.
    HighlightWindowTitleShadow,
    /// Highlighted window title bar stripes color.
    HighlightWindowTitleStripes,

    // Widget roles
    /// Standard window background color.
    Window,
    /// Standard window text color.
    WindowText,
    /// Background color of standard buttons.
    Button,
    /// Text color of standard buttons.
    ButtonText,
    /// Base background color, often used in text blocks.
    Base,
    /// Base text color for standard reads.
    BaseText,
    /// Standard selection background color.
    Selection,
    /// Standard selection text color.
    SelectionText,
    /// Selection color when the window/element is inactive.
    InactiveSelection,
    /// Selection text color when inactive.
    InactiveSelectionText,
    /// Highlight background color when hovering over an interactive item.
    HoverHighlight,
    /// Text color for disabled elements (foreground).
    DisabledTextFront,
    /// Secondary accent color for disabled elements (background or shadow).
    DisabledTextBack,
    /// Text color indicating a placeholder value.
    PlaceholderText,

    // Menu roles
    /// Background color of menus.
    MenuBase,
    /// Default text color for menus.
    MenuBaseText,
    /// Background color of a selected menu item.
    MenuSelection,
    /// Text color of a selected menu item.
    MenuSelectionText,
    /// Color for menu item stripes or separators.
    MenuStripe,

    // Link roles
    /// Default color for hyperlinks.
    Link,
    /// Color for an active (clicked) hyperlink.
    ActiveLink,
    /// Color for a visited hyperlink.
    VisitedLink,

    // Syntax highlighting roles
    /// Syntax highlighting: comments.
    SyntaxComment,
    /// Syntax highlighting: keywords.
    SyntaxKeyword,
    /// Syntax highlighting: control flow keywords.
    SyntaxControlKeyword,
    /// Syntax highlighting: strings.
    SyntaxString,
    /// Syntax highlighting: numbers.
    SyntaxNumber,
    /// Syntax highlighting: built-in types.
    SyntaxType,
    /// Syntax highlighting: identifiers/variables.
    SyntaxIdentifier,
    /// Syntax highlighting: function names.
    SyntaxFunction,
    /// Syntax highlighting: general variables.
    SyntaxVariable,
    /// Syntax highlighting: custom or user-defined types.
    SyntaxCustomType,
    /// Syntax highlighting: namespaces or modules.
    SyntaxNamespace,
    /// Syntax highlighting: struct/class members.
    SyntaxMember,
    /// Syntax highlighting: function parameters.
    SyntaxParameter,
    /// Syntax highlighting: preprocessor statements.
    SyntaxPreprocessorStatement,
    /// Syntax highlighting: preprocessor values.
    SyntaxPreprocessorValue,
    /// Syntax highlighting: punctuation marks.
    SyntaxPunctuation,
    /// Syntax highlighting: operators.
    SyntaxOperator,

    // Terminal colors
    /// Base terminal/standard color: Black.
    Black,
    /// Base terminal/standard color: Red.
    Red,
    /// Base terminal/standard color: Green.
    Green,
    /// Base terminal/standard color: Yellow.
    Yellow,
    /// Base terminal/standard color: Blue.
    Blue,
    /// Base terminal/standard color: Magenta.
    Magenta,
    /// Base terminal/standard color: Cyan.
    Cyan,
    /// Base terminal/standard color: White.
    White,
    /// Base terminal/standard color: Bright Black (Gray).
    BrightBlack,
    /// Base terminal/standard color: Bright Red.
    BrightRed,
    /// Base terminal/standard color: Bright Green.
    BrightGreen,
    /// Base terminal/standard color: Bright Yellow.
    BrightYellow,
    /// Base terminal/standard color: Bright Blue.
    BrightBlue,
    /// Base terminal/standard color: Bright Magenta.
    BrightMagenta,
    /// Base terminal/standard color: Bright Cyan.
    BrightCyan,
    /// Base terminal/standard color: Bright White.
    BrightWhite,
    /// Terminal or code block explicit background.
    ColorSchemeBackground,
    /// Terminal or code block explicit foreground.
    ColorSchemeForeground,

    // Other roles
    /// Accent color used to emphasize controls and primary actions.
    Accent,
    /// Desktop background color or wallpaper base.
    DesktopBackground,
    /// Outline color for focused elements.
    FocusOutline,
    /// Color of text cursors or carets.
    TextCursor,
    /// 3D highlight color (usually lighter border).
    ThreedHighlight,
    /// Primary 3D shadow color.
    ThreedShadow1,
    /// Secondary, darker 3D shadow color.
    ThreedShadow2,
    /// Selection box fill color (often translucent).
    RubberBandFill,
    /// Selection box border color.
    RubberBandBorder,
    /// Gutter color (e.g., in a code editor).
    Gutter,
    /// Border color for gutters.
    GutterBorder,
    /// Color for an unselected ruler.
    Ruler,
    /// Border color for a ruler.
    RulerBorder,
    /// Active text on a ruler.
    RulerActiveText,
    /// Inactive text on a ruler.
    RulerInactiveText,
    /// Background color of tooltips.
    Tooltip,
    /// Text color on tooltips.
    TooltipText,
    /// System tray background color.
    Tray,
    /// System tray text color.
    TrayText,
    /// Background color of overlays.
    OverlayBackground,
    /// Text color of overlays.
    OverlayText,
    /// Background color emphasizing matching search terms.
    HighlightSearching,
    /// Text color emphasizing matching search terms.
    HighlightSearchingText,
}

crate::impl_role_string_conversion!(ColorRole, {
    ActiveWindowBorder1 => "ActiveWindowBorder1",
    ActiveWindowBorder2 => "ActiveWindowBorder2",
    ActiveWindowTitle => "ActiveWindowTitle",
    ActiveWindowTitleShadow => "ActiveWindowTitleShadow",
    ActiveWindowTitleStripes => "ActiveWindowTitleStripes",
    InactiveWindowBorder1 => "InactiveWindowBorder1",
    InactiveWindowBorder2 => "InactiveWindowBorder2",
    InactiveWindowTitle => "InactiveWindowTitle",
    InactiveWindowTitleShadow => "InactiveWindowTitleShadow",
    InactiveWindowTitleStripes => "InactiveWindowTitleStripes",
    MovingWindowBorder1 => "MovingWindowBorder1",
    MovingWindowBorder2 => "MovingWindowBorder2",
    MovingWindowTitle => "MovingWindowTitle",
    MovingWindowTitleShadow => "MovingWindowTitleShadow",
    MovingWindowTitleStripes => "MovingWindowTitleStripes",
    HighlightWindowBorder1 => "HighlightWindowBorder1",
    HighlightWindowBorder2 => "HighlightWindowBorder2",
    HighlightWindowTitle => "HighlightWindowTitle",
    HighlightWindowTitleShadow => "HighlightWindowTitleShadow",
    HighlightWindowTitleStripes => "HighlightWindowTitleStripes",
    Window => "Window",
    WindowText => "WindowText",
    Button => "Button",
    ButtonText => "ButtonText",
    Base => "Base",
    BaseText => "BaseText",
    Selection => "Selection",
    SelectionText => "SelectionText",
    InactiveSelection => "InactiveSelection",
    InactiveSelectionText => "InactiveSelectionText",
    HoverHighlight => "HoverHighlight",
    DisabledTextFront => "DisabledTextFront",
    DisabledTextBack => "DisabledTextBack",
    PlaceholderText => "PlaceholderText",
    MenuBase => "MenuBase",
    MenuBaseText => "MenuBaseText",
    MenuSelection => "MenuSelection",
    MenuSelectionText => "MenuSelectionText",
    MenuStripe => "MenuStripe",
    Link => "Link",
    ActiveLink => "ActiveLink",
    VisitedLink => "VisitedLink",
    SyntaxComment => "SyntaxComment",
    SyntaxKeyword => "SyntaxKeyword",
    SyntaxControlKeyword => "SyntaxControlKeyword",
    SyntaxString => "SyntaxString",
    SyntaxNumber => "SyntaxNumber",
    SyntaxType => "SyntaxType",
    SyntaxIdentifier => "SyntaxIdentifier",
    SyntaxFunction => "SyntaxFunction",
    SyntaxVariable => "SyntaxVariable",
    SyntaxCustomType => "SyntaxCustomType",
    SyntaxNamespace => "SyntaxNamespace",
    SyntaxMember => "SyntaxMember",
    SyntaxParameter => "SyntaxParameter",
    SyntaxPreprocessorStatement => "SyntaxPreprocessorStatement",
    SyntaxPreprocessorValue => "SyntaxPreprocessorValue",
    SyntaxPunctuation => "SyntaxPunctuation",
    SyntaxOperator => "SyntaxOperator",
    Black => "Black",
    Red => "Red",
    Green => "Green",
    Yellow => "Yellow",
    Blue => "Blue",
    Magenta => "Magenta",
    Cyan => "Cyan",
    White => "White",
    BrightBlack => "BrightBlack",
    BrightRed => "BrightRed",
    BrightGreen => "BrightGreen",
    BrightYellow => "BrightYellow",
    BrightBlue => "BrightBlue",
    BrightMagenta => "BrightMagenta",
    BrightCyan => "BrightCyan",
    BrightWhite => "BrightWhite",
    ColorSchemeBackground => "ColorSchemeBackground",
    ColorSchemeForeground => "ColorSchemeForeground",
    Accent => "Accent",
    DesktopBackground => "DesktopBackground",
    FocusOutline => "FocusOutline",
    TextCursor => "TextCursor",
    ThreedHighlight => "ThreedHighlight",
    ThreedShadow1 => "ThreedShadow1",
    ThreedShadow2 => "ThreedShadow2",
    RubberBandFill => "RubberBandFill",
    RubberBandBorder => "RubberBandBorder",
    Gutter => "Gutter",
    GutterBorder => "GutterBorder",
    Ruler => "Ruler",
    RulerBorder => "RulerBorder",
    RulerActiveText => "RulerActiveText",
    RulerInactiveText => "RulerInactiveText",
    Tooltip => "Tooltip",
    TooltipText => "TooltipText",
    Tray => "Tray",
    TrayText => "TrayText",
    OverlayBackground => "OverlayBackground",
    OverlayText => "OverlayText",
    HighlightSearching => "HighlightSearching",
    HighlightSearchingText => "HighlightSearchingText",
});

impl fmt::Display for ColorRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
