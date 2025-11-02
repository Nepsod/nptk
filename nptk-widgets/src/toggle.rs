use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout;
use nptk_core::layout::{Dimension, LayoutNode, LayoutStyle, LengthPercentageAuto, StyleNode};
use nptk_core::signal::MaybeSignal;
use nptk_core::vg::kurbo::{Affine, Rect, Stroke};
use nptk_core::vg::peniko::{Brush, Color, Fill};
use nptk_core::vg::Scene;
use nptk_core::widget::{Widget, WidgetLayoutExt};
use nptk_core::window::{ElementState, MouseButton};
use nptk_theme::id::WidgetId;
use nptk_theme::properties::ThemeProperty;
use nptk_theme::theme::Theme;
use nalgebra::Vector2;

/// A toggle/switch button widget with Win8 Metro style.
/// 
/// The toggle has two states: on (true) and off (false).
/// When toggled, the thumb slides between the left (off) and right (on) positions.
/// 
/// ### Theming
/// Uses similar colors to the slider:
/// - ON state: primary-dark track, accent thumb
/// - OFF state: gray track, gray thumb
pub struct Toggle {
    /// Whether the toggle is on (true) or off (false)
    state: MaybeSignal<bool>,
    /// Callback when the toggle state changes
    on_toggle: Option<Box<dyn Fn(bool) + Send + Sync>>,
    /// Layout styling
    layout_style: MaybeSignal<LayoutStyle>,
    /// Whether the toggle is disabled
    disabled: bool,
}

impl Toggle {
    /// Create a new toggle switch.
    /// 
    /// # Arguments
    /// * `state` - A signal containing the boolean state (true = on, false = off)
    pub fn new(state: impl Into<MaybeSignal<bool>>) -> Self {
        Self {
            state: state.into(),
            on_toggle: None,
            layout_style: LayoutStyle {
                size: Vector2::<Dimension>::new(Dimension::length(36.0), Dimension::length(16.0)),
                margin: layout::Rect::<LengthPercentageAuto> {
                    left: LengthPercentageAuto::length(0.0),
                    right: LengthPercentageAuto::length(0.0),
                    top: LengthPercentageAuto::length(0.0),
                    bottom: LengthPercentageAuto::length(0.0),
                },
                ..Default::default()
            }
            .into(),
            disabled: false,
        }
    }

    /// Set a callback to be called when the toggle state changes.
    pub fn with_on_toggle(mut self, callback: impl Fn(bool) + Send + Sync + 'static) -> Self {
        self.on_toggle = Some(Box::new(callback));
        self
    }

    /// Set whether the toggle is disabled.
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Toggle the state (flip from on to off or vice versa).
    pub fn toggle(&mut self) {
        if !self.disabled {
            let new_state = !*self.state.get();
            if let Some(sig) = self.state.as_signal() {
                sig.set(new_state);
            }
            
            if let Some(callback) = &self.on_toggle {
                callback(new_state);
            }
        }
    }
}

impl WidgetLayoutExt for Toggle {
    fn with_layout_style(mut self, style: impl Into<MaybeSignal<LayoutStyle>>) -> Self {
        self.layout_style = style.into();
        self
    }

    fn set_layout_style(&mut self, style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.layout_style = style.into();
    }
}

