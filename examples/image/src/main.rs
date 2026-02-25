use nptk::core::app::context::AppContext;
use nptk::core::app::Application;
use nptk::core::signal::fixed::FixedSignal;
use nptk::core::signal::Signal;
use nptk::core::vg::peniko::{Blob, ImageAlphaType, ImageFormat, ImageBrush};
use nptk::core::widget::Widget;
use nptk::widgets::image::{Image, ImageData};

const IMAGE_DATA: &[u8] = include_bytes!("../pelican.jpg");

struct MyApp;

impl Application for MyApp {
    type State = ();

    fn build(context: AppContext, _: Self::State) -> impl Widget {
        let rgba = image::load_from_memory(IMAGE_DATA)
            .expect("failed to load embedded image")
            .into_rgba8();
        let (width, height) = rgba.dimensions();

        let brush = FixedSignal::new(ImageBrush::new(ImageData {
            data: Blob::from(rgba.into_raw()),
            format: ImageFormat::Rgba8,
            alpha_type: ImageAlphaType::Alpha,
            width,
            height,
        }))
        .hook(&context);

        Image::new(brush.maybe())
    }
}

fn main() {
    MyApp.run(())
}
