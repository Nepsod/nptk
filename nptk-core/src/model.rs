use std::any::Any;
use std::sync::Arc;
use crate::signal::state::StateSignal;

/// Role for data lookup in the model
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ItemRole {
    /// The primary text to display (String)
    Display,
    /// The icon to display (usually some Icon type or path)
    Icon,
    /// Tooltip text (String)
    ToolTip,
    /// Sort key (for sorting, defaults to Display)
    Sort,
    /// User defined role starting point
    User(u32),
}

/// Generic data variant for model data
#[derive(Debug, Clone)]
pub enum ModelData {
    String(String),
    Bool(bool),
    Int(i64),
    Float(f64),
    Custom(Arc<dyn Any + Send + Sync>),
    None,
}

impl From<String> for ModelData {
    fn from(s: String) -> Self {
        ModelData::String(s)
    }
}

impl From<&str> for ModelData {
    fn from(s: &str) -> Self {
        ModelData::String(s.to_string())
    }
}

/// Trait defining a data model for ItemViews
pub trait ItemModel: Send + Sync {
    /// Number of rows in the model
    fn row_count(&self) -> usize;
    
    /// Number of columns in the model
    fn column_count(&self) -> usize;
    
    /// Get data for a specific index and role
    fn data(&self, row: usize, col: usize, role: ItemRole) -> ModelData;
    
    /// Get header data for a specific column
    fn header_data(&self, section: usize, orientation: Orientation, role: ItemRole) -> ModelData {
        if role == ItemRole::Display && orientation == Orientation::Horizontal {
            ModelData::String(format!("Column {}", section))
        } else {
            ModelData::None
        }
    }
    
    /// Called when the view needs to refresh/invalidate
    /// This is a simple mechanism; in a real generic implementation we'd need signals.
    /// For now, we assume the View holds the Model and subscribes to it externally
    /// or the Model exposes a signal.
    fn on_update(&self) -> Option<StateSignal<()>> {
       None 
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}
