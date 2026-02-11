//! Android SDK binding tests
//!
//! Note: the bindings are generated in `build.rs` and the main library for
//! this crate is what really runs the tests.
//!
//! This just automates the process of building the APK, installing it on a
//! device or emulator, and running the tests via `cargo test`.

use core::panic;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use adb_client::ADBDeviceExt;
use adb_client::server::ADBServer;
use android_emulator::{EmulatorConfig, proto::ImageFormat};

/// Minimum required Android NDK version
const MIN_NDK_VERSION: &str = "27.3.0";
/// Minimum required cargo-ndk version
const MIN_CARGO_NDK_VERSION: &str = "4.1.2";

/// Represents either a physical device or an emulator
#[allow(clippy::large_enum_variant)]
enum Device {
    Physical {
        serial: String,
    },
    Emulator {
        client: android_emulator::EmulatorClient,
        instance: Arc<android_emulator::Emulator>,
    },
}

impl Device {
    /// Get the device serial number
    fn serial(&self) -> &str {
        match self {
            Device::Physical { serial } => serial,
            Device::Emulator { instance, .. } => instance.serial(),
        }
    }
}

fn get_android_home() -> PathBuf {
    env::var("ANDROID_HOME")
        .or_else(|_| env::var("ANDROID_SDK_ROOT"))
        .map(PathBuf::from)
        .expect("ANDROID_HOME or ANDROID_SDK_ROOT must be set")
}

/// Find and validate the Android NDK installation
/// Returns the path to a valid NDK directory
fn find_ndk() -> PathBuf {
    // First, try ANDROID_NDK_ROOT
    if let Ok(ndk_root) = env::var("ANDROID_NDK_ROOT") {
        let ndk_path = PathBuf::from(&ndk_root);
        if ndk_path.exists() {
            println!("Found NDK via ANDROID_NDK_ROOT: {}", ndk_path.display());
            if let Some(version) = get_ndk_version(&ndk_path) {
                if is_version_sufficient(&version, MIN_NDK_VERSION) {
                    println!(
                        "NDK version {} meets minimum requirement {}",
                        version, MIN_NDK_VERSION
                    );
                    return ndk_path;
                } else {
                    panic!(
                        "NDK version {} is below minimum required version {}. Please upgrade your NDK.",
                        version, MIN_NDK_VERSION
                    );
                }
            } else {
                panic!(
                    "Could not determine NDK version from {}",
                    ndk_path.display()
                );
            }
        }
    }

    // Fallback: look for NDK under ANDROID_HOME/ndk/ or ANDROID_SDK_ROOT/ndk/
    let android_home = get_android_home();

    let ndk_dir = PathBuf::from(&android_home).join("ndk");
    if !ndk_dir.exists() {
        panic!(
            "NDK directory not found at {}. Please install the Android NDK.",
            ndk_dir.display()
        );
    }

    // Find all NDK versions in the ndk directory
    let mut ndk_versions: Vec<(String, PathBuf)> = Vec::new();

    if let Ok(entries) = fs::read_dir(&ndk_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            // Only consider directories that contain source.properties
            if path.is_dir()
                && path.join("source.properties").exists()
                && let Some(version) = get_ndk_version(&path)
            {
                ndk_versions.push((version, path));
            }
        }
    }

    if ndk_versions.is_empty() {
        panic!(
            "No valid NDK installations found in {}. Please install the Android NDK.",
            ndk_dir.display()
        );
    }

    // Sort by version (descending) to get the highest version
    ndk_versions.sort_by(|a, b| compare_versions(&b.0, &a.0));

    // Check if the highest version meets the minimum requirement
    let (highest_version, highest_path) = &ndk_versions[0];
    if is_version_sufficient(highest_version, MIN_NDK_VERSION) {
        println!(
            "Using NDK {} from {}",
            highest_version,
            highest_path.display()
        );
        highest_path.clone()
    } else {
        panic!(
            "Highest available NDK version {} is below minimum required version {}. Please upgrade your NDK.",
            highest_version, MIN_NDK_VERSION
        );
    }
}

