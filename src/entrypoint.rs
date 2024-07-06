#![doc(hidden)]

use crate::{App, runner::Runner};

pub use android_activity::AndroidApp;

/// Our implementation of an Android main for `NativeActivity`.
pub fn main<T: App>(android_app: AndroidApp) {
    let mut runner = Runner::<T>::new(android_app);

    loop {
        runner.run_once();
    }
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
