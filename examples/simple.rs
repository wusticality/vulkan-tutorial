use std::{ffi::CStr, sync::Arc};

use anyhow::Result;
use tracing::{error, level_filters::LevelFilter, subscriber::set_global_default, Level};
use tracing_log::LogTracer;
use tracing_subscriber::FmtSubscriber;
use vulkan::Context;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Window, WindowId}
};

/// The app.
#[derive(Default)]
struct App {
    /// The window.
    window: Option<Arc<Window>>,

    /// The vulkan context.
    context: Option<Context>
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Create the window.
        let attributes = Window::default_attributes()
            .with_resizable(false)
            .with_inner_size(PhysicalSize::new(2048, 1536));
        let window = event_loop
            .create_window(attributes)
            .unwrap();
        let window = Arc::new(window);

        self.window = Some(window.clone());

        unsafe {
            // The application name.
            let name = CStr::from_bytes_with_nul_unchecked(b"vulkan-tutorial\0");

            // Create the vulkan context.
            self.context = Some(Context::new(window, &name).unwrap());
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            },

            WindowEvent::RedrawRequested => {
                if let Some(window) = &mut self.window {
                    window.request_redraw();
                }
            },

            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _
            } => match event.logical_key {
                Key::Named(key) if key == NamedKey::Escape => {
                    event_loop.exit();
                },

                _ => {}
            },

            _ => {}
        }
    }
}

fn main() -> Result<()> {
    // Catch panics and emit them as errors.
    std::panic::set_hook(Box::new(|panic_info| {
        error!("{}", panic_info);
    }));

    // This routes log macros through tracing.
    LogTracer::init()?;

    // Setup the tracing subscriber globally.
    let subscriber = FmtSubscriber::builder()
        .with_max_level(LevelFilter::from_level(Level::INFO))
        .finish();

    set_global_default(subscriber)?;

    // Create the event loop.
    let event_loop = EventLoop::new()?;

    // Poll continuously.
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default();

    // Run the app.
    event_loop.run_app(&mut app)?;

    Ok(())
}
