# Altis - An Alternate Untis Client

## Notifications

Altis polls Untis for timetable changes, upcoming exams and new messages, and shows a
native notification for each. Every poll looks at the current **and** the next week, so a
lesson dropped on Friday for the Monday after still gets noticed. The interval, and which
of the three kinds of notification you want, are set in Settings.

The poll itself lives in `core/` (shared code, `altis_core::notifications`). What differs
per platform is who runs it:

- **Desktop:** the frontend's WASM runs it in the webview. Closing the window only hides
  it to the tray, so this keeps working; quitting from the tray menu stops it. The app is
  not started on login, so nothing is checked until you open it again.
- **Android:** the webview is gone the moment the app is closed, so the poll runs natively
  instead. Two ways to do that, chosen under "When the app is closed" in Settings:

  - **Check about every 15 minutes** (the default) hands the schedule to WorkManager. No
    permanent notification, nothing kept running in between - Android starts the process
    when it is time. 15 minutes is WorkManager's hard floor and it is a floor rather than a
    promise: the system batches jobs and Doze stretches the gap while the phone is idle, so
    expect longer. Exam reminders are only ever as punctual as the poll that finds them.
  - **Check on the interval above** runs `PollService`, a foreground service that keeps to
    the configured interval. Exact, and the price is the ongoing notification Android
    demands for a process that survives the app being closed. Since Android 14 the user can
    swipe that notification away; the service keeps running regardless.

  The service runs in **its own process** (`android:process=":poller"`), and that is not a
  detail to undo: `tao` ends its Android event loop with `std::process::exit()`, so closing
  the app tears down the whole process it runs in. A service sharing that process would be
  killed along with it, taking its notification with it. (The WorkManager job has no such
  problem - it runs in the default process, which by then has no activity in it.)

  Because the processes share no memory, they talk through two files in the app's data dir -
  the app writes the settings to `synced_from_app.json` (the `sync_store` command) and the
  poller reads them; the poller owns `poller_state.json` for its own Untis session and
  bookkeeping. Neither file is ever written by both.

## How to build (ai generated)

You need Rust with the `wasm32-unknown-unknown` target, [Trunk](https://trunkrs.dev) and the Tauri CLI (`cargo install trunk tauri-cli`). You also need the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your platform.

**Debug builds:**
- `cargo tauri dev` runs the app with live reload.
- `cargo tauri android dev` does the same on a connected device or emulator.
- Add `--debug` to any build command below to get debug bundles.

### All at once

`scripts/release.sh 0.3.0` bumps the version in `Cargo.toml`, `src-tauri/Cargo.toml`,
`src-tauri/tauri.conf.json` and the `PKGBUILD` below, builds every bundle it has the
toolchain for, and collects them in `release/0.3.0/`. It does not commit, tag or push.

- `-t deb,appimage` builds only some of `deb`, `appimage`, `exe`, `apk`.
- `-n` skips the bump and rebuilds the version that is already in the manifests.
- Targets whose toolchain is missing are skipped with a warning, so a machine set up
  for Linux only still gets the .deb and AppImage.
- `ALTIS_FRONTEND_RELEASE=1` builds the frontend with `trunk build --release`. The
  default is a debug wasm, which is several times larger and slower.

Paths it guesses can be overridden: `ANDROID_HOME`, `NDK_HOME`, `ALTIS_BUILD_TOOLS`,
`ALTIS_KEYSTORE`, `ALTIS_KEY_ALIAS`, `ALTIS_KEYSTORE_PASS_FILE`.

The sections below describe what the script does for each target, in case you want to
run one by hand.

### Linux (.deb, AppImage)

```sh
cargo tauri build --bundles deb,appimage
```

The bundles are written to `target/release/bundle/`.

### Arch Linux

Either run the AppImage, or turn the .deb into a pacman package. For the package, put this `PKGBUILD` next to the .deb and run `makepkg -si` there:

```sh
pkgname=altis
pkgver=0.2.0
pkgrel=1
pkgdesc="An alternate Untis client"
arch=('x86_64')
license=('GPL-3.0-only')
depends=('webkit2gtk-4.1' 'gtk3')
options=('!strip' '!debug')
source=("altis_${pkgver}_amd64.deb")
sha256sums=('SKIP')

package() {
    bsdtar -xf data.tar.* -C "$pkgdir"
}
```

### Windows (.exe installer)

On Windows, run `cargo tauri build --bundles nsis`.

To cross-compile from Linux, you need NSIS, LLVM and lld (e.g. `sudo apt install nsis llvm lld`). Then run:

```sh
rustup target add x86_64-pc-windows-msvc
cargo install --locked cargo-xwin
cargo tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc --config '{"bundle":{"targets":["nsis"]}}'
```

`--bundles nsis` is rejected on non-Windows hosts, which is why the bundle target is set with `--config`. `llvm-lib` and `llvm-rc` must be on your `PATH`. Some distros only install them to `/usr/lib/llvm-<version>/bin`.

The installer is written to `target/x86_64-pc-windows-msvc/release/bundle/nsis/`.

### Android (.apk)

Note that a few files under `src-tauri/gen/android` are hand-written and checked in
(`MainActivity.kt`, `BackgroundPoller.kt`, `PollService.kt`, `PollWorker.kt`,
`BootReceiver.kt`, `AndroidManifest.xml`, `res/drawable/ic_notification.xml`, `altis.pro`
and `app/build.gradle.kts`, which carries the WorkManager dependency) - everything else
there is generated and ignored. `tauri android init` will not put them back, so don't
regenerate that directory without restoring them from git afterwards.

You need the Android SDK and NDK, with `ANDROID_HOME` and `NDK_HOME` set. You also need the Rust Android targets (`rustup target add aarch64-linux-android armv7-linux-androideabi`).

```sh
cargo tauri android build --apk --target aarch64 --target armv7
```

The release APK is unsigned. Create a keystore once:

```sh
keytool -genkeypair -keystore release.jks -alias altis -keyalg RSA -keysize 4096 -validity 10000
```

Always sign with that same keystore, otherwise updates won't install over older versions:

```sh
APK=src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk
zipalign -p -f 4 "$APK" aligned.apk
apksigner sign --ks release.jks --ks-key-alias altis --out altis.apk aligned.apk
```

`zipalign` and `apksigner` are in `$ANDROID_HOME/build-tools/<version>/`.
