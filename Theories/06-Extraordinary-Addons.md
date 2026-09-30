# Theory 06 — Extraordinary Add-ons (Beyond the Problem Statement)

The problem statement only demands: draw a maze, place `@`, arrow-key movement with
interrupt-driven input, wall collision, position tracking, screen refresh.
Here is everything *extraordinary* you can layer on top — with the technique needed
for each, ranked by wow-factor vs. effort.

---

## ⭐ Tier S — Show-stoppers (these get you the grade/the job)

### S1. Procedural maze generation (no static data at all!)

Replace the three hardcoded mazes with a **recursive-backtracker / DFS** generator.

```asm
; algorithm (25x25 byte grid, all '#')
; 1. fill grid with '#'
; 2. mark (1,1) as ' ', push onto a stack
; 3. while stack not empty:
;      peek cell; find unvisited neighbours 2 cells away (±2 in row/col)
;      if none → pop
;      else pick one (random), carve wall between (cell+neighbour)/2 as ' ',
;           push neighbour
; 4. knock a hole in the border for the exit, place 'E'
```

Randomness source: **`INT 1Ah / AH=00h`** returns the BIOS tick count in `CX:DX`
(18.2 Hz) → feed it into an **LCG**: `seed = seed*1103515245 + 12345` (32-bit via
two 16-bit `mul`s) and take a few bits.

**Wow factor:** infinite replayability; you can even generate a new maze per level and
claim "the game writes its own data".

### S2. BFS shortest-path hint / solver

Breadth-first search from the player to `E` using an **array as a queue** (SI=head,
DI=tail), a `dist[625]` byte table and a `prev[625]` parent table.

- Display: distance numbers in a debug mode, or an arrow pointing at the optimal next
  move, or "optimal = 87 moves, you took 143".
- Pre-computing BFS at level load also **proves the maze is solvable**.

**Wow factor:** turns a toy into a graph-algorithm demonstration — interview gold.

### S3. Colour + direct video memory + zero flicker

Text mode cells are 2 bytes at `B800h:0000`, laid out `[char][attribute]`:

```asm
    mov ax,0B800h
    mov es,ax
    mov di, (row*80 + col)*2
    mov al,'#'
    mov ah,1Fh        ; attribute: bg=blue(1), fg=bright white(F)
    stosw             ; write char+attr, DI+=2
```

- walls `1Fh` blue, path `07h` grey, exit `2Eh` green, player `4Eh` yellow on red.
- Paint the whole grid with `rep stosw` → **microseconds**, no mode reset → **no flicker**.
- Bonus: mode `13h` (320×200×256) graphical version with 8×8 tile blits and an
  animated `@` sprite.

### S4. Real keyboard interrupt hooking (INT 09h)

The statement says "handle keyboard interrupts" — do it literally:

```asm
    cli                       ; interrupts off while swapping vector
    xor ax,ax
    mov es,ax
    mov word ptr es:[9*4], offset my_isr
    mov word ptr es:[9*4+2], seg my_isr
    sti
```

`my_isr` reads port `60h` (scancode), pushes it into a 16-byte **ring buffer**
(`head`/`tail` bytes), then **must** send `EOI` (write `20h` to port `20h`) and
`IRET`. The main loop drains the buffer instead of calling `INT 16h`.

**Why it's extraordinary:** it lets you handle *key release* (scancode | 80h),
simultaneous keys (diagonals!), auto-repeat control, and non-blocking input — while
the game loop keeps running. Show you know `CLI/STI`, ring buffers and `IRET`.

### S5. Game feel: sound, timer, score

- **PC speaker beep on bump/win**: enable the speaker (IN AL,61h | 03h, OUT 61h),
  program the PIT channel 2 (`OUT 43h,0B6h` then divisor `OUT 42h` low/high) for the
  frequency, wait ~80 ms via `INT 1Ah`, then silence (port 61h & ~03h).
