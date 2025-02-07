use egui::FullOutput;
use egui_glow::{glow, Painter};
use glutin::{
    api::egl::{
        context::{NotCurrentContext, PossiblyCurrentContext},
        surface::Surface,
    },
    context::{NotCurrentGlContext, PossiblyCurrentGlContext},
    surface::{GlSurface, WindowSurface},
};
use ndk::native_window::NativeWindow;
use std::sync::Arc;

/// When a window exists, this provides the ability to render to that window.
pub(crate) struct Canvas {
    native_window: NativeWindow,
    surface: Surface<WindowSurface>,
    gl_context_current: Option<PossiblyCurrentContext>,
    gl_context_not_current: Option<NotCurrentContext>,
    painter: Painter,
}

impl Canvas {
    pub(crate) fn new(
        native_window: NativeWindow,
        surface: Surface<WindowSurface>,
        gl_context: PossiblyCurrentContext,
        glow_context: glow::Context,
    ) -> Self {
        Canvas {
            native_window,
            surface,
            gl_context_current: Some(gl_context),
            gl_context_not_current: None,
            painter: Painter::new(Arc::new(glow_context), "", None, false).unwrap(),
        }
    }

    pub(crate) fn handle_resize(&mut self) {
        let [width, height] = self.window_size();

        if let Some(context) = self.gl_context_current.as_mut() {
            self.surface.resize(
                context,
                width.try_into().unwrap(),
                height.try_into().unwrap(),
            );
        }
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

        egui_glow::painter::clear(self.painter.gl(), screen_size, [0.0, 0.0, 0.0, 0.0]);

        self.painter.paint_and_update_textures(
            screen_size,
            full_output.pixels_per_point,
            clipped_primitives,
            &full_output.textures_delta,
        );

        self.swap_buffers();
    }

    pub(crate) fn window_size(&self) -> [u32; 2] {
        [
            self.native_window.width() as _,
            self.native_window.height() as _,
        ]
    }

    fn make_current(&mut self) {
        if let Some(context) = self.gl_context_not_current.take() {
            self.gl_context_current = Some(unsafe { context.make_current(&self.surface) }.unwrap());
        }
    }

    fn make_not_current(&mut self) {
        if let Some(context) = self.gl_context_current.take() {
            self.gl_context_not_current = Some(context.make_not_current().unwrap());
        }
    }

    fn swap_buffers(&mut self) {
        if let Some(context) = self.gl_context_current.as_mut() {
            self.surface.swap_buffers(context).unwrap();
        }
    }
}

impl Drop for Canvas {
    fn drop(&mut self) {
        self.painter.destroy();
        self.make_not_current();
    }
}
