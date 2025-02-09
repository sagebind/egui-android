use egui::FullOutput;
use egui_glow::Painter;
use glutin::{
    api::egl::{context::PossiblyCurrentContext, surface::Surface},
    surface::{GlSurface, WindowSurface},
};
use ndk::native_window::NativeWindow;

/// When a window exists, this provides the ability to render to that window.
pub(crate) struct Canvas {
    pub(crate) native_window: NativeWindow,
    pub(crate) surface: Surface<WindowSurface>,
    pub(crate) gl_context: Option<PossiblyCurrentContext>,
    pub(crate) painter: Painter,
}

impl Canvas {
    pub(crate) fn handle_resize(&mut self) {
        let [width, height] = self.window_size();

        self.surface.resize(
            self.gl_context.as_ref().unwrap(),
            width.try_into().unwrap(),
            height.try_into().unwrap(),
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

        self.swap_buffers();
    }

    pub(crate) fn window_size(&self) -> [u32; 2] {
        [
            self.native_window.width() as _,
            self.native_window.height() as _,
        ]
    }

    fn swap_buffers(&mut self) {
        self.surface
            .swap_buffers(self.gl_context.as_ref().unwrap())
            .unwrap();
    }

    pub(crate) fn into_context(mut self) -> Painter {
        self.painter
    }
}
