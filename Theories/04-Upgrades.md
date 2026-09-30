# Theory 04 — Upgrades & Improvements to MAZE.ASM

Source of truth for everything below: **`Code/MAZE.ASM` (865 lines, as-upgraded)**.
Build: `jwasm -mz MAZE.ASM` → **0 errors, 0 warnings**.

This document is split into two halves:

| Part | What it covers | Status |
|---|---|---|
| **A** | Change log of what the current file *already does*, with before/after code for every item | Done — implemented in `MAZE.ASM` |
| **B** | Remaining / possible upgrades, tiered by kind, each with concrete HOW and WHY | Not implemented — proposals |

> Nothing in Part A is aspirational: every AFTER snippet is copied verbatim from the
> current `MAZE.ASM`. Everything in Part B is *not* in the file yet.

---

## Snapshot of the current program

### Constants (all `EQU`, no magic numbers left in the code)

| Symbol | Value | Meaning | Used by |
|---|---|---|---|
| `MW` | 25 | maze width (bytes per row) | `getcell`, `drawmaze`, `bfsolve`, `trynb`, `findnb` |
| `MH` | 25 | maze height (rows) | `drawmaze`, `trynb`, `findnb` |
| `MSZ` | `MW*MH` = 625 | bytes per level | `getcell`, `drawmaze`, `bfsolve`, `trynb`, `dist`, `queue` |
| `NLEV` | 3 | number of levels | `doexit` (`cmp level,NLEV`) |
| `A_WALL` | `1Fh` | bright white on blue | wall cells |
| `A_PATH` | `07h` | grey on black | floor cells |
| `A_EXIT` | `2Eh` | bright yellow on green | `E` cell |
| `A_PLAY` | `4Eh` | bright yellow on red | `@` cell |
| `A_HINT` | `3Fh` | bright white on cyan | highlighted hint cell |
| `A_HUD` | `0Bh` | cyan on black | sidebar labels and numbers |
| `A_WIN` | `2Fh` | bright white on green | win message |
| `A_MSG` | `0Fh` | bright white on black | "press any key" line |

### Procedure inventory

| Proc | Lines | Role | Called from |
|---|---|---|---|
| `main` | 161–288 | entry + state machine: `start_level` → `game` loop → `doexit`/`quit` | — |
| `getcell` | 295–310 | `AL = mazes[level*MSZ + row*MW + col]` | `move`, `paintcell`, `refreshhint` |
| `vcell` | 321–344 | write 1 char + attribute straight into `B800h` | almost everything |
| `vstr` | 351–368 | write a `'$'`-terminated string into video RAM | `drawhud`, `updhud`, win screen |
| `vnum2` | 374–392 | print `AL` (0..99) as 2 digits | `drawhud`, `updhud` |
| `vnum3` | 398–426 | print `AL` (0..255) as 3 digits | `updhud` (distance) |
| `paintcell` | 432–447 | repaint one maze cell in its natural colour | `clrplayer`, `clearhint` |
| `clrplayer` | 452–457 | repaint the cell the player leaves | `move` |
| `putplayer` | 459–466 | stamp `@` on the current cell | `start_level`, `move` |
| `clearhint` | 471–480 | remove the highlight if one is visible | `move`, `refreshhint`, `hintkey` |
| `drawmaze` | 487–516 | full 625-cell colourised draw | **only** `start_level` |
| `drawhud` | 522–592 | sidebar labels + initial values | **only** `start_level` |
| `updhud` | 598–624 | rewrite only the Dist/Row/Col fields | `move`, `hintkey` |
| `bfsolve` | 631–728 | fill `dist[]` from the exit, once per level | `start_level` |
| `trynb` | 735–772 | bounds-checked neighbour enqueue | `bfsolve` |
| `findnb` | 779–802 | bounds-checked "is this cell one step closer?" | `refreshhint` |
| `refreshhint` | 809–863 | highlight the next best cell + set `hintd` | `move`, `hintkey` |

### Interrupt inventory of the whole program (5 `int` instructions total)

| Where | Instruction | How often |
|---|---|---|
| `main` start (`MAZE.ASM:165`) | `mov ax,0003h / int 10h` | once, at program start |
| `doexit` win screen (`MAZE.ASM:253`) | `mov ax,0003h / int 10h` | once, at the very end |
| `game:` input (`MAZE.ASM:182`) | `mov ah,0 / int 16h` | once per keystroke |
| `doexit` wait (`MAZE.ASM:265`) | `mov ah,0 / int 16h` | once, after the win text |
| `quit:` (`MAZE.ASM:286`) | `mov ah,4Ch / int 21h` | once, on exit |

No `INT 21h` text call (`AH=02h`/`09h`) and no `INT 10h` cursor call (`AH=02h`)
remains anywhere in the program.

### Rendering model

| Event | What is redrawn | Video stores |
|---|---|---|
| Program start | screen mode set once (`0003h`) | — |
| Level start (`start_level`) | whole maze + whole HUD + player | 625 maze cells + HUD |
| Wall bump / unknown key | **nothing** (`je game`) | 0 |
| Successful move | old hint cell, old player cell, new player cell, new hint cell, then HUD | ≤ 4 maze cells + 7 HUD cells |
| Hint toggle (H) | highlighted cell (on/off) + HUD distance field | ≤ 1 cell + 3 cells |
| Win | mode set (2nd and last time) + 2 message strings | 2 strings |

---

# PART A — Already implemented (change log)

## A.1 Flicker-free rendering

**Problem it fixed.** The old code redrew the *entire* screen on every keystroke and
started that redraw by re-setting video mode `03h`. A mode set clears the 4 KB text
page and re-initialises the CRTC, so every keypress produced a visible blank flash —
on top of the cost of emitting 625 characters through DOS, one interrupt each.

