This is an Android project based on a custom Activity that can be used to
test Android SDK JNI bindings generated with jbindgen.

By default (with no features enabled), no SDK bindings are generated and so
running the app will just verify that the generated bindings for TestActivity
itself are correct.

Additional Cargo features can be enabled to generate bindings for specific
Android SDK APIs.

## Available SDK Binding Features

The following features can be enabled to test specific Android SDK bindings:

- `sdk_os_build` - Generates bindings for `android.os.Build` (device info)
- `sdk_utils_time_utils` - Generates bindings for `android.util.TimeUtils` + `android.icu.util.TimeZone` (time utilities)
- `sdk_content_intent` - Generates bindings for `android.content.Intent` (Intents)
- `sdk_net_uri` - Generates bindings for `android.net.Uri` (URI handling)
- `sdk_os_binder` - Generates bindings for `android.os.Binder` (IPC primitives)
- `sdk_bluetooth` - Generates bindings for `android.bluetooth.le` (Bluetooth LE)

TestActivity will run a test for each enabled SDK feature when the app is launched,
with some minimal UI feedback.

Most of the tests just involve calling `jni_init` for the generated binding which
will perform a Class lookup and cache method/field IDs (and would fail if the
class or any members were missing). Some tests perform additional operations
to smoke test that the bindings work as expected.

# Build

```
export ANDROID_NDK_HOME="path/to/ndk"
export ANDROID_HOME="path/to/sdk"

rustup target add aarch64-linux-android
cargo install cargo-ndk

# Build with no SDK features (default)
cargo ndk -t arm64-v8a -o app/src/main/jniLibs/  build

# Build with various SDK features enabled
cargo ndk -t arm64-v8a -o app/src/main/jniLibs/ build --features sdk_os_build,sdk_os_binder

./gradlew build
./gradlew installDebug
```

# Run

```
adb shell am start -n io.github.jni_rs.jbindgen.testactivity/.TestActivity
```

# Test on Emulator or Device

See `tests/android_test_activity.rs` for a unit test that will automate building
the crate as a jniLib and launching the Android app on an emulator (or
optionally a device) and verify that the generated bindings are valid.

Run the tests on an emulator with:

```
cargo test --all-features -- --nocapture
```

or on an ARM64 device (automatically picks first, non-emulator device it finds):
```
env ANDROID_TEST_TARGET=aarch64-linux-android \
    cargo test --all-features -- --nocapture
```

or a specific connected device with:

```
env ANDROID_TEST_DEVICE_SERIAL=DEVICE_SERIAL \
    cargo test --all-features -- --nocapture
```