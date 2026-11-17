# DJI Mic Control

**Website:** [miccontrol.app](https://miccontrol.app/en/) — download, setup guides for
[Mac](https://miccontrol.app/en/dji-mic-3-mac/), [Windows](https://miccontrol.app/en/dji-mic-3-windows/)
and [Linux](https://miccontrol.app/en/dji-mic-3-linux/), and FAQ in 6 languages.

Control your DJI wireless microphones from the computer over USB — no phone,
no DJI Mimo app. Plug the receiver in and every setting, live levels, mute,
internal recording and battery status are right there.

This is a fork of [ShadowBitBasher/DJI-Mic-Control](https://github.com/ShadowBitBasher/DJI-Mic-Control)
that adds **DJI Mic 3** support, a redesigned interface and six languages.

![DJI Mic Control — dark theme](docs/screenshots/main-dark.png)

<table>
  <tr>
    <td><img alt="Light theme, one transmitter recording and one muted" src="docs/screenshots/main-light.png"></td>
    <td><img alt="Spanish UI with the device switcher open and the Outdoor scene applied" src="docs/screenshots/scenes-es.png"></td>
  </tr>
  <tr>
    <td align="center"><sub>Light theme · TX1 recording, TX2 muted</sub></td>
    <td align="center"><sub>Spanish · device switcher and scene presets</sub></td>
  </tr>
</table>

## Download

Latest release: **[all files and notes](https://github.com/qubik89/DJIMic3/releases/latest)**

| Platform | Download | Status |
| --- | --- | --- |
| macOS (Apple Silicon) | [DJI-Mic-Control_macOS_arm64.dmg](https://github.com/qubik89/DJIMic3/releases/latest/download/DJI-Mic-Control_macOS_arm64.dmg) | Tested with a DJI Mic 3 |
| Windows 10/11 (x64) | [Installer](https://github.com/qubik89/DJIMic3/releases/latest/download/DJI-Mic-Control_Windows_x64_setup.exe) · [portable .exe](https://github.com/qubik89/DJIMic3/releases/latest/download/DJI-Mic-Control_Windows_x64_portable.exe) | Built automatically, not yet tested on hardware |
| Linux (x64) | [.deb](https://github.com/qubik89/DJIMic3/releases/latest/download/DJI-Mic-Control_Linux_amd64.deb) · [.rpm](https://github.com/qubik89/DJIMic3/releases/latest/download/DJI-Mic-Control_Linux_x86_64.rpm) · [AppImage](https://github.com/qubik89/DJIMic3/releases/latest/download/DJI-Mic-Control_Linux_amd64.AppImage) | Built automatically, not yet tested on hardware |
| Android 7+ (arm64) | APK, [built from source](#android) for now | Preview, not yet tested on hardware |

Checksums are in `SHA256SUMS.txt` on each release.

**macOS**

1. Open the `.dmg` and drag **DJI Mic Control** to **Applications**.
2. The app isn't signed with an Apple developer account, so macOS may say it
   is "damaged". Clear the quarantine flag once:
   ```bash
   sudo xattr -rd com.apple.quarantine "/Applications/DJI Mic Control.app"
   ```
3. Connect the receiver with a USB-C cable and open the app.

**Windows**: install, then switch the receiver's control interface to WinUSB
with Zadig (see [Platform notes](#platform-notes)). Windows SmartScreen may
warn because the installer isn't signed: *More info → Run anyway*.

**Linux**: the `.deb` and `.rpm` install the USB permission rule for you; with
the AppImage, install the rule by hand (see [Platform notes](#platform-notes)),
`chmod +x` the file and run it.

**Android**: install the APK (allow installing from unknown sources when asked),
then plug the receiver into the phone's USB-C port. See [Android](#android).

## Features

- **Live channel strips** per transmitter: segmented level meter with peak hold
  and a clip indicator, Mute and Record buttons with a running timer, voice
  tone and input gain.
- **Receiver controls**: channel mode (mono / stereo / 4-channel), output gain,
  headphone volume, lossless audio, battery and indoor/outdoor environment.
- **Quick actions**: mute or record on every transmitter at once.
- **Scenes**: Interview, Outdoor, Vlog and Podcast presets that change several
  settings in one click (only what differs is sent).
- **All the settings**, grouped in tabs with an explanation under each one:
  noise cancelling, low cut, clipping control, loudness balance, auto-off,
  camera power sync, LEDs, vibration, 32-bit float, loop recording, automatic
  recording triggers…
- **Battery** for the receiver and every transmitter (≈ percentage), plus the
  internal recording time left on each transmitter.
- **Clock sync**: the transmitters' clock is set on connect so internal
  recordings get correct timestamps.
- **Dark and light themes**, following the system by default.
- **Languages**: English, Español (España), Français, Italiano, Deutsch and
  Nederlands. The app starts in the system language and can be changed from
  the globe button; the tray and macOS menus follow it.
- **Menu bar / tray** shortcuts for LEDs and noise cancelling, with a
  low-battery warning.
- **Command line tool** (`djimic`) for scripting.

## Supported devices

The app identifies the receiver when it connects (several share a USB id) and
shows only the controls that receiver and each connected transmitter support.

| Receiver | USB id | Status |
| --- | --- | --- |
| **DJI Mic 3** | `2ca3:4015` (`4115` in 4-channel mode) | Full control, **tested** with firmware 03.01.00.10 and two transmitters |
| **DJI Mic Mini** | `2ca3:4011` | Supported by the original project (v1 and v2 firmware) |
| **DJI Mic Series Mobile Receiver** | `2ca3:4011` | Receiver gain, headphone volume, read-only noise cancelling (set with its O/L/H switch). Based on hardware-tested work by others; not tested here |
| **DJI Mic Mini 2S** | `2ca3:4015` / `4115` | Same protocol as the Mic 3, plus Safety Track, automatic recording modes and dual-file recording. Based on [dji-mic-mo](https://github.com/usokawa/dji-mic-mo) and [lavboard](https://github.com/kroqdotdev/lavboard); not tested here |

| Transmitter | What you can control from the app |
| --- | --- |
| DJI Mic 3 | Mute, internal recording, voice tone, input gain, 32-bit float, loop and automatic recording |
| DJI Mic Mini 2S | Same as the Mic 3 |
| DJI Mic 2 (through a Mic Mini or Mobile receiver) | Mute, internal recording, 32-bit float, vibration, record on power-on, REC button; validated by [DJI-Mic-Utility](https://github.com/nicolasmmb/DJI-Mic-Utility) |
| DJI Mic Mini 2 | Voice tone |
| DJI Mic Mini | Basic settings |

Not supported: the **DJI Mic 2's own receiver** and the original **DJI Mic**
(2021) — no USB control interface has been documented for them. Not
available on the Mic 3 because the hardware doesn't offer it: Safety Track.
Not yet implemented: timecode, monitor source selection, formatting the
transmitters' storage, and changing the Mic 3's indoor/outdoor environment
(it's shown, but has to be changed on the receiver).

Untested combinations only send commands that other projects have confirmed
on real hardware, and the device's answer is checked: a rejected setting
shows an error instead of failing silently.

Battery percentages are estimates: the microphones only report seven battery
levels. Level-meter dB values are approximate too.

## Command line

```bash
cargo run -p cli -- list
cargo run -p cli -- status
cargo run -p cli -- set noise-cancel strong
cargo run -p cli -- set --tx 1 voice-tone rich
cargo run -p cli -- set lossless on
```

`djimic status` shows every setting and its current value; the setting ids are
defined in `crates/protocol/src/models/`.

## Platform notes

**macOS** works out of the box (see [Download](#download)).

**Linux** needs a udev rule for non-root USB access. The `.deb`/`.rpm` install
it; otherwise:

```bash
sudo cp packaging/60-dji-mic.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules && sudo udevadm trigger
```

Then replug the receiver. The app shows these steps if it can't access the
device.

**Windows** binds the receiver's control interface to its own driver, so it has
to be switched to WinUSB with [Zadig](https://zadig.akeo.ie/): interface 6 on
the Mic Mini, interface 4 (`com.dji.mic`) on the Mic 3 (untested on Windows).
Replug the receiver afterwards.

<img alt="Zadig set to WinUSB for the DJI Mic Mini's HID interface" src="zadig.png" width="49%">

### Android

The same app runs on Android phones and tablets with USB host (OTG) support,
which almost every phone with a USB-C port has. Plug the receiver in: Android
asks whether to let DJI Mic Control access it — tick *Always open DJI Mic
Control* and the app will open by itself the next time you plug it in. The app
only uses the receiver's separate control interface, so it should keep working
as the phone's microphone at the same time.

This is useful for the **DJI Mic Series Mobile Receiver**, which is made to plug
into a phone, and for the Mic 3 and Mini 2S receivers.

It is a preview: it builds and runs, but hasn't been tested with a receiver
plugged into a phone yet. iOS isn't possible: iPhones only let apps talk to
USB accessories certified through Apple's MFi programme.

## Build from source

Requirements: Rust 1.77+ and Node.js 18+ (Linux also needs the WebKitGTK/GTK
development packages Tauri requires).

```bash
./build-release.sh            # macOS/Linux → Release/<os>/ (.app + .dmg, or .deb/.rpm/.AppImage)
./build-release.ps1           # Windows
```

On macOS, `build-release.sh` can also sign the app with a Developer ID
certificate and notarize it with Apple: see
[docs/macos-signing.md](docs/macos-signing.md).

For development, from `gui/`:

```bash
npm install
npm run tauri dev
```

Always launch the GUI through `tauri dev`; a bare debug binary loads its
frontend from the dev server and shows a blank window. Run the tests with
`cargo test`.

**Android** needs the Android SDK and NDK (Android Studio installs both), JDK
17–21 and a rustup toolchain with the Android target:

```bash
rustup target add aarch64-linux-android
export ANDROID_HOME="$HOME/Library/Android/sdk"   # Linux: ~/Android/Sdk
export NDK_HOME="$ANDROID_HOME/ndk/<version>"
cd gui
npx tauri android build --apk --target aarch64
```

The APK ends up in
`gui/src-tauri/gen/android/app/build/outputs/apk/universal/release/`. To sign
it with your own key (needed for Google Play), create
`gui/src-tauri/gen/android/keystore.properties` with `storeFile`, `keyAlias`
and `password`; it's git-ignored. Without it the APK is signed with the local
debug key, which is fine for installing it yourself.

The release script sets path-remapping `RUSTFLAGS` so binaries don't embed the
build machine's home directory, and checks the result.

## Project layout

```
crates/protocol   Framing, CRC, commands, status decoding, per-model settings
crates/device     USB transport (nusb), command ACK/retry, multi-device manager
crates/cli        djimic command-line tool
gui               Tauri v2 app (Rust backend in src-tauri, Svelte 5 frontend)
gui/src-tauri/gen/android
                  Android project; UsbPlugin.kt asks for USB access and hands
                  the receiver to the Rust side
packaging         Linux udev rule shipped in the .deb/.rpm
docs/screenshots  README images
```

Adding a setting means editing one model definition in `crates/protocol`; the
CLI and GUI pick it up automatically. The wire protocol, including everything
learned about the Mic 3, is documented in [PROTOCOL.md](PROTOCOL.md).

## Credits

- [ShadowBitBasher/DJI-Mic-Control](https://github.com/ShadowBitBasher/DJI-Mic-Control):
  the original app, protocol work and Mic Mini support.
- [SanMurakami/mic3-bridge](https://github.com/SanMurakami/mic3-bridge),
  [usokawa/dji-mic-mo](https://github.com/usokawa/dji-mic-mo),
  [kroqdotdev/lavboard](https://github.com/kroqdotdev/lavboard),
  [piedol/lavdeck](https://github.com/piedol/lavdeck),
  [nicolasmmb/DJI-Mic-Utility](https://github.com/nicolasmmb/DJI-Mic-Utility) and
  the Mobile Receiver proposal in the upstream repository (PR #4): protocol
  details for the Mic 3, Mini 2S, Mobile Receiver and Mic 2 transmitters.
- Translation structure based on the Simplified Chinese proposal in the
  upstream repository (PR #3).

Not affiliated with or endorsed by DJI. DJI and DJI Mic are trademarks of SZ
DJI Technology Co., Ltd.; product images are used to identify the devices.

## License

Released into the public domain under [The Unlicense](LICENSE). Third-party
dependencies keep their own licenses; their notices are included in
distributed binaries.
