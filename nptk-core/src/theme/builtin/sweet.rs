// SPDX-License-Identifier: LGPL-3.0-only

//! Sweet theme implementation.

use std::path::PathBuf;
use super::super::roles::{AlignmentRole, ColorRole, FlagRole, MetricRole, PathRole, TextAlignment};
use super::super::terminal::TerminalColors;
use super::super::util::rgba8;
use super::super::Theme;
use super::{set_color_rgb, set_color_rgba};

// Color constants for Sweet theme
const SWEET_WINDOW_BG: (u8, u8, u8) = (22, 25, 37);
const SWEET_TEXT: (u8, u8, u8) = (195, 199, 209);
const SWEET_BASE: (u8, u8, u8) = (24, 27, 40);
const SWEET_ACCENT: (u8, u8, u8) = (197, 14, 210);
const SWEET_ACCENT_DARK: (u8, u8, u8) = (101, 78, 163);
const SWEET_CYAN: (u8, u8, u8) = (0, 232, 198);
const SWEET_GRAY: (u8, u8, u8) = (102, 106, 115);
const SWEET_GRAY_DARK: (u8, u8, u8) = (47, 52, 63);
const SWEET_HOVER: (u8, u8, u8) = (35, 40, 55);
const SWEET_GUTTER: (u8, u8, u8) = (30, 34, 51);
const SWEET_SELECTION_TEXT: (u8, u8, u8) = (218, 218, 220);
const SWEET_YELLOW: (u8, u8, u8) = (254, 207, 14);
const SWEET_RED: (u8, u8, u8) = (251, 43, 44);
const SWEET_GREEN: (u8, u8, u8) = (48, 211, 58);
const SWEET_BLUE: (u8, u8, u8) = (16, 106, 254);
const SWEET_LINK: (u8, u8, u8) = (100, 100, 100);
const SWEET_LINK_VISITED: (u8, u8, u8) = (127, 140, 141);
const SWEET_FUNCTION: (u8, u8, u8) = (82, 148, 226);
const SWEET_PREPROCESSOR: (u8, u8, u8) = (251, 43, 44);
const SWEET_WHITE: (u8, u8, u8) = (254, 254, 254);

