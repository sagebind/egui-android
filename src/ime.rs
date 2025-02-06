use android_activity::AndroidApp;
use jni::{objects::JObject, JavaVM};

fn show_hide_keyboard_fallible(app: &AndroidApp, show: bool) -> Result<(), jni::errors::Error> {
    // After Android R, it is no longer possible to show the soft keyboard
    // with `showSoftInput` alone.
    // Here we use `WindowInsetsController`, which is the other way.
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr() as _)? };
    let activity = unsafe { JObject::from_raw(app.activity_as_ptr() as _) };
    let mut env = vm.attach_current_thread()?;
    let window = env
        .call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])?
        .l()?;
    let wic = env
        .call_method(
            window,
            "getInsetsController",
            "()Landroid/view/WindowInsetsController;",
            &[],
        )?
        .l()?;
    let window_insets_types = env.find_class("android/view/WindowInsets$Type")?;
    let ime_type = env
        .call_static_method(&window_insets_types, "ime", "()I", &[])?
        .i()?;
    env.call_method(
        &wic,
        if show { "show" } else { "hide" },
        "(I)V",
        &[ime_type.into()],
    )?
    .v()
}

pub(crate) fn show_hide_keyboard(app: &AndroidApp, show: bool) {
    if let Err(e) = show_hide_keyboard_fallible(app, show) {
        log::error!("Showing or hiding the soft keyboard failed: {e:?}");
    };
}
