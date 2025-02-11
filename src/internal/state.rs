use crate::App;
use egui::{Context, FullOutput, RawInput};
use std::time::Instant;

/// Wrap an `App` and manages its egui state and execution lifecycle.
///
/// This does not handle any platform APIS, just compartmentalizes purely
/// functional egui state management.
pub(crate) struct AppState<T> {
    app: T,
    context: Context,
    time_started: Instant,
    now: Instant,
}

impl<T: App> AppState<T> {
    /// Create a new `AppState` with the given `App`.
    pub(crate) fn new(app: T) -> Self {
        let now = Instant::now();

        Self {
            app,
            context: Context::default(),
            time_started: now,
            now,
        }
    }

    pub(crate) fn inner(&self) -> &T {
        &self.app
    }

    pub(crate) fn inner_mut(&mut self) -> &mut T {
        &mut self.app
    }

    pub(crate) fn context(&self) -> &Context {
        &self.context
    }

    pub(crate) fn now(&self) -> Instant {
        self.now
    }

    /// Run the app's update logic.
    pub(crate) fn update(&mut self, mut raw_input: RawInput) -> FullOutput {
        raw_input.time = Some(self.now.duration_since(self.time_started).as_secs_f64());

        self.context.run(raw_input, |context| {
            self.app.update(context);
        })
    }

    /// Export the current egui memory to a byte array.
    pub(crate) fn save_memory(&self) -> Option<Vec<u8>> {
        self.context.memory(bincode::serialize).ok()
    }

    /// Restore egui memory from a byte array.
    pub(crate) fn load_memory(&mut self, bytes: &[u8]) {
        match bincode::deserialize(bytes) {
            Ok(saved_memory) => {
                self.context.memory_mut(|memory| {
                    *memory = saved_memory;
                });
            }
            Err(e) => {
                log::warn!("failed to deserialize memory: {e}");
            }
        }
    }

    pub(crate) fn update_clock(&mut self) {
        self.now = Instant::now();
    }
}
