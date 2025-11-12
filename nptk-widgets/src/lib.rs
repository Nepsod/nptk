#![warn(missing_docs)]

//! Widget library for nptk => See `nptk` crate.
//!
//! Contains a collection of beautiful nptk widgets.

/// Contains the [text::Text] widget.
pub mod text;

/// Contains the [button::Button] widget.
pub mod button;

/// Contains the [container::Container] widget.
pub mod container;

/// Contains the [image::Image] widget.
pub mod image;

/// Contains the [checkbox::Checkbox] widget.
pub mod checkbox;

/// Contains the [slider::Slider] widget.
pub mod slider;

/// Contains the [fetcher::WidgetFetcher] widget.
pub mod fetcher;

/// Contains the [canvas::Canvas] widget.
#[cfg(feature = "canvas")]
pub mod canvas;

/// Contains the [gesture_detector::GestureDetector] widget.
pub mod gesture_detector;

/// Contains the [icon::Icon] widget.
pub mod icon;

/// Contains the [animator::Animator] widget and associated structures.
pub mod animator;

/// Contains the [text_input::TextInput] widget.
pub mod text_input;

/// Contains the [secret_input::SecretInput] widget.
pub mod secret_input;

/// Contains the [value_input::ValueInput] widget.
pub mod value_input;

/// Contains the [radio_button::RadioButton] widget.
pub mod radio_button;

/// Contains the [menubar::MenuBar] widget.
pub mod menubar;

/// Contains the [menu_popup::MenuPopup] widget.
pub mod menu_popup;

/// Contains the [scroll_container::ScrollContainer] widget.
pub mod scroll_container;

/// Contains the [tabs_container::TabsContainer] widget.
pub mod tabs_container;

/// Contains the [menu_button::MenuButton] widget.
pub mod menu_button;

/// Contains the [progress::Progress] widget.
pub mod progress;

/// Contains the [toggle::Toggle] widget.
pub mod toggle;

/// Contains theme rendering bridge functionality.
pub mod theme_rendering;
