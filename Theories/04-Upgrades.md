# Theory 04 — Upgrades & Improvements to MAZE.ASM

Organised in three tiers: **bug fixes**, **code-quality upgrades**, and
**feature/performance upgrades**. Each one says *what*, *why*, and *how*.

---

## Tier 1 — Bugs / correctness fixes

### 1.1 `pnum` clobbers the DOS function number ⚠️ (highest priority)

```asm
pnum:
    mov ah,0
    aam
    add ax,3030h
    push ax
    mov dl,ah
    mov ah,2
    int 21h        ; prints tens
    pop ax         ; <-- AH restored to 'tens' ASCII (e.g. 32h), NOT 2
    mov dl,al
    int 21h        ; <-- calls DOS function AH=tens+30h  (undefined!)
```

Fix:

```asm
    pop ax
    mov dl,al
    mov ah,2        ; re-arm the function number
    int 21h
```

### 1.2 No bounds check in `getcell`

`row`/`col` are never range-checked; the only thing preventing an out-of-bounds read
is the solid `#` border. If a maze ever has an opening at the edge, `mazes[si]` reads
random memory (or wraps the segment).

Fix:

```asm
    cmp bl,25
    jae reject
    cmp bh,25
    jae reject
    ...
reject:
    mov al,'#'      ; treat out-of-range as wall
    ret
```

### 1.3 Win screen immediately exits

After `msgwin` the code **falls through** into `quit`, so the message is printed and
the program dies instantly — the player barely sees it.

Fix: `mov ah,0 / int 16h` (wait for a key) → optionally loop back to a title screen.

### 1.4 `level` is never reset

If a "play again" path is ever added, `level` still holds `3` → instant win. Reset
`level` whenever the game restarts.

### 1.5 Registers that survive across `INT 21h/02h`

`draw`'s `DH` row counter depends on DOS not destroying `DH`. DOS functions are only
*guaranteed* to preserve `SI/DI/BP/DS/ES`; relying on `DH` is fragile. Fix: `push dx`
around the character output, or keep the row counter in a variable/`SI`.

### 1.6 `or al,20h` runs before every comparison

Harmless today, but it means `AL` for arrow keys becomes `20h`/`E0h`. Cleaner: only
lower-case when `AL` is an alphabetic character:

```asm
    cmp al,'A'
    jb  not_alpha
    cmp al,'Z'
    ja  not_alpha
    or  al,20h
not_alpha:
```

---

## Tier 2 — Code quality / structure

### 2.1 Magic numbers → `EQU` constants

```asm
WIDTH  equ 25
HEIGHT equ 25
CELLS  equ WIDTH*HEIGHT      ; 625
NLEV   equ 3
```

Then `mov cx,CELLS`, `mul cx`, `cmp level,NLEV`, `mov cl,WIDTH`. Changing the maze to
30×30 becomes a **one-line** change instead of hunting every literal.

### 2.2 Level pointer table instead of repeated arithmetic

```asm
levptr  dw offset mazes, offset mazes+625, offset mazes+1250
...
    mov bl,0
    mov bh,level
    shl bx,1                 ; ×2 (word table)
    mov si, levptr[bx]       ; base of this level
```

Removes the `mul` from `getcell` and makes levels data-driven (you could even point at
dynamically generated mazes).

### 2.3 Split `draw` into pieces

`draw` currently does: clear screen + maze + 6 HUD lines + player + park cursor.

```asm
draw_all   proc   ; clear, maze, hud, player
draw_maze  proc   ; only the 25×25 grid
draw_hud   proc   ; only col 30 sidebar
draw_player proc  ; only '@'
erase_player proc ; restore the cell under '@'
```

### 2.4 Document the calling convention

Add a comment block above each `proc`: inputs / outputs / clobbered registers
(the table in Theory 02 §5). Prevents the classic "which register did that `call`
destroy?" bug.

### 2.5 Data-driven messages & aligned HUD

Generate the HUD rows from a table of `(row, string)` pairs with a loop, instead of
six copy-pasted blocks.

