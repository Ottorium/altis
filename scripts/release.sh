#!/usr/bin/env bash
# Bump the version everywhere and build the release artifacts.
#
#   scripts/release.sh 0.3.0                  # bump + build everything available
#   scripts/release.sh 0.3.0 -t deb,appimage  # only some targets
#   scripts/release.sh -n -t apk              # no bump, just rebuild the APK
#
# Artifacts land in release/<version>/. Nothing is committed, tagged or pushed.
#
# Toolchain locations can be overridden with env vars; see the defaults below.
# The Windows and Android targets are skipped with a warning when their
# toolchain is missing, so this still works on a machine set up for Linux only.
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT=$PWD

ANDROID_HOME=${ANDROID_HOME:-$HOME/Android/Sdk}
NDK_HOME=${NDK_HOME:-}                              # default: newest under $ANDROID_HOME/ndk
ALTIS_BUILD_TOOLS=${ALTIS_BUILD_TOOLS:-}            # default: newest under $ANDROID_HOME/build-tools
ALTIS_KEYSTORE=${ALTIS_KEYSTORE:-$HOME/.android/altis-release.jks}
ALTIS_KEY_ALIAS=${ALTIS_KEY_ALIAS:-altis}
ALTIS_KEYSTORE_PASS_FILE=${ALTIS_KEYSTORE_PASS_FILE:-$HOME/.android/altis-release.password}
ALTIS_FRONTEND_RELEASE=${ALTIS_FRONTEND_RELEASE:-0} # 1 = trunk build --release (smaller, faster wasm)
CACHE=${ALTIS_CACHE:-$ROOT/target/.release-tools}   # scratch for the NSIS/clang shims

say()  { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m warn\033[0m %s\n' "$*" >&2; }
die()  { printf '\033[1;31merror\033[0m %s\n' "$*" >&2; exit 1; }

# ---------------------------------------------------------------- arguments
VERSION=
TARGETS=deb,appimage,exe,apk
BUMP=1
while [ $# -gt 0 ]; do
    case $1 in
        -t|--targets) TARGETS=$2; shift 2 ;;
        -n|--no-bump) BUMP=0; shift ;;
        -h|--help)    sed -n '2,12p' "$0" | sed 's/^# \?//'; exit 0 ;;
        -*)           die "unknown option $1" ;;
        *)            [ -z "$VERSION" ] || die "version given twice"; VERSION=$1; shift ;;
    esac
done

wants() { case ",$TARGETS," in *",$1,"*) return 0 ;; *) return 1 ;; esac; }

if [ "$BUMP" = 1 ]; then
    [ -n "$VERSION" ] || die "no version given (use -n to build without bumping)"
    echo "$VERSION" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$' || die "version must be MAJOR.MINOR.PATCH"
else
    # Reuse whatever is already in the manifest.
    VERSION=$(sed -n 's/^ *"version": *"\(.*\)",*$/\1/p' src-tauri/tauri.conf.json | head -1)
    [ -n "$VERSION" ] || die "could not read current version from src-tauri/tauri.conf.json"
fi

export PATH="$HOME/.cargo/bin:$PATH"
command -v trunk >/dev/null      || die "trunk not found (cargo install trunk)"
command -v cargo-tauri >/dev/null || die "tauri CLI not found (cargo install tauri-cli)"

# ------------------------------------------------------------------- bump
# Rewrites the version in the [package] section of a Cargo.toml.
bump_cargo() {
    sed -i "/^\[package\]/,/^\[/ s/^version = \".*\"/version = \"$VERSION\"/" "$1"
}

if [ "$BUMP" = 1 ]; then
    say "bumping to $VERSION"
    bump_cargo Cargo.toml
    bump_cargo src-tauri/Cargo.toml
    sed -i "0,/\"version\":/ s/\"version\": *\".*\"/\"version\": \"$VERSION\"/" src-tauri/tauri.conf.json
    sed -i "s/^pkgver=.*/pkgver=$VERSION/" README.md
    # Keep Cargo.lock in step with the new package versions.
    cargo update --workspace --offline >/dev/null 2>&1 || cargo update --workspace >/dev/null
