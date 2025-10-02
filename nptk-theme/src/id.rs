//! # Widget Identifiers
//!
//! This module provides widget identification functionality for the NPTK theming system.
//! Widget IDs are used to uniquely identify widget types and associate them with
//! their corresponding theme styles.
//!
//! ## Overview
//!
//! The id module provides:
//!
//! - **[WidgetId]**: Unique identifier for widget types
//! - **Namespace Support**: Hierarchical widget identification
//! - **Type Safety**: Compile-time widget type identification
//!
//! ## Key Features
//!
//! - **Unique Identification**: Each widget type has a unique identifier
//! - **Namespace Support**: Hierarchical organization of widget types
//! - **Type Safety**: Compile-time identification of widget types
//! - **Hash Support**: Efficient storage in HashMaps and HashSets
//! - **Ordering Support**: Consistent ordering for sorting and comparison
//!
//! ## Usage Examples
//!
//! ### Basic Widget ID Creation
//!
//! ```rust
//! use nptk_theme::id::WidgetId;
//!
//! // Create a widget ID for a button
//! let button_id = WidgetId::new("nptk-widgets", "Button");
//!
//! // Create a widget ID for a text input
//! let text_input_id = WidgetId::new("nptk-widgets", "TextInput");
//!
//! // Create a widget ID for a custom widget
//! let custom_id = WidgetId::new("my-crate", "MyCustomWidget");
//! ```
//!
//! ### Widget ID Usage in Themes
//!
//! ```rust
//! use nptk_theme::id::WidgetId;
//! use nptk_theme::theme::Theme;
//! use nptk_theme::style::Style;
//!
//! impl Theme for MyTheme {
//!     fn of(&self, id: WidgetId) -> Option<Style> {
//!         match id.namespace() {
//!             "nptk-widgets" => match id.id() {
//!                 "Button" => Some(/* button style */),
//!                 "TextInput" => Some(/* text input style */),
//!                 _ => None,
//!             },
//!             "my-crate" => match id.id() {
//!                 "MyCustomWidget" => Some(/* custom widget style */),
//!                 _ => None,
//!             },
//!             _ => None,
//!         }
//!     }
//! }
//! ```
//!
//! ### Widget ID Comparison and Ordering
//!
//! ```rust
//! use nptk_theme::id::WidgetId;
//!
//! let button_id = WidgetId::new("nptk-widgets", "Button");
//! let text_id = WidgetId::new("nptk-widgets", "Text");
//! let custom_id = WidgetId::new("my-crate", "Custom");
//!
//! // Widget IDs can be compared
//! assert!(button_id != text_id);
//! assert!(button_id == button_id);
//!
//! // Widget IDs can be ordered (lexicographically)
//! let mut ids = vec![custom_id, button_id, text_id];
//! ids.sort();
//! // Order: button_id, custom_id, text_id
//! ```
//!
//! ### Widget ID in HashMaps
//!
//! ```rust
//! use nptk_theme::id::WidgetId;
//! use std::collections::HashMap;
//!
//! let mut widget_styles: HashMap<WidgetId, String> = HashMap::new();
//!
//! let button_id = WidgetId::new("nptk-widgets", "Button");
//! let text_id = WidgetId::new("nptk-widgets", "Text");
//!
//! widget_styles.insert(button_id, "button_style".to_string());
//! widget_styles.insert(text_id, "text_style".to_string());
//!
//! // Retrieve styles
//! let button_style = widget_styles.get(&WidgetId::new("nptk-widgets", "Button"));
//! ```
//!
//! ## Namespace Conventions
//!
//! ### Standard Namespaces
//!
//! Use consistent namespace conventions for better organization:
//!
//! ```rust
//! use nptk_theme::id::WidgetId;
//!
//! // Standard NPTK widgets
//! let button_id = WidgetId::new("nptk-widgets", "Button");
//! let text_id = WidgetId::new("nptk-widgets", "Text");
//! let input_id = WidgetId::new("nptk-widgets", "TextInput");
//!
//! // Custom application widgets
//! let custom_id = WidgetId::new("my-app", "CustomWidget");
//! let special_id = WidgetId::new("my-app", "SpecialButton");
//!
//! // Third-party widget libraries
//! let third_party_id = WidgetId::new("third-party-crate", "ThirdPartyWidget");
//! ```
//!
//! ### Namespace Best Practices
//!
//! 1. **Use Crate Names**: Use the actual crate name as the namespace
//! 2. **Be Consistent**: Use consistent naming across your application
//! 3. **Avoid Conflicts**: Use unique namespaces to avoid conflicts
//! 4. **Document Conventions**: Document your namespace conventions
//! 5. **Use Hierarchical Names**: Use hierarchical names for complex widgets
//!
//! ## Performance Considerations
//!
//! - **String Storage**: Widget IDs store strings, so they have some memory overhead
//! - **Hash Performance**: Widget IDs implement Hash for efficient HashMap usage
//! - **Comparison Performance**: Widget IDs implement efficient comparison operations
//! - **Ordering Performance**: Widget IDs support efficient ordering operations
//!
//! ## Best Practices
//!
//! 1. **Use Consistent Namespaces**: Use consistent namespace conventions
//! 2. **Document Widget IDs**: Document what each widget ID represents
//! 3. **Use Type Safety**: Use widget IDs for type-safe theme access
//! 4. **Avoid String Literals**: Use constants for widget IDs to avoid typos
//! 5. **Test Widget IDs**: Test that widget IDs work correctly in your themes

