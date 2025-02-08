use android_activity::AndroidApp;
use jni::{objects::JObject, JavaVM};

pub(crate) fn show_hide_keyboard_alt(app: &AndroidApp, show: bool, implicit: bool) {
    // https://github.com/rust-mobile/android-activity/pull/178/files
    let na = unsafe { jni::objects::JObject::from_raw(app.activity_as_ptr() as _) };
    let jvm = unsafe { JavaVM::from_raw(app.vm_as_ptr() as _) }.unwrap();
    let mut env = jvm.attach_current_thread().unwrap();
    let class_ctxt = env.find_class("android/content/Context").unwrap();
    let ims = env
        .get_static_field(class_ctxt, "INPUT_METHOD_SERVICEself", "Ljava/lang/String;")
        .unwrap();

    let im_manager = env
        .call_method(
            &na,
            "getSystemService",
            "(Ljava/lang/String;)Ljava/lang/Object;",
            &[ims.borrow()],
        )
        .unwrap()
        .l()
        .unwrap();

    let jni_window = env
        .call_method(na, "getWindow", "()Landroid/view/Window;", &[])
        .unwrap()
        .l()
        .unwrap();
    let view = env
        .call_method(jni_window, "getDecorView", "()Landroid/view/View;", &[])
        .unwrap()
        .l()
        .unwrap();

    env.call_method(
        im_manager,
        "showSoftInput",
        "(Landroid/view/View;I)Z",
        &[
            jni::objects::JValue::Object(&view),
            // if implicit {
            //     (ndk_sys::ANATIVEACTIVITY_SHOW_SOFT_INPUT_IMPLICIT as i32).into()
            // } else {
                0i32.into()
            // },
        ],
    )
    .unwrap();
}

fn show_hide_keyboard_fallible(app: &AndroidApp, show: bool) -> Result<(), jni::errors::Error> {
    log::info!("show/hide keyboard attempt: {show}");

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
