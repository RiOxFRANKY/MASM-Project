# Theory 06 — Extraordinary Add-ons: What Is Done, What Is Next

The problem statement only demands: draw a maze, place `@`, arrow-key movement,
wall collision, position tracking and a screen refresh. `Code/MAZE.ASM` already
goes far beyond that. This document is split in two halves:

* **Section 0** lists the extraordinary features that are *already in the shipped
  code*, so nobody tries to "add" them again.
* **Sections 1–5** are the genuine remaining wish-list: what it is, how you would
  build it in 16-bit real-mode assembly, and in what order.

Source of truth for everything marked "implemented" is `Code/MAZE.ASM`
(assembled with `jwasm -mz MAZE.ASM` → **0 errors, 0 warnings**). The deep dive
into the video/BFS machinery lives in
[`Theories/07-BFS-Hinter-and-Video.md`](07-BFS-Hinter-and-Video.md); this file
only summarises it.

Equates in force everywhere below:

```asm
MW      equ 25                  ; maze width  (columns per row)
MH      equ 25                  ; maze height (rows)
MSZ     equ MW * MH             ; 625 bytes per level
NLEV    equ 3                   ; number of levels

A_WALL  equ 1Fh                 ; bright white on blue
A_PATH  equ 07h                 ; grey on black
A_EXIT  equ 2Eh                 ; bright yellow on green
A_PLAY  equ 4Eh                 ; bright yellow on red
A_HINT  equ 3Fh                 ; bright white on cyan
A_HUD   equ 0Bh                 ; cyan on black
A_WIN   equ 2Fh                 ; bright white on green
A_MSG   equ 0Fh                 ; bright white on black
```

---

## SECTION 0 — Already implemented (do NOT re-add these)

### 0.1 Feature status table

| # | Feature | Status | Implementing procedures | Deep dive |
|---|---------|--------|-------------------------|-----------|
| 1 | Colour + direct video memory drawing (`B800h`, attribute bytes) | **Done** | `vcell`, `vstr`, `vnum2`, `vnum3`, `paintcell`, `drawmaze` | `Theories/07-BFS-Hinter-and-Video.md` |
| 2 | Zero-flicker rendering (mode set twice in the whole program; partial repaint per move) | **Done** | `drawmaze` (level start only), `move` → `clearhint`, `clrplayer`, `putplayer`, `refreshhint`, `updhud` | `Theories/07-BFS-Hinter-and-Video.md` |
| 3 | BFS optimal-path hinter on key `H` | **Done** | `bfsolve`, `trynb`, `findnb`, `refreshhint`, `hintkey`, plus `updhud` for the `Dist` field | `Theories/07-BFS-Hinter-and-Video.md` |

Also done, and therefore no longer to-do items: EQU constants
(`MW`/`MH`/`MSZ`/`NLEV`), bounds checks inside BFS and hint lookup, the
byte-distance overflow guard (`cmp dl,0FEh` in `bfsolve`), and a win screen
that waits for a key before returning to DOS.

### 0.2 Colour + direct video memory

Every visible byte in the game is written by the program itself into text-mode
video RAM at segment `B800h`, two bytes per cell (even = ASCII character, odd =
attribute). `vcell` is the single low-level store
(`MAZE.ASM:321`): it computes `offset = (row*80 + col)*2` and writes
`es:[di]` / `es:[di+1]`. On top of it sit `vstr` (`$`-terminated strings),
`vnum2` (2 digits via `aam`), `vnum3` (3 digits via two `div`s — needed because
the BFS distance reaches 140), `paintcell` (repaint one maze cell in its
natural colour) and `drawmaze` (full 625-cell colourised sweep). **No BIOS
`INT 10h` teletype calls are ever used for drawing**, which is what makes
redrawing cheap.

### 0.3 Zero flicker

`INT 10h AX=0003h` executes exactly **twice in the entire program**: once at
`main` entry and once on the win screen. The screen is therefore never cleared
during play. `drawmaze` runs only at `start_level`; the `move` path repaints at
most four cells — `clearhint` (old highlight), `clrplayer` (cell left),
`putplayer` (cell entered), `refreshhint` (new highlight) — plus three HUD
fields through `updhud` (Row, Col, Dist digits). A rejected move into a wall
jumps straight back to `game` with **zero** screen writes. That is the whole
flicker-elimination strategy: incremental stores instead of mode resets.

### 0.4 BFS optimal-path hinter

`bfsolve` runs **once per level** from `start_level`:

1. Scans the level inside `mazes` for `'E'` and records `exrow`/`excol`
   (division by `MW` converts the flat index back to row/col).
2. Fills the byte array `dist` (625 bytes) with `0FFh` using `rep stosb`.
3. Seeds the word queue `queue dw MSZ dup(0)` — one entry per cell, packed as
   `AL = row`, `AH = col` — with the exit at distance 0.
4. Expands level by level via `trynb`, which rejects out-of-bounds rows/cols
   (`cmp bl,MH` / `cmp bh,MW`), walls (`mazes[di] = '#'`), already-visited
   cells (`dist[si] <> 0FFh`) and a full queue; it stops expanding a cell whose
   incremented distance would reach `0FEh` (byte overflow guard).

