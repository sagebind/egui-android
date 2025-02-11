# Default to the first one defined in the dev environment
EMULATOR_AVD := $(shell $(ANDROID_HOME)/emulator/emulator -list-avds | grep -v INFO | head -n1)
export ANDROID_NDK_ROOT := $(ANDROID_HOME)/ndk/27.2.12479018

.PHONY: example
example:
	cargo apk build -p egui-android-demo

.PHONY: run
run:
	cargo apk run -p egui-android-demo

.PHONY: test
test:
	cargo test --features ndk/test

.PHONY: emulator
emulator:
	$(ANDROID_HOME)/emulator/emulator -avd $(EMULATOR_AVD) -netdelay none -netspeed full -no-snapshot -restart-when-stalled
