# MASM Maze

`Code/MAZE.ASM` is a 16-bit DOS maze game with 3 randomly generated levels, for one player or two players (PvP).

| Level | Grid | PvP time limit |
|---|---|---|
| 1 | 15 × 15 | 30 s |
| 2 | 24 × 24 | 45 s |
| 3 | 36 × 36 | 60 s |

- Every run builds new mazes with a **randomised depth-first search (DFS)**.
- Extra walls are then knocked out, so there is always **more than one path to the exit**.
- Every square, wall or path, is one small square pixel.
- **Blue** squares are walls and **magenta** is the exit.
- The start menu offers **1** = one player, **2** = two players, and **ESC** = back to DOS.

**One player**
- You are green. Move with the arrow keys or W A S D.
- **H** shows the shortest path from where you stand, in cyan. **ESC** returns to the menu.

**Two players (PvP)**
- Both players get **the same maze**, side by side.
- **Player 1** (green) plays the left maze with **W A S D**. **Player 2** (cyan) plays the right maze with the **arrow keys**.
- The first player to reach the exit wins the level.
- If the time runs out, the player **closer to the exit** wins, counted in steps along the maze. Equal distance is a draw.
- The level winner scores a point, and the final screen shows the match winner.

**After every level**
- **yellow** = the optimal (shortest) path
- **red** = the path the player actually walked
- **orange** = squares on both
- In PvP, each maze shows its own player's path.
- Press **ENTER** for the next level.

## Problem Statement
Write a MASM program to draw a simple maze using the # character to represent walls and empty spaces for paths. Place a player character (e.g., @) at a defined starting point in the maze. Allow the user to navigate the maze using the arrow keys (Up, Down, Left, Right) by handling keyboard interrupts. Ensure the player cannot move through walls (#) and can only move along valid paths. Continuously update and store the player's current position, and refresh the maze view after each move.

## Build and run

### DOSBox-RS (this repository)

The `dosbox` folder contains an 8086 emulator with a built-in MASM and LINK. From the repository root:

```
cargo +stable-x86_64-pc-windows-gnu build --release --manifest-path dosbox/Cargo.toml
dosbox/target/release/dosbox Code
```

Then, at the `C:\>` prompt:

```
masm MAZE.ASM;
link MAZE.OBJ;
MAZE
```

See [dosbox/README.md](dosbox/README.md) for details.

### MASM + LINK (in DOSBox)

Run these from inside the `Code` folder.

```
masm MAZE.ASM;
link MAZE.OBJ;
MAZE
```

### JWasm + DOSBox on Linux

A DOS `.EXE` cannot run directly on Linux, so it is run inside DOSBox. On Ubuntu or Debian, from the repository root:

```
sudo apt install dosbox build-essential git
git clone https://github.com/Baron-von-Riedesel/JWasm.git
make -C JWasm -f GccUnix.mak
cd Code
../JWasm/build/GccUnixR/jwasm -mz MAZE.ASM
dosbox MAZE.EXE
```

Click inside the DOSBox window before pressing keys, so it receives them.

---

# How the code works

A few terms come up often:

- **Registers:** `AX`, `BX`, `CX` and `DX` are 16-bit. Each can be used as two 8-bit halves, such as `AH` (high byte) and `AL` (low byte).
- **Interrupts:** `INT 10h` calls the BIOS video services, `INT 16h` the BIOS keyboard, `INT 1Ah` the BIOS clock and `INT 21h` the DOS services. The number put in `AH` before the call picks the service.
- **Square index:** the grid is one flat byte array, stored row after row. Square (row, col) is at index `row * side + col`. The four neighbours of index `i` are `i - side` (up), `i + side` (down), `i - 1` (left) and `i + 1` (right). These four steps are kept in the `deltas` table.

## 1. Data

| Name | Meaning |
|---|---|
| `players` | 1 = one player, 2 = PvP |
| `msize`, `cells` | Side of the current grid (15, 24 or 36) and `side * side` |
| `grid` | 0 = wall, 1 = open, one byte per square. Both PvP players share it. |
| `who` | The player being moved or drawn (0 or 1) |
| `ppos`, `steps` | Square and move count of each player (2 words each) |
| `orgx` | x position of each player's maze on the screen (2 and 42) |
| `trail` | 1 = walked on during the level. It holds two blocks of 1296 bytes, one per player. |
| `mark` | 1 = on the shortest path being shown (hint or result) |
| `dstart`, `dexit` | BFS distance of every square from the start and from the exit (words, `FFFFh` = not reached) |
| `queue`, `dstack` | Work lists for the BFS and the DFS |
| `spos`, `epos` | Start and exit square indices |
| `score`, `winner`, `reason` | PvP: levels won, the level winner (2 = draw), and how it was won (exit or time-out) |
| `tstart`, `tlimit` | PvP: clock tick at the level start, and the ticks allowed (`limits` = 546, 819, 1092) |

All arrays are sized for the largest level (36 × 36 = 1296 squares). Smaller levels use only the first `side * side` entries.

## 2. Screen: two square pixels per text cell

Text mode has only 25 rows, which is too few for a 36-row grid drawn one character per square. So every text cell shows the **upper half block** character (`DFh`, `▀`):

- the top half is drawn in the cell's **foreground** colour
- the bottom half is drawn in its **background** colour

That gives an 80 × 50 screen of square pixels. `setpix` writes one pixel straight into video memory at `B800h`: it changes the low nibble of the attribute byte for an even `y`, or the high nibble for an odd `y`.

