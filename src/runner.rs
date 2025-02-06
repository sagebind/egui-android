#![cfg(target_os = "android")]

use crate::{
    graphics::glutin::{GraphicsContext, GraphicsSurface},
    input::InputHandler,
    App,
};
use android_activity::{AndroidApp, MainEvent, PollEvent};
use egui::{FullOutput, Pos2, RawInput, Rect};
use egui_glow::Painter;
use ndk::native_window::NativeWindow;
use std::mem::take;

pub struct Runner<T: App> {
    app: T,
    android_app: AndroidApp,
    egui_context: egui::Context,
    raw_input: RawInput,
    graphics_context: GraphicsContext,
    window: Option<WindowSurfaceContext>,
    native_window: Option<NativeWindow>,
    focused: bool,
    wants_keyboard_input: bool,
    input_handler: InputHandler,
}

/// When a window exists, this holds the rendering context for that window.
struct WindowSurfaceContext {
    surface: GraphicsSurface,
    painter: Painter,
}

impl<T: App> Runner<T> {
    pub fn new(android_app: AndroidApp) -> Self {
        let graphics_context = GraphicsContext::new();

        let egui_context = egui::Context::default();

        // Configure repaint requests to trigger a wake up of the Android event
        // loop.
        let waker = android_app.create_waker();
        egui_context.set_request_repaint_callback(move |_info| {
            waker.wake();
        });

        Self {
            app: T::create(),
            android_app,
            egui_context,
            raw_input: RawInput::default(),
            graphics_context,
            native_window: None,
            window: None,
            focused: false,
            wants_keyboard_input: false,
            input_handler: InputHandler::new(),
        }
    }

    pub fn run_once(&mut self) {
        self.android_app.clone().poll_events(
            Some(std::time::Duration::from_secs(1)),
            move |event| {
                self.process_event(event);
            },
        );
    }

    /// Export the current egui memory to a byte array.
    fn save_memory(&self) -> Option<Vec<u8>> {
        self.egui_context.memory(bincode::serialize).ok()
    }

    /// Restore egui memory from a byte array.
    fn load_memory(&mut self, bytes: &[u8]) {
        match bincode::deserialize(bytes) {
            Ok(saved_memory) => {
                self.egui_context.memory_mut(|memory| {
                    *memory = saved_memory;
                });
            }
            Err(e) => {
                log::warn!("failed to deserialize memory: {e}");
            }
        }
    }

    fn initialize_surface(&mut self) {
        let Some(window) = self.android_app.native_window() else {
            return;
        };

        let mut surface = self.graphics_context.create_surface(&window);
        surface.make_current();

        self.window = Some(WindowSurfaceContext {
            painter: Painter::new(surface.glow_context.clone(), "", None).unwrap(),
            surface,
        });
    }

    fn destroy_surface(&mut self) {
        if let Some(mut window) = self.window.take() {
            window.painter.destroy();
            window.surface.make_not_current();
        }
    }

    /// Run one frame of egui.
    fn update_egui(&mut self) -> FullOutput {
        let mut raw_input = take(&mut self.raw_input);
        raw_input.focused = self.focused;

        let full_output = self.egui_context.run(raw_input, |_| {
            self.app.update(&self.egui_context);
        });

        // Check if egui wants to show or hide the keyboard, based on the last UI
        // update.
        match (
            self.wants_keyboard_input,
            self.egui_context.wants_keyboard_input(),
        ) {
            (false, true) => {
                self.android_app.show_soft_input(true);
                self.wants_keyboard_input = true;
            }
            (true, false) => {
                self.android_app.hide_soft_input(true);
                self.wants_keyboard_input = false;
            }
            _ => {}
        }

        full_output
    }

    fn process_event(&mut self, event: PollEvent) {
        let mut redraw = false;

        match event {
            PollEvent::Wake => {
                // info!("Early wake up");
            }
            PollEvent::Timeout => {
                // info!("Timed out");
                // Real app would probably rely on vblank sync via graphics API...
                redraw = true;
            }
            PollEvent::Main(main_event) => match main_event {
                MainEvent::Destroy => {
                    self.native_window = None;
                    self.destroy_surface();
                }

                MainEvent::InitWindow { .. } => {
                    log::info!("init window");
                    self.native_window = self.android_app.native_window();
                    self.initialize_surface();
                }

                MainEvent::TerminateWindow { .. } => {
                    log::info!("terminate window");
                    self.native_window = None;
                    self.destroy_surface();
                }

                MainEvent::WindowResized { .. } => {
                    self.destroy_surface();
                    self.native_window = self.android_app.native_window();
                    self.initialize_surface();
                }

                MainEvent::GainedFocus => self.focused = true,
                MainEvent::LostFocus => self.focused = false,

                // The app is going to be suspended.
                MainEvent::SaveState { saver, .. } => {
                    // To make the app easily resumable, we serialize the egui
                    // memory into the persistence region Android provides.
                    if let Some(memory) = self.save_memory() {
                        saver.store(&memory);
                    }
                }

                MainEvent::Resume { loader, .. } => {
                    // If Android remembers the data we saved previously,
                    // re-hydrate it.
                    if let Some(bytes) = loader.load() {
                        self.load_memory(&bytes);
                    }
                }

                MainEvent::RedrawNeeded { .. } => {
                    redraw = true;
                }

                MainEvent::LowMemory => self.app.on_low_memory(),

                MainEvent::ContentRectChanged { .. } => {
                    let content_rect = self.android_app.content_rect();
                    let egui_rect = Rect::from_two_pos(
                        Pos2::new(content_rect.left as _, content_rect.top as _),
                        Pos2::new(content_rect.right as _, content_rect.bottom as _),
                    );
                    self.app.on_content_rect_changed(egui_rect);
                }

                main_event => log::trace!("unknown main event: {main_event:?}"),
            },
            _ => {}
        }

        match self.android_app.input_events_iter() {
            Ok(mut iter) => loop {
                let read_input = iter.next(|event| {
                    self.input_handler.process(event, |event| {
                        log::debug!("sending event to egui: {:?}", event);

                        self.raw_input.events.push(event);

                        // Any sort of input should trigger a redraw.
                        // TODO: It would be smarter to ask egui if a redraw is needed as a result of input.
                        redraw = true;
                    })
                });

                if !read_input {
                    break;
                }
            },
            Err(err) => {
                log::error!("failed to get input events iterator: {err:?}");
            }
        }

        if redraw {
            let mut raw_input = take(&mut self.raw_input);
            raw_input.focused = self.focused;

            let full_output = self.update_egui();

            let clipped_primitives = self
                .egui_context
                .tessellate(full_output.shapes, full_output.pixels_per_point);

            if let Some(native_window) = self.native_window.as_mut() {
                if let Some(window) = self.window.as_mut() {
                    window.painter.paint_and_update_textures(
                        [native_window.width() as _, native_window.height() as _],
                        3.0,
                        &clipped_primitives,
                        &full_output.textures_delta,
                    );

                    window.surface.swap_buffers();
                }
            }
        }
    }
}
