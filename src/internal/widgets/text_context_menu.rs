//! A context menu that appears when the user long-presses on text inputs.

use egui::{Response, Ui, ViewportCommand, Widget};

pub(crate) struct TextContextMenu;

impl Widget for TextContextMenu {
    fn ui(self, ui: &mut Ui) -> Response {
        if ui.button("Copy").clicked() {
            log::info!("copy button clicked");
            ui.ctx().send_viewport_cmd(ViewportCommand::RequestCopy);
        }

        if ui.button("Cut").clicked() {
            log::info!("cut button clicked");
            ui.ctx().send_viewport_cmd(ViewportCommand::RequestCut);
        }

        if ui.button("Paste").clicked() {
            log::info!("paste button clicked");
            ui.ctx().send_viewport_cmd(ViewportCommand::RequestPaste);
        }

        ui.response()
    }
}
