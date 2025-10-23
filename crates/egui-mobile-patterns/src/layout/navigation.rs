use std::fmt::Display;

use egui::{
    vec2, Color32, Context, Frame, Id, Rect, RichText, Sense, Stroke, StrokeKind, Ui, Widget,
};

pub struct NavigationBar {}

pub struct NavigationWrapper<'a> {
    id: Id,
    tabs: Vec<NavigationTab<'a>>,
}

impl<'a> NavigationWrapper<'a> {
    pub fn new() -> Self {
        Self {
            id: Id::new("navigation"),
            tabs: Vec::new(),
        }
    }

    pub fn tab(mut self, tab: NavigationTab<'a>) -> Self {
        self.tabs.push(tab);
        self
    }

    pub fn show(mut self, ctx: &Context) {
        let state = ctx
            .memory_mut(|memory| memory.data.get_persisted::<NavigationWrapperState>(self.id))
            .unwrap_or_default();

        egui::TopBottomPanel::bottom("toolbar")
            .frame(Frame::none().fill(Color32::from_white_alpha(2)))
            .show_separator_line(false)
            .show(ctx, |ui| {
                ui.add_space(6.0);

                ui.columns(self.tabs.len(), |columns| {
                    for (i, tab) in self.tabs.iter().enumerate() {
                        let ui = &mut columns[i];
                        let active = i == state.active_tab;
                        if ui.add(toolbar_button('A', &tab.title, active)).clicked() {
                            ctx.memory_mut(|memory| {
                                memory.data.insert_persisted(
                                    self.id,
                                    NavigationWrapperState { active_tab: i },
                                );
                            });
                        }
                    }
                });

                ui.add_space(36.0);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            (self.tabs[state.active_tab].show)(ui);
        });
    }
}

pub struct NavigationTabBuilder {
    title: String,
}

impl NavigationTabBuilder {
    pub fn new(title: impl Display) -> Self {
        Self {
            title: title.to_string(),
        }
    }

    pub fn contents<'a>(self, show: impl FnMut(&mut Ui) + 'a) -> NavigationTab<'a> {
        NavigationTab {
            title: self.title,
            show: Box::new(show),
        }
    }
}

pub struct NavigationTab<'a> {
    title: String,
    show: Box<dyn FnMut(&mut Ui) + 'a>,
}

#[derive(Clone, Default, serde::Deserialize, serde::Serialize)]
struct NavigationWrapperState {
    active_tab: usize,
}

fn toolbar_button(icon: char, label: &str, selected: bool) -> impl Widget + '_ {
    let text_color = if selected {
        Color32::LIGHT_BLUE
    } else {
        Color32::GRAY
    };

    move |ui: &mut Ui| {
        ui.push_id(label, |ui| {
            if selected {
                let center_top = ui.available_rect_before_wrap().center_top();
                let shadow_area =
                    Rect::from_two_pos(center_top - vec2(24.0, 2.0), center_top + vec2(24.0, 36.0));

                ui.painter().rect(
                    shadow_area,
                    8.0,
                    Color32::from_rgba_unmultiplied(255, 255, 255, 2),
                    Stroke::NONE,
                    StrokeKind::Middle,
                );
            }

            ui.vertical_centered(|ui| {
                ui.label(RichText::new(icon.to_string()).size(18.0).color(text_color));
                ui.label(RichText::new(label).small().color(text_color));
            });
        })
        .response
        .interact(Sense::click())
    }
}