/// Extract NDK version from source.properties file
fn get_ndk_version(ndk_path: &Path) -> Option<String> {
    let source_props = ndk_path.join("source.properties");
    if !source_props.exists() {
        return None;
    }

    let content = fs::read_to_string(&source_props).ok()?;

    for line in content.lines() {
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            if key == "Pkg.Revision" {
                return Some(value.to_string());
            }
        }
    }

    None
}

fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let a_parts: Vec<u32> = a.split('.').filter_map(|s| s.parse().ok()).collect();
    let b_parts: Vec<u32> = b.split('.').filter_map(|s| s.parse().ok()).collect();

    for i in 0..a_parts.len().max(b_parts.len()) {
        let a_part = a_parts.get(i).copied().unwrap_or(0);
        let b_part = b_parts.get(i).copied().unwrap_or(0);

        match a_part.cmp(&b_part) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }

    std::cmp::Ordering::Equal
}

/// Check if version meets minimum requirement
fn is_version_sufficient(version: &str, min_version: &str) -> bool {
    matches!(
        compare_versions(version, min_version),
        std::cmp::Ordering::Greater | std::cmp::Ordering::Equal
    )
}

/// Check cargo-ndk version meets minimum requirement
fn check_cargo_ndk_version() -> bool {
    let output = match Command::new("cargo").arg("install").arg("--list").output() {
        Ok(output) => output,
        Err(e) => {
            eprintln!("Failed to run cargo install --list: {}", e);
            return false;
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Look for a line like "cargo-ndk v4.1.2:"
    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("cargo-ndk v")
            && let Some(version_str) = rest.split(':').next()
        {
            let version = version_str.trim();
            println!("Found cargo-ndk version: {}", version);

            if is_version_sufficient(version, MIN_CARGO_NDK_VERSION) {
                println!(
                    "cargo-ndk version {} meets minimum requirement {}",
                    version, MIN_CARGO_NDK_VERSION
                );
                return true;
            } else {
                eprintln!(
                    "cargo-ndk version {} is below minimum required version {}.",
                    version, MIN_CARGO_NDK_VERSION
                );
                eprintln!("Please upgrade: cargo install cargo-ndk --force");
                return false;
            }
        }
    }

    eprintln!("cargo-ndk not found in cargo install --list output.");
    eprintln!("Install with: cargo install cargo-ndk");
    false
}

/// Build the test-activity project for Android
fn build_android_test_activity(project_dir: &PathBuf, features: &str) -> bool {
    // Check cargo-ndk version
    if !check_cargo_ndk_version() {
        return false;
    }

    // Find and validate NDK
    let ndk_path = find_ndk();

    // Determine target based on environment variables
    // If ANDROID_TEST_TARGET is set, use that
    // Otherwise, if ANDROID_TEST_SERIAL is set, assume physical device (aarch64)
    // Otherwise, use emulator target (x86_64)
    let target = if let Ok(target) = env::var("ANDROID_TEST_TARGET") {
        println!("Using target from ANDROID_TEST_TARGET: {target}");
        target
    } else if env::var("ANDROID_TEST_SERIAL").is_ok() {
        println!("ANDROID_TEST_SERIAL set, using aarch64-linux-android for physical device");
        "aarch64-linux-android".to_string()
    } else {
        println!("Using x86_64-linux-android for emulator");
        "x86_64-linux-android".to_string()
    };

    println!("Building Rust library for Android target: {target}");

    let mut cmd = Command::new("cargo");
    cmd.arg("ndk")
        .arg("--platform")
        .arg("35")
        .arg("-o")
        .arg("app/src/main/jniLibs/")
        .arg("build")
        .arg("--target")
        .arg(&target)
        .arg("--features")
        .arg(features)
        .env("ANDROID_NDK_HOME", &ndk_path)
        .current_dir(project_dir);

    println!("Executing: {:?}", cmd);
    let output = cmd.output();

    match output {
        Ok(output) => {
            if !output.status.success() {
                eprintln!("Build failed for {target}:");
                eprintln!("{}", String::from_utf8_lossy(&output.stderr));
                return false;
            }
        }
        Err(e) => {
            eprintln!("Failed to run cargo ndk for {target}: {e}");
            return false;
        }
    }

    true
}

/// Build the Android APK using gradle
fn build_apk_with_gradle(project_dir: &PathBuf) -> Result<PathBuf, String> {
    println!("Building APK with gradle...");

    let gradlew = if cfg!(windows) {
        project_dir.join("gradlew.bat")
    } else {
        project_dir.join("gradlew")
    };

    if !gradlew.exists() {
        return Err(format!("gradlew not found at {}", gradlew.display()));
    }

    let output = Command::new(&gradlew)
        .arg("assembleDebug")
        .current_dir(project_dir)
        .output()
        .map_err(|e| format!("Failed to run gradlew: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Gradle build failed:\n{}", stderr));
    }

    let apk_path = project_dir.join("app/build/outputs/apk/debug/app-debug.apk");
    if !apk_path.exists() {
        return Err(format!("APK not found at {}", apk_path.display()));
    }

    println!("APK built: {}", apk_path.display());
    Ok(apk_path)
}

