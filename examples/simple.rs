use anyhow::Result;
use tracing::{level_filters::LevelFilter, subscriber::set_global_default, Level};
use tracing_log::LogTracer;
use tracing_subscriber::FmtSubscriber;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Window, WindowId}
};

// The app.
#[derive(Default)]
struct App {
    window: Option<Window>
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop
            .create_window(Window::default_attributes())
            .unwrap();

        self.window = Some(window)
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
    // This routes log macros through tracing.
    LogTracer::init()?;

    // Setup the tracing subscriber globally.
    let subscriber = FmtSubscriber::builder()
        .with_max_level(LevelFilter::from_level(Level::DEBUG))
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
