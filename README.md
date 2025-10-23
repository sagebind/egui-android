# egui-android

This is an opinionated library and framework for creating [egui] apps for Android, that acts as an alternative backend to [eframe]. The goal is to make it relatively easy to write Android apps using pure Rust and egui.

**This project is considered highly experimental! The demo should run, but not everything you would expect out of a production-ready Android app may be working yet.**

While eframe *can* be used on Android, it does currently have certain limitations and missing features, primarily caused by winit which eframe always uses.

This backend instead removes winit as the middleman and instead provides glue from Android to egui directly. This offers improved user experience for at least the following:

- Better handling of sleep and resume without panics
- Automatic activity save and resume state
- Native-feeling soft keyboard support

While winit's Android support will likely continue to improve, much of the value that winit provides doesn't apply as much on mobile devices, and will always have to add another abstraction layer on top of the underlying platform. Ultimately winit will always be at a disadvantage as far as tight platform integration.

The downside is that this crate is specific to egui, and cannot be used with any other UI toolkit. This only works because egui already has its own backend-agnostic API, for which it provides a good deal of information to allow the backend to integrate well with it.

## Building

Managing Android SDKs in a reproducible way can be annoying, especially when not using Android Studio. This project attempts to avoid all use of Gradle, which means we have to do some things from scratch.

First prerequisite is [devenv](https://devenv.sh) which is a tool that helps manage reproducible developer environments. This tool will install all other required dependencies for you.

Once installed, you can build an APK for the provided demo app with:

```sh
devenv tasks run demo:build
```

Note that the first time you run a devenv command, devenv will need to download and build all dependencies for the development environment. This includes the Android developer SDK, which is quite large and can take a while to install, so be patient.

You can even run the demo app in an Android emulator once compiled. To do this, first run the Android emulator in the background by running:

```sh
devenv up -d
```

This should open an emulated Android device in a window after a while. Then while it is open, you can run

```sh
devenv tasks run demo:build
```

This will automatically deploy the demo app APK to the emulated device, and open the app. The app's log output will be shown in the terminal while the task is running.

## License

This project's source code and documentation is licensed under the MIT license. See the [LICENSE](LICENSE) file for details.


[eframe]: https://crates.io/crates/eframe
[egui]: https://crates.io/crates/eframe