/// Avoid using AdbServer::default() because that depends on `adb` being in the `PATH`
fn adb_server() -> adb_client::server::ADBServer {
    let android_home = get_android_home();
    let adb_path =
        android_home
            .join("platform-tools")
            .join(if cfg!(windows) { "adb.exe" } else { "adb" });

    if !adb_path.exists() {
        panic!(
            "adb not found at {}. Please ensure Android SDK platform-tools are installed.",
            adb_path.display()
        );
    }
    let adb_path: String = adb_path.to_str().expect("Invalid adb path").to_string();
    let addr = std::net::SocketAddrV4::new(std::net::Ipv4Addr::LOCALHOST, 5037);

    ADBServer::new_from_path(addr, Some(adb_path))
}

/// Get the serial number of the first non-emulator device (physical device)
fn get_physical_device_serial() -> Option<String> {
    let mut server = adb_server();
    if let Ok(devices) = server.devices() {
        // Find the first device that does NOT start with "emulator-"
        devices
            .iter()
            .find(|d| !d.identifier.starts_with("emulator-"))
            .map(|d| d.identifier.clone())
    } else {
        None
    }
}

/// Install APK on the device using adb_client
fn install_apk(apk_path: &PathBuf, device_serial: &str) -> Result<(), String> {
    println!(
        "Installing APK on device {}: {}",
        device_serial,
        apk_path.display()
    );

    let mut server = adb_server();
    let mut device = server
        .get_device_by_name(device_serial)
        .map_err(|e| format!("Failed to get device {}: {}", device_serial, e))?;

    device
        .install(apk_path, None)
        .map_err(|e| format!("Failed to install APK: {}", e))?;

    println!("APK installed successfully");
    Ok(())
}

/// Launch the TestActivity on the device
fn launch_test_activity(device_serial: &str) -> Result<(), String> {
    println!("Launching TestActivity on device {}...", device_serial);

    let mut server = adb_server();
    let mut device = server
        .get_device_by_name(device_serial)
        .map_err(|e| format!("Failed to get device {}: {}", device_serial, e))?;

    let mut output = Vec::new();
    let cmd = "am start -n io.github.jni_rs.jbindgen.testactivity/.TestActivity";
    device
        .shell_command(&cmd, Some(&mut output), None)
        .map_err(|e| format!("Failed to launch activity: {}", e))?;

    let result = String::from_utf8_lossy(&output);
    if result.contains("Error") {
        return Err(format!("Activity launch failed: {}", result));
    }

    println!("Activity launched");
    println!("Output: {}", result);
    Ok(())
}