Normally attribute bit 7 makes text blink. `INT 10h` with `AX=1003h, BL=0` turns blinking off, so all 16 colours can be used as backgrounds.

`drawcell` paints one square as one pixel at `(orgx[who] + column, 2 + row)`.

- A 36 × 36 maze uses 36 columns and 18 text rows.
- In PvP the two mazes sit at x = 2 and x = 42, side by side.
- `drawmazes` draws the maze of every player.

## 3. Making the maze (`buildmaze`)

1. **`genmaze` – randomised DFS.** Everything starts as wall, and the start square (1,1) is carved and pushed on a stack. Then, repeatedly:
   - look at the square on top of the stack;
   - list the neighbours that may be carved (`carvable`);
   - pick one at random (`random`), carve it and push it;
   - if none can be carved, pop the stack (backtrack).

   A neighbour may be carved only if all of these hold:
   - it is inside the border and still a wall;
   - it touches no open square except the one we come from;
   - the two squares diagonally in front of it are walls.

   This keeps corridors one square wide and gives a "perfect" maze, with exactly one route between any two squares.
2. **`placeexit`** puts the exit on the open square with the largest `row + col`, which is nearest the bottom-right corner.
3. **`bfs`** runs twice: from the start into `dstart`, and from the exit into `dexit`. A square lies on the route from start to exit exactly when `dstart + dexit` equals the route length (`onpath`).
4. **A guaranteed second path.** `countwalls` with `strict = 1` counts the walls that meet all of these:
   - open squares on two opposite sides and walls on the other two;
   - both open squares lie on the route;
   - those two squares are at least 4 steps apart.

   Opening such a wall (`openrandom`) creates a shortcut, so the start and exit are now joined by two different routes. If the maze has no such wall, it is generated again.
5. **Extra loops.** `side / 2` more random walls between two open squares are opened (`strict = 0`).
6. A final `bfs` from the exit fills `dexit` for the finished maze. This table drives both the hint and the optimal path.

`random` is a 16-bit linear congruential generator, `seed = seed * 25173 + 13849`. The seed comes from the BIOS tick count (`INT 1Ah`), so every run is different. `random` returns the high word of `seed * n`, which is a number from 0 to n−1.

## 4. Playing

**Reading keys.** Keys are read with `INT 16h`, which returns the scan code in `AH` and the character in `AL`. `keydir` turns a key into a direction (an offset into `deltas`) and an owner:

| Keys | `AL` | Owner (`DL`) |
|---|---|---|
| W A S D | the letter | player 1 |
| Arrows | 0 or `E0h` (scan code in `AH`) | player 2 |

**Moving.** `trymove` moves player `who`:
- If the new square is a wall, the key is ignored.
- Otherwise `ppos` and `steps` are updated, the square is set in that player's `trail`, and only the old and new squares are repainted.

**One player (`solo`).**
- It waits for a key (`INT 16h AH=0`), and every movement key moves player 1.
- **H** toggles the hint. `tracepath` follows `dexit` downhill from the player, always to a neighbour exactly one step closer to the exit, and sets `mark`. Those squares are shown in cyan.

**Two players (`duel`).** It loops without blocking:
1. `timeleft` reads the clock (`INT 1Ah`) and compares the ticks since `tstart` with `tlimit`. The clock ticks 18.2 times a second, so 30 s = 546 ticks. It returns the seconds left, which `showtime` displays (in red for the last 10 s).
2. `INT 16h AH=01h` checks for a key without waiting. When there is none, `HLT` sleeps until the next interrupt (a key or a clock tick) instead of spinning the CPU.
3. A key moves its owner's maze. The first player whose move lands on the exit wins (`reason = 0`).
4. When the time runs out, the two players' `dexit` values are compared: the smaller one wins, and equal values are a draw (`reason = 1`). `dexit` counts real steps through the maze to the exit. The same numbers are shown live as `Dist` under each maze.

## 5. Level result

When a level ends, `showresult` runs:

1. `tracepath` is run from the **start**, so `mark` now holds one optimal path.
2. `mode` becomes 1 and every maze is redrawn. `cellcolour` uses `mark` and the trail of the maze's own player:

| `mark` | `trail` | Colour |
|---|---|---|
| 1 | 0 | yellow (optimal only) |
| 0 | 1 | red (walked only) |
| 1 | 1 | orange (both) |

A player who stopped before the exit (time-out) is still drawn where they stood.

3. The text panel is filled in:
   - **One player:** `result1` prints your move count next to the optimal one (`dexit[start]`), and says so when they are equal.
   - **PvP:** `result2` adds a point to the level winner and prints the winner and the reason.
4. **ENTER** continues and **ESC** returns to the menu. After level 3, `finalscreen` shows the match result.

## 6. Summary

```
menu (1 / 2 / ESC)
  -> for each level (15, 24, 36):
       buildmaze: DFS -> exit -> BFS x2 -> open a shortcut -> extra loops -> BFS
       draw 1 maze + sidebar   |   2 identical mazes + bottom panel, start clock
       solo:  wait key -> move / H hint / ESC                  until the exit
       duel:  time left? -> key waiting? -> move P1 (WASD) or P2 (arrows), else HLT
              first on the exit wins; at time-out the smaller dexit wins
       result: yellow optimal / red walked / orange both, score, wait for ENTER
  -> final screen (match winner in PvP) -> menu
```
