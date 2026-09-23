# Altis - An Alternate Untis Client

A WebUntis client for desktop and Android that does the things the official one makes hard:
a timetable you can actually read, notifications that arrive on their own, and a view of your
absences you can act on. Written in Rust - [Yew](https://yew.rs) in WebAssembly for the UI,
[Tauri](https://tauri.app) around it, and no server of its own: the app talks to WebUntis
directly with your credentials.

Downloads for Linux, Windows and Android are on the
[releases page](https://github.com/Ottorium/altis/releases). The app checks once a day
whether a newer one is out and says so in a banner.

## Setting it up

Untis is signed into with a **school**, a **username** and a **secret** - not your password.
The secret is the one WebUntis hands out for other apps: profile → *Data access* →
*Display secret key*. Altis turns it into a one-time code for every login, the same way the
official app does.

The canteen (Book2Eat) is separate and optional: a canteen ID, a mail address and a password.

Settings can be moved to another device from the Settings screen, as a QR code to scan or a
JSON file to open - with or without the credentials in them.

## What it does

### Timetable

The week for **you**, or for any class, teacher or room, picked from the two dropdowns.
**Available Rooms** turns the same data inside out and shows which rooms are free in each
period, which is the one question the official client cannot answer at all.

Move between weeks with the arrows, the date picker or a swipe. Cancelled, moved and
substituted lessons are marked as such, exams stand out, and a red line marks the current
time. Tapping a block opens its details; tapping a teacher, class or room in there jumps
straight to that timetable.

Loaded weeks are cached for an hour, so paging back and forth is instant and does not hammer
Untis.

The Visual Settings shape what the grid looks like:

- **Always Shown Time Range** draws every day over at least the times you set, even where no
  lessons are, so a short day doesn't stretch its lessons over the whole screen and the grid
  keeps its scale from week to week. Lessons outside the range still show and widen it.
- **Weekday Override** decides whether empty weekdays are shown at all, and which ones.
- **Subject Colour Overrides** give a subject a colour of your own.
- **Force ASCII Timetable** renders the week as text, for when that is what you want.

### Absences

Every absence of the school year with its excuse status, newest first, and a count of the
ones nobody has excused yet. Where the school allows it you can report an absence yourself,
change one you reported, and withdraw it again; where it doesn't, Untis' own refusal is shown
rather than a generic error. The excuse note - the PDF the school wants signed - downloads
from the same screen, and the year picker reaches back into previous ones.

### Messages

The Untis inbox with unread marked, and attachments that download to wherever you point them.

### Canteen

The week's menu with the QR code the canteen scans, a swipe from day to day.

## Notifications

Altis polls Untis for timetable changes, upcoming exams, new messages and new absences, and
shows a native notification for each. Every poll looks at the current **and** the next week,
so a lesson dropped on Friday for the Monday after still gets noticed. The interval, and
which kinds of notification you want, are set in Settings.

Absences you report yourself stay quiet - you typed them in a minute ago. What is worth
hearing about is a teacher marking you absent.

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

## How the code is laid out

- `core/` - everything Untis that isn't tied to a browser: the client, the data models, the
  notification poll, the settings. It must stay free of `web-sys`/`js-sys`/`wasm-bindgen`.
  Whatever it needs from a platform - a key-value store, an HTTP client that CORS can't stop,
  the clock, a notification - it asks for through the `Env` trait. That is what lets the same
  poll run in the webview on desktop and natively in the Android service.
- `src/` - the Yew frontend, which binds `Env` to localStorage and the Tauri `proxy` command.
- `src-tauri/` - the shell: the `proxy`, file and store commands, the tray, and the Android
  background poller that binds `Env` to a JSON file and reqwest.

Two notes on talking to WebUntis, both of which cost an afternoon to find out:

- The timetable and message APIs take a bearer token, but the older `classreg` API the
  absences live on only knows the session cookie.
- Writing through that API additionally needs the session's CSRF token, which WebUntis ships
  in the config its pages bootstrap from rather than handing it out through an endpoint.
  Without it, a write is answered with the login page.

## Building

You need Rust with the `wasm32-unknown-unknown` target, [Trunk](https://trunkrs.dev) and the
Tauri CLI (`cargo install trunk tauri-cli`). You also need the
[Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your platform.

**Debug builds:**
- `cargo tauri dev` runs the app with live reload.
- `cargo tauri android dev` does the same on a connected device or emulator.
- Add `--debug` to any build command below to get debug bundles.

### All at once

`scripts/release.sh 1.0.0` bumps the version in `Cargo.toml`, `src-tauri/Cargo.toml`,
`src-tauri/tauri.conf.json` and the `PKGBUILD` below, builds every bundle it has the
toolchain for, and collects them in `release/1.0.0/`. It does not commit, tag or push.

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
pkgver=1.0.0
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

## License

GPL-3.0-only. See [LICENSE](LICENSE).
