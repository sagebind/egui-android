# egui-android

This is an opinionated library and framework for creating [egui] apps for Android, that acts as an alternative backend to [eframe]. The goal is to make it relatively easy to write Android apps using pure Rust and egui.

While eframe *can* be used on Android, it does currently have certain limitations and missing features, primarily caused by winit which eframe always uses.

This backend instead removes winit as the middleman and instead provides glue from Android to egui directly. This offers improved user experience for at least the following:

- Better handling of sleep and resume without panics
- Automatic activity save and resume state
- Native-feeling soft keyboard support

While winit's Android support will likely continue to improve, much of the value that winit provides doesn't apply as much on mobile devices, and will always have to add another abstraction layer on top of the underlying platform. Ultimately winit will always be at a disadvantage as far as tight platform integration.

The downside is that this crate is specific to egui, and cannot be used with any other UI toolkit. This only works because egui already has its own backend-agnostic API, for which it provides a good deal of information to allow the backend to integrate well with it.

## License

This project's source code and documentation is licensed under the MIT license. See the [LICENSE](LICENSE) file for details.


[eframe]: https://crates.io/crates/eframe
[egui]: https://crates.io/crates/eframe
