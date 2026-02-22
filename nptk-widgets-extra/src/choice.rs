// SPDX-License-Identifier: LGPL-3.0-only
use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{Layout, LayoutContext, LayoutNode, LayoutStyle, StyleNode};
use nptk_core::signal::{state::StateSignal, MaybeSignal, Signal};
use nptk_core::vg::kurbo::Rect;
use nptk_core::vgi::vello_vg::VelloGraphics;
use nptk_core::vgi::Graphics;
use nptk_core::widget::{Widget, WidgetLayoutExt};
use nptk_core::window::{ElementState, MouseButton};
use std::sync::Arc;
use async_trait::async_trait;

use nptk_widgets::button::Button;
use nptk_widgets::text::Text;
use crate::menu_popup::MenuPopup;
use nptk_core::menu::unified::MenuItem as UnifiedMenuItem;
use nptk_core::menu::commands::MenuCommand;
use nptk_core::menu::unified::MenuTemplate;

/// A dropdown widget for selecting a single item from a list of options.
pub struct Choice {
    child: Box<dyn Widget>,
    is_open: Arc<StateSignal<bool>>,
    options: Vec<String>,
    selected_index: Arc<StateSignal<usize>>,
    on_changed: Option<Arc<dyn Fn(usize, String) + Send + Sync>>,
    popup_data: Option<MenuPopup>,
    layout_style: MaybeSignal<LayoutStyle>,
    tooltip: Option<String>,
}

impl std::fmt::Debug for Choice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Choice")
            .field("is_open", &*self.is_open.get())
            .field("options", &self.options.len())
            .field("selected_index", &*self.selected_index.get())
            .field("popup_data", &self.popup_data.is_some())
            .field("layout_style", &*self.layout_style.get())
            .finish()
    }
}

impl Choice {
    /// Create a new choice dropdown with the given options and default selection.
    pub fn new(options: Vec<String>, default_index: usize) -> Self {
        use nptk_core::layout::{Dimension, LengthPercentage};

        // Determine the longest option to size the button appropriately
        let max_len = options.iter().map(|s| s.chars().count()).max().unwrap_or(5);
        let font_size = 16.0;
        let estimated_text_width = (max_len as f32) * 8.0; 
        let horizontal_padding = font_size + 20.0; // Extra padding for a dropdown arrow indicator
        let button_width = estimated_text_width + horizontal_padding;
        let bottom_padding = font_size + 2.0;

        let display_text = options.get(default_index).cloned().unwrap_or_else(|| String::from("..."));
        
        let text = Text::new(format!("{} ▼", display_text))
            .with_font_size(font_size)
            .with_layout_style(LayoutStyle {
                size: nalgebra::Vector2::new(
                    Dimension::percent(1.0),
                    Dimension::auto(),
                ),
                ..Default::default()
            });

        let button = Button::new(text)
            .with_style_id("ChoiceButton")
            .with_layout_style(LayoutStyle {
                size: nalgebra::Vector2::new(
                    Dimension::length(button_width),
                    Dimension::length(bottom_padding + 4.0),
                ),
                padding: nptk_core::layout::Rect::<LengthPercentage> {
                    left: LengthPercentage::length(font_size / 2.0),
                    right: LengthPercentage::length(font_size / 2.0),
                    top: LengthPercentage::length(0.0),
                    bottom: LengthPercentage::length(bottom_padding),
                },
                ..Default::default()
            });

        Self {
            child: Box::new(button),
            is_open: Arc::new(StateSignal::new(false)),
            options,
            selected_index: Arc::new(StateSignal::new(default_index)),
            on_changed: None,
            popup_data: None,
            layout_style: MaybeSignal::value(LayoutStyle::default()),
            tooltip: None,
        }
    }

    /// Set the callback for when an option is selected.
    pub fn with_on_changed<F>(mut self, callback: F) -> Self
    where
        F: Fn(usize, String) + Send + Sync + 'static,
    {
        self.on_changed = Some(Arc::new(callback));
        self
    }

