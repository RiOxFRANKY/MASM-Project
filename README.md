# MASM Maze

`Code/MAZE.ASM` is a 16-bit DOS maze race with 3 randomly generated levels: you against the computer, or two players (PvP).

| Level | Grid | Time limit | Computer: speed / sight |
|---|---|---|---|
| 1 | 15 × 15 | 30 s | 3 steps/s, sees 2 squares around it (5 × 5) |
| 2 | 24 × 24 | 45 s | 3.6 steps/s, sees 3 squares (7 × 7) |
| 3 | 36 × 36 | 60 s | 4.5 steps/s, sees 4 squares (9 × 9) |

- Every run builds new mazes with a **randomised depth-first search (DFS)**.
- Extra walls are then knocked out, so there is always **more than one path to the exit**.
- Every square, wall or path, is one small square pixel.
- **Blue** squares are walls and **magenta** is the exit.
- Both sides get **the same maze**, side by side, and race to the exit.
- The start menu offers **1** = you vs the computer, **2** = two players, **3** = cognitive study, and **ESC** = back to DOS.

**Rules (both modes)**
- Every round starts with a **3, 2, 1, START!** countdown. The clock and the computer start only at START!, and keys pressed during the countdown are ignored.
- The first one to reach the exit wins the level.
- If the time runs out, whoever is **closer to the exit** wins, counted in steps along the maze. Equal distance is a draw.
- The level winner scores a point, and the final screen shows the match winner.
- **ESC** returns to the menu.

**One player vs the computer**
- You are green, on the left. Move with the arrow keys or W A S D.
- The computer (cyan, on the right) is an **A\* search bot with limited visibility**, so it plays fair. Unlike you, it cannot see the whole maze:
  - At the start it knows only the outer border and where the exit is, which you can see too.
  - On every step it uncovers the squares within 2, 3 or 4 rows/columns of itself (levels 1, 2, 3). Only those go into its own map. **You** still see its whole maze on screen, so you can watch where it goes.
  - It plans the shortest path with A\* over what it knows, treating unseen squares as open. It replans every time it spots a new wall, so it sometimes walks into a dead end, like a person would.
  - It never uses the game's own distance table. It moves at the fixed speed shown above, set by the BIOS clock.

**Mode 3: cognitive study (the bot learns, then you do)**
- One maze is shown per stage. Each stage has two runs on **the same maze**, under the same rule: walk from the start to the exit **again and again** until one try is exactly as long as the **optimal path**.
  1. **The bot runs first.** It keeps its own map between tries, so each try is planned with everything it has seen before. Its sight is 1 square around it. The game knows the optimal length from its BFS table, but the bot never reads it.
  2. **Then it's your turn** (arrows or W A S D). You see the whole maze.
- During each run the screen shows:
  - in the maze: the current try's path in **red** and earlier tries in **grey**
  - on the right: the length of every try, with **OPTIMAL** on the one that hit the shortest length
  - once it's found: the optimal try's path in **yellow**
- **Comparison:** after your run, the two histories are listed side by side, together with the tries needed and the total steps walked.
  - Whoever needed **fewer tries** learned the maze faster. If the tries are equal, fewer total steps wins.
- Stages: 15 × 15, 24 × 24 and 36 × 36. ENTER moves on, and the results screen lists every stage (optimal length, bot tries, your tries, who was faster) plus the overall count.
- The bot moves at full speed. **SPACE** switches its run to a slow, 18 steps/s view and back. **ESC** returns to the menu.
- **It always finishes.** If a try finds no new wall, the bot walked its first plan. That plan is a shortest path on a map where unseen squares count as open, so it can't be longer than the real shortest path. Every other try uncovers at least one new wall, and a maze has only so many walls.

**Two players (PvP)**
- **Player 1** (green) plays the left maze with **W A S D**. **Player 2** (cyan) plays the right maze with the **arrow keys**.

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

### On an Android phone

