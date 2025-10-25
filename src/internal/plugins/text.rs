use crate::internal::widgets::text_context_menu::TextContextMenu;
use egui::{
    pos2, vec2, Align2, Color32, Context, Event, FontId, FullOutput, Id, LayerId, Plugin, Popup,
    PopupAnchor, PopupKind, RawInput, Rect, ViewportCommand,
};

pub(crate) struct TextPlugin {
    id: Id,
    menu: Option<MenuInfo>,
    captured_events: Vec<Event>,
}

struct MenuInfo {
    layer: LayerId,
    area: Rect,
}

impl Default for TextPlugin {
    fn default() -> Self {
        Self {
            id: Id::new("egui_android.text_popup"),
            menu: None,
            captured_events: Vec::new(),
        }
    }
}

impl Plugin for TextPlugin {
    fn debug_name(&self) -> &'static str {
        "Android Text Support"
    }

    fn input_hook(&mut self, input: &mut RawInput) {
        // We don't want any user interaction with the input menu to steal focus
        // from any currently-focused text element, so we look for such events
        // here, record them, and then remove them to prevent them from causing
        // text focus to be lost.
        if let Some(menu) = self.menu.as_mut() {
            input.events.retain(|event| {
                if let Event::PointerButton {
                    pos,
                    button,
                    pressed,
                    modifiers,
                } = event
                {
                    if menu.area.contains(*pos) {
                        self.captured_events.push(event.clone());
                        return false;
                    }
                }

                true
            });
        }
    }

    fn output_hook(&mut self, output: &mut FullOutput) {
        // Check if we need to show the text context menu.
        if let Some(ime) = output.platform_output.ime.as_mut() {
            // Configure the popup menu.
            let had_menu_before = self.menu.is_some();

            self.menu = Some(MenuInfo {
                layer: LayerId::debug(),
                area: Rect::from_min_size(ime.rect.min - vec2(0.0, 40.0), vec2(100.0, 30.0)),
            });

            if !had_menu_before {
                // need repaint
            }
        } else {
            self.menu = None;
        }
    }

    fn on_end_pass(&mut self, ctx: &Context) {
        if let Some(menu) = self.menu.as_ref() {
            let painter = ctx.layer_painter(menu.layer).with_clip_rect(menu.area);

            painter.rect_filled(menu.area, 3.0, Color32::GRAY);
            painter.text(
                menu.area.left_center() + vec2(0.0, 0.0),
                Align2::LEFT_CENTER,
                "Cut",
                FontId::proportional(12.0),
                Color32::BLACK,
            );
            painter.text(
                menu.area.left_center() + vec2(40.0, 0.0),
                Align2::LEFT_CENTER,
                "Copy",
                FontId::proportional(12.0),
                Color32::BLACK,
            );
            painter.text(
                menu.area.left_center() + vec2(80.0, 0.0),
                Align2::LEFT_CENTER,
                "Paste",
                FontId::proportional(12.0),
                Color32::BLACK,
            );
        }
    }
}
