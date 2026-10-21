# foodshare-device

Device discovery for the app: the iOS simulators and the Android
emulators.

The two tools that own the devices — `xcrun simctl` and `adb` — report
them in different shapes, and this crate is the one place that knows
both. The app tools drive a device; the end-to-end runner picks one to
run on. Neither of them should be parsing the output.

A tool that is not installed is not an error: it means that platform has
no devices.

## Driving a device

The crate owns the commands as well as the discovery, so the tools that
own the devices are known in one place.

```rust,no_run
use foodshare_device::{boot_simulator, install_app, launch_app};

boot_simulator("ABC-123")?;
install_app("ABC-123", "/path/to/FoodShare.app")?;
launch_app("ABC-123", "com.flutterflow.foodshare")?;
```

## Simulators

```rust,no_run
use foodshare_device::{simulators, DeviceState};

for simulator in simulators()? {
    if simulator.state == DeviceState::Booted {
        println!("{} — {}", simulator.name, simulator.udid);
    }
}
```

## Emulators

```rust,no_run
use foodshare_device::{emulators, DeviceState};

for emulator in emulators()? {
    if emulator.state == DeviceState::Booted {
        println!("{} — {:?}", emulator.model, emulator.serial);
    }
}
```

## States

The tools use their own wording: `simctl` says `Booted` and `Shutdown`,
and `adb` says `device` and `offline`. The crate maps both onto one
enum, so a caller does not have to know which tool produced the line.
