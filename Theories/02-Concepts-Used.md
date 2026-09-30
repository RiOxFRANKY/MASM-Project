# Theory 02 — Core Concepts Used in This Project

The programming concepts the maze demonstrates: DOS/BIOS interrupts, the keyboard,
flags & jumps, arithmetic instructions, the stack, and the screen model.

---

## 1. Interrupts: the OS/BIOS API of real mode

An **interrupt** temporarily transfers control to a handler routine. `INT n` is how a
DOS program asks the OS or BIOS to do something it cannot (or should not) do itself
(printing, reading keys, setting video mode).

| Vector | Provided by | Purpose |
|---|---|---|
| `INT 10h` | BIOS | Video services (mode set, cursor) |
| `INT 16h` | BIOS | Keyboard services (read keystroke) |
| `INT 21h` | DOS | "Swiss army knife" — print, exit, file I/O |
| `INT 1Ah` | BIOS | Time-of-day ticks (useful for add-ons) |
| `INT 09h` | BIOS | Hardware keyboard ISR (not used directly by us) |

Calling convention: **AH (or AX) = function number**, other registers = arguments,
then `INT n`. Return values come back in AX/AL/DX/CF.

### The exact calls in MAZE.ASM

| Call | Function | Registers | Used for |
|---|---|---|---|
| `INT 10h / AH=00h` | set video mode | `AL=03h` | 80×25 colour text mode, **clears the screen** |
| `INT 10h / AH=02h` | set cursor position | `BH=page, DH=row, DL=col` | move the "pen" before drawing |
| `INT 16h / AH=00h` | wait for keystroke | returns `AL`=ASCII, `AH`=scancode | blocking input |
| `INT 21h / AH=02h` | write character | `DL`=char | print one maze cell / digit |
| `INT 21h / AH=09h` | write string | `DS:DX` → `$`-terminated | print HUD labels |
| `INT 21h / AH=4Ch` | terminate with return code | `AL`=code | clean exit to DOS |

> Rule of thumb: **BIOS** = hardware-level, fast, no file system; **DOS** = OS-level,
> portable across DOS versions, handles files and strings.

---

## 2. The keyboard: ASCII vs. scancode

`INT 16h / AH=00h` returns **two** values:

- **`AL` = ASCII character** — what the key *types* (`'w'`=77, `'a'`=97, ESC=27).
- **`AH` = scancode** — *which physical key* was pressed (hardware position).

| Key | AH (scancode) | AL |
|---|---|---|
| ↑ Up | `48h` | `00h` (or `E0h` on extended keys) |
| ↓ Down | `50h` | `00h`/`E0h` |
| ← Left | `4Bh` | `00h`/`E0h` |
| → Right | `4Dh` | `00h`/`E0h` |
| ESC | `01h` | `1Bh` (27) |
| W/A/S/D | (varies) | `77h`/`61h`/`73h`/`64h` |

**Why the code checks both:**

```asm
cmp al,27      ; ESC typed as ASCII?
je quit
cmp ah,01h     ; ...or by scancode?
je quit
cmp ah,48h     ; arrow up has NO useful ASCII → must use AH
je goup
cmp al,'w'     ; but 'w' HAS an ASCII code → use AL
je goup
```

Arrow keys produce `AL=0`, so their ASCII is useless — that's why `AH` matters.
Conversely letters are matched on `AL`. This dual check is the practical meaning of
"handle the keyboard interrupt" in the problem statement.

**Scancodes are positional** (they identify the key, not the character) — this is also
what allows non-US layouts and, in add-ons, key *combinations* (Shift/Ctrl).

---

## 3. Flags and conditional jumps

Every `cmp` sets the CPU's arithmetic flags; the following `jcc` reads them.

```asm
cmp al,'#'   ; equivalent to: compute AL - '#' and set flags
je game      ; ZF=1 → they were equal
```

| Instruction | Condition | Used in our code |
|---|---|---|
| `je` / `jz` | ZF=1 (equal) | ESC, scancode matches, wall collision, exit cell |
| `jne` / `jnz` | ZF=0 (not equal) | cell was not the exit |
| `jb` / `jnae` | unsigned below (CF=1) | `cmp level,3 / jb newlevel` |
| `jmp` | unconditional | after a rejected move, fall through carefully |

Note that `jb` is **unsigned** — correct here because `level` and `dh` are small
non-negative values. Mixing signed (`jl`) and unsigned (`jb`) is a classic bug source.

### Control-flow shape of the main loop

```
newlevel:  reset prow/pcol
   │
   ▼
game:  draw screen ──▶ int 16h (BLOCK until key)
   │                        │
   │                  read key, compute candidate (bl,bh)
   │                        │
   │        ┌───────────────┴─────────────┐
   │     ESC/quit?                    arrow/WASD?
   │        │                             │
   │      quit                    goup/godown/goleft/goright
   │                                    │
   │                                  move: getcell(bl,bh)
   │                                    │
   │                          ┌─────────┴─────────┐
   │                       '#' wall            free cell
   │                          │                   │
   │                       game            commit prow/pcol
   │                                    │           │
   │                               cell=='E'     cell!='E'
   │                                    │           │
   │                          inc level ─┴──────▶ game
   │                                    │
   │                       level<3 → newlevel
   │                       level=3 → win message → quit
   └──────────────────────────────────────┘
```