[dosbox/android](dosbox/android/README.md) builds the same emulator as an Android app (`.apk`). On the first start, the app assembles and links `MAZE.ASM` itself, so you just type `MAZE`. The gamepad button shows two D-pads, so two players can play PvP on one phone. Other `.EXE` files can be added with **Import files**.

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
| `players` | 1 = you vs the computer, 2 = PvP |
| `msize`, `cells` | Side of the current grid (15, 24 or 36) and `side * side` |
| `grid` | 0 = wall, 1 = open, one byte per square. Both sides share it. |
| `who` | The player being moved or drawn (0 = left, 1 = right / computer) |
| `ppos`, `steps` | Square and move count of each player (2 words each) |
| `orgx` | x position of each player's maze on the screen (2 and 42) |
| `trail` | 1 = walked on during the level. It holds two blocks of 1296 bytes, one per player. |
| `mark` | 1 = on the optimal path shown in the result |
| `dstart`, `dexit` | BFS distance of every square from the start and from the exit (words, `FFFFh` = not reached) |
| `queue`, `dstack` | Work lists for the BFS and the DFS |
| `spos`, `epos` | Start and exit square indices |
| `score`, `winner`, `reason` | Levels won, the level winner (2 = draw), and how it was won (exit or time-out) |
| `tstart`, `tlimit` | Clock tick at the level start, and the ticks allowed (`limits` = 546, 819, 1092) |
| `known` | The computer's own map: 0 = not seen, 1 = open, 2 = wall |
| `bpath`, `bplen` | The computer's current plan (exit first) and how many squares are left on it |
| `parent` | A\*: the square each square was reached from |
| `botspd`, `botview` | Per level: clock ticks between bot steps (6, 5, 4), and its sight radius (2, 3, 4) |

All arrays are sized for the largest level (36 × 36 = 1296 squares). Smaller levels use only the first `side * side` entries.

The A\* search runs during the race, when the maze-building arrays are no longer needed. So it reuses them through `EQU` aliases:

| Alias | Reuses | Meaning |
|---|---|---|
| `gcost` | `dstart` | g, the steps from the bot |
| `closed` | `mark` | squares already expanded |
| `bucket0`, `bucket1` | `queue`, `dstack` | the two open-list buckets |

## 2. Screen: two square pixels per text cell

Text mode has only 25 rows, which is too few for a 36-row grid drawn one character per square. So every text cell shows the **upper half block** character (`DFh`, `▀`):

- the top half is drawn in the cell's **foreground** colour
- the bottom half is drawn in its **background** colour

That gives an 80 × 50 screen of square pixels. `setpix` writes one pixel straight into video memory at `B800h`: it changes the low nibble of the attribute byte for an even `y`, or the high nibble for an odd `y`.

Normally attribute bit 7 makes text blink. `INT 10h` with `AX=1003h, BL=0` turns blinking off, so all 16 colours can be used as backgrounds.

`drawcell` paints one square as one pixel at `(orgx[who] + column, 2 + row)`.

- A 36 × 36 maze uses 36 columns and 18 text rows.
- The two mazes sit at x = 2 and x = 42, side by side.
- `drawmazes` draws both mazes, always in full. The computer's limited view exists only in its own map (`known`), not on the screen.

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
6. A final `bfs` from the exit fills `dexit` for the finished maze. This table drives the time-out rule and the optimal path. The computer never reads it.

`random` is a 16-bit linear congruential generator, `seed = seed * 25173 + 13849`. The seed comes from the BIOS tick count (`INT 1Ah`), so every run is different. `random` returns the high word of `seed * n`, which is a number from 0 to n−1.

## 4. Playing

**Reading keys.** Keys are read with `INT 16h`, which returns the scan code in `AH` and the character in `AL`. `keydir` turns a key into a direction (an offset into `deltas`) and an owner:

| Keys | `AL` | Owner (`DL`) |
|---|---|---|
| W A S D | the letter | player 1 |
| Arrows | 0 or `E0h` (scan code in `AH`) | player 2 (against the computer, both groups move you) |

**Moving.** `trymove` checks the square in direction `BX` for player `who`, and ignores walls. `applymove` then:
- updates `ppos` and `steps`;
- sets the square in that player's `trail`;
- repaints only the old and new squares.

The computer uses `applymove` too.