/// Create the built-in Sweet theme.
///
/// Sweet is a modern dark theme with vibrant purple/magenta accents.
pub fn create_sweet_theme() -> Theme {
    let mut theme = Theme::new();

    // Window colors
    set_color_rgb(&mut theme, ColorRole::Window, SWEET_WINDOW_BG.0, SWEET_WINDOW_BG.1, SWEET_WINDOW_BG.2);
    set_color_rgb(&mut theme, ColorRole::WindowText, SWEET_TEXT.0, SWEET_TEXT.1, SWEET_TEXT.2);
    set_color_rgb(&mut theme, ColorRole::ActiveWindowBorder1, SWEET_ACCENT.0, SWEET_ACCENT.1, SWEET_ACCENT.2);
    set_color_rgb(&mut theme, ColorRole::ActiveWindowBorder2, SWEET_ACCENT.0, SWEET_ACCENT.1, SWEET_ACCENT.2);
    set_color_rgb(&mut theme, ColorRole::ActiveWindowTitle, SWEET_TEXT.0, SWEET_TEXT.1, SWEET_TEXT.2);
    set_color_rgb(&mut theme, ColorRole::ActiveWindowTitleShadow, 0, 0, 0);
    set_color_rgb(&mut theme, ColorRole::ActiveWindowTitleStripes, SWEET_ACCENT.0, SWEET_ACCENT.1, SWEET_ACCENT.2);
    set_color_rgb(&mut theme, ColorRole::InactiveWindowBorder1, SWEET_GRAY.0, SWEET_GRAY.1, SWEET_GRAY.2);
    set_color_rgb(&mut theme, ColorRole::InactiveWindowBorder2, SWEET_GRAY.0, SWEET_GRAY.1, SWEET_GRAY.2);
    set_color_rgb(&mut theme, ColorRole::InactiveWindowTitle, SWEET_TEXT.0, SWEET_TEXT.1, SWEET_TEXT.2);
    set_color_rgb(&mut theme, ColorRole::InactiveWindowTitleShadow, 0, 0, 0);
    set_color_rgb(&mut theme, ColorRole::InactiveWindowTitleStripes, SWEET_GRAY.0, SWEET_GRAY.1, SWEET_GRAY.2);
    set_color_rgb(&mut theme, ColorRole::MovingWindowBorder1, SWEET_ACCENT.0, SWEET_ACCENT.1, SWEET_ACCENT.2);
    set_color_rgb(&mut theme, ColorRole::MovingWindowBorder2, SWEET_ACCENT.0, SWEET_ACCENT.1, SWEET_ACCENT.2);
    set_color_rgb(&mut theme, ColorRole::MovingWindowTitle, SWEET_TEXT.0, SWEET_TEXT.1, SWEET_TEXT.2);
    set_color_rgb(&mut theme, ColorRole::MovingWindowTitleShadow, 0, 0, 0);
    set_color_rgb(&mut theme, ColorRole::MovingWindowTitleStripes, SWEET_ACCENT.0, SWEET_ACCENT.1, SWEET_ACCENT.2);
    set_color_rgb(&mut theme, ColorRole::HighlightWindowBorder1, SWEET_CYAN.0, SWEET_CYAN.1, SWEET_CYAN.2);
    set_color_rgb(&mut theme, ColorRole::HighlightWindowBorder2, SWEET_CYAN.0, SWEET_CYAN.1, SWEET_CYAN.2);
    set_color_rgb(&mut theme, ColorRole::HighlightWindowTitle, SWEET_TEXT.0, SWEET_TEXT.1, SWEET_TEXT.2);
    set_color_rgb(&mut theme, ColorRole::HighlightWindowTitleShadow, 0, 0, 0);
    set_color_rgb(&mut theme, ColorRole::HighlightWindowTitleStripes, SWEET_CYAN.0, SWEET_CYAN.1, SWEET_CYAN.2);

    // Widget colors
    set_color_rgb(&mut theme, ColorRole::Button, SWEET_BASE.0, SWEET_BASE.1, SWEET_BASE.2);
    set_color_rgb(&mut theme, ColorRole::ButtonText, SWEET_TEXT.0, SWEET_TEXT.1, SWEET_TEXT.2);
    set_color_rgb(&mut theme, ColorRole::Base, SWEET_BASE.0, SWEET_BASE.1, SWEET_BASE.2);
    set_color_rgb(&mut theme, ColorRole::BaseText, SWEET_TEXT.0, SWEET_TEXT.1, SWEET_TEXT.2);
    set_color_rgb(&mut theme, ColorRole::Selection, SWEET_ACCENT.0, SWEET_ACCENT.1, SWEET_ACCENT.2);
    set_color_rgb(&mut theme, ColorRole::SelectionText, SWEET_SELECTION_TEXT.0, SWEET_SELECTION_TEXT.1, SWEET_SELECTION_TEXT.2);
    set_color_rgb(&mut theme, ColorRole::InactiveSelection, SWEET_ACCENT_DARK.0, SWEET_ACCENT_DARK.1, SWEET_ACCENT_DARK.2);
    set_color_rgb(&mut theme, ColorRole::InactiveSelectionText, SWEET_TEXT.0, SWEET_TEXT.1, SWEET_TEXT.2);
    set_color_rgb(&mut theme, ColorRole::HoverHighlight, SWEET_HOVER.0, SWEET_HOVER.1, SWEET_HOVER.2);
    set_color_rgb(&mut theme, ColorRole::DisabledTextFront, SWEET_GRAY.0, SWEET_GRAY.1, SWEET_GRAY.2);
    set_color_rgb(&mut theme, ColorRole::DisabledTextBack, SWEET_WINDOW_BG.0, SWEET_WINDOW_BG.1, SWEET_WINDOW_BG.2);
    set_color_rgb(&mut theme, ColorRole::PlaceholderText, SWEET_GRAY.0, SWEET_GRAY.1, SWEET_GRAY.2);

    // Menu colors
    set_color_rgb(&mut theme, ColorRole::MenuBase, SWEET_BASE.0, SWEET_BASE.1, SWEET_BASE.2);
    set_color_rgb(&mut theme, ColorRole::MenuBaseText, SWEET_TEXT.0, SWEET_TEXT.1, SWEET_TEXT.2);
    set_color_rgb(&mut theme, ColorRole::MenuSelection, SWEET_ACCENT_DARK.0, SWEET_ACCENT_DARK.1, SWEET_ACCENT_DARK.2);
    set_color_rgb(&mut theme, ColorRole::MenuSelectionText, SWEET_SELECTION_TEXT.0, SWEET_SELECTION_TEXT.1, SWEET_SELECTION_TEXT.2);
    set_color_rgb(&mut theme, ColorRole::MenuStripe, SWEET_GRAY.0, SWEET_GRAY.1, SWEET_GRAY.2);

    // Link colors
    set_color_rgb(&mut theme, ColorRole::Link, SWEET_LINK.0, SWEET_LINK.1, SWEET_LINK.2);
    set_color_rgb(&mut theme, ColorRole::ActiveLink, SWEET_ACCENT.0, SWEET_ACCENT.1, SWEET_ACCENT.2);
    set_color_rgb(&mut theme, ColorRole::VisitedLink, SWEET_LINK_VISITED.0, SWEET_LINK_VISITED.1, SWEET_LINK_VISITED.2);

    // Syntax highlighting (using Sweet theme colors)
    set_color_rgb(&mut theme, ColorRole::SyntaxComment, SWEET_GRAY.0, SWEET_GRAY.1, SWEET_GRAY.2);
    set_color_rgb(&mut theme, ColorRole::SyntaxKeyword, SWEET_ACCENT.0, SWEET_ACCENT.1, SWEET_ACCENT.2);
    set_color_rgb(&mut theme, ColorRole::SyntaxControlKeyword, SWEET_ACCENT.0, SWEET_ACCENT.1, SWEET_ACCENT.2);
    set_color_rgb(&mut theme, ColorRole::SyntaxString, SWEET_CYAN.0, SWEET_CYAN.1, SWEET_CYAN.2);
    set_color_rgb(&mut theme, ColorRole::SyntaxNumber, SWEET_YELLOW.0, SWEET_YELLOW.1, SWEET_YELLOW.2);
    set_color_rgb(&mut theme, ColorRole::SyntaxType, SWEET_CYAN.0, SWEET_CYAN.1, SWEET_CYAN.2);
    set_color_rgb(&mut theme, ColorRole::SyntaxIdentifier, SWEET_TEXT.0, SWEET_TEXT.1, SWEET_TEXT.2);
    set_color_rgb(&mut theme, ColorRole::SyntaxFunction, SWEET_FUNCTION.0, SWEET_FUNCTION.1, SWEET_FUNCTION.2);
    set_color_rgb(&mut theme, ColorRole::SyntaxVariable, SWEET_TEXT.0, SWEET_TEXT.1, SWEET_TEXT.2);
    set_color_rgb(&mut theme, ColorRole::SyntaxCustomType, SWEET_CYAN.0, SWEET_CYAN.1, SWEET_CYAN.2);
    set_color_rgb(&mut theme, ColorRole::SyntaxNamespace, SWEET_CYAN.0, SWEET_CYAN.1, SWEET_CYAN.2);
    set_color_rgb(&mut theme, ColorRole::SyntaxMember, SWEET_ACCENT.0, SWEET_ACCENT.1, SWEET_ACCENT.2);
    set_color_rgb(&mut theme, ColorRole::SyntaxParameter, SWEET_YELLOW.0, SWEET_YELLOW.1, SWEET_YELLOW.2);
    set_color_rgb(&mut theme, ColorRole::SyntaxPreprocessorStatement, SWEET_CYAN.0, SWEET_CYAN.1, SWEET_CYAN.2);
    set_color_rgb(&mut theme, ColorRole::SyntaxPreprocessorValue, SWEET_PREPROCESSOR.0, SWEET_PREPROCESSOR.1, SWEET_PREPROCESSOR.2);
    set_color_rgb(&mut theme, ColorRole::SyntaxPunctuation, SWEET_TEXT.0, SWEET_TEXT.1, SWEET_TEXT.2);
    set_color_rgb(&mut theme, ColorRole::SyntaxOperator, SWEET_TEXT.0, SWEET_TEXT.1, SWEET_TEXT.2);

    // Terminal colors (ANSI)
    set_color_rgb(&mut theme, ColorRole::Black, SWEET_WINDOW_BG.0, SWEET_WINDOW_BG.1, SWEET_WINDOW_BG.2);
    set_color_rgb(&mut theme, ColorRole::Red, SWEET_RED.0, SWEET_RED.1, SWEET_RED.2);
    set_color_rgb(&mut theme, ColorRole::Green, SWEET_GREEN.0, SWEET_GREEN.1, SWEET_GREEN.2);
    set_color_rgb(&mut theme, ColorRole::Yellow, SWEET_YELLOW.0, SWEET_YELLOW.1, SWEET_YELLOW.2);
    set_color_rgb(&mut theme, ColorRole::Blue, SWEET_BLUE.0, SWEET_BLUE.1, SWEET_BLUE.2);
    set_color_rgb(&mut theme, ColorRole::Magenta, SWEET_ACCENT.0, SWEET_ACCENT.1, SWEET_ACCENT.2);
    set_color_rgb(&mut theme, ColorRole::Cyan, SWEET_CYAN.0, SWEET_CYAN.1, SWEET_CYAN.2);
    set_color_rgb(&mut theme, ColorRole::White, SWEET_TEXT.0, SWEET_TEXT.1, SWEET_TEXT.2);
    set_color_rgb(&mut theme, ColorRole::BrightBlack, SWEET_GRAY_DARK.0, SWEET_GRAY_DARK.1, SWEET_GRAY_DARK.2);
    set_color_rgb(&mut theme, ColorRole::BrightRed, SWEET_RED.0, SWEET_RED.1, SWEET_RED.2);
    set_color_rgb(&mut theme, ColorRole::BrightGreen, SWEET_GREEN.0, SWEET_GREEN.1, SWEET_GREEN.2);
    set_color_rgb(&mut theme, ColorRole::BrightYellow, SWEET_YELLOW.0, SWEET_YELLOW.1, SWEET_YELLOW.2);
    set_color_rgb(&mut theme, ColorRole::BrightBlue, SWEET_BLUE.0, SWEET_BLUE.1, SWEET_BLUE.2);
    set_color_rgb(&mut theme, ColorRole::BrightMagenta, SWEET_ACCENT.0, SWEET_ACCENT.1, SWEET_ACCENT.2);
    set_color_rgb(&mut theme, ColorRole::BrightCyan, SWEET_CYAN.0, SWEET_CYAN.1, SWEET_CYAN.2);
    set_color_rgb(&mut theme, ColorRole::BrightWhite, SWEET_WHITE.0, SWEET_WHITE.1, SWEET_WHITE.2);
    set_color_rgb(&mut theme, ColorRole::ColorSchemeBackground, SWEET_WINDOW_BG.0, SWEET_WINDOW_BG.1, SWEET_WINDOW_BG.2);
    set_color_rgb(&mut theme, ColorRole::ColorSchemeForeground, SWEET_TEXT.0, SWEET_TEXT.1, SWEET_TEXT.2);

    // Other colors
    set_color_rgb(&mut theme, ColorRole::Accent, SWEET_ACCENT.0, SWEET_ACCENT.1, SWEET_ACCENT.2);
    set_color_rgb(&mut theme, ColorRole::DesktopBackground, SWEET_WINDOW_BG.0, SWEET_WINDOW_BG.1, SWEET_WINDOW_BG.2);
    set_color_rgb(&mut theme, ColorRole::FocusOutline, SWEET_ACCENT.0, SWEET_ACCENT.1, SWEET_ACCENT.2);
    set_color_rgb(&mut theme, ColorRole::TextCursor, SWEET_TEXT.0, SWEET_TEXT.1, SWEET_TEXT.2);
    set_color_rgb(&mut theme, ColorRole::ThreedHighlight, SWEET_GRAY.0, SWEET_GRAY.1, SWEET_GRAY.2);
    set_color_rgb(&mut theme, ColorRole::ThreedShadow1, SWEET_GRAY_DARK.0, SWEET_GRAY_DARK.1, SWEET_GRAY_DARK.2);
    set_color_rgb(&mut theme, ColorRole::ThreedShadow2, SWEET_WINDOW_BG.0, SWEET_WINDOW_BG.1, SWEET_WINDOW_BG.2);
    set_color_rgba(&mut theme, ColorRole::RubberBandFill, SWEET_ACCENT.0, SWEET_ACCENT.1, SWEET_ACCENT.2, 60);
    set_color_rgb(&mut theme, ColorRole::RubberBandBorder, SWEET_ACCENT.0, SWEET_ACCENT.1, SWEET_ACCENT.2);
    set_color_rgb(&mut theme, ColorRole::Gutter, SWEET_GUTTER.0, SWEET_GUTTER.1, SWEET_GUTTER.2);
    set_color_rgb(&mut theme, ColorRole::GutterBorder, SWEET_GRAY.0, SWEET_GRAY.1, SWEET_GRAY.2);
    set_color_rgb(&mut theme, ColorRole::Ruler, SWEET_GUTTER.0, SWEET_GUTTER.1, SWEET_GUTTER.2);
    set_color_rgb(&mut theme, ColorRole::RulerBorder, SWEET_GRAY.0, SWEET_GRAY.1, SWEET_GRAY.2);
    set_color_rgb(&mut theme, ColorRole::RulerActiveText, SWEET_TEXT.0, SWEET_TEXT.1, SWEET_TEXT.2);
    set_color_rgb(&mut theme, ColorRole::RulerInactiveText, SWEET_GRAY.0, SWEET_GRAY.1, SWEET_GRAY.2);
    set_color_rgb(&mut theme, ColorRole::Tooltip, SWEET_BASE.0, SWEET_BASE.1, SWEET_BASE.2);
    set_color_rgb(&mut theme, ColorRole::TooltipText, SWEET_TEXT.0, SWEET_TEXT.1, SWEET_TEXT.2);
    set_color_rgb(&mut theme, ColorRole::Tray, SWEET_WINDOW_BG.0, SWEET_WINDOW_BG.1, SWEET_WINDOW_BG.2);
    set_color_rgb(&mut theme, ColorRole::TrayText, SWEET_TEXT.0, SWEET_TEXT.1, SWEET_TEXT.2);
    set_color_rgb(&mut theme, ColorRole::OverlayBackground, SWEET_WINDOW_BG.0, SWEET_WINDOW_BG.1, SWEET_WINDOW_BG.2);
    set_color_rgb(&mut theme, ColorRole::OverlayText, SWEET_TEXT.0, SWEET_TEXT.1, SWEET_TEXT.2);
    set_color_rgb(&mut theme, ColorRole::HighlightSearching, SWEET_YELLOW.0, SWEET_YELLOW.1, SWEET_YELLOW.2);
    set_color_rgb(&mut theme, ColorRole::HighlightSearchingText, SWEET_WINDOW_BG.0, SWEET_WINDOW_BG.1, SWEET_WINDOW_BG.2);

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
        background: rgba8(SWEET_WINDOW_BG.0, SWEET_WINDOW_BG.1, SWEET_WINDOW_BG.2, 255),
        foreground: rgba8(SWEET_TEXT.0, SWEET_TEXT.1, SWEET_TEXT.2, 255),
        normal: [
            rgba8(SWEET_WINDOW_BG.0, SWEET_WINDOW_BG.1, SWEET_WINDOW_BG.2, 255),    // Black
            rgba8(SWEET_RED.0, SWEET_RED.1, SWEET_RED.2, 255),   // Red
            rgba8(SWEET_GREEN.0, SWEET_GREEN.1, SWEET_GREEN.2, 255),   // Green
            rgba8(SWEET_YELLOW.0, SWEET_YELLOW.1, SWEET_YELLOW.2, 255),  // Yellow
            rgba8(SWEET_BLUE.0, SWEET_BLUE.1, SWEET_BLUE.2, 255),  // Blue
            rgba8(SWEET_ACCENT.0, SWEET_ACCENT.1, SWEET_ACCENT.2, 255),  // Magenta
            rgba8(SWEET_CYAN.0, SWEET_CYAN.1, SWEET_CYAN.2, 255),   // Cyan
            rgba8(SWEET_TEXT.0, SWEET_TEXT.1, SWEET_TEXT.2, 255), // White
        ],
        bright: [
            rgba8(SWEET_GRAY_DARK.0, SWEET_GRAY_DARK.1, SWEET_GRAY_DARK.2, 255),    // BrightBlack
            rgba8(SWEET_RED.0, SWEET_RED.1, SWEET_RED.2, 255),   // BrightRed
            rgba8(SWEET_GREEN.0, SWEET_GREEN.1, SWEET_GREEN.2, 255),   // BrightGreen
            rgba8(SWEET_YELLOW.0, SWEET_YELLOW.1, SWEET_YELLOW.2, 255), // BrightYellow
            rgba8(SWEET_BLUE.0, SWEET_BLUE.1, SWEET_BLUE.2, 255),  // BrightBlue
            rgba8(SWEET_ACCENT.0, SWEET_ACCENT.1, SWEET_ACCENT.2, 255),  // BrightMagenta
            rgba8(SWEET_CYAN.0, SWEET_CYAN.1, SWEET_CYAN.2, 255),   // BrightCyan
            rgba8(SWEET_WHITE.0, SWEET_WHITE.1, SWEET_WHITE.2, 255), // BrightWhite
        ],
    };
    theme.set_terminal_colors(terminal_colors);

    theme
}