---

## 4. Arithmetic instructions used

### `MUL` — unsigned multiply

```asm
mov al,level
mov ah,0        ; zero-extend AL into AX
mov cx,625
mul cx          ; AX = AX * CX   (level * 625)
```

- `mul r8` → `AX = AL * operand`
- `mul r16` → `DX:AX = AX * operand` (64-bit result; we ignore DX because our results
  are tiny)
- Sets CF/OF if the upper half is non-zero.
- **Clobbers AX (and DX for 16-bit)** — you must save anything you still need.

This is what flattens the 3-D concept `(level, row, col)` into a 1-D byte offset.

### `AAM` — ASCII adjust after multiply

```asm
pnum:            ; prints the number in AL as 2 decimal digits
    mov ah,0
    aam          ; AH = AL / 10 (tens),  AL = AL % 10 (units)
    add ax,3030h ; convert both nibbles to ASCII '0'..'9'
    ...
```

`AAM` with the default divisor 10 does an implicit divide: quotient→AL, remainder→AH.
Then `+30h` turns them into ASCII. **Limitation:** it only works for 0–99 — our rows/
cols/levels are ≤ 24, so it is safe. Printing ≥100 needs a different routine (an upgrade).

### `OR AL,20h` — ASCII case folding

Sets bit 5 → forces lower case. Cheap branch-free alternative to checking both cases.

### `INC` / `DEC`

Used for movement: `dec bl` = step up, `inc bl` = step down, `inc level` = next level.
They are 1-byte instructions and set ZF/SF/OF but **not CF** — a nice property.

---

## 5. The stack

- LIFO memory region addressed by `SP`, growing **downwards**.
- `push ax` / `pop ax` in `pnum` save the converted digits across a DOS call that
  destroys `AX`.
- `call` pushes the return address; `ret` pops it.
- `.stack 100h` gives 256 bytes — plenty for our shallow call depth (2 levels max).

**Calling conventions in this program** (implicit, worth stating in interviews):

| Proc | Inputs | Outputs | Clobbers |
|---|---|---|---|
| `getcell` | `BL`=row, `BH`=col, `level` | `AL`=cell char | `SI,AX,CX` |
| `print` | `DS:DX`→string | — | `AX` |
| `setcur` | `DH`=row, `DL`=col | — | `AX,BH` |
| `pnum` | `AL`=0..99 | prints | `AX,DX` |
| `draw` | — | — | `AX,BX,CX,DX,SI` |

---

## 6. The screen model

Text mode 03h is an **80×25 grid of character cells**. Physically it is video memory
at segment `B800h`, each cell being 2 bytes: `[char][attribute]`.

We never touch `B800h` directly — we use BIOS/DOS:

```asm
mov dh,2      ; row
mov dl,30     ; column
call setcur   ; INT 10h / AH=02h
```

- Screen coordinates are **(row, col)**, 0-based → maze uses rows 0–24, cols 0–24;
  the HUD sidebar starts at **col 30**, so it never overlaps the 25-wide maze.
- Printing with `INT 21h/02h` automatically advances the cursor, which is why
  `nextcol` can simply `inc si` and loop.
- `INT 10h / AL=03h` **resets the mode → clears the screen**. Our `draw` calls it on
  every keystroke, which guarantees a clean frame but also causes **flicker** —
  the single biggest target for an upgrade (see Theory 04).

---

## 7. 2-D arrays in assembly (row-major layout)

There is no `maze[row][col]`. The maze is a **flat byte array**, and we compute the
index ourselves:

```
offset = level × 625  +  row × 25  +  col
                  ▲               ▲
          3 mazes laid end-to-end   one row = 25 bytes
```

This is exactly the same formula C uses for `char mazes[3][25][25]`. In C the compiler
generates the multiply for you; in assembly **you** write the `mul`s — see
`getcell` in Theory 03.

Consequences / gotchas:
- 625 = 25×25 must be a **compile-time constant** (`equ WIDTH equ 25`, `SIZE equ 625`).
- No bounds checking is performed → relies on the maze's solid `#` border.
- To move to level *n*, multiply by the size once and add a constant — cheap and
  branch-free.

---

## 8. Blocking vs. event-driven input

```asm
mov ah,0
int 16h      ; HALTS the CPU until a key is pressed
```

This is **polled/blocking** input: the game only acts when the player presses a key,
then redraws. It is simple and needs no timers, but it means:
- no animation, no enemies, no timer (they'd need a non-blocking read,
  `INT 16h / AH=01h` "peek", or a hardware timer ISR — see Theory 06).

---

## 9. Program life-cycle

```
DOS loads EXE → sets DS/ES/SS:SP from header → jumps to `end main` target
   → main sets DS from @data
   → game loop
   → INT 21h / AH=4Ch  (returns control + exit code to COMMAND.COM)
```

Exiting by falling off the end of the code would crash; **always** terminate through
`INT 21h/4Ch`.
