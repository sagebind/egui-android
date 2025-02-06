# Default to the first one defined in the dev environment
EMULATOR_AVD := $(shell $(ANDROID_HOME)/emulator/emulator -list-avds | grep -v INFO | head -n1)


.PHONY: example
example:
	cd examples/hello-world && cargo ndk build

.PHONY: test
test:
	cargo test --features ndk/test

.PHONY: emulator
emulator:
	$(ANDROID_HOME)/emulator/emulator -avd $(EMULATOR_AVD) -netdelay none -netspeed full -no-snapshot -restart-when-stalled
