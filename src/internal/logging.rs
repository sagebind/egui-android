pub(crate) fn init(package_name: String) {
    #[cfg(feature = "logger")]
    {
        use std::sync::Once;

        static ONCE: Once = Once::new();

        ONCE.call_once(|| {
            android_logger::init_once(
                android_logger::Config::default()
                    .with_max_level(log::LevelFilter::Debug)
                    .with_tag(package_name),
            );

            log_panics::init();
        });
    }
}
