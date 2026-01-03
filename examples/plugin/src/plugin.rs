use nptk::core::app::info::AppInfo;
use nptk::core::app::update::UpdateManager;
use nptk::core::config::MayConfig;
use nptk::core::layout::{NodeId, TaffyTree};
use nptk::core::plugin::{Plugin, PluginManager};
use nptk::core::vg::{Renderer, Scene};
use nptk::core::vgi::Surface;
use nptk::core::window::{ActiveEventLoop, Window, WindowEvent};
use std::sync::Arc;
use std::time::Instant;

pub struct MyPlugin;

impl Plugin for MyPlugin {
    fn name(&self) -> &'static str {
        "my_plugin"
    }

    fn on_register(&mut self, _manager: &mut PluginManager) {
        println!("Hello World!");
    }

    fn on_unregister(&mut self, _manager: &mut PluginManager) {
        println!("Bye World!");
    }

    fn on_window_event(
        &mut self,
        event: &mut WindowEvent,
        _config: &mut MayConfig,
        _window: &Arc<Window>,
        _renderer: &mut nptk::core::vgi::Renderer,
        _scene: &mut nptk::core::vgi::Scene,
        _surface: &mut Surface,
        _taffy: &mut TaffyTree,
        _window_node: NodeId,
        _info: &mut AppInfo,
        _gpu_context: &nptk::core::vgi::GpuContext,
        _update: &UpdateManager,
        _last_update: &mut Instant,
        _event_loop: &ActiveEventLoop,
    ) {
        if let WindowEvent::DroppedFile(path) = event {
            println!("Dropped file: {}", path.to_string_lossy());
        }
    }
}
