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
    fn update(&mut self, ctx: &egui::Context);
}
