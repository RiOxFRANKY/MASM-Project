#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MOBILE="$ROOT/mobile"
if [[ -n "${ANDROID_SDK_ROOT:-}" ]]; then
  SDK="$ANDROID_SDK_ROOT"
elif [[ -n "${ANDROID_HOME:-}" ]]; then
  SDK="$ANDROID_HOME"
elif [[ -d /opt/homebrew/share/android-commandlinetools ]]; then
  SDK="/opt/homebrew/share/android-commandlinetools"
else
  SDK="$HOME/Library/Android/sdk"
fi
NDK_VERSION="27.2.12479018"

export ANDROID_HOME="$SDK"
export ANDROID_SDK_ROOT="$SDK"

if ! command -v brew >/dev/null 2>&1; then
  echo "Homebrew is required to install the Android command-line tools and Gradle on macOS." >&2
  exit 1
fi
if ! command -v sdkmanager >/dev/null 2>&1; then
  brew install --cask android-commandlinetools
  export PATH="/opt/homebrew/share/android-commandlinetools/cmdline-tools/latest/bin:$PATH"
fi
if ! command -v rustup >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --no-modify-path
fi
source "$HOME/.cargo/env"

sdkmanager --sdk_root="$SDK" --install "platforms;android-35" "build-tools;35.0.0" "ndk;$NDK_VERSION"
rustup target add aarch64-linux-android
if ! cargo ndk --version >/dev/null 2>&1; then
  cargo install cargo-ndk --locked
fi

mkdir -p "$MOBILE/app/src/main/jniLibs/arm64-v8a" "$MOBILE/app/src/main/assets/game"
cp "$ROOT/Code/MAZE.ASM" "$MOBILE/app/src/main/assets/game/MAZE.ASM"
(cd "$ROOT/dosbox" && cargo ndk -t arm64-v8a -P 26 -o "$MOBILE/app/src/main/jniLibs" build --lib --release)

cd "$MOBILE"
./gradlew --no-daemon assembleDebug
cp app/build/outputs/apk/debug/app-debug.apk "$ROOT/RandomMaze.apk"
echo "APK created at $ROOT/RandomMaze.apk"
