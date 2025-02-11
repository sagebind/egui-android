//! Automatically provides an appropriate entrypoint for initializing an Android
//! app.

#![doc(hidden)]

use crate::{
    internal::{bindings::application_info::ApplicationInfo, logging, runner::Runner},
    App,
};

pub use android_activity::AndroidApp;

/// Our implementation of an Android main for `NativeActivity`.
pub fn main<T: App>(android_app: AndroidApp) {
    let app_info = ApplicationInfo::for_android_app(&android_app).unwrap();

    logging::init(app_info.package_name().unwrap());

    Runner::<T>::new(android_app).run_until_closed();

    log::debug!("app exited cleanly");
}

/// Define the entrypoint for an Android app.
#[macro_export]
macro_rules! entrypoint {
    (
        app = $app:ty
    ) => {
        #[doc(hidden)]
        #[no_mangle]
        pub fn android_main(android_app: $crate::entrypoint::AndroidApp) {
            $crate::entrypoint::main::<$app>(android_app);
        }
    };
}