    pub fn with_tooltip(mut self, text: impl Into<String>) -> Self {
        self.tooltip = Some(text.into());
        self
    }

    pub fn is_open(&self) -> bool {
        *self.is_open.get()
    }

    pub fn close(&mut self) {
        self.is_open.set(false);
    }

    fn update_display_text(&mut self) {
        // Find our button -> text child and update its label
        // Since we created it with a specific structure, we'd ideally replace `self.child`
        // or recreate it. For now, we will simply recreate the button to update the text.
        use nptk_core::layout::{Dimension, LengthPercentage};
        
        let max_len = self.options.iter().map(|s| s.chars().count()).max().unwrap_or(5);
        let font_size = 16.0;
        let estimated_text_width = (max_len as f32) * 8.0; 
        let horizontal_padding = font_size + 20.0;
        let button_width = estimated_text_width + horizontal_padding;
        let bottom_padding = font_size + 2.0;

        let idx = *self.selected_index.get();
        let display_text = self.options.get(idx).cloned().unwrap_or_else(|| String::from("..."));
        
        let text = Text::new(format!("{} ▼", display_text))
            .with_font_size(font_size)
            .with_layout_style(LayoutStyle {
                size: nalgebra::Vector2::new(
                    Dimension::percent(1.0),
                    Dimension::auto(),
                ),
                ..Default::default()
            });

        let button = Button::new(text)
            .with_style_id("ChoiceButton")
            .with_layout_style(LayoutStyle {
                size: nalgebra::Vector2::new(
                    Dimension::length(button_width),
                    Dimension::length(bottom_padding + 4.0),
                ),
                padding: nptk_core::layout::Rect::<LengthPercentage> {
                    left: LengthPercentage::length(font_size / 2.0),
                    right: LengthPercentage::length(font_size / 2.0),
                    top: LengthPercentage::length(0.0),
                    bottom: LengthPercentage::length(bottom_padding),
                },
                ..Default::default()
            });
            
        self.child = Box::new(button);
    }

    fn show_popup(&mut self) {
        let mut items = Vec::new();
        
        for (idx, option) in self.options.iter().enumerate() {
            let label = option.clone();
            
            // Re-bind variables for the closure
            let selected_index_signal = self.selected_index.clone();
            let on_changed_callback = self.on_changed.clone();
            let option_label = option.clone();
            
            let item = UnifiedMenuItem::new(MenuCommand::Custom(0x1000 + idx as u32), &label).with_action(move || {
                selected_index_signal.set(idx);
                if let Some(cb) = &on_changed_callback {
                    cb(idx, option_label.clone());
                }
                Update::FORCE
            });
            
            items.push(item);
        }

        let template = MenuTemplate::from_items("choice_popup", items);
        let menu_popup = MenuPopup::new(template);
        self.popup_data = Some(menu_popup);
    }
}

impl WidgetLayoutExt for Choice {
    fn set_layout_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.layout_style = layout_style.into();
    }
}