**BEFORE** (old `draw` proc, no longer in the file):

```asm
; OLD - called from the input loop after EVERY accepted keystroke
draw:
    mov ax,0003h        ; RESET the video mode -> full clear -> FLICKER
    int 10h
    ; then re-print all 625 maze cells with INT 21h AH=02h (1 interrupt per cell)
    ; then print the HUD labels with INT 21h AH=09h
    ; then park the cursor with INT 10h AH=02h (setcur)
    call setcur
    ret
```

**AFTER** — the mode set lives at the top of `main` only, the full maze draw is
reachable only from `start_level`, and a move repaints a handful of cells:

```asm
    mov ax,0003h                ; 80x25 text mode - clears the screen ONCE
    int 10h

; ---- a level starts here -------------------------------------------------
start_level:
    mov prow,1
    mov pcol,1
    mov hinton,0
    mov hintvis,0
    mov hintd,0
    call bfsolve                ; dist[] = shortest distance of every cell
    call drawmaze               ; full colourised maze (only on level change)
    call drawhud                ; sidebar
    call putplayer              ; stamp '@' on the start cell
```

```asm
move:
    call getcell                ; AL = cell we want to step into
    cmp al,'#'
    je game                     ; wall  -> reject, screen untouched
    cmp al,'E'
    je doexit                   ; exit  -> level complete

    call clearhint              ; repaint the old highlighted cell
    call clrplayer              ; repaint the cell the player leaves
    mov prow,bl                 ; commit the new position
    mov pcol,bh
    call putplayer              ; '@' onto the new cell
    cmp hinton,0
    je mv_hud
    call refreshhint            ; highlight the next cell of the shortest path
mv_hud:
    call updhud                 ; rewrite only the Row/Col/Dist digits
    jmp game
```

Note also the cheapest case: a **wall bump jumps straight back to `game`** with
`je game`, so a rejected key touches zero video bytes.

**Worked numbers.**

| Metric | Old | New |
|---|---|---|
| `INT 10h` mode sets per keystroke | 1 | 0 (2 in the whole program) |
| `INT 21h` calls per keystroke | ≈ 625 (one per cell) + label prints | 0 |
| Video bytes written per move | ~4000 (full screen) | ≤ 4×2 (maze) + 7 (HUD cells) = **15 bytes** |
| Full 625-cell draw | every keystroke | once per level start |

**Why it matters.** Flicker was not a cosmetic bug: each mode set cost a screen
clear plus CRTC reprogram, so the game was literally unreadable while moving. The
fix also removed ~625 interrupts of DOS work per key, which is what makes movement
feel instant.

---

## A.2 Colour + direct video memory (`vcell`)

**Problem it fixed.** Old output went through `INT 21h AH=02h` (one byte at a time,
no colour) plus `INT 10h AH=02h` to move the cursor first. Text attributes — the
whole point of a colour maze — could not be expressed that way at all, and cursor
positioning was a separate interrupt per cell.

**BEFORE** (old output style):

```asm
; OLD - print one cell: move the cursor, then emit the byte through DOS
    mov dh,row           ; row
    mov dl,col           ; col
    call setcur          ; INT 10h AH=02h, set cursor position
    mov dl,al            ; the maze character
    mov ah,02h
    int 21h              ; AH=02h: write DL to stdout - no colour, no attribute
```

**AFTER** — `vcell` computes the video RAM offset itself and stores the character
and the attribute as two adjacent bytes:

```asm
vcell proc
    push es
    push di
    push bx
    push ax                     ; remember character + attribute
    mov ax,0B800h               ; colour text segment
    mov es,ax
    mov al,dh                   ; row
    xor ah,ah
    mov bl,80                   ; 80 cells per row
    mul bl                      ; AX = row * 80
    xor bh,bh
    mov bl,dl                   ; column
    add ax,bx                   ; AX = row*80 + col
    shl ax,1                    ; 2 bytes per cell
    mov di,ax
    pop ax                      ; character + attribute back
    mov es:[di],al              ; even byte = character
    mov es:[di+1],ah            ; odd  byte = attribute
    pop bx
    pop di
    pop es
    ret
vcell endp
```

Every text cell of mode `03h` is two bytes at `ES:B800h + (row*80+col)*2`:
even byte = ASCII, odd byte = attribute (high nibble = background, low nibble =
foreground). Because `vcell` takes `AH` as the attribute, colour became free for
all callers — `drawmaze` picks `A_WALL`/`A_PATH`/`A_EXIT` per character,
`putplayer` stamps `A_PLAY`, `refreshhint` lights `A_HINT`.

**Why it matters.** One store pair replaces *two* interrupts and a cursor
positioning. It is also the enabler for every colour in the game and for the
partial-repaint trick in A.1 (you can change exactly one cell without disturbing
its neighbours).

---

## A.3 `EQU` constants instead of magic numbers

**Problem it fixed.** The old file used raw literals — `625`, `25`, `3` — sprinkled
through arithmetic, loop limits and comparisons. A grid resize meant hunting every
occurrence, and a typo (`mov cx,652`) assembled happily.

**BEFORE:**

```asm
; OLD - no constants
    mov cx,625          ; bytes per level, written out everywhere
    mov cl,25           ; row stride
    cmp level,3         ; number of levels
    cmp dl,25           ; loop limit
```

**AFTER:**

```asm
MW      equ 25                  ; maze width  (columns, bytes per row)
MH      equ 25                  ; maze height (rows)
MSZ     equ MW * MH             ; bytes per level (625)
NLEV    equ 3                   ; number of levels
```