/// Get current Unix epoch time from the device
fn get_device_epoch_time(device_serial: &str) -> Result<u64, String> {
    let mut server = adb_server();
    let mut device = server
        .get_device_by_name(device_serial)
        .map_err(|e| format!("Failed to get device {}: {}", device_serial, e))?;

    let mut output = Vec::new();
    device
        .shell_command(&"date +%s", Some(&mut output), None)
        .map_err(|e| format!("Failed to get device time: {}", e))?;

    let time_str = String::from_utf8_lossy(&output);
    time_str
        .trim()
        .parse::<u64>()
        .map_err(|e| format!("Failed to parse device time '{}': {}", time_str, e))
}

/// Stream logcat from device and monitor for test logs
fn stream_logcat(device_serial: &str, start_epoch: u64) -> Result<Vec<String>, String> {
    let mut server = adb_server();
    let mut device = server
        .get_device_by_name(device_serial)
        .map_err(|e| format!("Failed to get device {}: {}", device_serial, e))?;

    // Run logcat with time filter
    let logcat_cmd = format!("logcat -T {}.0 TestActivity:* *:E", start_epoch);
    println!("Running: {}", logcat_cmd);

    // Custom writer that captures output line by line
    struct LogCapture {
        logs: Vec<String>,
        buffer: Vec<u8>,
        test_complete: bool,
        timeout: std::time::Instant,
        max_duration: Duration,
    }

    impl std::io::Write for LogCapture {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            // Check for timeout
            if self.timeout.elapsed() > self.max_duration {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "Log streaming timeout",
                ));
            }

            // Check if test is complete
            if self.test_complete {
                // Return an error to stop the shell command from continuing
                return Err(std::io::Error::other("Test complete"));
            }

            self.buffer.extend_from_slice(buf);

            // Process complete lines
            while let Some(newline_pos) = self.buffer.iter().position(|&b| b == b'\n') {
                let line_bytes = self.buffer.drain(..=newline_pos).collect::<Vec<_>>();
                if let Ok(line) = String::from_utf8(line_bytes) {
                    print!("{}", line);

                    // Always add the line to logs first
                    self.logs.push(line.clone());

                    if line.contains("TEST_ACTIVITY_TEST_COMPLETE") {
                        println!("\nTest completion marker found!");
                        self.test_complete = true;
                        // Return error to immediately stop shell_command
                        return Err(std::io::Error::other("Test complete"));
                    }
                }
            }

            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let mut log_capture = LogCapture {
        logs: Vec::new(),
        buffer: Vec::new(),
        test_complete: false,
        timeout: std::time::Instant::now(),
        max_duration: Duration::from_secs(30),
    };

    // Run logcat - this will block until timeout or test complete
    let _result = device.shell_command(&logcat_cmd, Some(&mut log_capture), None);

    if !log_capture.test_complete {
        println!("\n⚠ Test completion marker not found within timeout");
    }

    Ok(log_capture.logs)
}

/// Capture a screenshot from the emulator
async fn capture_screenshot(
    client: &mut android_emulator::EmulatorClient,
    filename: &str,
) -> Result<(), String> {
    println!("\nCapturing screenshot...");

    let format = ImageFormat {
        format: android_emulator::proto::image_format::ImgFormat::Png.into(),
        width: 0,  // Use device width
        height: 0, // Use device height
        display: 0,
        rotation: None,
        transport: None,
        folded_display: None,
        display_mode: 0,
    };

    let screenshot = client
        .protocol_mut()
        .get_screenshot(format)
        .await
        .map_err(|e| format!("Failed to get screenshot: {}", e))?
        .into_inner();

    println!("Screenshot captured!");
    println!("  Size: {} bytes", screenshot.image.len());
    println!(
        "  Width: {}",
        screenshot.format.as_ref().map(|f| f.width).unwrap_or(0)
    );
    println!(
        "  Height: {}",
        screenshot.format.as_ref().map(|f| f.height).unwrap_or(0)
    );

    // Save to file
    std::fs::write(filename, &screenshot.image)
        .map_err(|e| format!("Failed to write screenshot to {}: {}", filename, e))?;
    println!("Screenshot saved to: {}", filename);

    Ok(())
}