- **Timer**: `INT 1Ah/AH=00h` returns ticks since midnight in `CX:DX`; `div` by 18` to
  get seconds → "Time: 00:42" and a countdown for a speed mode.
- **Score**: `moves × 10 + time × 2 + bonus for remaining...`, printed in the HUD.

---

## 🅰 Tier A — Clever mechanics inside the maze

| # | Add-on | Technique |
|---|---|---|
| A1 | **Fog of war / torch radius** | `seen[625]` flag array; draw cells > radius as `·` or blank |
| A2 | **Minimap** | second, scaled-down render (1 cell → 1 char) in the sidebar |
| A3 | **Keys & locked doors** | items array; `'D'` cell passes only if `keys>0` |
| A4 | **Traps / teleporters** | cell types `'T'` (respawn at start) and `'P'` (jump to pair coords) |
| A5 | **Enemies with patrol/chase AI** | enemy struct (row,col,dir); BFS-lite chase when same row/col |
| A6 | **Moving walls / periodic hazards** | recompute wall positions from tick count each frame |
| A7 | **Dash (hold Shift)** | move 2 cells if the intermediate cell is free |
| A8 | **Diagonal movement** | accept 4 extra candidates; require both orthogonal neighbours free |
| A9 | **Wrap-around portals at borders** | if candidate leaves 0..24, wrap to the opposite edge |
| A10 | **Gravity mode** | player falls until hitting `#` unless it presses a direction |
| A11 | **Undo** | 256-byte ring buffer of old (row,col) pairs; backspace pops |
| A12 | **Step counter + par (optimal) moves** | BFS distance as `par`; show ★ if `moves == par` |
| A13 | **Golden items to collect before `E` opens** | `bitmask` of 4 pickups; exit checks `mask==0Fh` |
| A14 | **Mines that reveal a hint** | one-shot cell that flashes the next BFS step |
| A15 | **Maze rotation (gravity puzzle)** | rotate the *grid* 90° in RAM and recompute player position |

---

## 🅱 Tier B — Game structure & polish

| # | Add-on | Technique |
|---|---|---|
| B1 | **Title / help screen** | state byte: `0=title, 1=playing, 2=paused, 3=win, 4=gameover` driving one big `game:` dispatcher |
| B2 | **Pause (P) + ESC-confirm** | second `INT 16h` read; draw an overlay box |
| B3 | **Lives system** | `lives db 3`; trap costs one → `game over` at 0 |
| B4 | **Countdown timer → game over** | tick comparison, then forced state change |
| B5 | **High-score table on disk** | `INT 21h`: AH=3Ch create, AH=40h write, AH=3Fh read; 10 × 12-byte records, insertion sort |
| B6 | **Save / load game** | write `level,prow,pcol,moves,time` (6 bytes) to `MAZE.SAV` |
| B7 | **Replay** | store the whole key stream, replay it with a delayed reader |
| B8 | **Config / difficulty menu** | read a `MAZE.CFG` file or arrow-key menu changing maze size & time |
| B9 | **Maze editor mode** | free-roam cursor, paint `#`/space/`E`, export to file |
| B10 | **Localisation** | message table indexed by a language byte |
| B11 | **Achievements / stats** | persistent counters (total moves, mazes finished) |
| B12 | **Credit / animation sequence** | scrolling text row via direct `B800h` writes |

---

## 🅲 Tier C — Deep-system / "hardcore" add-ons

| # | Add-on | Technique |
|---|---|---|
| C1 | **Hook INT 1Ch (timer tick)** | 18.2 Hz callback driving animation/enemies independently of input |
| C2 | **Soundtrack via PIT + IRQ0** | sequenced note table, ISR plays next note each tick |
| C3 | **DMA-free sampled audio** — too far? use square-wave effects | port 42h programming |
| C4 | **VGA Mode X page flipping** | two pages, flip on retrace — for the graphical port |
| C5 | **Mouse support** | `INT 33h` (mouse driver): `AX=0` init, `AX=1` show, read pos in CX/DX |
| C6 | **EMS/XMS large worlds** | >64 KB mazes via `INT 67h` (EMS) — shows segmented-memory mastery |
| C7 | **COM version (`.model tiny`)** | same game in one 64 KB segment, `org 100h` — a "size-coding" flex |
| C8 | **Interrupt-safe driver style** | make the whole game event-driven: ISRs push events, main loop is a state machine |
| C9 | **Network/serial co-op** | `INT 14h` serial port — two players, two mazes |
| C10 | **Self-modifying code / table-driven dispatch** | replace the cmp-chain with a jump table (`dw` offsets + `jmp cs:[bx]`) — elegant and fast |

Jump-table example (replaces ~12 `cmp/je` in `game`):

```asm
    mov al,ah          ; scancode
    xor ah,ah
    shl ax,1           ; ×2 for word table
    mov bx,ax
    jmp cs:dispatch[bx]      ; needs a table covering 0..4Dh (sparse → use range check first)
dispatch dw default, esc, ... up, ...
```

---

## 🏆 The 5 add-ons I'd actually build, in order

1. **Partial redraw + colour via `B800h`** — fixes flicker *and* looks impressive
   (S3) — ~40 lines.
2. **Timer + move counter + score** (S5) — makes it a *game*, uses `INT 1Ah`.
3. **BFS solver showing optimal vs. actual** (S2) — algorithmic depth.
4. **Procedural generation** (S1) — removes 1875 bytes of static data entirely.
5. **INT 09h hook + ring buffer** (S4) — proves you understand interrupts for real.

Together they turn a 289-line maze demo into a full system-programming portfolio piece:
**video memory, BIOS timers, keyboard ISR, graph algorithms, RNG, and file I/O.**