During play, `refreshhint` reads `dist[player]`, stores it in `hintd`, sets
`wantd = dist[player]-1` and probes the four neighbours in **up, down, left,
right** order through `findnb`, which returns the answer in **CF** and parks the
winner in `hrow`/`hcol`. The cell is then stamped with its real character but
the `A_HINT` attribute. `updhud` prints `hintd` with `vnum3` (3 digits) or
`'---'` when the hint is off / the cell is unreachable.

Verified facts for the shipped mazes: optimal distances **level 0 = 100,
level 1 = 68, level 2 = 140**; every walkable cell is reachable from the exit
(**299 / 292 / 287** cells); the exit sits at **(23, 23)** on all three levels.

### 0.5 Original wish-list entries that are now closed

The old version of this file listed these as future tasks. They are **not**
tasks any more: colour via `B800h`, partial redraw / flicker removal, the BFS
hint, BFS-based solvability proof (implicitly proven every level start), the
`Dist` HUD field, EQU-based constants, and bounds/overflow safety in the
search. Everything in Sections 1–5 below is still genuinely open.

---

## SECTION 1 — Tier S: remaining show-stoppers

### S1. Procedural maze generation (recursive backtracker with an explicit stack)

**What it is.** Delete the 1875 bytes of `mazes` data and synthesise each level
at run time. **Why it impresses:** the program stops being a data file with a
viewer and becomes an algorithm; infinite replayability.

**Algorithm (25×25 grid, carve on odd coordinates).**

1. Fill all 625 bytes of the level with `'#'`.
2. Mark `(1,1)` as `' '`; push the packed coordinate onto an explicit stack.
3. While the stack is not empty:
   * peek the top cell `(r,c)`;
   * collect the unvisited neighbours at `(r±2, c±2)` that are still `'#'`;
   * if none exist → pop;
   * else pick one at random, carve the wall between them
     (`(r+nr)/2, (c+nc)/2` becomes `' '`), push the neighbour.
4. Knock the exit into the border: `mazes[level*MSZ + 23*MW + 23] = 'E'`.
5. Call `bfsolve` immediately — a DFS-carved maze is a tree (fully connected),
   so BFS will reach every walkable cell; this both fills `dist[]` and proves
   solvability before the player sees the screen.

**ASM-level implementation notes.**

* Data: `mstack dw 400 dup(?)`, `mtop dw 0` (explicit stack — recursion in
  `.model small` with a 100h stack would risk blowing the 256-byte stack on a
  625-cell worst case); `grid db MSZ dup(?)` if you no longer keep `mazes`.
* Push/pop are one `mov mstack[si],ax` / `inc/dec mtop` pair; the coordinate is
  packed exactly like the BFS queue (`AL = row`, `AH = col`) so `trynb`-style
  unpacking code can be reused verbatim.
* Hook points: a new `genmaze proc` called from `start_level` *before*
  `bfsolve`; `drawmaze` and `getcell` need no changes if the grid keeps the
  same layout and `'#'`/`' '`/`'E'` alphabet.

**Randomness: INT 1Ah + LCG.**

`INT 1Ah AH=00h` returns the BIOS tick count (18.2 Hz since midnight) in
`CX:DX`. Use it once to seed a linear congruential generator.

Small 16-bit LCG (period 65536, plenty for picking directions):

```asm
; ---- sketch, not existing code ----
; seed = seed * 25173 + 13849  (mod 65536); result in AX
    mov ax,seed
    mov bx,25173
    mul bx              ; DX:AX = seed*25173
    add ax,13849
    mov seed,ax
    and al,3            ; 0..3 = one of the four directions
```

Full 32-bit LCG (`seed = seed*1103515245 + 12345`, `1103515245 = 41C6 4E6Dh`)
mod 2^32 in 16-bit registers — note that only the low 16 bits of the
cross-terms survive a 32-bit result:

```asm
; ---- sketch, not existing code ----
; seedlo/seedhi hold the 32-bit seed; ALO=4E6Dh, AHI=41C6h
    mov ax,seedlo
    mov bx,ALO
    mul bx              ; DX:AX = seedlo*ALO
    mov t0lo,ax
    mov t0hi,dx
    mov ax,seedlo
    mov bx,AHI
    mul bx              ; low 16 bits of this term live at bit 16
    mov di,ax
    mov ax,seedhi
    mov bx,ALO
    mul bx              ; seedhi*AHI<<32 falls off the top of 32 bits -> drop
    add ax,di           ; middle = (seedlo*AHI + seedhi*ALO) mod 65536
    mov mid,ax
    mov ax,t0lo
    add ax,12345
    mov seedlo,ax
    mov ax,t0hi
    adc ax,0
    add ax,mid
    mov seedhi,ax
```

Seeding once at level start:

```asm
; ---- sketch ----
    mov ah,0
    int 1Ah             ; CX:DX = ticks since midnight
    mov seed,dx
    xor seed,cx         ; mix both halves so consecutive levels differ
```