#[async_trait(?Send)]
impl Widget for Choice {
    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        layout: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        if !layout.children.is_empty() {
            let mut child_scene = nptk_core::vg::Scene::new();
            let mut child_graphics = VelloGraphics::new(&mut child_scene);
            self.child.render(
                &mut child_graphics,
                &layout.children[0],
                info,
                context.clone(),
            );
            graphics.append(&child_scene, None);
        }
    }

    fn render_postfix(
        &mut self,
        graphics: &mut dyn Graphics,
        layout: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        if *self.is_open.get() {
            if let Some(ref mut popup) = self.popup_data {
                let (popup_width, popup_height) = popup.calculate_size();
                let popup_x = layout.layout.location.x as f64;
                let popup_y = (layout.layout.location.y + layout.layout.size.height) as f64;

                let mut popup_layout = LayoutNode {
                    layout: Layout::default(),
                    children: Vec::new(),
                };
                popup_layout.layout.location.x = popup_x as f32;
                popup_layout.layout.location.y = popup_y as f32;
                popup_layout.layout.size.width = popup_width as f32;
                popup_layout.layout.size.height = popup_height as f32;

                popup.render(graphics, &popup_layout, info, context);
            }
        }
    }

    fn layout_style(&self, context: &LayoutContext) -> StyleNode {
        StyleNode {
            style: self.layout_style.get().clone(),
            children: vec![self.child.layout_style(context)],
            measure_func: None,
        }
    }

    fn tooltip(&self) -> Option<String> {
        self.tooltip.clone()
    }

    async fn update(&mut self, layout: &LayoutNode, context: AppContext, info: &mut AppInfo) -> Update {
        let mut update = Update::empty();
        let mut was_clicked = false;
        let cursor_pos = info.cursor_pos;

        for (_, button, state) in &info.buttons {
            if *button == MouseButton::Left {
                if let Some(pos) = cursor_pos {
                    if pos.x as f32 >= layout.layout.location.x
                        && pos.x as f32 <= layout.layout.location.x + layout.layout.size.width
                        && pos.y as f32 >= layout.layout.location.y
                        && pos.y as f32 <= layout.layout.location.y + layout.layout.size.height
                    {
                        if *state == ElementState::Released {
                            was_clicked = true;
                        }
                    }
                }
            }
        }

        if !layout.children.is_empty() {
            update |= self
                .child
                .update(&layout.children[0], context.clone(), info).await;
        }

        let currently_open = *self.is_open.get();
        let old_index = *self.selected_index.get();

        if currently_open {
            if let Some(ref mut popup) = self.popup_data {
                let (popup_width, popup_height) = popup.calculate_size();
                let popup_x = layout.layout.location.x as f64;
                let popup_y = (layout.layout.location.y + layout.layout.size.height) as f64;

                let mut popup_layout = LayoutNode {
                    layout: Layout::default(),
                    children: Vec::new(),
                };
                popup_layout.layout.location.x = popup_x as f32;
                popup_layout.layout.location.y = popup_y as f32;
                popup_layout.layout.size.width = popup_width as f32;
                popup_layout.layout.size.height = popup_height as f32;

                let popup_update = popup.update(&popup_layout, context.clone(), info).await;
                update |= popup_update;

                if popup_update.contains(Update::FORCE) {
                    self.close();
                }
            }

            let mut click_outside = false;
            if let Some(pos) = cursor_pos {
                if let Some(ref popup) = self.popup_data {
                    let (popup_width, popup_height) = popup.calculate_size();
                    let popup_rect = Rect::new(
                        layout.layout.location.x as f64,
                        layout.layout.location.y as f64 + layout.layout.size.height as f64,
                        layout.layout.location.x as f64 + popup_width,
                        layout.layout.location.y as f64
                            + layout.layout.size.height as f64
                            + popup_height,
                    );
                    let button_rect = Rect::new(
                        layout.layout.location.x as f64,
                        layout.layout.location.y as f64,
                        layout.layout.location.x as f64 + layout.layout.size.width as f64,
                        layout.layout.location.y as f64 + layout.layout.size.height as f64,
                    );

                    for (_, button, state) in &info.buttons {
                        if *button == MouseButton::Left && *state == ElementState::Pressed {
                            if !popup_rect.contains((pos.x, pos.y))
                                && !button_rect.contains((pos.x, pos.y))
                            {
                                click_outside = true;
                            }
                        }
                    }
                }
            }

            if was_clicked || click_outside {
                self.close();
                update |= Update::DRAW;
            }
            
            // If the selection index changed during popup update, we should update our button text
            if old_index != *self.selected_index.get() {
                self.update_display_text();
                // We changed the child structure, trigger LAYOUT and DRAW
                update |= Update::LAYOUT | Update::DRAW;
            }
            
        } else if was_clicked {
            self.is_open.set(true);
            self.show_popup();
            update |= Update::DRAW;
        }

        update
    }
}
