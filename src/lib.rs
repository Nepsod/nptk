#![warn(missing_docs)]

//! Create beautiful and lightning fast UI Applications with Rust.

pub use nalgebra as math;
pub use vello::peniko as color;

pub use nptk_core as core;
#[cfg(feature = "macros")]
pub use nptk_macros as macros;
pub use nptk_services as services;

/// Widgets module aggregating core and extra widgets.
pub mod widgets {
    pub use nptk_widgets::*;
    #[cfg(feature = "lgpl-widgets")]
    pub use nptk_widgets_extra::*;
}

/// A "prelude" for users of the nptk toolkit.
///
/// Importing this module brings into scope the most common types
/// needed to build a basic nptk application.
///
/// ```rust
/// use nptk::prelude::*;
/// ```
pub mod prelude {
    pub use crate::core::app::{context::AppContext, update::Update, Application};
    pub use crate::core::layout::*;
    pub use crate::core::reference::Ref;
    pub use crate::core::signal::{
        eval::EvalSignal, fixed::FixedSignal, map::MapSignal, state::StateSignal, *,
    };
    pub use crate::core::widget::{Widget, WidgetLayoutExt};

    // Theme (new role-based system)
    pub use crate::core::theme::{Palette, ColorRole, ThemeResolver};

    // Math
    pub use nalgebra::Vector2;

    // Color
    pub use crate::core::vg::*;

    // Widgets (MIT)
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
    pub use crate::widgets::slider::Slider;
    pub use crate::widgets::text::Text;

    // Widgets (LGPL)
    #[cfg(feature = "lgpl-widgets")]
    pub use crate::widgets::menu_button::*;
    #[cfg(feature = "lgpl-widgets")]
    pub use crate::widgets::menu_popup::MenuPopup;
    #[cfg(feature = "lgpl-widgets")]
    pub use crate::widgets::menubar::*;
    #[cfg(feature = "lgpl-widgets")]
    pub use crate::widgets::progress::Progress;
    #[cfg(feature = "lgpl-widgets")]
    pub use crate::widgets::radio_button::{RadioButton, RadioButtonState};
    #[cfg(feature = "lgpl-widgets")]
    pub use crate::widgets::scroll_container::ScrollContainer;
    #[cfg(feature = "lgpl-widgets")]
    pub use crate::widgets::secret_input::SecretInput;
    #[cfg(feature = "lgpl-widgets")]
    pub use crate::widgets::tabs_container::TabsContainer;
    #[cfg(feature = "lgpl-widgets")]
    pub use crate::widgets::text_input::TextInput;
    #[cfg(feature = "lgpl-widgets")]
    pub use crate::widgets::toggle::Toggle;
    #[cfg(feature = "lgpl-widgets")]
    pub use crate::widgets::toolbar::{Toolbar, ToolbarButton, ToolbarSeparator, ToolbarSpacer};
    #[cfg(feature = "lgpl-widgets")]
    pub use crate::widgets::value_input::ValueInput;
    #[cfg(feature = "lgpl-widgets")]
    pub use crate::widgets::file_icon::FileIcon;
    #[cfg(feature = "lgpl-widgets")]
    pub use crate::widgets::expandable_section::ExpandableSection;
    #[cfg(feature = "lgpl-widgets")]
    pub use crate::widgets::sidebar::{Sidebar, SidebarItem, SidebarSection};
}