use std::fmt::{Debug, Display, Formatter};

/// An identifier for a widget type in the theming system.
///
/// This struct represents a unique identifier for a widget type, not for individual
/// widget instances. It consists of a namespace (typically the crate name) and an
/// ID (the widget type name). This allows for hierarchical organization and
/// prevents naming conflicts between different widget libraries.
///
/// # Examples
///
/// ```rust
/// use nptk_theme::id::WidgetId;
///
/// // Create a widget ID for a button
/// let button_id = WidgetId::new("nptk-widgets", "Button");
///
/// // Create a widget ID for a custom widget
/// let custom_id = WidgetId::new("my-crate", "MyCustomWidget");
///
/// // Access namespace and ID
/// assert_eq!(button_id.namespace(), "nptk-widgets");
/// assert_eq!(button_id.id(), "Button");
/// ```
///
/// # Namespace Conventions
///
/// - **Standard Widgets**: Use `"nptk-widgets"` for standard NPTK widgets
/// - **Custom Widgets**: Use your crate name for custom widgets
/// - **Third-party**: Use the third-party crate name for external widgets
///
/// # Type Safety
///
/// Widget IDs provide type safety by ensuring that each widget type has a unique
/// identifier. This prevents accidental mixing of widget types and provides
/// compile-time safety for theme access.
///
/// # Performance
///
/// - **String Storage**: Stores strings for namespace and ID
/// - **Hash Support**: Implements Hash for efficient HashMap usage
/// - **Comparison**: Efficient comparison and ordering operations
/// - **Memory**: Some memory overhead due to string storage
///
/// # Best Practices
///
/// 1. **Use Constants**: Define widget IDs as constants to avoid typos
/// 2. **Consistent Namespaces**: Use consistent namespace conventions
/// 3. **Document IDs**: Document what each widget ID represents
/// 4. **Test Themes**: Test that widget IDs work correctly in themes
/// 5. **Avoid Conflicts**: Use unique namespaces to prevent conflicts
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct WidgetId {
    namespace: String,
    id: String,
}

impl WidgetId {
    /// Create a new widget id by a namespace and custom id.
    /// The namespace should be the crate name and the id should be the widget type name.
    ///
    /// Example:
    /// ```
    /// let id = nptk_theme::id::WidgetId::new("my_crate", "MyWidget");
    /// ```
    pub fn new(namespace: impl ToString, id: impl ToString) -> Self {
        Self {
            namespace: namespace.to_string(),
            id: id.to_string(),
        }
    }

    /// Returns the namespace of the widget id.
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Returns the actual widget id.
    pub fn id(&self) -> &str {
        &self.id
    }
}

impl Display for WidgetId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.namespace, self.id)
    }
}