```asm
    mov cx,MSZ
    mul cx                      ; AX = level * 625
    ...
    mov cl,MW
    mul cl                      ; AX = row * 25
    ...
    cmp level,NLEV
    jb start_level
    ...
    cmp dl,MW
    jb dm_col
    cmp dh,MH
    jb dm_row
```

Related: the colour attributes were also given names (`A_WALL equ 1Fh` … `A_MSG
equ 0Fh`), so `paintcell`/`drawmaze` compare against `'#'`/`'E'` but *paint* with
symbols whose comment states the exact colour.

**Worked numbers.** `MSZ equ MW * MH` = 25 × 25 = 625; the maze data block is
`NLEV * MSZ` = 3 × 625 = 1875 bytes, and `getcell`'s index formula
`level*MSZ + row*MW + col` maxes out at `2*625 + 24*25 + 24` = 1874 — the last
byte of the data, i.e. exactly in range.

**Why it matters.** Changing the grid to 30×30 is now a one-line edit of `MW`/`MH`
(and a data edit), not a grep for `25` across the file. The assembler recomputes
`MSZ` everywhere.

---

## A.4 The `pnum` / DOS-function bug — eliminated by removing DOS text output

**Problem it fixed.** The old number printer loaded `AH=2` for `INT 21h` function
02h, printed the tens digit, then `pop ax` restored a value whose `AH` byte was the
*tens digit in ASCII* — not `2`. The second `int 21h` therefore invoked whatever
DOS function happened to have that number.

**BEFORE** (old `pnum`, removed):

```asm
pnum:
    mov ah,0
    aam                     ; AH = tens, AL = units
    add ax,3030h            ; both digits become ASCII
    push ax
    mov dl,ah
    mov ah,2
    int 21h                 ; prints tens digit  (AH=02h = putc)
    pop ax                  ; *** AH is now the TENS ASCII, e.g. '1' = 31h ***
    mov dl,al
    int 21h                 ; calls DOS function AH = tens + 30h  -> UNDEFINED
```

What "undefined" means concretely: `AH` after the pop is always in `30h..39h`,
and those DOS function numbers are:

| Tens digit | `AH` | Real DOS function actually called | Effect on screen |
|---|---|---|---|
| `'0'` | `30h` | get DOS version | nothing printed |
| `'1'` | `31h` | **terminate and stay resident** (AL = exit code!) | program may end right there |
| `'2'` | `32h` | reserved | nothing printed |
| `'4'` | `34h` | get INDOS flag | nothing printed |
| `'5'` | `35h` | get interrupt vector | nothing printed |
| `'6'` | `36h` | get free disk space | nothing printed |
| `'9'` | `39h` | make directory | nothing printed |

So the units digit either never appeared, or (worst case, tens = `'1'`) the routine
called the TSR function with `AL` = units ASCII — a units digit of `'5'` (35h)
would make the program *terminate-and-stay-resident* mid-print. Nothing in the
function list ever writes a character to stdout.

**AFTER** — there is no `INT 21h` text output left to be buggy. Numbers are printed
by `vnum2`/`vnum3`, which never touch DOS; they only call `vcell` (A.2):

```asm
vnum2 proc
    push bx
    push cx
    mov cl,ah                   ; attribute
    mov ah,0
    aam                         ; AH = tens, AL = units
    add ax,3030h                ; both digits become ASCII
    mov bl,al                   ; units character
    mov al,ah                   ; tens character
    mov ah,cl
    call vcell
    inc dl
    mov al,bl
    mov ah,cl
    call vcell
    pop cx
    pop bx
    ret
vnum2 endp
```

**Why `vnum2`/`vnum3` cannot reproduce the bug:**

1. They never execute `int 21h`, so there is no function number to clobber — the
   class of bug is *gone*, not patched.
2. `AH` is saved once in `CL` (`mov cl,ah`) and reloaded (`mov ah,cl`) before every
   `vcell` call, so the attribute cannot be corrupted between digits either.
3. `vcell` itself pushes/pops `AX`, `BX`, `DI`, `ES`, so the digit-carrying `BL`
   and the position `DL` survive each call.
4. The old `aam`-only approach also silently broke for values ≥ 100 (`aam` on
   AL=140 yields AH=14, which `add ax,3030h` turns into `'>'`, not `'1'`). That is
   precisely why the distance printer is a *separate* routine — see A.8.

**Why it matters.** This was a real, load-bearing bug (TSR/undefined behaviour),
not a style issue. It is fixed structurally: the whole DOS text layer was deleted.

---

## A.5 Win screen now waits for a key

**Problem it fixed.** Old code printed `msgwin`/`msgany` and then **fell
through** into `quit:`, so DOS returned before the player could read anything.

**BEFORE:**

```asm
; OLD
    mov dx,offset msgwin
    mov ah,09h
    int 21h             ; print "You finished all 3 levels!"
    mov dx,offset msgany
    int 21h             ; print "Press any key to exit"
    ; <-- no wait here: control falls straight into
quit:
    mov ah,4Ch
    int 21h             ; ...the program dies instantly
```

**AFTER** — the win branch sets the mode once more (second and last `INT 10h`),
draws both strings with `vstr`, then blocks on `INT 16h`:

```asm
doexit:
    inc level
    cmp level,NLEV
    jb start_level
    ; all levels done: win screen, wait for a key, leave
    mov ax,0003h
    int 10h
    mov dh,12
    mov dl,26
    mov si,offset msgwin
    mov ah,A_WIN
    call vstr
    mov dh,14
    mov dl,29
    mov si,offset msgany
    mov ah,A_MSG
    call vstr
    mov ah,0
    int 16h                     ; let the player read the message
    jmp quit
```