**Countdown (`countdown`).** Before every round, `cdstep` shows `3`, `2` and `1` for 18 clock ticks (one second) each, then `START!` for half a second. It shows them on row 19, between the level info and the clock.
- While it waits, it takes keys out of the buffer and ignores them. ESC returns to the menu.
- Only after `START!` are `tstart` and `botlast` set, so the time limit and the computer begin together.

**The race loop (`duel`).** One loop serves both modes, and it never blocks:
1. `timeleft` reads the clock (`INT 1Ah`) and compares the ticks since `tstart` with `tlimit`. The clock ticks 18.2 times a second, so 30 s = 546 ticks. It returns the seconds left, which `showtime` displays (in red for the last 10 s).
2. Against the computer, `botturn` lets the bot take one step whenever `botspd` ticks have passed since its last step.
3. `INT 16h AH=01h` checks for a key without waiting. When there is none, `HLT` sleeps until the next interrupt (a key or a clock tick) instead of spinning the CPU.
4. A key moves its owner's maze. The first one whose move lands on the exit wins (`reason = 0`).
5. When the time runs out, the two `dexit` values are compared: the smaller one wins, and equal values are a draw (`reason = 1`). `dexit` counts real steps through the maze to the exit. The same numbers are shown live as `Dist` under each maze.

## 4b. The computer player

**What it knows.**
- `botinit` fills `known` with "not seen", except the outer border, which it marks as wall.
- It also stores the exit's row and column (`erow`, `ecol`). That's the same information you get from looking at the screen.

**Looking (`botlook`).**
- After every step, every square within `botsee` rows and columns of the bot becomes known (1 = open, 2 = wall).
- If one of them is a wall it didn't know about, `newwall` is set.

**Planning (`botplan`), A\* over its own map.**
- **Costs:** each step costs 1, and `g` = steps from the bot.
- **Heuristic:** `h` (`hcost`) is the Manhattan distance to the exit, `|row − erow| + |col − ecol|`. It never over-estimates, so the path found is the shortest one on the bot's current map.
- **Map rules:** squares marked as wall are avoided. Unseen squares are assumed open, the "free space" assumption.
- **Two buckets instead of a heap:** a step changes `g` by +1 and `h` by exactly ±1. So a neighbour's `f = g + h` is either the current `f` or `f + 2`. The open list therefore needs only two stacks, `bucket0` and `bucket1`: one for squares with the current `f`, one for `f + 2`. When the current bucket is empty, the two swap and `f` grows by 2.
  - That makes one search linear in the maze size, fast enough for an 8086 to replan after any step.
  - Squares popped from the same bucket come out last-in, first-out. This prefers deeper squares (larger `g`), the usual A\* tie-break.
- **Building the plan:** when the exit is expanded, the `parent` links are followed back to the bot. The squares are stored in `bpath`, exit first.

**Stepping (`botstep`).**
1. It takes the last square of `bpath`, which is always a neighbour of the bot.
2. It moves there with `applymove`, then calls `botlook`.
3. If a new wall came into sight, it runs `botplan` again. That way the bot reacts to dead ends as soon as it can see them.

**Why it's fair.** The bot never reads `grid` beyond its sight radius, and never reads `dexit`. It moves at a fixed speed from the clock, not as fast as the CPU allows.

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

3. `result2` adds a point to the level winner and prints the winner and the reason, plus the colour legend. The computer's maze shows its walked path in red, so you can see where it went.
4. **ENTER** continues and **ESC** returns to the menu. After level 3, `finalscreen` shows the match result.

## 6. Summary

```
menu (1 = vs computer / 2 = PvP / ESC)
  -> for each level (15, 24, 36):
       buildmaze: DFS -> exit -> BFS x2 -> open a shortcut -> extra loops -> BFS
       draw 2 identical mazes + bottom panel, start the clock
       vs computer: botinit (border + exit only) -> botlook -> botplan (A*)
       countdown: 3, 2, 1, START! -> start the clock and the bot
       duel: time left? -> bot due? (step, look, replan on new wall)
             -> key waiting? -> move you / P1 (WASD) / P2 (arrows), else HLT
             first on the exit wins; at time-out the smaller dexit wins
       result: yellow optimal / red walked / orange both, score, wait for ENTER
  -> final screen (match winner) -> menu
```
