// SPDX-License-Identifier: LGPL-3.0-only

//! Built-in themes.
//!
//! Colors can use alpha transparency by using `Color::from_rgba8(r, g, b, a)`.
//! In TOML theme files, alpha colors are specified as 8-character hex strings:
//! - `#rrggbbaa` where `aa` is the alpha channel (0-255, 0x00-0xff)
//! - Example: `#ff000080` is red with 50% opacity

use std::path::PathBuf;
use vello::peniko::Color;
use super::roles::{AlignmentRole, ColorRole, FlagRole, MetricRole, PathRole, TextAlignment};
use super::terminal::TerminalColors;
use super::Theme;

/// Create the built-in Sweet theme.
///
/// Sweet is a modern dark theme with vibrant purple/magenta accents.
pub fn create_sweet_theme() -> Theme {
    let mut theme = Theme::new();

    // Window colors
    theme.set_color(ColorRole::Window, Color::from_rgba8(22, 25, 37, 255)); // #161925 from Kvantum
    theme.set_color(ColorRole::WindowText, Color::from_rgba8(195, 199, 209, 255)); // #C3C7D1 from Kvantum
    theme.set_color(ColorRole::ActiveWindowBorder1, Color::from_rgba8(197, 14, 210, 255));
    theme.set_color(ColorRole::ActiveWindowBorder2, Color::from_rgba8(197, 14, 210, 255));
    theme.set_color(ColorRole::ActiveWindowTitle, Color::from_rgba8(195, 199, 209, 255)); // #C3C7D1 from Kvantum
    theme.set_color(ColorRole::ActiveWindowTitleShadow, Color::from_rgba8(0, 0, 0, 255));
    theme.set_color(ColorRole::ActiveWindowTitleStripes, Color::from_rgba8(197, 14, 210, 255));
    theme.set_color(ColorRole::InactiveWindowBorder1, Color::from_rgba8(102, 106, 115, 255));
    theme.set_color(ColorRole::InactiveWindowBorder2, Color::from_rgba8(102, 106, 115, 255));
    theme.set_color(ColorRole::InactiveWindowTitle, Color::from_rgba8(195, 199, 209, 255));
    theme.set_color(ColorRole::InactiveWindowTitleShadow, Color::from_rgba8(0, 0, 0, 255));
    theme.set_color(ColorRole::InactiveWindowTitleStripes, Color::from_rgba8(102, 106, 115, 255));
    theme.set_color(ColorRole::MovingWindowBorder1, Color::from_rgba8(197, 14, 210, 255));
    theme.set_color(ColorRole::MovingWindowBorder2, Color::from_rgba8(197, 14, 210, 255));
    theme.set_color(ColorRole::MovingWindowTitle, Color::from_rgba8(195, 199, 209, 255)); // #C3C7D1 from Kvantum
    theme.set_color(ColorRole::MovingWindowTitleShadow, Color::from_rgba8(0, 0, 0, 255));
    theme.set_color(ColorRole::MovingWindowTitleStripes, Color::from_rgba8(197, 14, 210, 255));
    theme.set_color(ColorRole::HighlightWindowBorder1, Color::from_rgba8(0, 232, 198, 255));
    theme.set_color(ColorRole::HighlightWindowBorder2, Color::from_rgba8(0, 232, 198, 255));
    theme.set_color(ColorRole::HighlightWindowTitle, Color::from_rgba8(195, 199, 209, 255)); // #C3C7D1 from Kvantum
    theme.set_color(ColorRole::HighlightWindowTitleShadow, Color::from_rgba8(0, 0, 0, 255));
    theme.set_color(ColorRole::HighlightWindowTitleStripes, Color::from_rgba8(0, 232, 198, 255));

    // Widget colors
    theme.set_color(ColorRole::Button, Color::from_rgba8(24, 27, 40, 255)); // #181b28 (base.color) from Kvantum - matches base background
    theme.set_color(ColorRole::ButtonText, Color::from_rgba8(195, 199, 209, 255)); // #C3C7D1 from Kvantum
    theme.set_color(ColorRole::Base, Color::from_rgba8(24, 27, 40, 255)); // #181b28 from Kvantum
    theme.set_color(ColorRole::BaseText, Color::from_rgba8(195, 199, 209, 255)); // #C3C7D1 from Kvantum
    theme.set_color(ColorRole::Selection, Color::from_rgba8(197, 14, 210, 255)); // #c50ed2 from Kvantum
    theme.set_color(ColorRole::SelectionText, Color::from_rgba8(218, 218, 220, 255)); // #dadadc from Kvantum
    theme.set_color(ColorRole::InactiveSelection, Color::from_rgba8(101, 78, 163, 255)); // #654ea3 from Kvantum
    theme.set_color(ColorRole::InactiveSelectionText, Color::from_rgba8(195, 199, 209, 255)); // #C3C7D1 from Kvantum
    theme.set_color(ColorRole::HoverHighlight, Color::from_rgba8(35, 40, 55, 255)); // Lighter hover color for better visibility
    theme.set_color(ColorRole::DisabledTextFront, Color::from_rgba8(102, 106, 115, 255));
    theme.set_color(ColorRole::DisabledTextBack, Color::from_rgba8(22, 25, 37, 255));
    theme.set_color(ColorRole::PlaceholderText, Color::from_rgba8(102, 106, 115, 255));

    // Menu colors
    theme.set_color(ColorRole::MenuBase, Color::from_rgba8(24, 27, 40, 255)); // #181b28 (base.color) from Kvantum
    theme.set_color(ColorRole::MenuBaseText, Color::from_rgba8(195, 199, 209, 255)); // #C3C7D1 from Kvantum
    theme.set_color(ColorRole::MenuSelection, Color::from_rgba8(101, 78, 163, 255)); // #654ea3 (inactive.highlight.color - darker) from Kvantum
    theme.set_color(ColorRole::MenuSelectionText, Color::from_rgba8(218, 218, 220, 255)); // #dadadc from Kvantum
    theme.set_color(ColorRole::MenuStripe, Color::from_rgba8(102, 106, 115, 255));

    // Link colors
    theme.set_color(ColorRole::Link, Color::from_rgba8(100, 100, 100, 255)); // #646464 from Kvantum
    theme.set_color(ColorRole::ActiveLink, Color::from_rgba8(197, 14, 210, 255)); // #c50ed2 from Kvantum
    theme.set_color(ColorRole::VisitedLink, Color::from_rgba8(127, 140, 141, 255)); // #7f8c8d from Kvantum

    // Syntax highlighting (using Sweet theme colors)
    theme.set_color(ColorRole::SyntaxComment, Color::from_rgba8(102, 106, 115, 255));
    theme.set_color(ColorRole::SyntaxKeyword, Color::from_rgba8(197, 14, 210, 255));
    theme.set_color(ColorRole::SyntaxControlKeyword, Color::from_rgba8(197, 14, 210, 255));
    theme.set_color(ColorRole::SyntaxString, Color::from_rgba8(0, 232, 198, 255));
    theme.set_color(ColorRole::SyntaxNumber, Color::from_rgba8(254, 207, 14, 255));
    theme.set_color(ColorRole::SyntaxType, Color::from_rgba8(0, 232, 198, 255));
    theme.set_color(ColorRole::SyntaxIdentifier, Color::from_rgba8(195, 199, 209, 255)); // #C3C7D1 from Kvantum
    theme.set_color(ColorRole::SyntaxFunction, Color::from_rgba8(82, 148, 226, 255));
    theme.set_color(ColorRole::SyntaxVariable, Color::from_rgba8(195, 199, 209, 255)); // #C3C7D1 from Kvantum
    theme.set_color(ColorRole::SyntaxCustomType, Color::from_rgba8(0, 232, 198, 255));
    theme.set_color(ColorRole::SyntaxNamespace, Color::from_rgba8(0, 232, 198, 255));
    theme.set_color(ColorRole::SyntaxMember, Color::from_rgba8(197, 14, 210, 255));
    theme.set_color(ColorRole::SyntaxParameter, Color::from_rgba8(254, 207, 14, 255));
    theme.set_color(ColorRole::SyntaxPreprocessorStatement, Color::from_rgba8(0, 232, 198, 255));
    theme.set_color(ColorRole::SyntaxPreprocessorValue, Color::from_rgba8(251, 43, 44, 255));
    theme.set_color(ColorRole::SyntaxPunctuation, Color::from_rgba8(195, 199, 209, 255)); // #C3C7D1 from Kvantum
    theme.set_color(ColorRole::SyntaxOperator, Color::from_rgba8(195, 199, 209, 255)); // #C3C7D1 from Kvantum

    // Terminal colors (ANSI)
    theme.set_color(ColorRole::Black, Color::from_rgba8(22, 25, 37, 255));
    theme.set_color(ColorRole::Red, Color::from_rgba8(251, 43, 44, 255));
    theme.set_color(ColorRole::Green, Color::from_rgba8(48, 211, 58, 255));
    theme.set_color(ColorRole::Yellow, Color::from_rgba8(254, 207, 14, 255));
    theme.set_color(ColorRole::Blue, Color::from_rgba8(16, 106, 254, 255));
    theme.set_color(ColorRole::Magenta, Color::from_rgba8(197, 14, 210, 255));
    theme.set_color(ColorRole::Cyan, Color::from_rgba8(0, 232, 198, 255));
    theme.set_color(ColorRole::White, Color::from_rgba8(195, 199, 209, 255)); // #C3C7D1 from Kvantum
    theme.set_color(ColorRole::BrightBlack, Color::from_rgba8(47, 52, 63, 255));
    theme.set_color(ColorRole::BrightRed, Color::from_rgba8(251, 43, 44, 255));
    theme.set_color(ColorRole::BrightGreen, Color::from_rgba8(48, 211, 58, 255));
    theme.set_color(ColorRole::BrightYellow, Color::from_rgba8(254, 207, 14, 255));
    theme.set_color(ColorRole::BrightBlue, Color::from_rgba8(16, 106, 254, 255));
    theme.set_color(ColorRole::BrightMagenta, Color::from_rgba8(197, 14, 210, 255));
    theme.set_color(ColorRole::BrightCyan, Color::from_rgba8(0, 232, 198, 255));
    theme.set_color(ColorRole::BrightWhite, Color::from_rgba8(254, 254, 254, 255));
    theme.set_color(ColorRole::ColorSchemeBackground, Color::from_rgba8(22, 25, 37, 255));
    theme.set_color(ColorRole::ColorSchemeForeground, Color::from_rgba8(195, 199, 209, 255)); // #C3C7D1 from Kvantum

    // Other colors
    theme.set_color(ColorRole::Accent, Color::from_rgba8(197, 14, 210, 255));
    theme.set_color(ColorRole::DesktopBackground, Color::from_rgba8(22, 25, 37, 255));
    theme.set_color(ColorRole::FocusOutline, Color::from_rgba8(197, 14, 210, 255));
    theme.set_color(ColorRole::TextCursor, Color::from_rgba8(195, 199, 209, 255)); // #C3C7D1 from Kvantum
    theme.set_color(ColorRole::ThreedHighlight, Color::from_rgba8(102, 106, 115, 255));
    theme.set_color(ColorRole::ThreedShadow1, Color::from_rgba8(47, 52, 63, 255));
    theme.set_color(ColorRole::ThreedShadow2, Color::from_rgba8(22, 25, 37, 255));
    theme.set_color(ColorRole::RubberBandFill, Color::from_rgba8(197, 14, 210, 60));
    theme.set_color(ColorRole::RubberBandBorder, Color::from_rgba8(197, 14, 210, 255));
    theme.set_color(ColorRole::Gutter, Color::from_rgba8(30, 34, 51, 255));
    theme.set_color(ColorRole::GutterBorder, Color::from_rgba8(102, 106, 115, 255));
    theme.set_color(ColorRole::Ruler, Color::from_rgba8(30, 34, 51, 255));
    theme.set_color(ColorRole::RulerBorder, Color::from_rgba8(102, 106, 115, 255));
    theme.set_color(ColorRole::RulerActiveText, Color::from_rgba8(195, 199, 209, 255)); // #C3C7D1 from Kvantum
    theme.set_color(ColorRole::RulerInactiveText, Color::from_rgba8(102, 106, 115, 255));
    theme.set_color(ColorRole::Tooltip, Color::from_rgba8(24, 27, 40, 255)); // #181b28 (base.color) from Kvantum
    theme.set_color(ColorRole::TooltipText, Color::from_rgba8(195, 199, 209, 255)); // #C3C7D1 from Kvantum
    theme.set_color(ColorRole::Tray, Color::from_rgba8(22, 25, 37, 255)); // #161925 (window.color) from Kvantum
    theme.set_color(ColorRole::TrayText, Color::from_rgba8(195, 199, 209, 255)); // #C3C7D1 from Kvantum
    theme.set_color(ColorRole::OverlayBackground, Color::from_rgba8(22, 25, 37, 255)); // #161925 from Kvantum
    theme.set_color(ColorRole::OverlayText, Color::from_rgba8(195, 199, 209, 255)); // #C3C7D1 from Kvantum
    theme.set_color(ColorRole::HighlightSearching, Color::from_rgba8(254, 207, 14, 255));
    theme.set_color(ColorRole::HighlightSearchingText, Color::from_rgba8(22, 25, 37, 255));

    // Alignments
    theme.set_alignment(AlignmentRole::TitleAlignment, TextAlignment::Center);

    // Flags
    theme.set_flag(FlagRole::BoldTextAsBright, true);
    theme.set_flag(FlagRole::TitleButtonsIconOnly, false);

    // Metrics
    theme.set_metric(MetricRole::BorderThickness, 4);
    theme.set_metric(MetricRole::BorderRadius, 0);
    theme.set_metric(MetricRole::TitleHeight, 19);
    theme.set_metric(MetricRole::TitleButtonWidth, 15);
    theme.set_metric(MetricRole::TitleButtonHeight, 15);
    theme.set_metric(MetricRole::TitleButtonInactiveAlpha, 255);

    // Paths
    theme.set_path(PathRole::TitleButtonIcons, PathBuf::from("/res/icons/16x16/"));

    // Terminal colors
    let terminal_colors = TerminalColors {
        show_bold_as_bright: true,
        background: Color::from_rgb8(22, 25, 37),
        foreground: Color::from_rgb8(195, 199, 209), // #C3C7D1 from Kvantum
        normal: [
            Color::from_rgb8(22, 25, 37),    // Black
            Color::from_rgb8(251, 43, 44),   // Red
            Color::from_rgb8(48, 211, 58),   // Green
            Color::from_rgb8(254, 207, 14),  // Yellow
            Color::from_rgb8(16, 106, 254),  // Blue
            Color::from_rgb8(197, 14, 210),  // Magenta
            Color::from_rgb8(0, 232, 198),   // Cyan
            Color::from_rgb8(195, 199, 209), // White (#C3C7D1 from Kvantum)
        ],
        bright: [
            Color::from_rgb8(47, 52, 63),    // BrightBlack
            Color::from_rgb8(251, 43, 44),   // BrightRed
            Color::from_rgb8(48, 211, 58),   // BrightGreen
            Color::from_rgb8(254, 207, 14), // BrightYellow
            Color::from_rgb8(16, 106, 254),  // BrightBlue
            Color::from_rgb8(197, 14, 210),  // BrightMagenta
            Color::from_rgb8(0, 232, 198),   // BrightCyan
            Color::from_rgb8(254, 254, 254), // BrightWhite
        ],
    };
    theme.set_terminal_colors(terminal_colors);

    theme
}
