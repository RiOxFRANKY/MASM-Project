# Android build using the MASM game engine

This Android app runs the repository's `Code/MAZE.ASM` inside the Rust 8086/DOS emulator. The APK bundles the MASM source, starts the emulator, assembles and links the maze inside DOS, then runs it. The Android controls send the same BIOS keyboard scan codes as a physical keyboard, so maze generation, movement, hints, and match rules remain in MASM.

The native Android view reads the emulator's 80×25 text framebuffer and renders its VGA foreground/background colors. Touch arrows, **HINT**, **MENU**, **ENTER**, **1 PLAYER**, and **2 PLAYERS** map to DOS keys. In a two-player game, tap the D-pad center to switch its arrows between Player 1's WASD keys and Player 2's arrow keys. Portrait phones rotate to landscape so the full maze screen and touch pad remain usable.

## Build the APK

Run from the project root on macOS:

```sh
./mobile/build-apk.sh
```

The script installs Android command-line tools with Homebrew if needed, downloads Android SDK platform 35 and NDK 27.2, installs the Rust Android target and `cargo-ndk`, builds the emulator for 64-bit ARM Android, then creates `RandomMaze.apk` in the project root. The first run may ask you to accept Android SDK licenses. The Android SDK path can be overridden with `ANDROID_HOME` or `ANDROID_SDK_ROOT`.

The resulting debug APK is installable on ARM64 Android devices. To install it over USB after enabling developer options, run `adb install -r RandomMaze.apk`.