fi

OUT=$ROOT/release/$VERSION
mkdir -p "$OUT"

# --------------------------------------------------------------- frontend
# Built once here so the per-target builds can skip beforeBuildCommand and not
# race each other over dist/.
say "building frontend"
if [ "$ALTIS_FRONTEND_RELEASE" = 1 ]; then trunk build --release; else trunk build; fi
NO_FRONTEND='{"build":{"beforeBuildCommand":""}'

# ------------------------------------------------------------------ linux
LINUX_BUNDLES=
wants deb      && LINUX_BUNDLES=deb
wants appimage && LINUX_BUNDLES=${LINUX_BUNDLES:+$LINUX_BUNDLES,}appimage
if [ -n "$LINUX_BUNDLES" ]; then
    say "building $LINUX_BUNDLES"
    cargo tauri build --bundles "$LINUX_BUNDLES" --config "$NO_FRONTEND}"
    wants deb      && cp "target/release/bundle/deb/altis_${VERSION}_amd64.deb" "$OUT/"
    wants appimage && cp "target/release/bundle/appimage/altis_${VERSION}_amd64.AppImage" "$OUT/"
    # PKGBUILD for Arch, kept in sync with the copy in README.md.
    sed -n '/^pkgname=altis$/,/^}$/p' README.md > "$OUT/PKGBUILD"
fi

# ---------------------------------------------------------------- windows
# On a Linux host this cross-builds via cargo-xwin. NSIS, lld-link and clang-cl
# are put on PATH from $CACHE/bin; makensis needs a wrapper because Tauri does
# not pass NSISDIR through to it.
setup_windows_toolchain() {
    rustup target list --installed | grep -q x86_64-pc-windows-msvc \
        || { warn "rust target x86_64-pc-windows-msvc not installed"; return 1; }
    command -v cargo-xwin >/dev/null || { warn "cargo-xwin not installed"; return 1; }

    mkdir -p "$CACHE/bin"

    if ! command -v makensis >/dev/null; then
        if [ ! -x "$CACHE/nsis/root/usr/bin/makensis" ]; then
            command -v apt-get >/dev/null || { warn "makensis not found and no apt-get to fetch it"; return 1; }
            say "fetching NSIS into $CACHE"
            mkdir -p "$CACHE/nsis"
            ( cd "$CACHE/nsis" && apt-get download nsis nsis-common >/dev/null \
              && for d in *.deb; do dpkg-deb -x "$d" root; done ) || { warn "could not unpack NSIS"; return 1; }
        fi
        cat > "$CACHE/bin/makensis" <<EOF
#!/bin/sh
export NSISDIR="$CACHE/nsis/root/usr/share/nsis"
exec "$CACHE/nsis/root/usr/bin/makensis" "\$@"
EOF
        chmod +x "$CACHE/bin/makensis"
    fi

    # llvm-lib and llvm-rc are often only in /usr/lib/llvm-<ver>/bin.
    if ! command -v llvm-lib >/dev/null; then
        local llvmdir
        llvmdir=$(ls -d /usr/lib/llvm-*/bin 2>/dev/null | sort -V | tail -1) || true
        [ -n "${llvmdir:-}" ] && [ -x "$llvmdir/llvm-lib" ] \
            || { warn "llvm-lib not found (install llvm)"; return 1; }
        PATH="$llvmdir:$PATH"
    fi
    if ! command -v clang-cl >/dev/null; then
        local clang
        clang=$(command -v clang || ls /usr/lib/llvm-*/bin/clang 2>/dev/null | sort -V | tail -1) || true
        [ -n "${clang:-}" ] || { warn "clang not found"; return 1; }
        ln -sf "$clang" "$CACHE/bin/clang-cl"
    fi
    if ! command -v lld-link >/dev/null; then
        local rustlld="$(rustc --print sysroot)/lib/rustlib/x86_64-unknown-linux-gnu/bin/rust-lld"
        [ -x "$rustlld" ] || { warn "lld-link not found and rust-lld missing"; return 1; }
        ln -sf "$rustlld" "$CACHE/bin/lld-link"
    fi

    export PATH="$CACHE/bin:$PATH"
}

