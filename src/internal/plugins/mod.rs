//! egui plugins that are internal to the Android integration. These plugins are
//! loaded into the egui context automatically.

use egui::Context;

mod text;

/// Register all internal plugins to the given egui context.
pub(crate) fn register_all_plugins(ctx: &Context) {
    ctx.add_plugin(text::TextPlugin::default());
}