/// Get the test-activity project directory
fn get_test_activity_project() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Helper to spawn an emulator (or use existing device), run a test closure, and handle cleanup
/// Uses a global lock to serialize emulator tests
async fn spawn_device_test<F, Fut>(features: &str, test_fn: F)
where
    F: FnOnce(Device) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send,
{
    println!("\n=== Android Test Activity Emulator Test (features = {features}) ===");

    let project_dir = get_test_activity_project();
    println!("Project: {}", project_dir.display());

    assert!(
        project_dir.exists(),
        "android-test-activity project not found"
    );

    println!("\n--- Building for Android ---");
    if !build_android_test_activity(&project_dir, features) {
        panic!(
            "Failed to build Android test activity. Ensure cargo-ndk is installed and the build succeeds."
        );
    }

    // Build APK with gradle
    println!("\n--- Building APK with gradle ---");
    let apk_path = match build_apk_with_gradle(&project_dir) {
        Ok(path) => path,
        Err(e) => {
            panic!("Gradle build failed: {}", e);
        }
    };

    // Check if ANDROID_TEST_SERIAL is set (use existing device)
    let device = if let Ok(serial) = env::var("ANDROID_TEST_SERIAL") {
        println!("\n--- Using device from ANDROID_TEST_SERIAL ---");
        println!("Device serial: {}", serial);
        Device::Physical { serial }
    } else if env::var("ANDROID_TEST_TARGET").as_deref() == Ok("aarch64-linux-android") {
        // If target is aarch64 but no serial is set, try to find a physical device
        println!("\n--- Looking for physical device (aarch64 target) ---");
        match get_physical_device_serial() {
            Some(serial) => {
                println!("Found physical device: {}", serial);
                Device::Physical { serial }
            }
            None => {
                panic!(
                    "No physical device found. Set ANDROID_TEST_SERIAL to specify a device, or connect a physical device"
                );
            }
        }
    } else {
        // Spawn emulator using android_emulator crate
        println!("\n--- Finding and starting emulator with android_emulator crate ---");

        let avds = android_emulator::list_avds()
            .await
            .expect("Failed to list AVDs");
        let avd_name = avds.first().expect("No AVDs found. Create one with: avdmanager create avd -n test_avd -k 'system-images;android-30;default;x86_64'");

        println!("Using AVD: {}", avd_name);

        let config = EmulatorConfig::new(avd_name)
            .with_window(false)
            .with_snapshot_load(false)
            .with_snapshot_save(false)
            .with_boot_animation(false);

        let instance = config.spawn().await.expect("Failed to spawn emulator");
        println!("Emulator spawned");

        // Connect and wait for boot
        let mut client = instance
            .connect(Some(Duration::from_secs(10)), true)
            .await
            .expect("Failed to connect to emulator");

        println!("Connected to emulator, waiting for boot...");

        let elapsed = client
            .wait_until_booted(Duration::from_secs(200), None)
            .await
            .expect("Emulator failed to boot");

        println!("Emulator ready at: {}", instance.grpc_endpoint());
        println!("Booted in {:.1} seconds", elapsed.as_secs_f64());

        println!("Emulator serial: {}", instance.serial());

        let instance_arc = Arc::new(instance);
        Device::Emulator {
            client,
            instance: instance_arc,
        }
    };

    // Install APK on device/emulator
    println!("\n--- Installing APK on device ---");
    install_apk(&apk_path, device.serial()).expect("Failed to install APK");

    // Clone emulator instance for cleanup if needed
    let emulator_arc_clone = match &device {
        Device::Emulator { instance, .. } => Some(instance.clone()),
        Device::Physical { .. } => None,
    };

    test_fn(device).await;

    // Terminate emulator if we started one
    if let Some(emulator_arc) = emulator_arc_clone {
        match emulator_arc
            .connect(Some(Duration::from_secs(10)), true)
            .await
        {
            Ok(mut client) => {
                println!("\n--- Shutting down emulator gracefully ---");
                if let Err(err) = client.shutdown(None).await {
                    eprintln!("Failed to shutdown emulator gracefully: {}", err);
                } else {
                    println!("Emulator shutdown command sent successfully");
                }
            }
            Err(e) => {
                eprintln!("Failed to connect to emulator for shutdown: {}", e);
            }
        }

        println!("\n--- Terminating emulator ---");
        emulator_arc
            .kill()
            .await
            .expect("Failed to terminate emulator");
    }
}

