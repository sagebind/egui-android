pub(crate) fn init() {
    #[cfg(feature = "logger")]
    {
        use std::sync::Once;

        static ONCE: Once = Once::new();

        ONCE.call_once(|| {
            android_logger::init_once(
                android_logger::Config::default()
                    .with_max_level(log::LevelFilter::Debug)
                    .with_tag("rust"),
            );

            log_panics::init();
        });
    }
}