Positioning check: `msgwin` is 27 chars, so column 26 centres it on 80 columns
(`(80-27)/2 ≈ 26`); `msgany` is 22 chars, column 29 (`(80-22)/2 = 29`); rows 12 and
14 sit just below centre.

**Why it matters.** The reward for finishing 1875 bytes of maze was previously
invisible. The `jmp quit` also makes the control flow explicit — no fall-through
assumption.

---

## A.6 Bounds checking in the BFS helpers + queue-full guard

**Problem it fixed.** Neighbour coordinates are produced by `dec`/`inc` on a byte,
so row `0 - 1` becomes `255` and `24 + 1` becomes `25`. An unchecked
`dist[row*25+col]` with row=255 would index 6375 bytes past `dist` — wild write.

**BEFORE:** no range check at all in the old neighbour handling (the only thing
keeping `getcell` in range was the solid `#` border — and `getcell` *still* has no
check; see Part B, Tier 1).

**AFTER** — `trynb`:

```asm
trynb proc
    cmp bl,MH
    jae tn_done                 ; rows are 0 .. MH-1 (BL may be 255 after DEC)
    cmp bh,MW
    jae tn_done                 ; columns are 0 .. MW-1
```

`findnb` uses the identical prologue:

```asm
findnb proc
    cmp bl,MH
    jae fn_no
    cmp bh,MW
    jae fn_no
```

and the queue write is capacity-checked before it happens:

```asm
    mov ax,btail
    cmp ax,MSZ
    jae tn_done                 ; queue full (cannot happen: 625 cells max)
    shl ax,1
    mov di,ax
    mov al,bl
    mov ah,bh
    mov queue[di],ax
    inc btail
```

**Worked numbers.** The comparison is `jae` (unsigned), which is required: after
`dec bl` with `bl=0`, `bl = 0FFh = 255`, and `cmp bl,MH` / `jae` correctly rejects
it — a signed `jge` would treat 255 as −1 and *accept* it. Maximum queue length is
`MSZ` = 625 entries because BFS marks a cell visited (`dist != 0FFh`) before
enqueueing, so each of the 625 cells can be enqueued at most once; the guard is
therefore belt-and-braces, exactly as the comment says.

**Why it matters.** The BFS runs on data (`dist`, `queue`) that sits right next to
other variables in `.data`; an overflow index would corrupt `bhead`/`btail`/`lbase`
and then corrupt itself — a classic self-amplifying memory bug.

---

## A.7 Byte-distance overflow guard and the `0FFh` sentinel

**Problem it fixed.** `dist` is a **byte** array. Without a guard, a cell at
distance 255 would `inc` to 0 (wrap), which would (a) look "unvisited" — the
unvisited test *is* `== 0FFh`… and worse, (b) a wrap to `00h`/`FEh` would poison
the `wantd` search and make the hint light up arbitrary cells.

**AFTER** — the sentinel convention, declared once:

```asm
dist    db MSZ dup(0FFh)        ; distance of a cell to 'E', FFh = not reached
```

cleared in bulk at the start of every solve:

```asm
    push ds
    pop es                      ; ES = DS for REP STOSB
    mov di,offset dist
    mov cx,MSZ
    mov al,0FFh
    rep stosb
```

and the overflow guard in the expansion loop:

```asm
    mov dl,dist[si]
    inc dl                      ; distance the neighbours will get
    cmp dl,0FEh
    jae bs_loop                 ; byte would overflow: stop expanding
```

plus the matching "already visited?" test in `trynb`:

```asm
    mov al,dist[si]
    cmp al,0FFh
    jne tn_done                 ; already visited
    mov dist[si],dl
```

**Worked numbers.** `dl` after `inc` is the distance the *neighbours* would get.
`cmp dl,0FEh / jae` rejects `FEh` (254) and `FFh` (255), so the largest distance
ever stored is `FDh` = 253 and `inc` can never wrap. Measured maximum distance is
**140** (level 3), i.e. well under the 253 ceiling — the guard is safety margin
for future, longer mazes. Because `0FFh` is reserved as "unreachable", `bfsolve`
also short-circuits cleanly: if a level had no `E`, it returns early and every cell
stays `0FFh`, which `updhud` renders as `---`.

**Why it matters.** It turns a silent wrap-around into a defined "stop expanding"
behaviour, and it keeps the sentinel `0FFh` unambiguous — one constant means
"unknown" to the solver, the hint and the HUD simultaneously.

---

## A.8 Three-digit distance printer `vnum3`

**Problem it fixed.** `vnum2` prints exactly two digits and is documented for
`0..99`. The BFS start distance on level 3 is **140**, which `aam`-style 2-digit
formatting would mangle (140 → `aam` gives AH=14 → `+30h` = `'>'`, so the player
would see `>0`). A distance field also has to keep a fixed width so the HUD does
not shift.

**AFTER:**

```asm
; ===========================================================================
; vnum3 - print AL (0..255) as exactly three digits (leading zeros)
;         used for the BFS distance, which can pass 99 (e.g. level 3 = 140)
; ===========================================================================
vnum3 proc
    push bx
    push cx
    mov cl,ah                   ; attribute
    xor ah,ah
    mov bl,100
    div bl                      ; AL = hundreds, AH = remainder
    mov ch,ah                   ; CH = value still to print (0..99)
    add al,30h
    mov ah,cl
    call vcell
    inc dl
    mov al,ch
    xor ah,ah
    mov bl,10
    div bl                      ; AL = tens, AH = units
    mov bl,ah                   ; BL = units
    add al,30h
    mov ah,cl
    call vcell                  ; tens digit
    inc dl
    mov al,bl
    add al,30h
    mov ah,cl
    call vcell                  ; units digit
    pop cx
    pop bx
    ret
vnum3 endp
```