**Complexity.** Generation is O(MSZ) — each cell is pushed and popped at most
once; 625 cells is microseconds. **Pitfalls in 16-bit ASM:** `DEC bl` on row 0
wraps to `FFh` (the same trap `trynb` already defends against — reuse its
`cmp bl,MH` guard); `mul` clobbers `DX`; the ±2 walk must stay inside
`1..MW-2` or you carve through the border; forgetting to place `'E'` before
`bfsolve` leaves `dist[]` all `0FFh` (BFS's no-exit early return) and the hint
forever shows `'---'`.

**Rough effort:** 60–80 lines + deleting 1875 bytes of data.

### S4. Real INT 09h keyboard hook

**What it is.** The problem says "handle keyboard interrupts" — do it literally:
replace the BIOS keyboard ISR with your own. **Why it impresses:** `CLI/STI`
vector patching, port I/O, ring buffers and `IRET` are the classic
"do you actually know hardware?" checklist.

**Vector patching (sketch, not existing code).**

```asm
    cli                         ; interrupts off while the vector is half-written
    xor ax,ax
    mov es,ax
    mov word ptr es:[9*4],   offset kbd_isr
    mov word ptr es:[9*4+2], seg kbd_isr
    sti
```

Restore on exit by writing back `es:[9*4]`/`es:[9*4+2]` (saved first, inside
the same `CLI` window), or DOS will jump into unloaded memory after you quit.

**The ISR itself, step by step.**

1. `in al,60h` — scancode. Make a copy: `test al,80h` → **bit 7 set means key
   release**; clear it to get the make code. (Press = `01h` for ESC, `48h/4Bh/
   4Dh/50h` for arrows — the same values `game:` already compares against.)
2. Push the byte into a **ring buffer**: `buf db 64 dup(?)`,
   `head db 0`, `tail db 0`. Producer: `mov bl,head; inc head; and bl,3Fh;
   mov buf[bx],al` (the `AND` makes 64 a power-of-two so wrap is free).
   Overflow policy: drop the byte, or overwrite — but never block.
3. Acknowledge the keyboard controller: read port `61h`, set bit 7, write it
   back, then clear bit 7 again (the controller latches on the 0→1→0 edge);
   some designs only need the toggle, but the full pulse is the safe form.
4. Send EOI: `mov al,20h; out 20h,al` — IRQ1 is on the master PIC, so `20h`
   to `20h` is enough (no slave involved).
5. `iret` — **not** `ret`: the CPU must pop FLAGS, CS and IP.

**Main loop change.** Replace `mov ah,0 / int 16h` at `game:` with a
non-blocking drain: `cmp tail,head; je game` … pop one byte, then run the
existing `cmp al,27` / `or al,20h` / `cmp ah,48h` dispatch unchanged. The
scancode stays in `AH`, ASCII in `AL` if you also maintain a translation table.

**What breaks if the ISR is too slow.** IRQ1 is edge-triggered through the
controller: bytes arriving while your ISR runs are lost, and typematic repeat
will stutter. Keep the ISR under a few dozen cycles — port reads and a couple
of stores only. **Never call `INT 21h`, `INT 10h` or any BIOS service from the
ISR**: DOS is non-reentrant, its internal "InDOS" flag will be corrupted and the
machine will wedge on the next disk call. That is the classic DOS re-entrancy
rule, and it is also why the game's own drawing code stays in the main loop.

**Complexity.** O(1) per interrupt; buffer of N holds N key events.
**Pitfalls:** forgetting `CLI` around the two-word vector write; forgetting to
save/restore `es`; sending EOI before reading port `60h` (the controller keeps
IRQ asserted); releasing the buffer index without `AND` masking; hooking
`INT 1Bh`/`INT 23h` and breaking Ctrl-C. **Rough effort:** 50–70 lines.

### S5. Game feel: speaker, timer, score, high-score file

**PC speaker beep (bump / win / step).** The speaker is PIT channel 2 wired to
port `61h` bits 0–1.

1. Program the divisor: `out 43h,0B6h` (channel 2, lobyte/hibyte, square wave
   mode 3), then divisor low byte to `42h`, high byte to `42h`.
   **Divisor = 1193180 / frequency.** Worked example: 440 Hz →
   `1193180/440 ≈ 2712 = 0A98h` → `out 42h,98h` / `out 42h,0Ah`. Another:
   880 Hz → `≈ 1356 = 054Ch`. Smaller divisor = higher pitch.
2. Gate the speaker on: `in al,61h; or al,03h; out 61h,al`.
3. Wait ~80 ms: `mov ah,0; int 1Ah` then spin until `DX` advances by 2 ticks
   (one tick ≈ 54.9 ms, so 2 ticks ≈ 110 ms; 1 tick ≈ 55 ms).
4. Silence: `in al,61h; and al,0FCh; out 61h,al`.

Hook points: inside `move` after a wall rejection (`je game` → beep first) and
in `doexit` for the win chime. Keep it out of `vcell` — sound must not slow the
renderer.

**Timer.** `INT 1Ah AH=00h` → `CX:DX` ticks since midnight. Store `tstart` at
`start_level`; each HUD refresh compute `elapsed = (now - tstart)`, then
`seconds = elapsed / 18` (more precisely 18.2065 ticks/s; the /18 drift is
~0.4 % per minute, fine for a game). Display `MM:SS` with two `vnum2` calls.

**Move counter.** `moves dw 0`, `inc moves` once per *accepted* move (i.e. in
`move` after the `cmp al,'#'` / `cmp al,'E'` rejections), reset at
`start_level`. Show it as a new HUD row — `drawhud` already establishes the
pattern: label at column 30, value at column 36.

**Score formula** (evaluate at `doexit`, add to a running `score`):

```
score += (par*3 - moves)*10        ; par = dist at level entry = hintd initial
       + max(0, 600 - seconds)*5   ; speed bonus
       + 500                       ; completion bonus
```

`par` comes free: `bfsolve` already computed it, so just read
`dist[index of (1,1)]` at level start.

**High-score file (`MAZE.HSC`) with INT 21h.** Ten fixed 16-byte records:

| Offset | Size | Field |
|--------|------|-------|
| 0 | 8 | player name, space-padded, not NUL-terminated |
| 8 | 4 | score, little-endian `dd` |
| 12 | 1 | level reached (0-based) |
| 13 | 2 | moves, `dw` |
| 15 | 1 | reserved (`0`) |

File = 160 bytes exactly. Services:

```asm
; ---- sketch, not existing code ----
    mov ah,3Ch              ; create/truncate
    xor cx,cx               ; attribute: normal
    mov dx,offset fname     ; 'MAZE.HSC$'
    int 21h                 ; AX = handle (CF=1 on error!)
    mov bx,ax
    mov ah,40h              ; write
    mov cx,160
    mov dx,offset hstable
    int 21h
    mov ah,3Eh              ; close
    int 21h
```

Read path: `AH=3Dh` (AL=0 read-only) → handle, `AH=3Fh` with `CX=160`,
`DS:DX = hstable`, then `AH=3Eh`. Insertion-sort the 10 records in RAM (O(k²)
with k = 10 is nothing), then rewrite the whole file.

**Pitfalls:** ignoring `CF` after every DOS call (the single most common real
bug); writing to `42h` without the `43h` command byte first (PIT ignores it);
forgetting to clear the speaker gate (constant whine); using `INT 1Ah AH=01h`
(takes 5 ms and blocks); a high-score file whose first 4 bytes are not the
expected score — validate magic/length before trusting it.
**Rough effort:** 90–120 lines including the sort.

---

## SECTION 2 — Tier A: maze mechanics (expanded table)

Every entry names where it hooks into the *current* code. Mechanic names are
stable; "hooks" reference real procedures in `MAZE.ASM`.

| # | Mechanic | Algorithm / data | Hooks into | Pitfalls |
|---|----------|------------------|------------|----------|
| A1 | **Fog of war / torch radius** | `seen db MSZ dup(0)`; on each accepted move set `seen[idx]=1` for all cells within Chebyshev radius 3 (two nested loops, ±3); render unseen cells as blank (space, `A_PATH`) or `0B0h` shade | `drawmaze` (initial pass now consults `seen`), `paintcell` (add an `cmp seen[si],0 / je darken` branch), `putplayer`/`clrplayer` already repaint one cell so the reveal costs O(r²)=49 cells per step | Must be re-checked whenever `paintcell` is called, otherwise `clrplayer` reveals the map; radius loop must clamp at 0/24 — `sub` underflow wraps like `trynb`'s `FFh` case; interacting with `refreshhint` (do you highlight hidden cells? usually no) |
| A2 | **Minimap** | Second render of the same grid at 1 char/cell in the sidebar (columns 30–54 fit exactly 25 cells); colour = same attribute but dimmer (`A_WALL` 18h vs 1Fh); player marker `@`-in-1-cell at `(prow,pcol)` | New `drawmini proc` called from `start_level` after `drawhud`; `updhud` gains a 2-cell blit of the player's old/new minimap dots — fits the existing "repaint only what changed" discipline | The sidebar already occupies rows 2–10 at column 30; a 25-row minimap needs columns 30–54 and rows 12–36 — off the 25-row screen, so either shrink (2× scale, 13 rows) or move the minimap left of the maze; never call `drawmaze` for this |
| A3 | **Keys & locked doors** | Item array `keys db 8 dup(?)` (key id → count); cells `'K'` (pick up) and `'D'` (door): door passes only if `keys[id] > 0`, then `dec` | `move` right after `getcell`: `cmp al,'D' / je try_door`; `paintcell`/`drawmaze` need new attribute cases (e.g. `A_DOOR equ 6Eh`) | Doors must not be passable by the BFS either — add the same check in `trynb`, or the hint will happily route through a locked door (correct-looking, impossible-to-follow) |
| A4 | **Teleporters** | Two (or N) cells marked `'T'` with a partner table `tpair db 2*N dup(row,col)`; on stepping onto `T`, set `prow/pcol` to the partner and repaint both cells | `move`: after committing the position, a teleport branch rewrites `prow/pcol` and calls `clrplayer`+`putplayer` again; `refreshhint` runs afterwards as usual | Recursion: teleporting *onto* another `T` must not loop — either forbid a partner that is itself a `T`, or one-shot flag; the hint distance `dist[]` was computed with `trynb` treating `T` as ordinary floor, which is *correct* only if BFS also adds the teleport edges — otherwise `hintd` lies |
| A5 | **Enemies chasing via the existing `dist[]` gradient** | Enemy struct `en db row, col, dir, alive` × N; each turn pick the neighbour with the **smallest `dist[]` value** (greedy descent — no second BFS needed) | Tick/turn hook in `move` (after the player commits); reads `dist[]` exactly like `findnb` does, so the bounds guard (`cmp bl,MH`/`cmp bh,MW`) and index maths are reused verbatim | This is the synergy: `dist[]` is a precomputed potential field pointing at the exit, and the player is usually between the enemy and the exit — a *fleeing* enemy instead should take the neighbour with `dist[enemy] < dist[player]` still true, or pick the **largest** neighbour of the player's cell; enemies drawn with `vcell` must be repainted by `clrplayer` logic when the player steps on them |
| A6 | **Moving walls / periodic hazards** | Driven by `INT 1Ah` ticks (or the INT 1Ch hook of C1): every T ticks recompute 1–4 wall cells from `tick/T mod k` and flip `mazes[idx]` between `'#'` and `' '` | Needs a `tick` dword read in `move` or in a new `idleproc`; repaint via `paintcell` (which already reads the character back through `getcell`) | The player can be standing on a cell that becomes a wall — decide policy (push vs. pass-through) or `getcell` under `@` returns `'#'` and `clrplayer` paints a wall over the player; `dist[]` goes stale: either re-run `bfsolve` (625 cells, cheap) or accept a wrong hint |
| A7 | **Dash (Shift + direction)** | Candidate position moves 2 cells; legal iff the intermediate cell is also walkable; one `dashflag db` set by scancode `SHIFT` (00h/AAh make/break via S4, or ASCII 0 with `ah=2Ah` `int 16h` shift state) | `goup/godown/goleft/goright` become `add bl,2` when `dashflag=1`, then the normal `move` validation rejects walls | Must check the *intermediate* cell too, or you dash through walls; BFS `dist[]` assumes unit steps, so `hintd` no longer equals remaining moves — either forbid dashing when the hint is on, or display `dist` as "cells" not "moves" |
| A8 | **Diagonal movement** | Four extra candidates (±1, ±1); legal only if **both** orthogonal neighbours are floor (no corner cutting) | New `go*diag` labels next to `goup`… in `goright`'s fall-through area; the double-check is two `getcell` calls before `jmp move` | `getcell` clobbers `AX/CX/SI` — the second probe must re-load `BL/BH` from registers it preserves (it does preserve `BL,BH`, which is exactly why it was written that way); BFS must also expand 8 neighbours or `hintd` overestimates |
| A9 | **Border wrap** | If the candidate row/col leaves `0..MW-1`, wrap modulo 25: `cmp bh,MW / jb ok / xor bh,bh` (or `dec bh` from 25) | In `move`, *before* `getcell` — one small `wrap` block shared by all four directions | `trynb`'s bounds checks now actively fight you: with wrap enabled you must skip them there too, or the hint stops following you across the seam; the border is solid `'#'` in the shipped mazes, so wrap needs a generated maze (S1) or punched holes |
| A10 | **Gravity mode** | After each accepted move, `while` the cell below is floor: `inc bl`, commit; the player only lands on a `'#'` | One `gravity: cmp bl,MH-1 / jae done / mov al,bl / inc al …` loop at the end of `move` before `updhud` | Must repaint every intermediate cell (up to 25 `vcell` calls — still flicker-free, that's the point of the current renderer); a landing on `'E'` must route to `doexit`; hint becomes "where to fall", which BFS already answers if gravity is modelled as free downward edges |
| A11 | **Undo ring buffer** | `undo db 512 dup(?)` of `(row,col)` pairs + `uhead/utail` bytes; push on every accepted move, pop on Backspace (`ah=0Eh` / ASCII 08h) | Push inside `move` right after `mov prow,bl`; pop in a new `undokey:` label beside `hintkey:`; repaint with the existing `clrplayer`/`putplayer`/`refreshhint`/`updhud` quartet | Undoing a move that collected a key or triggered a teleport needs the *inverse* item op — store an event byte alongside the pair (3 bytes/record); do not undo across a level boundary (`doexit` must flush `uhead`) |
| A12 | **Par score from the already-computed `dist[]`** | At `start_level`, after `bfsolve`, read `dist[index of (1,1)]` → `par db`; HUD row "Par" via `vnum3`; a move counter compare `moves = par` shows a star attribute | `start_level` (one `getcell`-style index + load), `drawhud`/`updhud` (new field at column 36, same layout), `doexit` for the score formula of S5 | `par` is only meaningful if the player may not move diagonally/wrap — every movement rule change (A8/A9) invalidates it; if the maze mutates (A6) re-run `bfsolve` first. Note this synergy: **zero extra search cost**, the hinter already did the work |
| A13 | **Collectibles gate the exit** | `cbitmask db 0`, four `'C'` cells with ids 0..3; stepping on one does `or cbitmask, 1 shl id`; `doexit` checks `cmp cbitmask,0Fh / jne still_locked` | `move`'s `cmp al,'E' / je doexit` gains a mask test; `paintcell`/`drawmaze` render `'C'` with a new attribute (e.g. `A_ITEM equ 6Eh`) | The hinter will happily route to `'E'` while it is locked — either teach `trynb` to treat a locked exit as a wall (then `hintd` = `0FFh` = `'---'` until all items are in, which is honest UX), or show `hintd` alongside a separate "Items 3/4" field |
| A14 (bonus) | **One-shot hint mine** | Cell `'M'`: on pickup, set `hinton=1` for exactly one `refreshhint` then clear | Same branches as A13 in `move`; reuses `hintkey`'s toggle logic with a `hintuses db` counter | Must call `clearhint` on expiry or the highlight lingers forever |
| A15 (bonus) | **90° maze rotation** | Rotate `mazes[level*MSZ …]` in place with the transpose+reverse identity: 625 byte loads/stores; then map `(r,c) → (c, MH-1-r)` for `prow/pcol`, `exrow/excol`, and re-run `bfsolve` | New `rotate proc` called from a key handler; `bfsolve` already re-derives the exit by scanning, so it needs no changes | Transpose needs a temp row buffer (25 bytes) because in-place swaps visit each cell twice — track with a `seen`-style bit array or only iterate `r < c`; forget `prow` remapping and the player spawns inside a wall |

Complexity notes for this tier: everything that only touches `paintcell`/
`vcell` is O(1)–O(r²) per move; anything that mutates the maze grid invalidates
`dist[]` and should re-run `bfsolve` (O(MSZ) = 625 dequeues — still under a
millisecond, so re-running on every mutation is a legitimate simplification).

---

## SECTION 3 — Tier B: game structure & polish

| # | Add-on | Design |
|---|--------|--------|
| B1 | **Title / pause / game-over state machine** | Replace the linear `start_level → game → doexit → quit` flow with a `state db` byte and one dispatcher: `0=title, 1=playing, 2=paused, 3=levelclear, 4=win, 5=gameover`. Each state is a label plus `jmp dispatcher`. The current `game:` loop becomes state 1; `hintkey`/`quit` stay as handlers inside it. The win screen's `INT 10h AX=0003h` + `vstr` + `INT 16h` block is the template for every full-screen state. |
| B2 | **Pause (P) + ESC-confirm** | In state 1, `or al,20h / cmp al,'p'` → state 2; draw a centred box with nested `vcell` loops (border `A_MSG`, fill `A_HUD`); ESC in state 1 → a "Really quit? Y/N" overlay instead of the current unconditional `jmp quit` (this only needs a second `INT 16h` read — no new interrupt work). |
| B3 | **Lives** | `lives db 3`; a trap cell `'X'` or an enemy collision does `dec lives / jz gameover`. Repaint via the existing one-cell path; `updhud` gains a field (column 36, row 6 is still free). |
| B4 | **Countdown timer → game over** | `limit dw 18*60` (60 s), `tstart` captured at `start_level`; each move compares `(now - tstart) >= limit`. Show it as `MM:SS` with two `vnum2` calls at a new HUD row; on expiry jump to state 5. |
| B5 | **High-score table** | See S5 for the exact 16-byte record and the `INT 21h` calls; add a "Hall of fame" full-screen state that renders the 10 records with `vstr`/`vnum3`. |
| B6 | **Save / load** | `MAZE.SAV` — exact byte layout below. Write with `AH=3Ch` + `AH=40h`, read with `AH=3Dh` + `AH=3Fh`, always test `CF`. |
| B7 | **Replay** | Log every accepted move as a direction byte (0..3) into `log db 1024 dup(?)`, `loglen dw 0`; a replay state feeds them back through the *same* `move` entry point at a fixed tick rate. Because rendering is already incremental, replay is exactly as flicker-free as live play. |
| B8 | **Maze editor** | Free cursor (a `@` that ignores walls) painting `'#'`/`' '`/`'E'` with keys 1/2/3; export writes the 1875-byte `mazes` block with `AH=40h`; import reads it with `AH=3Fh`. Validate on exit: exactly one `'E'`, border closed, then run `bfsolve` and require `dist[start] <> 0FFh` — solvability for free. |
| B9 | **Config file** | `MAZE.CFG`: 4 bytes `version, mw, mh, difficulty`. Read at startup *before* any `equ`-derived value is used — note `MW`/`MH` are `equ`s, so supporting variable sizes means converting them to memory variables (`mw db 25`) and auditing every `mul cl` that assumes 25 (`getcell`, `trynb`, `findnb`, `refreshhint`, `drawmaze`, `bfsolve` all use `mov cl,MW`). This is the single biggest refactor in Tier B. |
| B10 | **Achievements / stats** | Persistent 32-byte `MAZE.STA`: total moves dd, mazes finished db, deaths db, best time dw, flags dw … merged with `AH=3Fh` at startup, rewritten at `quit`. |
| B11 | **Credits animation** | Scrolling text: build a 25-row window into a long string, shift it one row per 2 ticks with `INT 1Ah`, blit using `rep movsw` from a RAM buffer (see C9) or per-row `vstr`. |
| B12 | **Localisation** | `lang db 0`; all message labels collected into one table of word offsets (`msgtab dw offset msglvl_en, offset msglvl_es, …`), addressed with `mov si,msgtab[bx]`. |

### Save file `MAZE.SAV` — exact byte layout (version 1, 32 bytes header + 625-byte grid)

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| 0 | 4 | magic | `'M','Z','S','V'` — refuse to load otherwise |
| 4 | 1 | version | `1` |
| 5 | 1 | level | `0..NLEV-1` |
| 6 | 1 | prow | player row |
| 7 | 1 | pcol | player column |
| 8 | 2 | moves | `dw`, little-endian |
| 10 | 2 | tstart | `dw`, BIOS tick at level start |
| 12 | 2 | elapsed | `dw`, ticks played so far |
| 14 | 4 | score | `dd`, little-endian |
| 18 | 1 | lives | |
| 19 | 1 | hinton | 0/1 |
| 20 | 1 | cbitmask | collectibles (0 if A13 absent) |
| 21 | 1 | lives-left pad | reserved, write 0 |
| 22 | 10 | reserved | zero-filled, room for growth |
| 32 | 625 | grid | the current level's `mazes` bytes, so a mutated/generated maze survives |

Loading: `AH=3Fh`, `CX=657`, `DS:DX=buf`; check `CX` actually returned 657,
check magic+version, copy the grid back into `mazes[level*MSZ …]`, then jump to
`start_level` (which will happily re-run `bfsolve` and repaint).

---

## SECTION 4 — Tier C: deep systems

| # | Add-on | Technique & notes |
|---|--------|-------------------|
| C1 | **INT 1Ch timer tick ISR** | `INT 1Ch` is a BIOS user hook called 18.2×/s *after* IRQ0 has been serviced, so it never needs an EOI — the safest place to put animation. Patch `es:[1Ch*4]` under `CLI/STI` like S4. ISR body: `inc tickword` (and optionally a state-2 redraw flag), then `iret`. Everything heavy stays in the main loop, which now polls `tickword` instead of blocking. Restore the vector before `AH=4Ch`. |
| C2 | **Sequenced music driven from the tick ISR** | PIT channel 0 only raises IRQ0 — it cannot make sound — so the note itself must still be played on **channel 2** exactly as in S5, while the *sequencer* runs from a hooked `INT 08h` (re-hook `INT 1Ch` instead if you do not need exact timing: it never needs an EOI). ISR body: read `notes dw 0A98h, 0, 08E8h, …` (divisors, `0` = rest) at `noteptr`, write low/high to port `42h`, gate port `61h` accordingly, advance `noteptr`, wrap at `noteend`, `iret`. **Pitfall:** an 18.2 Hz step is ~55 ms — fine for beeps, chunky for melody; see C3 for finer timing (reprogram channel 0 to 100 Hz, but then DOS's own timekeeping drifts unless you keep chaining the old vector). |
| C3 | **Finer timing** | Program PIT channel 0 with divisor `1193180/100 = 11931 = 2E9Bh` for a 100 Hz tick; hook INT 08h; increment a `centisec` counter; **you must call the old INT 08h vector yourself** or the clock stops. |
| C4 | **VGA Mode X page flipping** (graphical port) | `AX=0013h`, then unlock the CRTC (`3D4h` index `11h`, clear bit 7), set `CRTC` index `0Ch/0Dh` start address to the hidden page, wait for vertical retrace (`IN AL,0DAh`, test bit 3), flip. Two 64000-byte pages at `A000h`. Doubles as the justification for C9's RAM-buffer design in text mode first. |
| C5 | **Mouse** | `INT 33h`: `AX=0` reset (returns `AX=FFFFh` if present), `AX=1` show cursor, `AX=3` → `BX` buttons, `CX` column, `DX` row (text mode: divide pixel coords by 8×16). Map button press to the same candidate-position logic as `goright` etc. **Pitfall:** mouse ISR also runs at hardware level — keep the handler as light as the keyboard one (S4). |
| C6 | **EMS for >64 KB worlds** | `INT 67h`: `AH=40h` query free pages, `AH=41h` allocate handle → `BX` page frame segment, `AH=44h` map page into one of four 16 KB windows, `AH=45h` release. A 512×512 maze = 256 KB = 16 pages. Every far access needs `es` loaded from the frame segment — audit `mazes[si]` addressing in `getcell`/`drawmaze`, which assume `DS`. |
| C7 | **Tiny .COM port** | `.model tiny` + `org 100h`; `CS=DS=ES=SS`, so `mov ax,@data` disappears and `ES=DS` for `rep stosb` in `bfsolve` becomes free. JWasm: `jwasm -mz` already produces flat output, so the same file can be converted with small edits. Saves the DOS `exec` stub; the 1875 bytes of `mazes` become a big chunk of the 64 KB — another argument for S1's generator. |
| C8 | **Jump-table dispatch** | Replaces the 10-instruction `cmp/je` chain in `game:`. With the ASCII path already normalised by `or al,20h`, index on `AL` for letters and on `AH` for arrows after a range check. Sketch (not existing code): |

```asm
; ---- sketch, not existing code ----
    cmp al,'a'
    jb try_scancode
    cmp al,'z'
    ja try_scancode
    sub al,'a'
    xor ah,ah
    shl ax,1                ; words in the table
    mov bx,ax
    jmp cs:letter_tab[bx]
letter_tab dw hintkey, goup, game, goright, ... ; 26 entries a..z
```

*Complexity:* O(1) vs O(n) compares; more importantly it makes the key map
data. **Pitfall:** the table must be in `CS` and addressed `cs:` if the program
ever runs with `DS` pointing elsewhere (it does during `INT 21h` calls), and
the sparse scancode space (01h, 48h, 4Bh, 4Dh, 50h) still needs a range check
before indexing or you need a 128-entry table.

| # | Add-on | Technique & notes |
|---|--------|-------------------|
| C9 | **Whole-frame writes with `rep stosw` / `rep movsw`** | A full 80×25 frame is 4000 words = one `mov cx,2000 / xor ax,ax / xor di,di / rep stosw`. For the maze, prefetch the attribute in `AH` and `mov al,mazes[si]` then `stosw` — replaces the per-cell `call vcell` + bounds maths with a single store, roughly 4–8× faster for `drawmaze`. `rep movsw` copies a prepared row from a RAM buffer to `B800h` (`es` = `B800h`, `ds` = your buffer, `si`/`di` indices, mind direction flag `cld`). |
| C10 | **Double buffering in RAM** | Keep `frame dw 2000 dup(?)` as a shadow of the text screen; all drawing (`drawmaze`, `paintcell`, `updhud`) writes the buffer, and once per iteration a single `rep movsw` (2000 words) publishes it to `B800h`. This buys true tear-free updates for A6/C1 animations, at the cost of **every** write path going through the buffer — including `vcell`, which must switch its `ES` target. Complexity: O(4000) bytes copied per frame ≈ still far below one frame's 55 ms budget. |
| C11 | **Serial co-op** | `INT 14h`: `AH=0` init (set 9600 baud: `AL = 0E3h`), `AH=1` send (`AL`=byte, `DX`=port 0), `AH=2` receive → `AL`=byte, `AH` status bit 7 = data ready. Two machines, two mazes, exchange direction bytes; combine with B7's replay log so a desync can be repaired by replaying. |
| C12 | **Interrupt-safe, event-driven core** | Endgame shape: ISRs (S4 keyboard, C1 tick) only push events into ring buffers; the main loop is `wait_event → dispatch(state) → render_diff`. Nothing in the loop blocks: `INT 16h` is gone entirely, `INT 1Ah` is read from the tick ISR's shadow counter. This is the design that makes C1/C2/C5 composable without re-entrancy bugs. |

---

## SECTION 5 — Recommended roadmap

Ordered by payoff-per-hour given the code that exists *today*. Each step names
the files and procedures it touches.

1. **HUD truth: move counter + timer + par (S5, A12).**
   Touches `MAZE.ASM`: `start_level` (reset `moves`, capture `tstart`, read
   `dist[start]` into `par`), `move` (`inc moves`), `drawhud` (three new label
   rows using the existing col-30/col-36 layout), `updhud` (digits only).
   Payoff: turns the demo into a measurable game using `INT 1Ah`; ~40 lines;
   zero risk to the renderer.

2. **Speaker feedback (S5).**
   New `beep proc` + calls in `move` (wall bump) and `doexit` (chime). Touches
   `move`, `doexit`. Payoff: audible game feel for ~25 lines; introduces ports
   43h/42h/61h safely (no interrupts involved).

3. **Procedural generation (S1).**
   New `genmaze proc` + LCG; `start_level` calls it *before* `bfsolve`;
   `mazes` data (lines 66–142) is deleted. `getcell`, `drawmaze`, `paintcell`,
   `trynb`, `refreshhint` are untouched — they only care about the alphabet.
   Payoff: infinite levels, ~1875 bytes smaller binary, and it *proves* BFS
   runs on every layout; the natural prerequisite for A9 border wrap.

4. **Title/pause/game-over state machine (B1, B2).**
   Restructure `main` around a `state` byte dispatcher; `quit` gains an ESC
   confirmation; add pause overlay drawing. Touches `main`, adds
   `drawtitle`/`drawpause`. Payoff: everything later (lives, editor, replay)
   plugs into a state instead of more `jmp`s.

5. **High-score file (B5) + save/load (B6).**
   New `loadhs`/`savehs`/`saves`/`loads` procs; the 16-byte and 32+625-byte
   layouts above. Touches `doexit`, `quit`, `start_level`. Payoff: first disk
   I/O, teaches `CF` discipline on every DOS call — the habit interviewers
   probe for.

6. **INT 09h hook + ring buffer (S4).**
   New `kbd_isr`, `install_kb`/`remove_kb`; `game:` switches from `INT 16h` to
   the non-blocking drain, keeping the whole existing `cmp` dispatch.
   Touches `main` (install at entry, remove before `AH=4Ch`) and `game:`.
   Payoff: the literal "handle keyboard interrupts" requirement, plus the
   prerequisite for A7 dash (shift state) and C12. Do this *after* steps 1–5
   so any regression is obvious.

7. **Tier A showcase pick: enemies on the `dist[]` gradient (A5) + undo (A11).**
   Touches `move` (turn resolution), new `enemies proc`, `undokey` handler,
   `updhud` (lives field if B3 is in). Payoff: reuses `dist[]` as a potential
   field — the single most elegant reuse in the codebase — and undo is pure
   data-structure work.

8. **Tier C polish: INT 1Ch tick + `rep movsw` frame publish (C1, C9, C10).**
   Touches `vcell`/`drawmaze` (buffer-aware stores), `main` (vector
   install/remove), new `publish proc`. Payoff: unlocks animation (A6 moving
   walls, B11 credits) with the same zero-flicker guarantee the project is
   already known for.

Steps 1–3 alone move the project from "static puzzle demo" to "generative
game"; steps 4–6 make it a systems program (states, disk, interrupts); steps
7–8 are the differentiators that show the `dist[]`/renderer investments paying
off twice.
