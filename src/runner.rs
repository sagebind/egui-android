#![cfg(target_os = "android")]

use crate::{
    graphics::glutin::{GraphicsContext, GraphicsSurface},
    input::InputHandler,
    state::AppState,
    App,
};
use android_activity::{AndroidApp, MainEvent, PollEvent};
use egui::{pos2, vec2, FullOutput, Margin, Pos2, RawInput, Rect, Theme};
use egui_glow::Painter;
use ndk::{configuration::UiModeNight, native_window::NativeWindow};
use std::mem::take;

pub(crate) struct Runner<T: App> {
    app_state: AppState<T>,
    android_app: AndroidApp,
    raw_input: RawInput,
    graphics_context: GraphicsContext,
    window: Option<WindowSurfaceContext>,
    focused: bool,
    wants_keyboard_input: bool,
    input_handler: InputHandler,
    screen_rect: Option<Rect>,
}

/// When a window exists, this holds the rendering context for that window.
struct WindowSurfaceContext {
    surface: GraphicsSurface,
    painter: Painter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ControlFlow {
    Continue,
    Quit,
}

impl<T: App> Runner<T> {
    pub fn new(android_app: AndroidApp) -> Self {
        let app_state = AppState::new(T::create());

        let graphics_context = GraphicsContext::new();

        // Configure repaint requests to trigger a wake up of the Android event
        // loop.
        let waker = android_app.create_waker();
        app_state
            .context()
            .set_request_repaint_callback(move |_info| waker.wake());

        Self {
            app_state,
            android_app,
            raw_input: RawInput::default(),
            graphics_context,
            window: None,
            focused: true,
            wants_keyboard_input: false,
            input_handler: InputHandler::new(),
            screen_rect: None,
        }
    }

    pub(crate) fn run_once(&mut self) -> ControlFlow {
        let mut control_flow = ControlFlow::Continue;

        self.android_app.clone().poll_events(
            self.app_state.inner().min_update_frequency(),
            move |event| {
                self.process_event(event, &mut control_flow);
            },
        );

        control_flow
    }

    fn initialize_surface(&mut self) {
        let Some(window) = self.android_app.native_window() else {
            return;
        };

        let mut surface = self.graphics_context.create_surface(&window);
        surface.make_current();

        self.window = Some(WindowSurfaceContext {
            painter: Painter::new(surface.glow_context.clone(), "", None, false).unwrap(),
            surface,
        });
    }

    fn destroy_surface(&mut self) {
        if let Some(mut window) = self.window.take() {
            window.painter.destroy();
            window.surface.make_not_current();
        }
    }

    fn process_event(&mut self, event: PollEvent, control_flow: &mut ControlFlow) {
        let mut redraw = false;

        match event {
            PollEvent::Wake => {
                // info!("Early wake up");
                redraw = true;
            }

            PollEvent::Timeout => {
                // info!("Timed out");
                // Real app would probably rely on vblank sync via graphics API...
                redraw = true;
            }

            PollEvent::Main(main_event) => match main_event {
                MainEvent::Destroy => {
                    self.destroy_surface();
                    *control_flow = ControlFlow::Quit;
                }

                MainEvent::InitWindow { .. } => {
                    log::info!("init window");
                    self.initialize_surface();
                    self.update_window_margin();
                }

                MainEvent::TerminateWindow { .. } => {
                    log::info!("terminate window");
                    self.destroy_surface();
                }

                MainEvent::WindowResized { .. } => {
                    // TODO: Dedup `InitWindow` immediately followed by `WindowResized`.
                    // self.destroy_surface();
                    // self.native_window = self.android_app.native_window();
                    // self.initialize_surface();
                }

                MainEvent::GainedFocus => self.focused = true,
                MainEvent::LostFocus => self.focused = false,

                // The app is going to be suspended.
                MainEvent::SaveState { saver, .. } => {
                    // To make the app easily resumable, we serialize the egui
                    // memory into the persistence region Android provides.
                    if let Some(memory) = self.app_state.save_memory() {
                        saver.store(&memory);
                    }
                }

                MainEvent::Resume { loader, .. } => {
                    // If Android remembers the data we saved previously,
                    // re-hydrate it.
                    if let Some(bytes) = loader.load() {
                        self.app_state.load_memory(&bytes);
                    }
                }

                MainEvent::RedrawNeeded { .. } => redraw = true,

                MainEvent::LowMemory => self.app_state.inner_mut().on_low_memory(),

                MainEvent::ContentRectChanged { .. } => {
                    let content_rect = self.android_app.content_rect();
                    let egui_rect = Rect::from_two_pos(
                        pos2(content_rect.left as _, content_rect.top as _),
                        pos2(content_rect.right as _, content_rect.bottom as _),
                    );
                    self.app_state
                        .inner_mut()
                        .on_content_rect_changed(egui_rect);
                    self.update_window_margin();
                }

                MainEvent::InputAvailable => {
                    self.process_pending_input();
                    // Any sort of input should trigger a redraw.
                    // TODO: It would be smarter to ask egui if a redraw is needed as a result of input.
                    redraw = true;
                }

                main_event => log::trace!("unknown main event: {main_event:?}"),
            },
            _ => {}
        }

        self.process_pending_input();

        if redraw {
            self.update();
        }
    }

