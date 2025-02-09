use egui_android::{egui, App};

struct MyApp {
    name: String,
    age: u32,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            name: "Arthur".to_owned(),
            age: 42,
        }
    }
}

impl App for MyApp {
    fn create() -> Self {
        Self::default()
    }

    fn update(&mut self, ctx: &egui::Context) {
        let time = ctx.input(|input| input.time);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(64.0);

            ui.heading("My egui Application");

            ui.horizontal(|ui| {
                let name_label = ui.label("Your name: ");
                ui.text_edit_singleline(&mut self.name)
                    .labelled_by(name_label.id);
            });

            ui.add(egui::Slider::new(&mut self.age, 0..=120).text("age"));

            if ui.button("Increment").clicked() {
                log::info!("incrementing age");
                self.age += 1;
            }

            ui.label(format!("Hello '{}', age {}", self.name, self.age));

            ui.label(format!("Last update timestamp: {}", time));
            ui.label(format!("Focused: {:?}", ui.input(|i| i.viewport().focused)));

            ui.heading("Inspector");
            ctx.inspection_ui(ui);
        });
    }
}

egui_android::entrypoint! {
    app = MyApp
}