#[tokio::test]
#[test_log::test]
async fn test_on_android() {
    let features = std::env::var("ANDROID_TEST_ACTIVITY_FEATURES")
        .expect("ANDROID_TEST_ACTIVITY_FEATURES not set");
    println!("Enabled features: {}", features);
    spawn_device_test(&features, |device| async move {
        // Get device serial
        let device_serial = device.serial().to_string();

        // Get device time before launching activity (for logcat filtering)
        println!("\n--- Getting device time for logcat filtering ---");
        let start_epoch = get_device_epoch_time(&device_serial).expect("Failed to get device time");
        println!("Device epoch time: {}", start_epoch);

        // Start log streaming in a separate task
        println!("\n--- Starting log stream ---");
        let serial_for_logging = device_serial.clone();
        let log_task =
            tokio::task::spawn_blocking(move || stream_logcat(&serial_for_logging, start_epoch));

        // Give log streaming a moment to start
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Launch TestActivity
        println!("\n--- Launching TestActivity ---");
        launch_test_activity(&device_serial).expect("Failed to launch activity");

        // Wait for logs to complete
        println!("\n--- Monitoring logs ---");
        let logs = tokio::time::timeout(Duration::from_secs(35), log_task)
            .await
            .expect("Log streaming timeout")
            .expect("Log task failed")
            .expect("Log streaming error");

        // Print summary of captured logs
        println!("\n--- Log Summary ---");
        println!("Captured {} log lines", logs.len());

        let rust_activity_logs: Vec<_> = logs
            .iter()
            .filter(|log| log.contains("TestActivity"))
            .collect();

        println!("TestActivity logs: {}", rust_activity_logs.len());

        // Verify we got the expected logs
        let has_oncreate = rust_activity_logs
            .iter()
            .any(|log| log.contains("onCreate called"));
        let has_native_message = rust_activity_logs
            .iter()
            .any(|log| log.contains("Native message"));
        let has_native_result = rust_activity_logs
            .iter()
            .any(|log| log.contains("Native result"));
        let has_update_ui = rust_activity_logs
            .iter()
            .any(|log| log.contains("updateUi called"));
        let has_completion = rust_activity_logs
            .iter()
            .any(|log| log.contains("TEST_ACTIVITY_TEST_COMPLETE"));

        assert!(has_oncreate, "Missing onCreate log");
        assert!(has_native_message, "Missing native message log");
        assert!(has_native_result, "Missing native result log");
        assert!(has_update_ui, "Missing updateUi log");
        assert!(has_completion, "Missing test completion marker");

        println!("\nAll expected logs found. Test successful!");

        // Capture screenshot if we have an emulator
        if let Device::Emulator { mut client, .. } = device {
            tokio::time::sleep(Duration::from_secs(5)).await; // Wait a moment for UI to stabilize
            if let Err(e) = capture_screenshot(&mut client, "test_activity_screenshot.png").await {
                eprintln!("Warning: Failed to capture screenshot: {}", e);
            }
        }
    })
    .await;
}
