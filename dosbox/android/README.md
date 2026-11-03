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

The app is laid out like a handheld game emulator, in portrait. The 80 × 25 DOS screen is on top and a controller is below it:

| Control | What it sends / does |
|---|---|
| D-pad | Arrow keys (hold to repeat) |
| **LEFT** | `1` |
| **RIGHT** | `2` |
| **►►** | Fast forward: 1× → 2× → 4× → 1×. It speeds up the emulated BIOS clock, so game timers and the computer player speed up together. |
| **SELECT** | Opens or closes the phone keyboard, for typing DOS commands |
| **START** | `3` (the cognitive study in the maze menu) |
| **B** | ESC (back / exit) |
| **A** | ENTER: confirms in menus, and shows the hint during a race |
| **⋮** (top right) | **Import files**, **Export a file**, Restart DOS |

- **Zooming the DOS screen:** the screen uses all the space above the controller.
  - **Double-tap** zooms in at that spot (2.2×), and double-tap again goes back.
  - **Pinch** zooms up to 4×, and **drag** moves around while zoomed.
  - The screen is drawn as a sharp 1280 × 800 picture, at 16 × 32 pixels per character, and scaled smoothly to fit.
- Typing `EXIT` restarts DOS.
- A Bluetooth or USB keyboard also works, including the arrow keys, F1–F12 and Ctrl (Ctrl+C stops a program).

### Running your own programs

1. Put the `.EXE` (or `.COM` / `.ASM`) file anywhere on the phone, for example in Downloads.
2. In the app, open the menu, choose **Import files**, and pick one or more files. They are copied to `C:\`, with names turned into DOS 8.3 form (`my game.exe` → `MY_GAME.EXE`).
3. Type the program's name at the prompt, e.g. `MY_GAME`.

### The maze game

The app starts `MAZE` by itself. On first launch, and after an update changes the bundled source, it copies `MAZE.ASM` to `C:\` and assembles it (`masm MAZE.ASM;` then `link MAZE.OBJ;`).

- **Menu:** LEFT (`1`) starts you vs the computer, RIGHT (`2`) starts network PvP, and START (`3`) starts the cognitive study.
- **Against the computer:** steer with the D-pad. **A** shows your shortest path to the exit until your next move. The A\* bot races you in the right-hand maze; you see its whole maze, but it only knows the squares it has seen near itself.
- **Network PvP:** start the [maze server](../../server/README.md) on a PC reachable by both phones or PCs. Press RIGHT (`2`) on each device, then use SELECT to type the server's IP address and port. The last address is saved in `C:\MAZE.CFG`. Both players steer the left maze with the D-pad; the right maze shows the opponent live. Each level starts after both players are ready. Network play needs access to the server's TCP port on the same Wi-Fi or another reachable network.
- **Speed:** ►► speeds everything up; **B** goes back.
- **Leaving:** B from the game's menu leaves the game. Type `MAZE` to start it again.

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

The build compiles the Rust core (`native/`) for each CPU type with the NDK linker. It then packs the libraries and `Code/MAZE.ASM` into the APK. The app has Android's internet permission for network PvP. When the bundled maze source changes, the app replaces its installed `MAZE.ASM` and rebuilds `MAZE.EXE` on the next launch. The release APK is signed with the debug key so it can be installed directly. Use your own key before publishing it on a store.

## How it is put together

| Folder | What it does |
|---|---|
| `native/src/lib.rs` | JNI functions called from Java: start, stop, send keys, read the screen |
| `native/src/jni.rs` | The three JNI calls it needs (array length, read bytes, write bytes), with no extra crates |
| `native/src/bridge.rs` | State shared between the emulator thread and the UI: key queue and screen copy |
| `native/src/terminal.rs` | `AndroidTerminal`: the emulator's screen and keyboard, backed by the bridge |
| `native/src/session.rs` | Runs the DOS shell on its own thread; first-run `masm`/`link` of the maze |
| `app/.../NativeBridge.java` | Java side of the JNI functions |
| `app/.../ScreenView.java` | Draws the 80 × 25 screen into an off-screen picture (CP437 glyphs, 16 colours, half-block pixels; only changed cells are redrawn), scales it to the view with zoom and pan, and handles keyboard input |
| `app/.../DosInputConnection.java` | Turns soft-keyboard text into DOS key presses |
| `app/.../KeyMapper.java` | Android key codes → DOS scan codes |
| `app/.../ControllerView.java` | Lays out the controller: each button placed by its position on the handheld-style layout, scaled to the phone |
| `app/.../DpadView.java` | The round D-pad (slide between directions, repeat while held) |
| `app/.../KeyButton.java` | Pill and round buttons |
| `app/.../DriveFiles.java` | Drive C: folder, starter files, import and export |
| `app/.../MainActivity.java` | Layout, menu and session restart |

The emulator thread blocks while it waits for a key, exactly like the desktop version. The UI thread copies the screen 60 times a second and redraws only when it changed.
