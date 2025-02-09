// https://github.com/emilk/egui/blob/master/crates/eframe/src/native/glow_integration.rs#L905

use egui_glow::{glow, Painter};
use glutin::{
    api::egl::{
        config::Config,
        context::{NotCurrentContext, PossiblyCurrentContext},
        display::Display,
    },
    context::{ContextApi, ContextAttributesBuilder, NotCurrentGlContext},
    display::GlDisplay,
    prelude::PossiblyCurrentGlContext,
    surface::{SurfaceAttributesBuilder, WindowSurface},
};
use ndk::native_window::NativeWindow;
use raw_window_handle::{
    AndroidDisplayHandle, AndroidNdkWindowHandle, RawDisplayHandle, RawWindowHandle,
};
use std::{ffi::CString, mem::replace, num::NonZeroU32, sync::Arc};

use super::canvas::Canvas;

/// Establishes a connection to Android's graphics API.
pub(crate) struct GraphicsContext {
    /// This is the primary EGL display connection.
    display: Display,

    /// This is the config that we use when creating surfaces.
    egl_config: Config,

    /// Sometimes we have a window surface, sometimes we don't. This holds
    /// objects specific to each of these possible states.
    state: GraphicsState,
}

#[derive(Default)]
enum GraphicsState {
    /// We have not initialized OpenGL yet. We only do so lazily, since we do
    /// need a window surface to do so.
    #[default]
    Uninitialized,

    Transitioning,

    Inactive {
        gl_context: NotCurrentContext,
        painter: Painter,
    },

    Active {
        canvas: Canvas,
    },
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

        let egl_config = unsafe { display.find_configs(config_template) }
            .unwrap()
            .next()
            .unwrap();

        GraphicsContext {
            display,
            egl_config,
            state: GraphicsState::default(),
        }
    }

    pub(crate) fn set_window(&mut self, native_window: NativeWindow) {
        log::info!("creating canvas");
        let raw_window_handle = as_raw_window_handle(&native_window);

        let surface_attributes = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            raw_window_handle,
            NonZeroU32::new(native_window.width().try_into().unwrap()).unwrap(),
            NonZeroU32::new(native_window.height().try_into().unwrap()).unwrap(),
        );

        let surface = unsafe {
            self.display
                .create_window_surface(&self.egl_config, &surface_attributes)
        }
        .unwrap();

        self.state = match replace(&mut self.state, GraphicsState::Transitioning) {
            // This is the first time a window has been created, initialize everything.
            GraphicsState::Uninitialized => {
                let context_attributes = ContextAttributesBuilder::new()
                    .with_context_api(ContextApi::Gles(None))
                    .build(None);

                let gl_context = unsafe {
                    self.display
                        .create_context(&self.egl_config, &context_attributes)
                }
                .unwrap();
                let gl_context = gl_context.make_current(&surface).unwrap();
                let glow_context = Arc::new(self.create_glow_context(&gl_context));

                GraphicsState::Active {
                    canvas: Canvas {
                        native_window,
                        surface,
                        gl_context: Some(gl_context),
                        painter: Painter::new(glow_context, "", None, false).unwrap(),
                    },
                }
            }

            GraphicsState::Inactive {
                gl_context,
                painter,
            } => {
                let gl_context = gl_context.make_current(&surface).unwrap();

                GraphicsState::Active {
                    canvas: Canvas {
                        native_window,
                        surface,
                        gl_context: Some(gl_context),
                        painter,
                    },
                }
            }

            _ => unreachable!(),
        };
    }

    pub(crate) fn remove_window(&mut self) {
        self.state = match replace(&mut self.state, GraphicsState::Transitioning) {
            GraphicsState::Active { canvas } => GraphicsState::Inactive {
                painter: canvas.painter,
                gl_context: canvas.gl_context.unwrap().make_not_current().unwrap(),
            },

            // We have no window to remove, do nothing.
            state => state,
        };
    }

    pub(crate) fn canvas_mut(&mut self) -> Option<&mut Canvas> {
        match &mut self.state {
            GraphicsState::Active { canvas } => Some(canvas),
            _ => None,
        }
    }

    fn create_glow_context(&self, _gl_context: &PossiblyCurrentContext) -> glow::Context {
        unsafe {
            glow::Context::from_loader_function(|s| {
                let s = CString::new(s).unwrap();
                self.display.get_proc_address(&s).cast()
            })
        }
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
