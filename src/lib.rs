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
    pub use crate::core::app::{context::AppContext, Application};
    pub use crate::core::widget::Widget;
    pub use crate::theme::theme::system::SystemTheme;
    pub use crate::widgets::text::Text;
    // Add more common types as needed
}