Called from exactly one place — the distance field in `updhud`:

```asm
    mov al,hintd
    call vnum3                  ; e.g. '140'
```

**Worked numbers (verified by BFS simulation).**

| Level | Start-cell distance to `E` | Printed as | Walkable cells reachable | Exit |
|---|---|---|---|---|
| 1 (index 0) | 100 | `100` | 299 / 625 | (23,23) |
| 2 (index 1) | 68 | `068` | 292 / 625 | (23,23) |
| 3 (index 2) | 140 | `140` | 287 / 625 | (23,23) |

`vnum3` always writes three cells (leading zeros), so the distance field is a fixed
3-character slot at column 36 — the same width the `---` placeholder occupies when
the hint is off, which is why toggling H never moves the rest of the HUD.

**Why it matters.** Level 3's 140 is not an edge case any more — it is a normal
play distance, so a 3-digit field is a correctness requirement, not polish. The
`div bl` route also avoids `aam`'s 0..99 limitation entirely.

---

## A.9 Hint state machine and the H toggle

**Problem it fixed.** The old program had no hint at all; even after a naive BFS
was bolted on, "is a highlight currently on screen?", "which cell?", "what distance?"
and "what distance am I searching for?" need explicit state, otherwise repaints
leak stale highlights or the HUD shows a distance for a hint that is off.

**The state (all in `.data`, lines 44–49):**

```asm
hinton  db 0                    ; hint mode   0 = off, 1 = on
hintvis db 0                    ; 1 = a hint cell is currently highlighted
hrow    db 0                    ; highlighted cell row
hcol    db 0                    ; highlighted cell column
hintd   db 0                    ; player -> exit distance (FFh = unknown)
wantd   db 0                    ; distance being searched for in findnb
```

| Variable | Set by | Read by | Invariant |
|---|---|---|---|
| `hinton` | `hintkey` (toggle), `start_level` (clear to 0) | `move`, `hintkey`, `updhud` | 0/1 only |
| `hintvis` | `refreshhint` (=1), `clearhint` (=0) | `clearhint` | 1 ⇒ `(hrow,hcol)` is currently painted `A_HINT` |
| `hrow`,`hcol` | `findnb` (on CF=1) | `clearhint` | valid only while `hintvis=1` |
| `hintd` | `refreshhint` (distance or `0FFh`), `hintkey` (0 when off), `start_level` (0) | `updhud` | `0FFh` ⇒ print `---` |
| `wantd` | `refreshhint` (`hintd-1`) | `findnb` | one step closer than the player |

**The toggle (verbatim):**

```asm
hintkey:
    mov al,hinton
    xor al,1                    ; toggle 0 <-> 1
    mov hinton,al
    cmp hinton,0
    je hk_off
    call refreshhint
    jmp hk_done
hk_off:
    call clearhint              ; remove the highlight
    mov hintd,0
hk_done:
    call updhud                 ; show '---' when the hint is off
    jmp game
```

Reached from the input loop *after* `or al,20h`, so both `H` and `h` work:

```asm
    or al,20h                   ; force lower case: 'W' and 'w' both become 'w'

    cmp al,'h'
    je hintkey
```

**The compute side (`refreshhint`) in outline:** clear any old highlight → read
`dist[player]` into `hintd` → if `0FFh` or `0`, nothing to show (`hintd := 0FFh`)
→ else set `wantd := hintd - 1` → probe up/down/left/right with `findnb` (each
bounds-checked, A.6) → the first neighbour whose `dist == wantd` wins → repaint that
cell with its real character but `A_HINT` attribute, set `hintvis=1`.

```asm
    mov al,dist[si]
    mov hintd,al                ; remaining optimal distance
    cmp al,0FFh
    je rh_none
    cmp al,0
    je rh_none                  ; standing on the exit: nothing to show
    dec al
    mov wantd,al                ; the next cell must be one step closer
```

```asm
rh_got:
    mov bl,hrow
    mov bh,hcol
    call getcell                ; keep the real character on the cell
    mov ah,A_HINT               ; ...but light it up in the hint colour
    mov dh,bl
    mov dl,bh
    call vcell
    mov hintvis,1
    ret
```

**Cost during play.** BFS runs **once per level** (`call bfsolve` in
`start_level`), so per move the hint costs: `clearhint` (0 or 1 cell) +
`refreshhint` (≤ 1 cell) + `updhud` (3 cells) — all O(1), never O(625).

**Why it matters.** `hintvis` is the key that makes partial repaint correct:
`clearhint` is a *no-op* when nothing is highlighted (`cmp hintvis,0 / je ch_done`),
so the common case (hint off) writes zero extra bytes, and the hint-on case never
paints a cell twice or forgets one.

---

# PART B — Remaining / possible upgrades

None of the following is in `MAZE.ASM` today.

| Tier | Theme | Risk if skipped | Effort |
|---|---|---|---|
| 1 | Correctness / robustness | latent memory bugs when data changes | small, do first |
| 2 | Structure / maintainability | slow, error-prone edits as features grow | medium |
| 3 | Performance | only matters on slow machines / bigger grids | medium |
| 4 | Features | gameplay depth | large — see Theory 06 |

## Tier 1 — Correctness / robustness

### B1.1 Guard `getcell` against out-of-range row/col

**Gap.** BFS has bounds checks (`trynb`, `findnb`) but `getcell` — the routine every
gameplay path uses (`move`, `paintcell`, `refreshhint`) — does not. Today the only
protection is the solid `#` border: a candidate move into row 24 is rejected *after*
`getcell` already read `mazes[level*625 + 24*25 + col]`, and if a border cell were
ever opened up, row 25 would read `mazes[625+…]` = the *next level's* data (or
`dist[]` on the last level), return a non-`'#'` byte, and let `putplayer` write to
video row 25 — past the 4000-byte text page.