    fn process_pending_input(&mut self) {
        let pixels_per_point = self.calculate_pixels_per_point().unwrap_or(1.0);

        match self.android_app.input_events_iter() {
            Ok(mut iter) => loop {
                let read_input = iter.next(|event| {
                    self.input_handler
                        .process(event, pixels_per_point, |event| {
                            log::debug!("sending event to egui: {:?}", event);
                            self.raw_input.events.push(event);
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
    }

    /// Do a full app update. Input events will be passed into egui, the user's
    /// update routine will be called, and the UI will be redrawn.
    fn update(&mut self) {
        if self.window.is_none() {
            return;
        }

        let pixels_per_point = self.calculate_pixels_per_point();

        if let Some(native_window) = self.android_app.native_window() {
            self.update_screen_rect(&native_window);

            let full_output = self.update_egui();

            let clipped_primitives = self
                .app_state
                .context()
                .tessellate(full_output.shapes, full_output.pixels_per_point);

            let window = self.window.as_mut().unwrap();

            let screen_size = [native_window.width() as _, native_window.height() as _];

            egui_glow::painter::clear(
                &window.surface.glow_context,
                screen_size,
                [0.0, 0.0, 0.0, 0.0],
            );

            window.painter.paint_and_update_textures(
                screen_size,
                pixels_per_point.unwrap_or(1.0),
                &clipped_primitives,
                &full_output.textures_delta,
            );

            window.surface.swap_buffers();
        }
    }

    /// Run one frame of egui.
    fn update_egui(&mut self) -> FullOutput {
        // Prepare the raw input for egui. This is when we have our best
        // opportunity to provide egui with as much useful context as possible.
        let mut raw_input = take(&mut self.raw_input);

        raw_input.focused = self.focused;
        raw_input.screen_rect = self.screen_rect;
        raw_input.system_theme = match self.android_app.config().ui_mode_night() {
            UiModeNight::No => Some(Theme::Light),
            UiModeNight::Yes => Some(Theme::Dark),
            _ => None,
        };

        let viewport_info = raw_input.viewports.get_mut(&raw_input.viewport_id).unwrap();
        viewport_info.focused = Some(self.focused);
        viewport_info.native_pixels_per_point = self.calculate_pixels_per_point();
        log::debug!("pixels per point: {:?}", self.calculate_pixels_per_point());

        let full_output = self.app_state.update(raw_input);

        // Check if egui wants to show or hide the keyboard, based on the last UI
        // update.
        match (
            self.wants_keyboard_input,
            self.app_state.context().wants_keyboard_input(),
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

    fn calculate_pixels_per_point(&self) -> Option<f32> {
        self.android_app
            .config()
            .density()
            .map(|density| density as f32 / 160.0)
            .map(|ppp| ppp.round())
    }

    fn update_screen_rect(&mut self, native_window: &NativeWindow) {
        let pixels_per_point = self.calculate_pixels_per_point().unwrap_or(1.0);
        let width = native_window.width() as f32 / pixels_per_point;
        let height = native_window.height() as f32 / pixels_per_point;

        // let android_activity::Rect {
        //     left,
        //     top,
        //     right,
        //     bottom,
        // } = self.android_app.content_rect();

        // let top_left = pos2(left as f32, top as f32) / pixels_per_point;
        // let bottom_right = pos2(right as f32, bottom as f32) / pixels_per_point;

        // self.screen_rect = Some(Rect::from_two_pos(top_left, bottom_right));
        self.screen_rect = Some(Rect::from_min_size(
            Pos2::new(0.0, 0.0),
            vec2(width, height),
        ));
    }

    fn window_margin(&self) -> Margin {
        let pixels_per_point = self.calculate_pixels_per_point().unwrap_or(1.0);
        let content_rect = self.android_app.content_rect();

        Margin {
            left: (content_rect.left as f32 / pixels_per_point) as i8,
            right: 0, // (content_rect.right as f32 / pixels_per_point) as i8,
            top: 127,// + (content_rect.top as f32 / pixels_per_point) as i8,
            bottom: 0, //(content_rect.bottom as f32 / pixels_per_point) as i8,
        }
    }

    fn update_window_margin(&mut self) {
        self.app_state
            .context()
            .style_mut(|style| style.spacing.window_margin = self.window_margin());
    }
}
