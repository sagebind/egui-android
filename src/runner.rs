#![cfg(target_os = "android")]

use crate::{
    graphics::{canvas::Canvas, context::GraphicsContext},
    ime::show_hide_keyboard,
    input::InputHandler,
    state::AppState,
    App,
};
use android_activity::{AndroidApp, MainEvent, PollEvent};
use egui::{pos2, vec2, Event, FullOutput, ImeEvent, Margin, Pos2, RawInput, Rect, Theme};
use ndk::configuration::UiModeNight;
use std::{
    mem::take,
    sync::{Arc, Mutex},
    time::Instant,
};

pub(crate) struct Runner<T: App> {
    app_state: AppState<T>,
    android_app: AndroidApp,
    graphics_context: GraphicsContext,
    raw_input: RawInput,
    input_handler: InputHandler,
    repaint_info: Arc<Mutex<RepaintInfo>>,
    keyboard_visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ControlFlow {
    Continue,
    Quit,
}

struct RepaintInfo {
    needs_repaint: bool,
    deadline: Instant,
}

impl<T: App> Runner<T> {
    pub fn new(android_app: AndroidApp) -> Self {
        let app_state = AppState::new(T::create());

        let repaint_info = Arc::new(Mutex::new(RepaintInfo {
            needs_repaint: false,
            deadline: Instant::now(),
        }));

        // Configure repaint requests to trigger a wake up of the Android event
        // loop.
        app_state.context().set_request_repaint_callback({
            let waker = android_app.create_waker();
            let repaint_info = repaint_info.clone();
            move |info| {
                let mut repaint_info = repaint_info.lock().unwrap();

                repaint_info.needs_repaint = true;
                repaint_info.deadline = Instant::now() + info.delay;

                waker.wake();
            }
        });

        Self {
            app_state,
            android_app: android_app.clone(),
            graphics_context: GraphicsContext::new(),
            raw_input: RawInput::default(),
            input_handler: InputHandler::new(android_app),
            repaint_info,
            keyboard_visible: false,
        }
    }

    pub(crate) fn run_once(&mut self) -> ControlFlow {
        let mut control_flow = ControlFlow::Continue;
        let mut timeout = self.app_state.inner().min_update_frequency();

        let repaint_info = self.repaint_info.lock().unwrap();
        if repaint_info.needs_repaint {
            let duration = repaint_info
                .deadline
                .saturating_duration_since(Instant::now());

            timeout = timeout.map(|d| d.min(duration)).or(Some(duration));
        }

        drop(repaint_info);

        self.android_app.clone().poll_events(timeout, move |event| {
            self.process_event(event, &mut control_flow);

            // Event handled, now check if we need to repaint.
            self.repaint_if_needed();
        });

        control_flow
    }

    fn request_repaint(&self) {
        self.app_state.context().request_repaint();
    }

    fn initialize_canvas_if_needed(&mut self) {
        if let Some(native_window) = self.android_app.native_window() {
            self.graphics_context.set_window(native_window);
        };
    }

    fn destroy_canvas(&mut self) {
        self.graphics_context.remove_window();
    }

    fn process_event(&mut self, event: PollEvent, control_flow: &mut ControlFlow) {
        match event {
            PollEvent::Wake => {}
            PollEvent::Timeout => {
                // info!("Timed out");
                // Real app would probably rely on vblank sync via graphics API...
                self.request_repaint();
            }
            PollEvent::Main(main_event) => match main_event {
                MainEvent::Destroy => {
                    self.destroy_canvas();
                    *control_flow = ControlFlow::Quit;
                }

                MainEvent::InitWindow { .. } => {
                    // TODO: Need to reset textures
                    // self.app_state = AppState::new(T::create());
                    self.apply_current_config();
                    self.initialize_canvas_if_needed();
                    self.request_repaint();
                }

                MainEvent::TerminateWindow { .. } | MainEvent::Stop => {
                    self.destroy_canvas();
                }

                MainEvent::WindowResized { .. } => {
                    self.apply_current_config();
                    if let Some(canvas) = self.graphics_context.canvas_mut() {
                        canvas.handle_resize();
                    }
                    self.request_repaint();
                }

                MainEvent::GainedFocus => {
                    self.update_focus(true);
                    self.request_repaint();
                }

                MainEvent::LostFocus => {
                    self.update_focus(false);
                    self.request_repaint();
                }

                MainEvent::RedrawNeeded { .. } => self.request_repaint(),

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
                    self.apply_current_config();
                }

                MainEvent::ConfigChanged { .. } => {
                    self.apply_current_config();
                    self.request_repaint();
                }

                MainEvent::InputAvailable => {
                    self.process_pending_input();
                    self.request_repaint();
                }

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

                main_event => log::trace!("unknown main event: {main_event:?}"),
            },
            _ => {}
        }
    }

