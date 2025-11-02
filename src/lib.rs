#![warn(missing_docs)]

//! Create beautiful and lightning fast UI Applications with Rust.

pub use nalgebra as math;
pub use peniko as color;

pub use nptk_core as core;
pub use nptk_theme as theme;
pub use nptk_widgets as widgets;

#[cfg(feature = "macros")]
pub use nptk_macros as macros;

/// A "prelude" for users of the nptk toolkit.
///
/// Importing this module brings into scope the most common types
/// needed to build a basic nptk application.
///
/// ```rust
/// use nptk::prelude::*;
/// ```
pub mod prelude {
    pub use crate::core::app::{context::AppContext, Application, update::Update};
    pub use crate::core::widget::{Widget, WidgetLayoutExt};
    pub use crate::core::signal::{*, fixed::FixedSignal, eval::EvalSignal, state::StateSignal, map::MapSignal, rw::RwSignal, actor::ActorSignal};
    pub use crate::core::layout::*;
    pub use crate::core::reference::{Ref, MutRef};

    // Theme
    pub use crate::theme::theme::{system::SystemTheme, dark::DarkTheme, celeste::CelesteTheme, sweet::SweetTheme};
    pub use crate::theme::config::{ThemeConfig, ThemeSource};
    pub use crate::theme::id::WidgetId;
    pub use crate::theme::globals::Globals;
    pub use crate::theme::properties::ThemeProperty;
    // Math
    pub use nalgebra::Vector2;

    // Color
    pub use crate::core::vg::*;

    // Widgets
    pub use crate::widgets::animator::Animator;
    pub use crate::widgets::button::Button;
    #[cfg(feature = "canvas")]
    pub use crate::widgets::canvas::Canvas;
    pub use crate::widgets::checkbox::{Checkbox, CheckboxState};
    pub use crate::widgets::container::Container;
    pub use crate::widgets::fetcher::WidgetFetcher;
    pub use crate::widgets::gesture_detector::GestureDetector;
    pub use crate::widgets::icon::Icon;
    pub use crate::widgets::image::{Image, ImageData};
    pub use crate::widgets::menu_button::*;
    pub use crate::widgets::menu_popup::MenuPopup;
    pub use crate::widgets::menubar::*;
    pub use crate::widgets::progress::Progress;
    pub use crate::widgets::radio_button::{RadioButton, RadioButtonState};
    pub use crate::widgets::scroll_container::ScrollContainer;
    pub use crate::widgets::secret_input::SecretInput;
    pub use crate::widgets::slider::Slider;
    pub use crate::widgets::toggle::Toggle;
    pub use crate::widgets::tabs_container::TabsContainer;
    pub use crate::widgets::text_input::TextInput;
    pub use crate::widgets::text::Text;
    pub use crate::widgets::value_input::ValueInput;

}