### 2.6 Consistent error/status codes

Return `AL=0` on failure from `getcell` instead of smuggling `'#'`, or define
`CELL_WALL equ '#'`, `CELL_EXIT equ 'E'`, `CELL_EMPTY equ ' '` and compare against
those symbols.

---

## Tier 3 — Performance upgrades

### 3.1 Kill the flicker (partial redraw) ⭐ the big one

`draw` starts with `INT 10h / AL=03h`, which **re-sets the video mode and clears the
whole screen** on *every keystroke* → visible flicker.

Fix: redraw only what changed.

```asm
move:
    call getcell
    cmp al,'#'
    je  game
    ; erase old '@' by printing the cell beneath it
    mov dl,prow ... call setcur ; mov dl,' ' (or re-print mazes cell)
    mov prow,bl
    mov pcol,bh
    ; draw new '@'
    ...
    ; update only the Row/Col digits of the HUD
    jmp game_no_clear
```

Alternative: **only redraw the maze when the level changes**, and update HUD digits in
place.

### 3.2 Write directly to video memory (`B800h`)

Each text cell is 2 bytes at `ES:B800h + 2*(row*80+col)`. A `rep stosw` can paint the
whole 25×25 grid in a few microseconds — no BIOS/DOS calls at all.

```asm
    mov ax,0B800h
    mov es,ax
    mov di, (row*80+col)*2
    mov ah, 1Fh          ; white on blue attribute
    mov al, '#'
    stosw
```

Also enables **colour** (`AH` = attribute) — walls blue, exit green, player yellow.

### 3.3 Double buffering

Build the frame in a RAM buffer, then copy it to `B800h` in one `rep movsw` → zero
flicker even when fully redrawing.

### 3.4 Non-blocking input

`INT 16h / AH=01h` checks the buffer without waiting (returns ZF=1 if empty). Needed
for anything animated — timers, enemies, moving traps.

### 3.5 Avoid re-printing the whole maze

Keep a "dirty rectangle" or just two cells (old position, new position). Turns an
O(625) redraw into O(1) per move.

---

## Tier 4 — Gameplay / feature upgrades (beyond the problem statement)

| # | Upgrade | Key technique |
|---|---|---|
| 1 | Move counter & timer | `INT 1Ah` BIOS tick count (18.2 Hz) |
| 2 | PC-speaker sound effects | toggle port `61h`, program PIT `42h` |
| 3 | Colourised maze | video attribute bytes / `B800h` |
| 4 | Minimap / fog of war | visited-flag array + dimmed cells |
| 5 | Save / load high score | `INT 21h` AH=3Ch/3Fh/40h file I/O |
| 6 | Procedural maze generation | recursive backtracker + LCG random |
| 7 | BFS shortest-path hint | queue in an array, distance table |
| 8 | Enemies with simple AI | patrol paths, chase when in line-of-sight |
| 9 | Keys & locked doors | item array + collision rules |
| 10 | Title / pause / game-over screens | state variable driving the main loop |
| 11 | Graphical mode 13h tiles | 320×200×256, blit 8×8 sprites |
| 12 | Hook INT 09h / INT 1Ch | real keyboard ISR & timer tick handler |
| 13 | Maze editor mode | type `#`/space into the grid, save to file |
| 14 | Undo / replay | ring buffer of moves |
| 15 | Multiple lives + reset-on-trap | lives counter, trap cells |
| 16 | Wrap-around portals / teleports | cell types with target coordinates |
| 17 | Dash / diagonal movement | extra candidate directions + wall-slide rule |
| 18 | Difficulty scaling | more levels, moving obstacles, countdown |

(Full detail on the stand-out ones in Theory 06.)

---

## Suggested order of work

1. Fix `pnum` (Tier 1.1) — it's a real bug.
2. Add `EQU` constants (2.1) — cheap, prevents future mistakes.
3. Partial redraw (3.1) — biggest visible win.
4. Bounds check (1.2) — robustness.
5. Title screen + win pause (1.3) — makes it feel like a game.
6. Then pick feature add-ons from Theory 06.