    fn process_pending_input(&mut self) {
        let pixels_per_point = self.pixels_per_point();

        match self.android_app.input_events_iter() {
            Ok(mut iter) => loop {
                let read_input = iter.next(|event| {
                    self.input_handler
                        .process(event, pixels_per_point, |event| {
                            self.raw_input.events.push(event);
                            self.app_state.update_clock();
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

    fn repaint_if_needed(&mut self) {
        self.app_state.update_clock();

        let mut repaint_info = self.repaint_info.lock().unwrap();

        if repaint_info.needs_repaint && self.app_state.now() >= repaint_info.deadline {
            repaint_info.needs_repaint = false;
            drop(repaint_info);
            self.repaint();
        }
    }

    /// Do a full app update. Input events will be passed into egui, the user's
    /// update routine will be called, and the UI will be redrawn.
    fn repaint(&mut self) {
        if let Some(canvas) = self.graphics_context.canvas_mut() {
            let mut full_output = self.app_state.update(self.raw_input.take());

            if full_output.platform_output.requested_discard() {
                self.request_repaint();
            } else {
                let clipped_primitives = self
                    .app_state
                    .context()
                    .tessellate(take(&mut full_output.shapes), full_output.pixels_per_point);

                canvas.repaint(&mut full_output, &clipped_primitives);
            }

            self.handle_output_events(&full_output);
        }
    }

    fn handle_output_events(&mut self, full_output: &FullOutput) {
        // Check if egui wants to show or hide the keyboard, based on the
        // last UI update.

        match (
            full_output.platform_output.ime.is_some(),
            self.keyboard_visible,
        ) {
            (true, false) => {
                log::info!("show keyboard requested");
                // crate::ime::show_hide_keyboard_alt(&self.android_app, true, true);
                show_hide_keyboard(&self.android_app, true);
                self.keyboard_visible = true;
                // self.raw_input.events.push(Event::Ime(ImeEvent::Enabled));
                self.request_repaint();
            }
            (false, true) => {
                log::info!("hide keyboard requested");
                // show_hide_keyboard(&self.android_app, false);
                self.android_app.hide_soft_input(false);
                self.keyboard_visible = false;
                // self.raw_input.events.push(Event::Ime(ImeEvent::Disabled));
                self.request_repaint();
            }
            _ => {}
        }
    }

    fn pixels_per_point(&self) -> f32 {
        self.raw_input
            .viewport()
            .native_pixels_per_point
            .unwrap_or(1.0)
    }

    fn update_focus(&mut self, focused: bool) {
        self.raw_input.focused = focused;
        self.raw_input.events.push(Event::WindowFocused(focused));

        let viewport_info = self
            .raw_input
            .viewports
            .get_mut(&self.raw_input.viewport_id)
            .unwrap();
        viewport_info.focused = Some(focused);
    }

    fn apply_current_config(&mut self) {
        let config = self.android_app.config();

        self.raw_input.system_theme = match config.ui_mode_night() {
            UiModeNight::No => Some(Theme::Light),
            UiModeNight::Yes => Some(Theme::Dark),
            _ => None,
        };

        let viewport_info = self
            .raw_input
            .viewports
            .get_mut(&self.raw_input.viewport_id)
            .unwrap();

        // Calculate pixels per point based on screen density.
        viewport_info.native_pixels_per_point = config
            .density()
            .map(|density| density as f32 / 160.0)
            .map(|ppp| ppp.round());

        let pixels_per_point = viewport_info.native_pixels_per_point.unwrap_or(1.0);

        if let Some(canvas) = self.graphics_context.canvas_mut() {
            let [width, height] = canvas.window_size();
            let width = width as f32 / pixels_per_point;
            let height = height as f32 / pixels_per_point;

            // let android_activity::Rect {
            //     left,
            //     top,
            //     right,
            //     bottom,
            // } = self.android_app.content_rect();

            // let top_left = pos2(left as f32, top as f32) / pixels_per_point;
            // let bottom_right = pos2(right as f32, bottom as f32) / pixels_per_point;

            // self.screen_rect = Some(Rect::from_two_pos(top_left, bottom_right));
            self.raw_input.screen_rect = Some(Rect::from_min_size(
                Pos2::new(0.0, 0.0),
                vec2(width, height),
            ));
        }

        self.app_state
            .context()
            .style_mut(|style| style.spacing.window_margin = self.window_margin(pixels_per_point));
    }

    fn window_margin(&self, pixels_per_point: f32) -> Margin {
        let content_rect = self.android_app.content_rect();

        Margin {
            left: (content_rect.left as f32 / pixels_per_point) as i8,
            right: 0,  // (content_rect.right as f32 / pixels_per_point) as i8,
            top: 127,  // + (content_rect.top as f32 / pixels_per_point) as i8,
            bottom: 0, //(content_rect.bottom as f32 / pixels_per_point) as i8,
        }
    }
}
