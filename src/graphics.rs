//! Internal graphics module.

// https://github.com/emilk/egui/blob/master/crates/eframe/src/native/glow_integration.rs#L905

use egui::FullOutput;
use egui_glow::{glow, Painter};
use glutin::{
    api::egl::{
        config::Config,
        context::{NotCurrentContext, PossiblyCurrentContext},
        display::Display,
        surface::Surface,
    },
    config::{ConfigTemplate, ConfigTemplateBuilder},
    context::{ContextApi, ContextAttributesBuilder, NotCurrentGlContext},
    display::GlDisplay,
    prelude::PossiblyCurrentGlContext,
    surface::{GlSurface, SurfaceAttributesBuilder, WindowSurface},
};
use ndk::native_window::NativeWindow;
use raw_window_handle::{
    AndroidDisplayHandle, AndroidNdkWindowHandle, RawDisplayHandle, RawWindowHandle,
};
use std::{ffi::CString, mem::replace, num::NonZeroU32, sync::Arc};

/// Establishes a connection to Android's graphics API.
pub(crate) struct GraphicsContext {
    /// This is the primary EGL display connection.
    display: Display,

    /// This is the config that we use when creating surfaces.
    egl_config: Config,

    /// Sometimes we have a window surface, sometimes we don't. This holds
    /// objects specific to each of these possible states.
    state: State,
}

#[derive(Default)]
enum State {
    /// We have not initialized OpenGL yet. We only do so lazily, since we do
    /// need a window surface to do so.
    #[default]
    Uninitialized,

    /// A window surface has been associated with the context and is currently
    /// active.
    Active {
        native_window: NativeWindow,
        surface: Surface<WindowSurface>,
        gl_context: PossiblyCurrentContext,
        painter: Painter,
    },

    /// Android took away our window, but we still need to maintain our graphics
    /// handles until the window comes back.
    Suspended {
        gl_context: NotCurrentContext,
        painter: Painter,
    },
}

impl GraphicsContext {
    /// Create a new instance. Only one instance should be created per process.
    pub(crate) fn new() -> Self {
        Self::with_config(
            ConfigTemplateBuilder::new()
                .with_depth_size(0)
                .with_stencil_size(0)
                .with_transparency(false)
                .build(),
        )
    }

    /// Create a new instance. Only one instance should be created per process.
    pub(crate) fn with_config(config_template: ConfigTemplate) -> Self {
        let display = get_default_display();

        let egl_config = unsafe { display.find_configs(config_template) }
            .unwrap()
            .next()
            .unwrap();

        GraphicsContext {
            display,
            egl_config,
            state: State::default(),
        }
    }

    /// Get a renderer for the current window. If a window is not currently
    /// associated with the context, then `None` is returned.
    pub(crate) fn renderer(&mut self) -> Option<Renderer<'_>> {
        match &mut self.state {
            State::Active {
                native_window,
                surface,
                gl_context,
                painter,
                ..
            } => Some(Renderer {
                native_window,
                surface,
                gl_context,
                painter,
            }),
            _ => None,
        }
    }

    /// Attach a window to the context. A graphics surface will be initialized
    /// within the window, and a renderer will become available for drawing to
    /// the surface.
    pub(crate) fn attach_window(&mut self, native_window: NativeWindow) {
        self.state = match self.state.take() {
            // This is the first time a window has been created, initialize everything.
            State::Uninitialized => {
                let context_attributes = ContextAttributesBuilder::new()
                    .with_context_api(ContextApi::Gles(None))
                    .build(None);

                let gl_context = unsafe {
                    self.display
                        .create_context(&self.egl_config, &context_attributes)
                }
                .unwrap();

                let surface = self.create_window_surface(&native_window);

                let gl_context = gl_context.make_current(&surface).unwrap();
                let glow_context = Arc::new(self.create_glow_context(&gl_context));

                State::Active {
                    native_window,
                    surface,
                    gl_context,
                    painter: Painter::new(glow_context, "", None, false).unwrap(),
                }
            }

            State::Suspended {
                gl_context,
                painter,
            } => {
                let surface = self.create_window_surface(&native_window);
                let gl_context = gl_context.make_current(&surface).unwrap();

                State::Active {
                    native_window,
                    surface,
                    gl_context,
                    painter,
                }
            }

            State::Active { .. } => todo!(),
        };
    }

    pub(crate) fn detach_window(&mut self) {
        self.state = match self.state.take() {
            State::Active {
                gl_context,
                painter,
                ..
            } => State::Suspended {
                painter,
                gl_context: gl_context.make_not_current().unwrap(),
            },

            // We have no window to remove, do nothing.
            state => state,
        };
    }

    fn create_glow_context(&self, _gl_context: &PossiblyCurrentContext) -> glow::Context {
        unsafe {
            glow::Context::from_loader_function(|s| {
                let s = CString::new(s).unwrap();
                self.display.get_proc_address(&s).cast()
            })
        }
    }

    fn create_window_surface(&self, native_window: &NativeWindow) -> Surface<WindowSurface> {
        let raw_window_handle =
            RawWindowHandle::from(AndroidNdkWindowHandle::new(native_window.ptr().cast()));

        let surface_attributes = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            raw_window_handle,
            NonZeroU32::new(native_window.width().try_into().unwrap()).unwrap(),
            NonZeroU32::new(native_window.height().try_into().unwrap()).unwrap(),
        );

        unsafe {
            self.display
                .create_window_surface(&self.egl_config, &surface_attributes)
        }
        .unwrap()
    }
}

impl State {
    fn take(&mut self) -> Self {
        replace(self, Self::Uninitialized)
    }
}

impl Drop for GraphicsContext {
    fn drop(&mut self) {
        match self.state.take() {
            State::Active {
                gl_context,
                mut painter,
                ..
            } => {
                gl_context.make_not_current().unwrap();
                painter.destroy();
            }

            State::Suspended { mut painter, .. } => {
                painter.destroy();
            }

            State::Uninitialized => {}
        }
    }
}

pub(crate) struct Renderer<'c> {
    native_window: &'c NativeWindow,
    surface: &'c Surface<WindowSurface>,
    gl_context: &'c PossiblyCurrentContext,
    painter: &'c mut Painter,
}

impl<'c> Renderer<'c> {
    pub(crate) fn handle_resize(&self) {
        self.surface.resize(
            self.gl_context,
            NonZeroU32::new(self.native_window.width() as _).unwrap(),
            NonZeroU32::new(self.native_window.height() as _).unwrap(),
        );
    }

    pub(crate) fn repaint(
        &mut self,
        full_output: &mut FullOutput,
        clipped_primitives: &[egui::ClippedPrimitive],
    ) {
        let screen_size = [
            self.surface.width().unwrap(),
            self.surface.height().unwrap(),
        ];

        self.painter.clear(screen_size, [0.0, 0.0, 0.0, 0.0]);

        self.painter.paint_and_update_textures(
            screen_size,
            full_output.pixels_per_point,
            clipped_primitives,
            &full_output.textures_delta,
        );

        self.surface.swap_buffers(self.gl_context).unwrap();
    }

    pub(crate) fn window_size(&self) -> [u32; 2] {
        [
            self.native_window.width() as _,
            self.native_window.height() as _,
        ]
    }
}

// There is only one way to get the default display on Android, and it is stateless.
fn get_default_display() -> Display {
    let raw_display_handle = RawDisplayHandle::Android(AndroidDisplayHandle::new());
    unsafe { Display::new(raw_display_handle) }.unwrap()
}
