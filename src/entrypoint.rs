#![doc(hidden)]

use crate::{logging, runner::Runner, App};

pub use android_activity::AndroidApp;

/// Our implementation of an Android main for `NativeActivity`.
pub fn main<T: App>(android_app: AndroidApp) {
    logging::init();

    Runner::<T>::new(android_app).run_until_closed();

    log::debug!("app exited cleanly");
}

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
