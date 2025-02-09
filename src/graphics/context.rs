// https://github.com/emilk/egui/blob/master/crates/eframe/src/native/glow_integration.rs#L905

use egui_glow::glow;
use glutin::{
    api::egl::display::Display,
    config::ConfigTemplate,
    context::{ContextApi, ContextAttributesBuilder, NotCurrentGlContext},
    display::GlDisplay,
    surface::{SurfaceAttributesBuilder, WindowSurface},
};
use ndk::native_window::NativeWindow;
use raw_window_handle::{
    AndroidDisplayHandle, AndroidNdkWindowHandle, RawDisplayHandle, RawWindowHandle,
};
use std::{ffi::CString, num::NonZeroU32};

use super::canvas::Canvas;

/// Establishes a connection to Android's graphics API.
pub(crate) struct GraphicsContext {
    display: Display,
    config_template: ConfigTemplate,
}

impl GraphicsContext {
    /// Create a new instance. Only one instance should be created per process.
    pub(crate) fn new() -> Self {
        let display = get_default_display();

        let config_template = glutin::config::ConfigTemplateBuilder::new()
            .with_depth_size(0)
            .with_stencil_size(0)
            .with_transparency(false)
            .build();

        GraphicsContext {
            display,
            config_template,
        }
    }

    /// Create a drawing surface for a window.
    pub(crate) fn create_surface(&mut self, window: NativeWindow) -> Canvas {
        let config_template = self.config_template.clone();
        let config = unsafe { self.display.find_configs(config_template) }
            .unwrap()
            .next()
            .unwrap();

        let raw_window_handle = as_raw_window_handle(&window);

        let surface_attributes = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            raw_window_handle,
            NonZeroU32::new(window.width().try_into().unwrap()).unwrap(),
            NonZeroU32::new(window.height().try_into().unwrap()).unwrap(),
        );

        let surface = unsafe {
            self.display
                .create_window_surface(&config, &surface_attributes)
        }
        .unwrap();

        let context_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::Gles(None))
            .build(Some(raw_window_handle));

        let gl_context =
            unsafe { self.display.create_context(&config, &context_attributes) }.unwrap();
        let gl_context = gl_context.make_current(&surface).unwrap();

        let glow_context = unsafe {
            glow::Context::from_loader_function(|s| {
                let s = CString::new(s).unwrap();
                self.display.get_proc_address(&s).cast()
            })
        };

        Canvas::new(window, surface, gl_context, glow_context)
    }
}

// There is only one way to get the default display on Android, and it is stateless.
fn get_default_display() -> Display {
    let raw_display_handle = RawDisplayHandle::Android(AndroidDisplayHandle::new());

    unsafe { Display::new(raw_display_handle) }.unwrap()
}

fn as_raw_window_handle(native_window: &NativeWindow) -> RawWindowHandle {
    RawWindowHandle::from(AndroidNdkWindowHandle::new(native_window.ptr().cast()))
}
