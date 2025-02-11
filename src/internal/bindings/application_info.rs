use android_activity::AndroidApp;
use jni::{
    objects::{GlobalRef, JObject, JString},
    JavaVM,
};

type Error = Box<dyn std::error::Error>;

pub(crate) struct ApplicationInfo {
    application_info: GlobalRef,
    vm: JavaVM,
}

impl ApplicationInfo {
    /// Get an instance of
    /// [`ApplicationInfo`](https://developer.android.com/reference/android/content/pm/ApplicationInfo)
    /// for the given Android application.
    pub(crate) fn for_android_app(android_app: &AndroidApp) -> Result<Self, Error> {
        let vm = unsafe { JavaVM::from_raw(android_app.vm_as_ptr() as _)? };
        let activity = unsafe { JObject::from_raw(android_app.activity_as_ptr() as _) };
        let mut env = vm.attach_current_thread()?;

        let application_context = env
            .call_method(
                &activity,
                "getApplicationContext",
                "()Landroid/content/Context;",
                &[],
            )?
            .l()?;
        let application_info = env
            .call_method(
                &application_context,
                "getApplicationInfo",
                "()Landroid/content/pm/ApplicationInfo;",
                &[],
            )?
            .l()?;

        let application_info = env.new_global_ref(application_info)?;

        drop(env);

        Ok(Self {
            application_info,
            vm,
        })
    }

    /// Public name of this item.
    pub(crate) fn name(&self) -> Result<String, Error> {
        let mut env = self.vm.attach_current_thread()?;

        let name = env
            .get_field(&self.application_info, "name", "Ljava/lang/String;")?
            .l()?;
        let name = JString::from(name);
        let name = env.get_string(&name)?;

        Ok(name.into())
    }

    /// Public name of this item.
    pub(crate) fn label_res(&self) -> Result<String, Error> {
        let mut env = self.vm.attach_current_thread()?;

        let label_res = env
            .get_field(&self.application_info, "labelRes", "Ljava/lang/String;")?
            .l()?;
        let label_res = JString::from(label_res);
        let label_res = env.get_string(&label_res)?;

        Ok(label_res.into())
    }

    /// Name of the package that this item is in.
    pub(crate) fn package_name(&self) -> Result<String, Error> {
        let mut env = self.vm.attach_current_thread()?;

        let package_name = env
            .get_field(&self.application_info, "packageName", "Ljava/lang/String;")?
            .l()?;
        let package_name = JString::from(package_name);
        let package_name = env.get_string(&package_name)?;

        Ok(package_name.into())
    }
}