if wants exe; then
    if [ "$(uname -s)" = Linux ]; then
        if setup_windows_toolchain; then
            say "cross-building windows installer"
            # --bundles nsis is rejected on a non-Windows host, so it goes via --config.
            CARGO_TARGET_DIR=$ROOT/target/windows \
            cargo tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc \
                --config "$NO_FRONTEND,\"bundle\":{\"targets\":[\"nsis\"]}}"
            cp "target/windows/x86_64-pc-windows-msvc/release/bundle/nsis/altis_${VERSION}_x64-setup.exe" "$OUT/"
        else
            warn "skipping windows installer"
        fi
    else
        say "building windows installer"
        cargo tauri build --bundles nsis --config "$NO_FRONTEND}"
        cp "target/release/bundle/nsis/altis_${VERSION}_x64-setup.exe" "$OUT/"
    fi
fi

# ---------------------------------------------------------------- android
if wants apk; then
    [ -n "$NDK_HOME" ] || NDK_HOME=$(ls -d "$ANDROID_HOME"/ndk/* 2>/dev/null | sort -V | tail -1) || true
    [ -n "$ALTIS_BUILD_TOOLS" ] || ALTIS_BUILD_TOOLS=$(ls -d "$ANDROID_HOME"/build-tools/* 2>/dev/null | sort -V | tail -1) || true

    if [ ! -d "$ANDROID_HOME" ] || [ -z "${NDK_HOME:-}" ]; then
        warn "skipping APK: android SDK/NDK not found (set ANDROID_HOME / NDK_HOME)"
    else
        say "building APK"
        ANDROID_HOME=$ANDROID_HOME NDK_HOME=$NDK_HOME \
            cargo tauri android build --apk --target aarch64 --target armv7 \
            --config "$NO_FRONTEND}"

        UNSIGNED=src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk
        [ -f "$UNSIGNED" ] || die "expected APK at $UNSIGNED"

        # Every release must be signed with the same key or updates will not
        # install over an older version.
        if [ ! -f "$ALTIS_KEYSTORE" ]; then
            warn "no keystore at $ALTIS_KEYSTORE - leaving the APK unsigned"
            warn "  keytool -genkeypair -keystore $ALTIS_KEYSTORE -alias $ALTIS_KEY_ALIAS -keyalg RSA -keysize 4096 -validity 10000"
            cp "$UNSIGNED" "$OUT/altis_${VERSION}-unsigned.apk"
        else
            [ -f "$ALTIS_KEYSTORE_PASS_FILE" ] \
                || die "keystore password file $ALTIS_KEYSTORE_PASS_FILE not found (set ALTIS_KEYSTORE_PASS_FILE)"
            say "signing APK"
            "$ALTIS_BUILD_TOOLS/zipalign" -p -f 4 "$UNSIGNED" "$CACHE/aligned.apk"
            # Only --ks-pass: the key password is the same, and passing the file
            # twice makes apksigner read a second line from it.
            "$ALTIS_BUILD_TOOLS/apksigner" sign \
                --ks "$ALTIS_KEYSTORE" --ks-key-alias "$ALTIS_KEY_ALIAS" \
                --ks-pass "file:$ALTIS_KEYSTORE_PASS_FILE" \
                --out "$OUT/altis_${VERSION}.apk" "$CACHE/aligned.apk"
            rm -f "$OUT/altis_${VERSION}.apk.idsig" "$CACHE/aligned.apk"
            "$ALTIS_BUILD_TOOLS/apksigner" verify --print-certs "$OUT/altis_${VERSION}.apk" | head -2
        fi
    fi
fi

say "artifacts in release/$VERSION"
ls -lh "$OUT"
[ "$BUMP" = 1 ] && say "version bumped but not committed - review with 'git diff'"
exit 0
