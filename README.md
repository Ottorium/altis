# Altis - An Alternate Untis Client

## How to build (ai generated)

You need Rust with the `wasm32-unknown-unknown` target, [Trunk](https://trunkrs.dev) and the Tauri CLI (`cargo install trunk tauri-cli`). You also need the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your platform.

**Debug builds:**
- `cargo tauri dev` runs the app with live reload.
- `cargo tauri android dev` does the same on a connected device or emulator.
- Add `--debug` to any build command below to get debug bundles.

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
