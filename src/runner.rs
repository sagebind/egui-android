#![cfg(target_os = "android")]

use crate::{window::AndroidSurfaceTarget, App};
use android_activity::{
    input::{InputEvent, KeyEvent},
    AndroidApp, MainEvent, PollEvent,
};
use egui::{Event, FullOutput, Modifiers, Pos2, RawInput, Rect};
use egui_wgpu::Renderer;
use pollster::block_on;
use std::mem::take;

pub struct Runner<T: App> {
    app: T,
    android_app: AndroidApp,
    egui_context: egui::Context,
    raw_input: RawInput,
    renderer: Option<Renderer>,
    wgpu_instance: wgpu::Instance,
    focused: bool,
    wants_keyboard_input: bool,
}

impl<T: App> Runner<T> {
    pub fn new(android_app: AndroidApp) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            ..Default::default()
        });

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
            renderer: None,
            wgpu_instance: instance,
            focused: false,
            wants_keyboard_input: false,
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

        let surface_target = AndroidSurfaceTarget::new(window);
        let surface = self.wgpu_instance.create_surface(surface_target).unwrap();

        let adapter = block_on(
            self.wgpu_instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: Some(&surface),
                    force_fallback_adapter: false,
                }),
        )
        .unwrap();

        let (device, _queue) = block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::POLYGON_MODE_LINE,
                required_limits: wgpu::Limits {
                    // max_buffer_size: u64::MAX,
                    ..Default::default()
                },
                label: None,
            },
            None,
        ))
        .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = *surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .unwrap_or(&surface_caps.formats[0]);

        let renderer = egui_wgpu::Renderer::new(&device, surface_format, None, 1);

        self.renderer = Some(renderer);
    }

    fn destroy_surface(&mut self) {
        self.renderer = None;
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
                MainEvent::Destroy => {}

                MainEvent::InitWindow { .. } => self.initialize_surface(),
                MainEvent::TerminateWindow { .. } => self.destroy_surface(),

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
                let read_input = iter.next(|event| match event {
                    InputEvent::KeyEvent(key_event) => {
                        if let Some(event) = to_egui_key_event(key_event) {
                            self.raw_input.events.push(event);
                        }
                        android_activity::InputStatus::Handled
                    }
                    InputEvent::MotionEvent(_motion_event) => {
                        android_activity::InputStatus::Handled
                    }
                    _event => android_activity::InputStatus::Handled,
                });

                if !read_input {
                    break;
                }
            },
            Err(_err) => {
                // log::error!("Failed to get input events iterator: {err:?}");
            }
        }

        if redraw {
            let mut raw_input = take(&mut self.raw_input);
            raw_input.focused = self.focused;

            let full_output = self.update_egui();

            let clipped_primitives = self
                .egui_context
                .tessellate(full_output.shapes, full_output.pixels_per_point);

            if let Some(renderer) = self.renderer.as_mut() {
                renderer.render(todo!(), &clipped_primitives, todo!());
            }
        }
    }
}

fn to_egui_key_event(key_event: &KeyEvent) -> Option<Event> {
    let physical_key = crate::keycodes::to_physical_key(key_event.key_code());

    Some(Event::Key {
        key: physical_key.unwrap(),
        physical_key,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::default(),
    })
}