impl Widget for Toggle {
    fn render(
        &mut self,
        scene: &mut Scene,
        theme: &mut dyn Theme,
        layout_node: &LayoutNode,
        _: &mut AppInfo,
        _: AppContext,
    ) {
        let is_on = *self.state.get();
        
        let track_width = layout_node.layout.size.width;
        let track_height = layout_node.layout.size.height;
        let track_x = layout_node.layout.location.x as f64;
        let track_y = layout_node.layout.location.y as f64;
        let track_rect = Rect::new(
            track_x,
            track_y,
            track_x + track_width as f64,
            track_y + track_height as f64,
        );

        // Get colors from theme
        let widget_id = self.widget_id();
        let (track_color, track_border_color, thumb_color, thumb_border_color) = if self.disabled {
            // Disabled: use theme disabled color
            let disabled_color = theme.get_property(widget_id.clone(), &ThemeProperty::ColorToggleDisabled)
                .unwrap_or_else(|| Color::from_rgb8(200, 200, 200));
            (
                disabled_color,
                disabled_color,
                disabled_color,
                disabled_color,
            )
        } else if is_on {
            // ON state: get colors from theme
            (
                theme.get_property(widget_id.clone(), &ThemeProperty::ColorToggleTrackOn)
                    .unwrap_or_else(|| Color::from_rgb8(157, 51, 213)),
                theme.get_property(widget_id.clone(), &ThemeProperty::ColorToggleTrackOn)
                    .unwrap_or_else(|| Color::from_rgb8(157, 51, 213)), // same as track (no visible border)
                theme.get_property(widget_id.clone(), &ThemeProperty::ColorToggleThumb)
                    .unwrap_or_else(|| Color::from_rgb8(255, 255, 255)),
                theme.get_property(widget_id.clone(), &ThemeProperty::ColorToggleThumbBorder)
                    .unwrap_or_else(|| Color::from_rgb8(180, 180, 180)),
            )
        } else {
            // OFF state: get colors from theme
            (
                theme.get_property(widget_id.clone(), &ThemeProperty::ColorToggleTrackOff)
                    .unwrap_or_else(|| Color::from_rgb8(240, 240, 240)),
                theme.get_property(widget_id.clone(), &ThemeProperty::ColorToggleTrackBorder)
                    .unwrap_or_else(|| Color::from_rgb8(180, 180, 180)),
                theme.get_property(widget_id.clone(), &ThemeProperty::ColorToggleThumb)
                    .unwrap_or_else(|| Color::from_rgb8(255, 255, 255)),
                theme.get_property(widget_id.clone(), &ThemeProperty::ColorToggleThumbBorder)
                    .unwrap_or_else(|| Color::from_rgb8(180, 180, 180)),
            )
        };

        // Draw track background (full width) - rectangular shape
        scene.fill(
            Fill::NonZero,
            Affine::default(),
            &Brush::Solid(track_color),
            None,
            &track_rect,
        );
        
        // Draw track border (darker outline for OFF state)
        scene.stroke(
            &Stroke::new(2.0),
            Affine::default(),
            &Brush::Solid(track_border_color),
            None,
            &track_rect,
        );

        // Draw thumb - thin rectangular shape (tall and narrow), positioned near edges
        let thumb_height = track_height as f64 * 1.6; // Slightly smaller than track height
        let thumb_width = thumb_height * 0.5; // Thin rectangle - much taller than wide (vertical thin rectangle)
        let thumb_vertical_margin = (track_height as f64 - thumb_height) / 2.0;
        let edge_offset = 1.0; // Small offset from edge for Metro style
        
        // Calculate thumb position: left when off, right when on (close to edges)
        let thumb_x = if is_on {
            track_x + track_width as f64 - thumb_width - thumb_vertical_margin - edge_offset
        } else {
            track_x + thumb_vertical_margin + edge_offset
        };
        let thumb_y = track_y + thumb_vertical_margin;
        
        let thumb_rect = Rect::new(
            thumb_x,
            thumb_y,
            thumb_x + thumb_width,
            thumb_y + thumb_height,
        );
        
        // Draw thumb fill (rectangular/square)
        scene.fill(
            Fill::NonZero,
            Affine::default(),
            &Brush::Solid(thumb_color),
            None,
            &thumb_rect,
        );
        
        // Draw subtle gray border on thumb (the "smaller bar" effect)
        scene.stroke(
            &Stroke::new(2.0),
            Affine::default(),
            &Brush::Solid(thumb_border_color),
            None,
            &thumb_rect,
        );
    }

    fn layout_style(&self) -> StyleNode {
        StyleNode {
            style: self.layout_style.get().clone(),
            children: Vec::new(),
        }
    }

    fn update(&mut self, layout: &LayoutNode, _: AppContext, info: &mut AppInfo) -> Update {
        let mut update = Update::empty();

        if self.disabled {
            return update;
        }

        // Check if mouse is over the toggle
        if let Some(cursor) = info.cursor_pos {
            let in_bounds = cursor.x as f32 >= layout.layout.location.x
                && cursor.x as f32 <= layout.layout.location.x + layout.layout.size.width
                && cursor.y as f32 >= layout.layout.location.y
                && cursor.y as f32 <= layout.layout.location.y + layout.layout.size.height;

            // Check for mouse clicks (on release to prevent multiple toggles)
            for (_, btn, el_state) in &info.buttons {
                if btn == &MouseButton::Left && *el_state == ElementState::Released && in_bounds {
                    self.toggle();
                    update.insert(Update::DRAW);
                    break;
                }
            }
        }

        update
    }

    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "Toggle")
    }
}

impl Default for Toggle {
    fn default() -> Self {
        Self::new(false)
    }
}
