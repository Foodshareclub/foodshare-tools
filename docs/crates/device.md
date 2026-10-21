# foodshare-device

Device discovery for the app: the iOS simulators and the Android
emulators.

The two tools that own the devices — `xcrun simctl` and `adb` — report
them in different shapes, and this crate is the one place that knows
both. The app tools drive a device; the end-to-end runner picks one to
run on. Neither of them should be parsing the output.

A tool that is not installed is not an error: it means that platform has
no devices.

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
