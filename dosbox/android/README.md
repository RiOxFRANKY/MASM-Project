# DOSBox-RS for Android

An Android app that runs the DOSBox-RS 8086 emulator on the phone itself.

- You get a real `C:\>` prompt with the built-in **MASM** and **LINK**.
- It runs DOS `.EXE` and `.COM` files, plus `.BAT` files.
- It contains no third-party libraries: the emulator core is the same Rust code as the desktop version, compiled for the phone's CPU.

## Install

1. Copy `app-release.apk` (from `app/build/outputs/apk/release/`) to the phone.
2. Open it and allow "install unknown apps" when Android asks.
3. Requires Android 8.0 or newer. The APK contains builds for ARM 64-bit, ARM 32-bit and x86_64.

## Using it

| Part of the screen | What it does |
|---|---|
| DOS screen | 80 × 25 text screen. Tap it to open the phone keyboard. |
| Extra-keys bar | `ESC`, `TAB`, `CTRL` (sticky: tap it, then a letter), the arrows, `↵` (Enter), `⌨` (keyboard on/off) and `🎮` (gamepad on/off) |
| Gamepad | Two D-pads for two players on one phone: P1 = W A S D (left), P2 = arrow keys (right). The middle buttons are ESC, ENTER, H, 1 and 2. |
| Menu (⋮) | Keyboard, Gamepad, **Import files**, **Export a file**, Restart DOS |

- **CTRL + C** stops the running DOS program.
- Typing `EXIT` restarts DOS.
- A Bluetooth or USB keyboard also works, including the arrow keys, F1–F12 and Ctrl.

### Running your own programs

1. Put the `.EXE` (or `.COM` / `.ASM`) file anywhere on the phone, for example in Downloads.
2. In the app, open the menu, choose **Import files**, and pick one or more files. They are copied to `C:\`, with names turned into DOS 8.3 form (`my game.exe` → `MY_GAME.EXE`).
3. Type the program's name at the prompt, e.g. `MY_GAME`.

### The maze game

On the first start the app copies `MAZE.ASM` to `C:\` and assembles it for you (`masm MAZE.ASM;` then `link MAZE.OBJ;`). Then type `MAZE`:

- **1 player vs the computer:** use the arrows on the extra-keys bar. The A\* bot races you in the right-hand maze. You see its whole maze, but it only knows the squares it has seen near itself.
- **2 players:** open the gamepad with `🎮`. One player uses the left pad and the other the right pad.

Landscape gives the biggest screen.

### Where drive C: lives

`C:\` is the folder `Android/data/com.rioxfranky.dosbox/files/C` on the phone's storage. You can also reach it over USB from a PC.

Use **Export a file** to save something from `C:\` (for example an `.EXE` you built with MASM) anywhere on the phone.

## Building the APK

Requirements:

| Tool | Notes |
|---|---|
| Android SDK with platform 36 and NDK `27.1.12297006` | e.g. installed through Android Studio |
| JDK 17 or newer | Android Studio's bundled `jbr` works |
| Rust with the Android targets | `rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android` |

From this folder:

```
gradlew assembleRelease
```

On Windows, set `JAVA_HOME` first if `java` is not on PATH:

```
set JAVA_HOME=C:\Program Files\Android\Android Studio\jbr
gradlew.bat assembleRelease
```

You can also open the `android` folder in Android Studio and press Run.

The build compiles the Rust core (`native/`) for each CPU type with the NDK linker. It then packs the libraries and `Code/MAZE.ASM` into the APK. The release APK is signed with the debug key so it can be installed directly. Use your own key before publishing it on a store.

## How it is put together

| Folder | What it does |
|---|---|
| `native/src/lib.rs` | JNI functions called from Java: start, stop, send keys, read the screen |
| `native/src/jni.rs` | The three JNI calls it needs (array length, read bytes, write bytes), with no extra crates |
| `native/src/bridge.rs` | State shared between the emulator thread and the UI: key queue and screen copy |
| `native/src/terminal.rs` | `AndroidTerminal`: the emulator's screen and keyboard, backed by the bridge |
| `native/src/session.rs` | Runs the DOS shell on its own thread; first-run `masm`/`link` of the maze |
| `app/.../NativeBridge.java` | Java side of the JNI functions |
| `app/.../ScreenView.java` | Draws the 80 × 25 screen (CP437 glyphs, 16 colours, half-block pixels) and handles keyboard input |
| `app/.../DosInputConnection.java` | Turns soft-keyboard text into DOS key presses |
| `app/.../KeyMapper.java` | Android key codes → DOS scan codes, plus sticky CTRL |
| `app/.../ExtraKeysBar.java`, `GamepadView.java`, `KeyButton.java` | On-screen keys with auto-repeat while held |
| `app/.../DriveFiles.java` | Drive C: folder, starter files, import and export |
| `app/.../MainActivity.java` | Layout, menu and session restart |

The emulator thread blocks while it waits for a key, exactly like the desktop version. The UI thread copies the screen 60 times a second and redraws only when it changed.
