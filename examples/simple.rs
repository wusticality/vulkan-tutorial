use std::{ffi::CStr, sync::Arc, time::Instant};

use anyhow::Result;
use tracing::{debug, error, level_filters::LevelFilter, subscriber::set_global_default, Level};
use tracing_log::LogTracer;
use tracing_subscriber::FmtSubscriber;
use vulkan::Context;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Window, WindowId}
};

/// The app.
struct App {
    /// Whether we are setup.
    initialized: bool,

    /// The window.
    window: Option<Arc<Window>>,

    /// The vulkan context.
    context: Option<Context>,

    /// The fps timer.
    fps_timer: Instant,

    /// The fps count.
    fps_count: u32
}

impl Default for App {
    fn default() -> Self {
        Self {
            initialized: false,
            window:      None,
            context:     None,
            fps_timer:   Instant::now(),
            fps_count:   0
        }
    }
}

impl App {
    /// Initialize the app. This creates the window and initializes Vulkan.
    fn initialize(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
        // If we're already initialized, return.
        if self.initialized {
            return Ok(());
        }

        // Create the window attributes.
        let attributes = Window::default_attributes()
            .with_resizable(false)
            .with_inner_size(PhysicalSize::new(2048, 1536));

        // Create the window.
        let window = event_loop.create_window(attributes)?;
        let window = Arc::new(window);

        // The application name.
        let name = CStr::from_bytes_with_nul(b"vulkan-tutorial\0")?;

        // Create the vulkan context.
        let context = unsafe { Context::new(window.clone(), &name)? };

        self.initialized = true;
        self.window = Some(window);
        self.context = Some(context);
        self.fps_timer = Instant::now();

        Ok(())
    }
}

impl ApplicationHandler for App {
    fn new_events(&mut self, _event_loop: &ActiveEventLoop, _cause: StartCause) {
        // Print the fps every second.
        if self
            .fps_timer
            .elapsed()
            .as_secs_f32()
            >= 1.0
        {
            debug!("fps: {}", self.fps_count);

            // Reset the timer / counter.
            self.fps_timer = Instant::now();
            self.fps_count = 0;
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // TODO: Teardown the vulkan context in suspended
        //  and recreate it here or you'll run into issues
        //  on mobile devices.

        // Setup the app.
        if let Err(e) = self.initialize(event_loop) {
            error!("{}", e);

            event_loop.exit();
        }

        // Request the first redraw.
        if let Some(window) = &mut self.window {
            window.request_redraw();
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
                // Render the frame.
                if let Some(context) = &mut self.context {
                    if let Err(e) = unsafe { context.draw() } {
                        error!("{}", e);

                        event_loop.exit();
                    }
                }

                // Increment the fps count.
                self.fps_count += 1;

                // Request a redraw.
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