**HOW:**

```asm
getcell proc
    cmp bl,MH                   ; row must be 0 .. MH-1
    jae gc_oob
    cmp bh,MW                   ; col must be 0 .. MW-1
    jae gc_oob
    ; ... existing index maths: level*MSZ + row*MW + col ...
    mov al,mazes[si]
    ret
gc_oob:
    mov al,'#'                  ; out of range == wall == always rejected
    ret
getcell endp
```

**WHY.** Defence in depth: the border is a *data* property, the check is a *code*
property. Returning `'#'` reuses the existing wall logic (`cmp al,'#' / je game`),
so no caller needs to change — a rejected key still writes 0 video bytes.

### B1.2 Validate maze row widths at assembly time

**Gap.** The 1875 bytes are 75 separate `db '…'` lines. A 24- or 26-character row
assembles silently and then shifts *every* subsequent level by one byte — the bug
surfaces only as a broken maze.

**HOW** — a row macro that measures the emitted bytes with the location counter:

```asm
mrow macro txt
rstart  = $
        db txt
        if ($ - rstart) ne MW
          .err <maze row width != MW>
        endif
endm

; data then becomes:
mrow '#########################'
mrow '#       #             # #'
...
```

Also worth asserting the block sizes once after the data:

```asm
if ($ - mazes) ne NLEV * MSZ
    .err <mazes block is not NLEV*MSZ bytes>
endif
```

**WHY.** Turns a silent, cascading data corruption into an assembly error on the
line that is wrong. Cost: zero at run time (all checks happen in `jwasm`).

### B1.3 Handle `AL = 00h / E0h` extended keys explicitly

**Gap.** `or al,20h` is executed on *every* key, including keys that have no ASCII
code (`AL=00h` for F-keys/navigation prefixes, `AL=0E0h` on BIOSes that use the
E0 prefix). Today `00h|20h = 20h` and `E0h|20h = E0h` happen not to collide with
`'h'/'w'/'a'/'d'`, so it works — but only by accident, and a future letter check
could collide without warning.

**HOW** — skip lower-casing for non-ASCII keys, before the comparisons:

```asm
    mov ah,0
    int 16h                     ; AL = ASCII, AH = scancode
    cmp al,0
    je  scan_only               ; no ASCII: dispatch on AH only
    cmp al,0E0h
    je  scan_only               ; E0 prefix: dispatch on AH only
    or  al,20h                  ; safe now: real character
    ...
scan_only:
    ; compare AH only (arrows, ESC scancode) - do not test AL letters
```

A related tidy-up: only lower-case alphabetic characters, so punctuation/digits are
left untouched:

```asm
    cmp al,'A'
    jb  not_alpha
    cmp al,'Z'
    ja  not_alpha
    or  al,20h
not_alpha:
```

**WHY.** Makes the input decoder's contract explicit ("letters are ASCII 41h..5Ah,
everything else is dispatched on scancode") instead of relying on bit-5 arithmetic
not colliding with any future key.

### B1.4 Reset everything for a replay

**Gap.** There is no restart path today; `level` is only ever incremented
(`inc level`) and the program quits after level 3. The moment a "play again?"
screen is added, `level` would still hold `3` → instant win, and stale
`prow/pcol/hint*` would leak into the new game.

**HOW** — give the level-start code a true entry point that zeroes all state:

```asm
playagain:
    mov level,0
    mov hinton,0
    mov hintvis,0
    mov hintd,0
    ; move counter / timer / score fields would be cleared here too
start_level:
    mov prow,1
    mov pcol,1
    ...
```

and point the win screen's `int 16h` branch at `playagain` instead of `jmp quit`.

**WHY.** State that is initialised only once at assembly time is fine for a
one-shot program and wrong for any loop-around. Listing every variable in one
place also documents the game state.

### B1.5 Check `mul`/`div` overflow if the grids grow

**Gap.** All index maths fits comfortably today (max index 1874 < 65535; `mul cl`
products ≤ 24×25 = 600), but none of it is checked. Two silent failure modes:

| Expression | Site | Failure when data grows |
|---|---|---|
| `mov cx,MSZ / mul cx` (level × cells) | `getcell`, `drawmaze`, `bfsolve` | product > 65535 → `AX` wraps, `DX` holds the lost high word |
| `mov cl,MW / mul cl` (row × width) | 8 places | 8-bit `mul cl` result > 65535 impossible (`max 255*255`), but > 255 in `AL`… result is in `AX`, fine; the real limit is `MSZ` growth |
| `xor dx,dx / div cx` (index ÷ width) | `bfsolve` exit lookup | `DX >= CX` → divide exception (#DE) — safe today only because `DX` is zeroed first |
| `mov bl,80 / mul bl` (row × 80) | `vcell` | row > 8191 impossible (byte), safe |

**HOW** — assert the product after each `mul` that feeds an index:

```asm
    mov al,level
    xor ah,ah
    mov cx,MSZ
    mul cx                      ; DX:AX = level * MSZ
    or  dx,dx
    jnz index_overflow          ; > 65535 -> refuse to run
    mov si,ax
```

and for `div`, guarantee a zero high word (already done in `bfsolve` via
`xor dx,dx` — worth a comment so nobody "optimises" it away).

**WHY.** A wrapped index is the worst kind of bug: it never traps, it just reads
and writes the wrong memory. Two instructions make it an explicit, debuggable
failure.

## Tier 2 — Structure

### B2.1 Split `drawhud` / `updhud` further

**Gap.** `drawhud` is eight near-identical blocks of
`mov dh,r / mov dl,30 / mov si,offset X / mov ah,A_HUD / call vstr`, and `updhud`
repeats the same address setup three times. Adding one HUD field means editing
several places.

**HOW** — two small helpers:

```asm
; print label SI at (DH, 30)
hudlabel proc
    mov dl,30
    mov ah,A_HUD
    call vstr
    ret
hudlabel endp

; print the number AL at (DH, 36) as N digits (NC = 2 or 3)
hudnum proc
    mov dl,36
    mov ah,A_HUD
    cmp cl,3
    je  hn3
    call vnum2
    ret
hn3:
    call vnum3
    ret
hudnum endp
```

`drawhud` then becomes `mov dh,2 / mov si,offset msglvl / call hudlabel` per line.

**WHY.** Column 30 and column 36 are layout facts that currently exist as literals
in 16 places; centralising them means a sidebar redesign is two edits.

### B2.2 Jump-table dispatch instead of the `cmp` chain

**Gap.** `game:` is a linear compare chain: 11 compares per keystroke, each
sequential even for the most common key.

**HOW** — dispatch on the scancode first (arrows/ESC), then on the ASCII:

```asm
    mov ah,0
    int 16h
    xor bx,bx
    mov bl,ah
    cmp bl,50h                  ; largest scancode we handle
    ja  game
    shl bx,1
    mov si,bx
    jmp word ptr keytab[si]     ; table in .data -> DS addressing is correct

    ; keytab (word offsets), index = scancode:
    ;   01h -> quit        48h -> goup      4Bh -> goleft
    ;   50h -> godown      4Dh -> goright   everything else -> game
```

(The table must live in `.data` — in `.model small`, `DS != CS`, so a table in
`.code` would need a `cs:` override on every indirect jump.)

**WHY.** O(1) dispatch instead of O(n) comparisons, and adding a key is one table
entry instead of two `cmp/je` pairs; the scancode/ASCII distinction stops being
interleaved logic.

### B2.3 Macro for "print label + number"

**Gap.** Every HUD field is assembled by hand from 6–8 instructions.

**HOW:**

```asm
hfield macro r, labelstr, value, digits
    mov dh,r
    mov si,offset labelstr
    mov ah,A_HUD
    call vstr
    mov dh,r
    mov al,value
    mov cl,digits
    mov ah,A_HUD
    call hudnum
endm

    hfield 2, msglvl, level, 2
    hfield 4, msgrow, prow,   2
    hfield 5, msgcol, pcol,   2
```

**WHY.** Field rows become declarative data-like lines; a typo in a row number is
visible at a glance rather than buried in a instruction block.

### B2.4 Move HUD row/column numbers into a table

**Gap.** Row numbers (`2,3,4,5,7,8,9,10`) and column (`30`, value column `36`) are
scattered through `drawhud` and `updhud`.

**HOW:**

```asm
hudrow  db 2,3,4,5,7,8,9,10
hudstr  dw offset msglvl, offset msgdst, offset msgrow, offset msgcol
        dw offset msgkey, offset mghint, offset msgend, offset msgesc
HUDCNT  equ 8
HUD_COL equ 30
HUD_VAL equ 36
...
    xor cx,cx                   ; field index
dh_loop:
    mov si,cx
    mov dh,hudrow[si]
    mov dl,HUD_COL
    shl si,1
    mov si,hudstr[si]
    mov ah,A_HUD
    call vstr                   ; vstr preserves CX
    inc cx
    cmp cx,HUDCNT
    jb dh_loop
```

**WHY.** The sidebar layout becomes one table you can re-order or extend without
touching control flow, and `updhud`'s address literals (`dh=3/4/5, dl=36`) get a
single definition to reference.

### B2.5 `NUMLEVEL`-driven level pointer table (kill the repeated multiply)

**Gap.** Every level lookup does `mov cx,MSZ / mul cx` — in `getcell` (called 1–2×
per keystroke), `drawmaze`, and `bfsolve`.

**HOW:**

```asm
NUMLEVEL equ 3
levptr   dw offset mazes
         dw offset mazes + 1*MSZ
         dw offset mazes + 2*MSZ

if (lengthof levptr / 2) ne NUMLEVEL
    .err <levptr does not cover NUMLEVEL levels>
endif
```

```asm
    mov si,level
    xor bh,bh / mov bl,0        ; SI = level (zero-extended)
    shl si,1                    ; words
    mov si,levptr[si]           ; SI = base of this level - no mul at all
    mov al,bl / mul cl          ; row*MW ... (or keep the row stride mul)
    add si,ax
    mov al,mazes[si]
```

**WHY.** Removes a 16-bit `mul` from the hottest path, makes the level count
data-driven (`NUMLEVEL` feeds both the table check and `cmp level,NLEV`), and
enables future levels that are *not* contiguous (e.g. generated mazes) by simply
changing pointer values.

## Tier 3 — Performance

### B3.1 Buffer the whole frame in RAM, blit with `rep movsw`

**HOW:**

```asm
frame   dw 2000 dup(0)          ; 80*25 words = one full text screen (4000 bytes)
...
blit proc
    mov ax,0B800h
    mov es,ax
    mov si,offset frame
    xor di,di
    mov cx,2000                 ; 2000 WORDS
    rep movsw
    ret
blit endp
```

Drawing code writes to `frame` (always safe, never visible mid-update), then
`blit` publishes it.

**WHY / worked numbers.** 2000 word-copies is roughly 10–12 cycles each on an
8086 ⇒ ~20–24k cycles ≈ **4–5 ms at 4.77 MHz** for a *complete* screen update.
Cost: 4000 bytes of DGROUP. It buys atomic screen updates — useful once anything
time-based (timers, animations) can update the display mid-frame.

### B3.2 Do not redraw the HUD label strings

**Gap.** `start_level` calls `drawhud`, which rewrites all eight label strings on
every level change — although the screen is *not* cleared between levels (the mode
set happens only at program start and at the win screen), so those strings are
already correct from the previous level.

**HOW** — split one-shot labels from per-level values:

```asm
hudinit  proc  ; called ONCE after the initial INT 10h: labels only (msglvl..msgesc)
hudreset proc  ; called per level: Level digit, '---', Row, Col   (= updhud + level digit)
```

`start_level` then calls `bfsolve`, `drawmaze`, `hudreset`, `putplayer` — no label
traffic at all.

**WHY.** Cuts the per-level redraw from 625 cells + ~120 label characters to 625
cells + ~12 value cells. Small now, but it is the same discipline that made
movement flicker-free (A.1), applied one level up.

### B3.3 Write whole rows with `stosw`, compute the video offset once per row

**Gap.** `drawmaze` calls `vcell` 625 times, and each `vcell` re-derives
`row*80+col` from scratch — 625 `mul bl` operations per level draw (~70–80 cycles
each ⇒ tens of thousands of cycles just recomputing the same 25 row bases).

**HOW** — point `ES:DI` at the start of the row once, then store words:

```asm
dm_row:
    mov ax,0B800h / mov es,ax
    mov al,dh / xor ah,ah
    mov di,80 / mul di          ; AX = row*80
    shl ax,1 / mov di,ax        ; DI = byte offset of this row
    xor dl,dl
dm_col:
    mov al,mazes[si]
    mov ah,A_PATH
    cmp al,'#'   / jne dm_nw / mov ah,A_WALL
dm_nw:
    cmp al,'E'   / jne dm_ne / mov ah,A_EXIT
dm_ne:
    stosw                       ; ES:[DI] = AX, DI += 2  - no offset maths
    inc si
    inc dl
    cmp dl,MW
    jb dm_col
    inc dh
    cmp dh,MH
    jb dm_row
```

`stosw` writes character **and** attribute in one instruction; the per-cell cost
drops to a load, two compares and a store.

**WHY.** Full-maze draw goes from "625 × (mul + shifts + stores)" to "25 muls +
625 stores". Matters most for level starts and if grids grow to 40×40+.

### B3.4 Non-blocking input (`INT 16h AH=01h`)

**Gap.** `mov ah,0 / int 16h` *blocks*; the loop cannot do anything between
keystrokes, so timers, animation or a live clock are impossible.

**HOW:**

```asm
game:
    mov ah,01h
    int 16h                     ; ZF=1 -> keyboard buffer empty
    jz  no_key
    mov ah,00h
    int 16h                     ; consume it: AL/AH as before
    ; ... existing ESC / hint / direction dispatch ...
no_key:
    ; ... poll INT 1Ah, advance animation, redraw changed cells ...
    jmp game
```

**WHY.** Required for any feature that must visibly progress while no key is
pressed (move counter clock, moving obstacles, blink effects). Note it pairs with
B3.1: with a frame buffer you can recompute and blit on a schedule, not on a
keystroke.

## Tier 4 — Features (brief; full detail in Theory 06)

| # | Feature | Key technique / interrupt | Notes for MAZE.ASM |
|---|---|---|---|
| 1 | Timer / move counter / score | `INT 1Ah` AH=00h → `CX:DX` BIOS ticks (18.2/s) | display via `vnum3`/`vnum2` in a new HUD field |
| 2 | PC-speaker sound | toggle port `61h` bits 0–1, program PIT channel `42h` (port `43h`) | beep on wall bump / level complete |
| 3 | Fog of war | visited-flag byte array + dim attribute per cell | reuse `paintcell` with `A_DIM` instead of `A_PATH` |
| 4 | Procedural maze generation | recursive backtracker + LCG random (INT 1Ah seed) | produces bytes for `mazes` at run time; BFS already works on any grid |
| 5 | Keyboard ISR hooking | `INT 08h/09h` via `INT 21h AH=25h` set-vector, restore on exit | removes polling; pair with `INT 1Ah` tick handler |
| 6 | Save / load | `INT 21h` AH=3Ch create, 3Dh open, 3Fh read, 40h write, 3Eh close | save `level`, `prow/pcol`, move count |
| 7 | Graphical mode `13h` | `INT 10h AX=0013h`, 320×200×256, `ES=A000h`, blit 8×8 tiles | replaces `vcell` entirely; the render split (A.1) is the prerequisite |

**Why these come last.** Every one of them builds on Part A: colours/direct writes
(1, 3, 7), the once-per-level solve (4), and the flicker-free partial repaint is
what makes animation (1, 5) watchable at all.

---

## What I would do next, in order

1. **B1.1 — bounds-check `getcell`.** Two compares; closes the last unchecked
   index path in the program.
2. **B1.2 — assembly-time maze-width macro.** Free (assemble-time only); prevents
   the worst class of data bug before it can exist.
3. **B1.3 — explicit `AL=00h/E0h` handling.** Makes the input decoder honest rather
   than accidentally correct.
4. **B2.5 — `levptr` table + `NUMLEVEL`.** Payoff on every keystroke and every
   level start, and it makes the level count data-driven.
5. **B3.2 + B2.4 — HUD labels once, rows in a table.** Shrinks level restarts and
   centralises the sidebar layout before adding more HUD fields (timer/score).
6. **B1.4 — replay/reset path** (the moment a "press any key to play again" screen
   is added, together with the win-screen wait from A.5).
7. **Only then** pick features from Tier 4 / Theory 06 — starting with the
   `INT 1Ah` timer, since it is the cheapest feature that is visible every frame.
