# MASM Maze — Live Demo Command Guide

All commands assume your shell starts in the repository root:

```bash
cd /Users/ritamghosh/Desktop/Class_Notes/MASM
```

---

## 1. Assemble, Link & Launch `Code/MAZE.ASM` (DOSBox-RS)

### Option A — Interactive DOSBox-RS (Recommended for Live Demo)
Launch the emulator mounting `Code/` as `C:\`:

```bash
./dosbox/target/release/dosbox Code
```

Inside the `C:\>` DOS prompt, run:

```dos
masm MAZE.ASM;
link MAZE.OBJ;
MAZE
```

### Option B — One-Command Assemble, Link & Start
```bash
./dosbox/target/release/dosbox Code -c "masm MAZE.ASM;" -c "link MAZE.OBJ;" -c MAZE
```

### Option C — Headless Build Verification (No Window)
```bash
./dosbox/target/release/dosbox Code --headless --exit -c "masm MAZE.ASM;" -c "link MAZE.OBJ;"
```

### Rebuild DOSBox-RS from Source (If Needed)
```bash
cargo build --release --manifest-path dosbox/Cargo.toml
```

*(Windows prebuilt binary: `bin\windows-x86_64\dosbox.exe Code`)*

---

## 2. In-Game Controls (`MAZE.EXE`)

| Screen / Mode | Key | Action |
|---|---|---|
| **Start Menu** | `1` | Single-Player Race vs. A\* Bot (3 levels: `15x15`, `24x24`, `36x36`) |
| **Start Menu** | `2` | Two-Player Network PvP (via `INT 61h` + WebSocket server) |
| **Start Menu** | `3` | Cognitive Study Mode (human learns first, then A\* bot; logs `COGLOAD.CSV`) |
| **Start Menu** | `ESC` | Exit back to DOS |
| **Gameplay** | Arrows / `W A S D` | Move player (green) on left maze |
| **Gameplay** | `H` or `Enter` | Flash A\* / BFS optimal path hint until next move |
| **Mode 3 (Bot Run)** | `Space` | Toggle bot playback speed (full speed vs. 18 steps/s) |
| **Emulator** | `Ctrl+F9` | Quit DOSBox-RS |

---

## 3. Mode 2: Network PvP Server & Live Leaderboard

### Step 1 — Start the Server (Terminal 1)
```bash
./server/target/release/maze 9000
```

*(Rebuild server if needed: `cargo build --release --manifest-path server/Cargo.toml` — Windows: `bin\windows-x86_64\server.exe 9000`)*

### Step 2 — Open the Live Leaderboard Web Dashboard
```bash
open http://localhost:9000/
```

- Auto-refreshes every 5 seconds with tournament rankings, win rates, speed bonuses, and recent matches.
- Live JSON endpoint: `http://localhost:9000/leaderboard.json`
- From another laptop/phone on the same Wi-Fi, replace `localhost` with the LAN IP printed in Terminal 1.

### Step 3 — Connect Player 1 & Player 2 (Terminals 2 & 3)
Open **two separate terminals** and launch two DOSBox-RS clients:

**Terminal 2 (Player 1):**
```bash
./dosbox/target/release/dosbox Code -c MAZE
```

**Terminal 3 (Player 2):**
```bash
./dosbox/target/release/dosbox Code -c MAZE
```

In both game windows:
1. Press **`2`** (*Network PvP*).
2. Enter Server IP: **`127.0.0.1`** (or the server's LAN IP if playing across two laptops).
3. Enter Port: **`9000`**.
4. Both clients pair automatically, receive the same 16-bit maze seed, race across 3 levels, and update the live leaderboard.

---

## 4. Mode 3: Cognitive Load vs. A\* Bot Comparison System

After completing stages in Mode **3** (which writes `Code/COGLOAD.CSV`), run the comparison analyzer:

```bash
python3 Code/compare_cogload.py
```

Custom CSV path example:

```bash
python3 Code/compare_cogload.py --csv Code/COGLOAD.CSV --out-dir Code
```

Open the interactive comparison dashboard (with learning curves, Stage CLI bar chart, and step-by-step 2D trajectory/backtrack viewer):

```bash
open Code/cogload_dashboard.html
```

Generated analysis outputs:
- `Code/cogload_dashboard.html` — Interactive dark-mode HTML dashboard (also supports loading any new `COGLOAD.CSV` via the UI button)
- `Code/cogload_metrics.json` — Structured per-attempt, per-maze, and per-stage JSON metrics
- `Code/cogload_report.md` — Markdown summary tables and findings

---

## 5. Android Mobile APK Demo

### Install Prebuilt APK onto a Connected Phone (USB Debugging / ADB)
```bash
adb install -r RandomMaze.apk
```

### Rebuild APK from Source (Requires Android SDK/NDK + Rust Android Targets)
```bash
./dosbox/android/gradlew -p dosbox/android assembleRelease
```

### Handheld Controller Mapping
| Phone Button | Mapped Key / Behavior |
|---|---|
| **D-Pad** | Arrow keys (move player) |
| **LEFT** | `1` (Single-Player vs. A\* Bot) |
| **RIGHT** | `2` (Network PvP) |
| **START** | `3` (Cognitive Study) |
| **A** | `ENTER` / Hint during gameplay |
| **B** | `ESC` (Back / Menu) |
| **SELECT** | Toggle soft keyboard |
| **►►** | Speed toggle (`1× → 2× → 4× → 1×`) |

---

## 6. Project Report & Assignment Programs

- **Compiled Group Project & Assignment Report:**
  ```bash
  open ~/Desktop/Systems_Lab_Project_Report.pdf
  ```
- **Run Any Lab Assignment Program in DOSBox-RS:**
  ```bash
  ./dosbox/target/release/dosbox /Users/ritamghosh/Desktop/Class_Notes/BCSE3.1/Systems
  ```
  Then at `C:\>`, assemble, link, and execute any `.ASM` file with `masm <FILE>.ASM;`, `link <FILE>.OBJ;`, `<FILE>`.
