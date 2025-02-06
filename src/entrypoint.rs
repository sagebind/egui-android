#![doc(hidden)]

use crate::{
    logging,
    runner::{ControlFlow, Runner},
    App,
};

pub use android_activity::AndroidApp;

/// Our implementation of an Android main for `NativeActivity`.
pub fn main<T: App>(android_app: AndroidApp) {
    logging::init();

    log::info!("screen density: {:?}", android_app.config().density());
    log::info!("content rect: {:?}", android_app.content_rect());

    let mut runner = Runner::<T>::new(android_app);

    loop {
        if runner.run_once() == ControlFlow::Quit {
            break;
        }
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
