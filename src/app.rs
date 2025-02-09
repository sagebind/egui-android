use egui::{Context, Rect};
use std::time::Duration;

/// Core trait for implementing the root of an egui Android application.
///
/// In the official egui ecosystem, this trait is equivalent to
/// [`eframe::App`](https://docs.rs/eframe/latest/eframe/trait.App.html).
pub trait App {
    /// Called when a new activity instance for this app is opened.
    ///
    /// It is important to note that this method may be called multiple times
    /// during the lifetime of the current process as an application is opened
    /// by the user multiple times. Be careful with global state, which may or
    /// may not be already initialized.
    fn create() -> Self;

    /// Called each time the UI needs repainting, which may be many times per
    /// second.
    fn update(&mut self, ctx: &Context);

    /// Called when the rectangle in the window in which content should be
    /// placed has changed.
    fn on_content_rect_changed(&mut self, _new_rect: Rect) {
        // By default, do nothing.
    }

    /// Called by Android when the system is running low on memory.
    fn on_low_memory(&mut self) {
        // By default, do nothing.
    }

    /// Controls a minimum update frequency. `update` will be called at _least_
    /// this often while your app is focused, even if no input events are
    /// received or the OS asks for a redraw.
    ///
    /// If `None` is returned, `update` will only be called if required.
    fn min_update_frequency(&self) -> Option<Duration> {
        None
    }
}
