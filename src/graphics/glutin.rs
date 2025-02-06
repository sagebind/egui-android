// https://github.com/emilk/egui/blob/master/crates/eframe/src/native/glow_integration.rs#L905

use egui_glow::glow;
use glutin::{
    api::egl::{
        context::{NotCurrentContext, PossiblyCurrentContext},
        display::Display,
        surface::Surface,
    },
    config::ConfigTemplate,
    context::{
        ContextApi, ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentGlContext,
    },
    display::GlDisplay,
    surface::{GlSurface, SurfaceAttributesBuilder, WindowSurface},
};
use ndk::native_window::NativeWindow;
use raw_window_handle::{AndroidDisplayHandle, RawDisplayHandle};
use std::{
    ffi::{CStr, CString},
    num::NonZeroU32,
    sync::Arc,
};

use super::window::as_raw_window_handle;

pub(crate) struct GraphicsContext {
    display: Display,
    config_template: ConfigTemplate,
}

impl GraphicsContext {
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

    pub(crate) fn create_surface(&mut self, window: &NativeWindow) -> GraphicsSurface {
        let raw_window_handle = as_raw_window_handle(window);

        let config_template = self.config_template.clone();
        let config = unsafe { self.display.find_configs(config_template) }
            .unwrap()
            .next()
            .unwrap();

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

        let egl_context =
            unsafe { self.display.create_context(&config, &context_attributes) }.unwrap();
        let egl_context = egl_context.make_current(&surface).unwrap();

        let glow_context = Arc::new(unsafe {
            glow::Context::from_loader_function(|s| {
                let s = CString::new(s).unwrap();
                self.display.get_proc_address(&s).cast()
            })
        });

        GraphicsSurface {
            display: self.display.clone(),
            surface,
            gl_context_current: Some(egl_context),
            gl_context_not_current: None,
            glow_context,
        }
    }
}

pub(crate) struct GraphicsSurface {
    display: Display,
    surface: Surface<WindowSurface>,
    gl_context_current: Option<PossiblyCurrentContext>,
    gl_context_not_current: Option<NotCurrentContext>,
    pub(crate) glow_context: Arc<glow::Context>,
}

impl GraphicsSurface {
    pub(crate) fn make_current(&mut self) {
        if let Some(context) = self.gl_context_not_current.take() {
            self.gl_context_current = Some(unsafe { context.make_current(&self.surface) }.unwrap());
        }
    }

    pub(crate) fn make_not_current(&mut self) {
        if let Some(context) = self.gl_context_current.take() {
            self.gl_context_not_current = Some(context.make_not_current().unwrap());
        }
    }

    pub(crate) fn swap_buffers(&mut self) {
        if let Some(context) = self.gl_context_current.as_mut() {
            self.surface.swap_buffers(context).unwrap();
        }
    }
}

impl Drop for GraphicsSurface {
    fn drop(&mut self) {
        self.make_not_current();
    }
}

// There is only one way to get the default display on Android, and it is stateless.
fn get_default_display() -> Display {
    let raw_display_handle = RawDisplayHandle::Android(AndroidDisplayHandle::new());

    unsafe { Display::new(raw_display_handle) }.unwrap()
}
