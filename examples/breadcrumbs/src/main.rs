use nptk::prelude::*;
use nptk_widgets_extra::breadcrumbs::{Breadcrumbs, BreadcrumbItem};

struct BreadcrumbsApp;

impl Application for BreadcrumbsApp {
    type State = ();

    fn build(_context: AppContext, _state: Self::State) -> impl Widget {
        Container::new_empty()
            .with_child(
                Text::new("Breadcrumbs Widget Demo".to_string())
                    .with_font_size(24.0)
                    .with_layout_style(LayoutStyle {
                        margin: Rect {
                            left: LengthPercentageAuto::length(20.0),
                            right: LengthPercentageAuto::length(20.0),
                            top: LengthPercentageAuto::length(20.0),
                            bottom: LengthPercentageAuto::length(20.0),
                        },
                        ..Default::default()
                    })
            )
            .with_child(
                Text::new("File System Navigation:".to_string())
                    .with_font_size(16.0)
                    .with_layout_style(LayoutStyle {
                        margin: Rect {
                            left: LengthPercentageAuto::length(20.0),
                            right: LengthPercentageAuto::length(20.0),
                            top: LengthPercentageAuto::length(10.0),
                            bottom: LengthPercentageAuto::length(5.0),
                        },
                        ..Default::default()
                    })
            )
            .with_child(
                Breadcrumbs::new()
                    .with_items(vec![
                        BreadcrumbItem::new("Home").with_id("/home/user"),
                        BreadcrumbItem::new("Documents").with_id("/home/user/Documents"),
                        BreadcrumbItem::new("Projects").with_id("/home/user/Documents/Projects"),
                        BreadcrumbItem::new("MyApp").with_clickable(false),
                    ])
                    .with_separator(" > ")
                    .with_on_click(|item| {
                        println!("Clicked breadcrumb: {} (ID: {:?})", item.label, item.id);
                        Update::empty()
                    })
                    .with_layout_style(LayoutStyle {
                        margin: Rect {
                            left: LengthPercentageAuto::length(20.0),
                            right: LengthPercentageAuto::length(20.0),
                            top: LengthPercentageAuto::length(5.0),
                            bottom: LengthPercentageAuto::length(10.0),
                        },
                        ..Default::default()
                    })
            )
            .with_child(
                Text::new("Web Navigation:".to_string())
                    .with_font_size(16.0)
                    .with_layout_style(LayoutStyle {
                        margin: Rect {
                            left: LengthPercentageAuto::length(20.0),
                            right: LengthPercentageAuto::length(20.0),
                            top: LengthPercentageAuto::length(10.0),
                            bottom: LengthPercentageAuto::length(5.0),
                        },
                        ..Default::default()
                    })
            )
            .with_child(
                Breadcrumbs::new()
                    .with_items(vec![
                        BreadcrumbItem::new("Documentation").with_id("/docs"),
                        BreadcrumbItem::new("API Reference").with_id("/docs/api"),
                        BreadcrumbItem::new("Widgets").with_id("/docs/api/widgets"),
                        BreadcrumbItem::new("Breadcrumbs").with_clickable(false),
                    ])
                    .with_separator(" / ")
                    .with_on_click(|item| {
                        println!("Web navigation: {} (ID: {:?})", item.label, item.id);
                        Update::empty()
                    })
                    .with_layout_style(LayoutStyle {
                        margin: Rect {
                            left: LengthPercentageAuto::length(20.0),
                            right: LengthPercentageAuto::length(20.0),
                            top: LengthPercentageAuto::length(5.0),
                            bottom: LengthPercentageAuto::length(10.0),
                        },
                        ..Default::default()
                    })
            )
            .with_child(
                Text::new("Collapsed Navigation (max 3 items):".to_string())
                    .with_font_size(16.0)
                    .with_layout_style(LayoutStyle {
                        margin: Rect {
                            left: LengthPercentageAuto::length(20.0),
                            right: LengthPercentageAuto::length(20.0),
                            top: LengthPercentageAuto::length(10.0),
                            bottom: LengthPercentageAuto::length(5.0),
                        },
                        ..Default::default()
                    })
            )
            .with_child(
                Breadcrumbs::new()
                    .with_items(vec![
                        BreadcrumbItem::new("Root").with_id("/"),
                        BreadcrumbItem::new("Very").with_id("/very"),
                        BreadcrumbItem::new("Long").with_id("/very/long"),
                        BreadcrumbItem::new("Path").with_id("/very/long/path"),
                        BreadcrumbItem::new("Structure").with_id("/very/long/path/structure"),
                        BreadcrumbItem::new("Here").with_clickable(false),
                    ])
                    .with_separator(" → ")
                    .with_max_items(3)
                    .with_show_root(true)
                    .with_on_click(|item| {
                        println!("Collapsed navigation: {} (ID: {:?})", item.label, item.id);
                        Update::empty()
                    })
                    .with_layout_style(LayoutStyle {
                        margin: Rect {
                            left: LengthPercentageAuto::length(20.0),
                            right: LengthPercentageAuto::length(20.0),
                            top: LengthPercentageAuto::length(5.0),
                            bottom: LengthPercentageAuto::length(20.0),
                        },
                        ..Default::default()
                    })
            )
            .with_layout_style(LayoutStyle {
                flex_direction: FlexDirection::Column,
                size: Vector2::new(Dimension::percent(1.0), Dimension::percent(1.0)),
                ..Default::default()
            })
    }
}

fn main() {
    BreadcrumbsApp.run(())
}