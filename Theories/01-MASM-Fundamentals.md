# Theory 01 — MASM Fundamentals

Everything below is written against the current program in `Code/MAZE.ASM` (16-bit DOS
maze game, 865 lines). Every code excerpt marked as such is copied **verbatim** from that
file, so the line numbers, labels and comments you see there are the ones discussed here.

What this document covers: what an assembler is and how MASM compares to other
assemblers, the full `ASM -> OBJ -> EXE` pipeline, every directive the program uses,
real-mode segment arithmetic, the register set, the addressing modes present in the code,
ASCII handling, machine-encoding facts, the errors you will realistically hit, and how to
debug the result.

---

## 1. Assemblers, compilers, and where MASM fits

### 1.1 What an assembler is

An **assembler** is a translator whose input and output are both essentially "machine
code": the input is human-readable mnemonics (`mov`, `cmp`, `int`, `mul`), the output is
the exact bytes the CPU fetches and executes. The mapping is close to 1:1 — one source
line usually produces one instruction (sometimes zero, for comments/labels/directives,
sometimes several, for macros).

| Term | Meaning | In MAZE.ASM |
|---|---|---|
| Assembler | Translates `.ASM` into an object file (`.OBJ`) | `masm` / `jwasm` |
| Directive | A command **to the assembler**, not to the CPU | `.model`, `.data`, `db`, `equ`, `proc`, `end` |
| Instruction | A real CPU operation emitted as bytes | `mov ax,4C00h`, `int 21h` |
| Macro | A named text template expanded at assembly time | none used here |
| Linker | Combines `.OBJ` files into an executable | `link` |
| Compiler | Translates a **high-level** language, with analysis and optimization passes | C/Pascal -> OBJ |

### 1.2 Assembler vs compiler

| Aspect | Compiler (C, ...) | Assembler (MASM) |
|---|---|---|
| Input | Structured source with types, scopes, expressions | Flat text: labels, directives, instructions |
| Optimization | Register allocation, inlining, loop transforms | Only instruction *encoding* choice (e.g. short vs near jump) |
| Memory model | Implicit (runtime, ABI) | Explicit and declared (`.model small`) |
| Errors | Semantic/type errors after parsing | Mostly "cannot encode this operand combination" |
| Output | Usually optimized object code | Essentially 1:1 with your source |
| Portability | Retargetable to other CPUs | Tied to one instruction set (x86 here) |
| Runtime support | Standard library, startup code | Nothing: *you* write the entry point and the exit |

**Why it matters:** assembly gives you byte-level control (video RAM writes in this
program are single `mov`s, so the display is flicker free), but nothing is done for you —
setting `DS`, laying out data, computing array indices and returning to DOS are all manual.

### 1.3 A short MASM history

| Version / era | Notes |
|---|---|
| 1981 | Microsoft Macro Assembler ships for DOS; Intel's ASM86 lineage |
| 4.0 / 5.0 / 5.1 (1988-89) | Classic DOS-era MASM; the `MASM FILE.OBJ; LINK ...` workflow; 5.1 is the best-documented classic manual |
| 6.0 (1991) | Huge rewrite: high-level language (HLL) directives (`.IF`, `.WHILE`, `.REPEAT`), `PROC`/`INVOKE` parameters, simplified segment directives (`.model`, `.data`, `.code`), the `ML` driver |
| 6.11 - 6.14 (1990s - 2000) | Last standalone retail releases; 6.14 is the classic "free" download |
| Visual Studio era | MASM survives as `ml.exe` (x86) / `ml64.exe` (x64) bundled with VS, mostly for intrinsics and compiler-generated asm |
| JWasm / UASM / Asmc | Free MASM-compatible reimplementations (JWasm is a fork of Open Watcom's WASM); JWasm is what this project builds with |

MAZE.ASM deliberately sticks to the **portable subset** of MASM syntax (simplified
segments + `proc` + `equ` + `db`), which is why the same source assembles under classic
MASM, MASM 6's `ml`, and JWasm without changes.

### 1.4 MASM vs TASM vs NASM vs JWasm

| Feature | MASM | TASM | NASM | JWasm |
|---|---|---|---|---|
| Vendor / origin | Microsoft, 1981 | Borland Turbo Assembler, 1988 | Open source, 1996 | Open source (fork of Open Watcom WASM), ~2004 |
| Status today | Bundled with Visual Studio | Effectively dormant | Actively maintained | Maintained (several forks) |
| Syntax family | Intel + MASM idioms | MASM-compatible "default mode" + own "ideal mode" | Own flat Intel-ish syntax | MASM 6 compatible |
| `.model small` / `.stack` / `.data` | yes | yes | no (no memory-model concept) | yes |
| `proc` / `endp` / `equ` / `dup` | yes | yes | no `proc`/`endp` (`dup` exists; `equ` exists) | yes |
| Case sensitivity | insensitive | insensitive | **sensitive** | insensitive |
| Memory models | tiny..huge | tiny..huge | none — you pick segments yourself | tiny..huge |
| Linking | `MASM` then `LINK`, or `ML` for both | `TLINK` | `ld` / custom linkers | `jwasm` never links by itself, except... |
| Output formats | OMF, COFF | OMF | `bin`, `obj` (ELF/COFF/Mach-O/...) | OMF, COFF, ELF, BIN, **MZ**, PE |
| Typical use now | Windows/legacy | legacy | OS kernels, bootloaders, demos | DOS/16-bit toolchains, FreeDOS |

#### Equivalent snippets — the same tiny program in each syntax

The program below just increments a byte variable and exits to DOS with code 0.

**MASM / JWasm (matches MAZE.ASM style):**

```asm
; TINY1.ASM - MASM / JWasm syntax, small model
.model small
.stack 100h
.data
count   db 0
.code
main proc
    mov ax,@data        ; DS cannot take an immediate
    mov ds,ax
    mov cl,count        ; load the variable through DS
    add cl,1
    mov count,cl
    mov ax,4C00h        ; AH=4Ch: terminate, AL=return code
    int 21h
main endp
end main
```

**TASM ideal mode** (its native dialect — note the dots are gone and segments are named
explicitly; TASM's *default* mode accepts the MASM source above verbatim):

```asm
; TINY1.ASM - Turbo Assembler, ideal mode
ideal
model   small
stack   100h
dataseg
count   db 0
codeseg
proc    main
    mov ax,@data
    mov ds,ax
    mov cl,[count]
    add cl,1
    mov [count],cl
    mov ax,4C00h
    int 21h
endp
end
```

**NASM** (no memory model, no `proc`, no `@data`; here a `.COM` file where
`CS = DS`, so the DS initialisation disappears entirely):

```asm
; TINY1.ASM - NASM syntax, .COM file  (nasm -f bin TINY1.ASM -o TINY1.COM)
        org 100h
        mov cl,[count]
        inc cl
        mov [count],cl
        mov ax,4C00h
        int 21h
count:  db 0
```

**Why it matters:** the *instructions* (`mov`, `add`, `int 21h`) are identical in all
four — that is the CPU's instruction set. What differs is the assembler's **directive
language** (how you declare segments, data and subroutines) and the **file format** it
emits. This project is written in the MASM dialect, so JWasm can assemble it as a drop-in
replacement for MASM.

---

## 2. Build pipeline: `MAZE.ASM -> MAZE.OBJ -> MAZE.EXE`

The two supported build paths are the ones printed in the program header, verbatim:

```asm
; build:  masm MAZE.ASM;  link MAZE.OBJ;      (MASM + LINK, inside DOS)
;         jwasm -mz MAZE.ASM                  (JWasm)
```

### 2.1 Stage 1 — the assembler (`masm` / `jwasm`)

The assembler performs (at least) two passes over the source:

* **Pass 1** collects every label and symbol and its (tentative) offset: `main`, `game`,
  `move`, `getcell`, `level`, `prow`, `mazes`, `dist`, `queue`, ...
* **Pass 2** encodes instructions, now that every forward reference (e.g. `je game` before
  `game:` is defined) has a known value.

Output is an **OMF object file** (`MAZE.OBJ`). OMF (Object Module Format) is a
record-based container, not an executable. The records that matter for this program:

| Record | Contents | Example from MAZE.ASM |
|---|---|---|
| `LHEADR` / `THEADR` | module name | `MAZE` |
| `LNAMES` | list of segment/group names | `_TEXT`, `_DATA`, `STACK`, `DGROUP` |
| `SEGDEF` | one per segment: size, alignment, class | `_TEXT` (code), `_DATA` (data), `STACK` (256 bytes) |
| `GRPDEF` | group (a bundle of segments addressed as one) | `DGROUP = DGROUP: _DATA, STACK` |
| `PUBDEF` | public (exported) symbols | the labels other modules could call |
| `EXTDEF` | external (imported) symbols | **none** — single-module program |
| `LEDATA` / `LIDATA` | literal / iterated data bytes | `LIDATA` is where `db MSZ dup(0FFh)` ends up: (count, value) repeated |
| `FIXUPP` | **relocation requests** — "patch this field when linking" | for `mov ax,@data` and every `offset` reference |
| `MODEND` | end of module + **entry point** | produced from `end main` |

The object file therefore contains the code and data bytes, but many 16-bit fields are
still placeholders (zeros) flagged by `FIXUPP` records.

### 2.2 Stage 2 — the linker (`link`)

The linker's jobs:

1. **Segment merging.** Every `SEGDEF` from every input object is laid out and, where
   segments share name/class/group, merged. For MAZE.ASM this means `_TEXT` (everything
   between `.code` and `end`), `_DATA` (everything between `.data` and `.code`) and the
   256-byte `STACK` segment, with `DGROUP` grouping the data/stack segments.
2. **Symbol resolution.** `EXTDEF` symbols are matched with `PUBDEF`s of other modules.
   There are none here, but the pass still runs.
3. **Applying fixups.** Each `FIXUPP` record tells the linker which field to fill in. Two
   kinds occur:
   * *Internal* fixups: `mov si,offset dist` — the offset of `dist` inside `_DATA` is now
     known, so the placeholder is written.
   * *Frame/segment* fixups: `mov ax,@data` — the assembler does not know where `DGROUP`
     will sit relative to the start of the image, so the linker writes the group's
     paragraph offset **relative to the module start**.
4. **Writing the MZ executable header.** The result is `MAZE.EXE`, which begins with the
   two ASCII bytes `MZ` (`5Ah 4Dh`):

| Header field | Meaning | How it is derived |
|---|---|---|
| signature | `MZ` | fixed |
| `CS:IP` | entry point | **from `end main`**: `IP = offset(main)`, `CS = paragraph of `_TEXT`` |
| `SS:SP` | initial stack | from `.stack 100h` -> `SP = 0100h`, `SS` = paragraph of the stack segment |
| min/max extra paragraphs | how much memory to allocate | segment sizes rounded up to paragraphs |
| relocation table offset/count | list of `seg:off` fields to patch at load time | see below |
| header size in paragraphs | where the load image starts | rounded-up header length |

5. **Emitting the load-time relocation table.** An `.EXE` may be loaded at *any* segment
   (DOS picks the first free block after the PSP). Every word in the image that holds a
   paragraph value relative to the module start must therefore be adjusted by the actual
   load segment when DOS loads the file. `mov ax,@data` is exactly such a word: the
   header's relocation table lists it, and `COMMAND.COM`'s loader adds
   `load_segment` to it before the first instruction runs.

**Why it matters:** this is why `.EXE` needs a relocation table while `.COM` (loaded at a
fixed `CS=DS=SS` offset 100h) does not, and why segment-relative quantities must never be
"pre-computed" at assembly time.

### 2.3 Exact commands

**Path A — classic MASM + LINK (both run inside DOS):**

```dos
MASM MAZE.ASM;
LINK MAZE.OBJ;
```

The trailing `;` silences MASM's interactive prompts (`Source listing [NUL.LST]:` etc.).
`LINK` then prompts:

```text
Object Files [MAZE.OBJ]:
Run File [MAZE.EXE]:
List File [NUL.MAP]:
Libraries [.LIB]:
```

Pressing Enter through all of them produces `MAZE.EXE` (and no `.MAP`).

**Path A' — MASM 6 / ML (assembles *and* links in one go):**

```dos
ML /Zi /Fl MAZE.ASM        ; /Zi = debug info in the OBJ, /Fl = MAZE.LST listing
```

Add `/c` (or use `ML /c`) to assemble only. With debug info you must link with
`LINK /CO MAZE.OBJ` so `TD.EXE` sees the symbols.

**Path B — JWasm (what this project is validated with):**

```dos
JWASM -MZ MAZE.ASM
```

`-mz` selects JWasm's native **DOS MZ output format**, i.e. it writes the executable
directly and *no linker is involved at all*. Default output format is OMF (`-omf`), in
which case you would link afterwards as in Path A. JWasm reports progress as:

```text
Assembling: MAZE.ASM
```

and errors as `MAZE.ASM(line): error A2xxx: message`. The project builds with
**0 errors / 0 warnings**. Useful switches: `-Fl<file>` (listing file), `-Zd`/`-Zi`
(line-number / full debug info), `-bin`, `-coff`, `-mz`, `-h` (full option list).

### 2.4 Running it in DOSBox

```text
DOSBox> mount c /Users/ritamghosh/Desktop/MASM/Code
DOSBox> c:
C:\> maze.exe
```

or non-interactively:

```bash
dosbox -c "mount c /Users/ritamghosh/Desktop/MASM/Code" -c "c:" -c "maze.exe"
```

To *assemble inside DOSBox* (the classic workflow), copy `MASM.EXE` + `LINK.EXE`
(or `JWASM.EXE`) onto the mounted drive first:

```text
C:\> masm maze.asm;
C:\> link maze.obj;
C:\> maze
```

Notes: the game sets 80x25 colour text mode itself (`mov ax,0003h / int 10h`), so any
emulated video card (`svga_s3`, `vga`...) works; arrow keys / WASD move, `H` hints,
`ESC` quits. `ctrl-F10` releases the mouse capture DOSBox grabs by default.

---

## 3. Every directive used by MAZE.ASM, in depth

The complete set of directives in the file is small:

```asm
.model small
.stack 100h
.data
.code
name  db <value>            ; and dw, with optional "count dup (value)"
name  equ <expression>
name  proc ... name endp
end main
```

### 3.1 `.model small` — the memory model

Real-mode programs must declare how they intend to use the 1 MB address space, because
the assembler has to know: how many segments exist, whether pointers are 16-bit or
32-bit (offset only, or segment:offset), and which registers may need segment prefixes.

| Model | Code segments | Data segments | Default pointer | CS/DS relationship | Used for |
|---|---|---|---|---|---|
| `tiny` | 1 | 1 | near (16-bit) | **CS = DS = SS** (one 64 KB block) | `.COM` files |
| **`small`** | **1** | **1** | **near (16-bit)** | **separate CS, DS, SS, all single** | **most 16-bit programs — MAZE.ASM** |
| `medium` | many | 1 | code far / data near | one DS, many code segments | large code, small data |
| `compact` | 1 | many | code near / data far | one CS, many data segments | large data |
| `large` | many | many | far (16:16) | many of both | big applications |
| `huge` | many | many | huge (16:16 + normalization) | segments whose data may exceed 64 KB | arrays larger than 64 KB |

What `small` buys this program:

* Code lives entirely in **one** segment (`_TEXT`, addressed by `CS`); every `call`,
  `jmp` and `je` is a *near* transfer (`E8 rel16` / `EB rel8`), never a 32-bit
  `seg:offset` far jump.
* Data lives entirely in **one** segment (`_DATA`, addressed by `DS`), so
  `mov al,mazes[si]`, `mov dist[si],dl`, `mov bl,prow` need **no** `ES:` override —
  the default segment is already correct.
* The whole program is tiny: code is a couple of KB and data is
  **3896 bytes** (9 state bytes + 129 string bytes + `mazes` 1875 + `dist` 625 +
  `queue` 1250 + 8 bytes of BFS bookkeeping), far below the 64 KB each segment allows.

`.model small` also implicitly defines:

* the group `DGROUP` and the predeclared equate **`@data`** (segment of the data),
* segment defaults `ASSUME CS:_TEXT, DS:DGROUP, SS:DGROUP`,
* that `proc` procedures are **near** by default (`call` = `E8 rel16`).

### 3.2 `.stack 100h`

```asm
.stack 100h
```

Creates a `STACK` segment of **100h = 256 bytes** and tells the linker to set
`SS:SP = <stack segment>:0100h` in the EXE header (so `SP` initially points just *past*
the end of the block, and the first `push` decrements it to `00FEh`).

What actually uses that stack in MAZE.ASM:

| Mechanism | Pushed size | Example |
|---|---|---|
| `call` / `ret` | 2 bytes (return IP) | `call getcell` at the `move:` label |
| `push ax` / `pop ax` | 2 bytes each | `vcell` saves char+attr (line `push ax`) |
| `push bx/cx/di/es` | 2 bytes each | `vcell` pushes 4 registers = 8 bytes |
| nesting | `main -> bfsolve -> trynb` | 3 frames deep at most |

Typical peak usage is well under 40 bytes (deepest chain: `main` -> `bfsolve` ->
`trynb` -> (no further calls)), so 256 bytes leaves a large safety margin.

**Why it matters:** the stack is where return addresses live; if `SP` pointed into your
data, `call` would silently corrupt variables. Declaring the stack size is the
assembler's way of reserving that space in the linker's memory map.

### 3.3 `.data` and `.code`

```asm
.data            ; everything after this line is initialized data in segment _DATA
.code            ; everything after this line is instructions in segment _TEXT
```

* `.data` switches the assembler's current location counter into `_DATA`. Labels defined
  there (`level`, `prow`, ..., `mazes`, `dist`, `queue`) get offsets relative to that
  segment and are referenced through `DS`.
* `.code` switches into `_TEXT`; labels there (`main`, `game`, `move`, ...) are referenced
  through `CS` implicitly.
* Ordering in the file is `.data` **before** `.code`; the linker places segments
  according to their `SEGDEF`s, not their textual order, but keeping data first is the
  house style used here.

### 3.4 `db` / `dw` — data definition, and `dup`

`db` allocates **one byte per item**; `dw` allocates **two bytes (one word)**. Nothing is
emitted at run time — the bytes simply appear in the `_DATA` segment of the object file.

Three flavours used in MAZE.ASM:

```asm
level   db 0                    ; numeric: one byte, value 0
msglvl  db 'Level $'            ; string: 7 consecutive bytes, no NUL appended
mazes   db '#########################'   ; 25 bytes, one per maze column
dist    db MSZ dup(0FFh)        ; replication: 625 bytes, every one = 0FFh
queue   dw MSZ dup(0)           ; 625 words = 1250 bytes, every one = 0000h
bhead   dw 0                    ; one word, value 0
```

| Form | Size in memory | Notes |
|---|---|---|
| `db 0` | 1 byte | `00h` |
| `db 0FFh` | 1 byte | `FFh` (hex literal needs a leading digit, hence `0FFh`) |
| `db '#'` | 1 byte | character literal = its ASCII code `23h` |
| `db 'Level $'` | 7 bytes | **exactly 7** — MASM does *not* append `00h` like C does |
| `dw 0` | 2 bytes | `00h 00h`, low byte first |
| `dw MSZ dup(0)` | 2 x 625 = 1250 bytes | word-sized repetition |
| `db 625 dup(0FFh)` | 625 bytes | here written as `db MSZ dup(0FFh)` |

The actual declarations in the file:

```asm
dist    db MSZ dup(0FFh)        ; distance of a cell to 'E', FFh = not reached
queue   dw MSZ dup(0)           ; BFS queue, packed: low byte = row, high = col
```

`MSZ` is `MW * MH = 25 * 25 = 625`, so `dist` expands to 625 bytes of `FFh` and `queue`
to 1250 zero bytes. The `LIDATA` record in the OBJ stores this compactly as
(count = 625, value = FFh); the loader/segment data is the flat expansion.

**Why it matters:** `dist` being pre-filled with `0FFh` is not decoration — `bfsolve`
relies on `FFh` meaning "not visited yet" (`cmp al,0FFh / jne tn_done`), and `vnum3`
relies on `hintd = 0FFh` meaning "print `---` instead of a number".

### 3.5 `equ` — symbolic constants

```asm
MW      equ 25                  ; maze width  (columns, bytes per row)
MH      equ 25                  ; maze height (rows)
MSZ     equ MW * MH             ; bytes per level (625)
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

Properties of `equ`:

* It is **assembly-time only** — no bytes are emitted, no label exists in the OBJ.
* The expression is evaluated by the assembler: `MSZ` is literally replaced by `625`, so
  `mov cx,MSZ` becomes `mov cx,625` (`B9 71 02`).
* `equ` symbols are **read-only**: `MW equ 30` after that line is an error (redefinition),
  unlike `=`, which allows reassignment.
* Attribute equates are the colour byte written to the odd byte of every video cell.
  High nibble = background, low nibble = foreground:

| Equate | Value | Foreground (low nibble) | Background (high nibble) | Used by |
|---|---|---|---|---|
| `A_WALL` | `1Fh` | F = bright white | 1 = blue | `drawmaze`, `paintcell` |
| `A_PATH` | `07h` | 7 = light grey | 0 = black | `drawmaze`, `paintcell` |
| `A_EXIT` | `2Eh` | E = bright yellow | 2 = green | `drawmaze`, `paintcell` |
| `A_PLAY` | `4Eh` | E = bright yellow | 4 = red | `putplayer` |
| `A_HINT` | `3Fh` | F = bright white | 3 = cyan | `refreshhint` |
| `A_HUD` | `0Bh` | B = light cyan | 0 = black | `drawhud`, `updhud` |
| `A_WIN` | `2Fh` | F = bright white | 2 = green | win screen (`vstr`) |
| `A_MSG` | `0Fh` | F = bright white | 0 = black | win screen (`vstr`) |

**Why `equ` beats magic numbers:**

* One place to change: switching the maze to 30x30 is `MW equ 30` — every `mul cl`
  (`mov cl,MW`), every `cmp dl,MW` loop bound and `MSZ` recomputes itself.
* Self-documenting: `mov ah,A_WALL` says what it means; `mov ah,1Fh` does not.
* Mistypable numbers pass silently (the CPU happily writes attribute `1Eh`), mistyped
  symbols stop the build (`undefined symbol`).

### 3.6 `proc` / `endp` — procedures

```asm
getcell proc
    ...
    ret
getcell endp
```

* `name proc` / `name endp` bracket a callable subroutine. In the `small` model the
  default (and used) type is **near**: `call getcell` assembles to
  `E8 <rel16>` (3 bytes) — push IP, jump within the same segment; `ret` (`C3`) pops IP.
* The assembler uses `proc`/`endp` for name scoping (labels inside are local in MASM 6
  with `OPTION SCOPED`), for argument type checking if you declare parameters (none are
  declared here), and to warn about fall-through paths.
* MAZE.ASM defines **17 procedures**:

| Procedure | Purpose | Key inputs |
|---|---|---|
| `main` | entry, state machine / input loop | — |
| `getcell` | maze byte at (BL,BH) of the current level | `level`, `BL`, `BH` |
| `vcell` | write one char+attr into video RAM | `AL`, `AH`, `DH`, `DL` |
| `vstr` | write a `'$'`-terminated string | `SI`, `AH`, `DH`, `DL` |
| `vnum2` | print `AL` (0..99) as 2 digits | `AL`, `AH`, `DH`, `DL` |
| `vnum3` | print `AL` (0..255) as 3 digits | `AL`, `AH`, `DH`, `DL` |
| `paintcell` | repaint one cell in its natural colour | `BL`, `BH` |
| `clrplayer` | repaint the cell under the player | `prow`, `pcol` |
| `putplayer` | stamp `'@'` | `prow`, `pcol` |
| `clearhint` | remove the hint highlight | `hrow`, `hcol`, `hintvis` |
| `drawmaze` | full colourised redraw of the level | `level` |
| `drawhud` | static sidebar labels | — |
| `updhud` | rewrite only Row/Col/Dist digits | `prow`, `pcol`, `hintd` |
| `bfsolve` | fill `dist[]` with shortest distance to `'E'` | `level`, `mazes` |
| `trynb` | enqueue one BFS neighbour | `BL`, `BH`, `DL` |
| `findnb` | is (BL,BH) the cell with distance `wantd`? | `BL`, `BH`, `wantd` |
| `refreshhint` | highlight the next cell of the optimal path | `prow`, `pcol`, `dist[]` |

Several headers document the register contract explicitly, which is the poor man's
calling convention — e.g. `getcell`:

```asm
; ===========================================================================
; getcell - AL = maze character at (BL = row, BH = col) of the current level
;           index = level*MSZ + row*MW + col        (row-major layout)
;           clobbers AX, CX, SI - preserves BL, BH
; ===========================================================================
```

### 3.7 `end main`

```asm
end main
```

Two jobs in one directive:

1. **Stop** — nothing after `end` is assembled (that is why `main endp` must come first).
2. **Entry point** — the operand becomes the module's start address. It is recorded in
   the `MODEND` record, and the linker writes it into the EXE header as `CS:IP`
   (`IP = offset of main`, `CS = paragraph of _TEXT`). When DOS starts `MAZE.EXE` it
   loads the image, applies the relocation table, sets `SS:SP` from the header and jumps
   to `CS:IP` — so execution begins at `main`.

Forgetting `end` is an error; `end` without an operand simply means "entry point = start
of the first segment", which would work here only by accident.

### 3.8 Directives vs instructions — the split

| Assembler-time (disappears except for its bytes) | Run-time (CPU executes it) |
|---|---|
| `.model small`, `.stack 100h` | `mov ax,@data`, `mov ds,ax` |
| `.data` / `.code` (segment switching) | `call bfsolve`, `ret` |
| `db 'Level $'` (bytes in the image) | `int 10h`, `int 16h`, `int 21h` |
| `MSZ equ MW * MH` (pure substitution) | `cmp dl,MW`, `mul cx` |
| `proc` / `endp` (bookkeeping) | `push ax` / `pop ax` |
| `end main` (EXE header entry) | `je game`, `jmp game` |

If it does not appear in the CPU manual's opcode tables, it is a directive.

---

## 4. Segments and real-mode addressing

### 4.1 `segment:offset` arithmetic

Real mode uses 20-bit physical addresses from two 16-bit registers:

```
physical = segment * 16 + offset        (segment shifted left by one hex digit)
```

`segment * 16` is always a multiple of 16 — such an address is called a **paragraph**.
The offset wraps at `FFFFh`, so the reachable space is `00000h .. 10FFEFh` (just over
1 MB).

**Worked example 1 — the video segment (used by `vcell`):**

```asm
    mov ax,0B800h               ; colour text segment
    mov es,ax
```

`B800h:0000h` = `B800h * 16` = `B8000h` = **753,664 decimal** — the start of text-mode
video RAM for an 80x25 colour adapter.

**Worked example 2 — one screen cell (this is `vcell`'s maths):**

```asm
    mov al,dh                   ; row
    xor ah,ah
    mov bl,80                   ; 80 cells per row
    mul bl                      ; AX = row * 80
    xor bh,bh
    mov bl,dl                   ; column
    add ax,bx                   ; AX = row*80 + col
    shl ax,1                    ; 2 bytes per cell
    mov di,ax
    ...
    mov es:[di],al              ; even byte = character
    mov es:[di+1],ah            ; odd  byte = attribute
```

For row 12, column 40 (the centre of an 80x25 screen):

```
(12 * 80 + 40) * 2 = (960 + 40) * 2 = 2000 = 07D0h
physical = B8000h + 07D0h = B87D0h = 755,664
```

For the player at `prow = 1`, `pcol = 1`: `(1*80 + 1) * 2 = 162 = 00A2h`
-> `B8000h + 00A2h = B80A2h`. The whole 80x25 screen is exactly
`80 * 25 * 2 = 4000 = 0FA0h` bytes: `B8000h .. B8FA0h`.

**Worked example 3 — a variable through `DS`:**

Suppose the linker puts `level` at offset `0100h` in the data segment and DOS loads the
data segment at `0C16h`:

```
prow (one byte after level) -> DS:0101h
physical = 0C16h * 16 + 0101h = 0C160h + 0101h = 0C261h
mazes (9 state bytes + 129 string bytes = 138 = 008Ah later) -> DS:018Ah
```

Offsets *inside* `.data` are fixed relative to each other; the absolute value depends on
where the linker places the segment (the listing file shows the final numbers). The
program never hard-codes them — it always writes `prow`, `mazes[si]`, `dist[si]`, which
the assembler/linker resolve.

### 4.2 Why `mov ax,@data / mov ds,ax` is mandatory

```asm
main proc
    mov ax,@data
    mov ds,ax
```

* At EXE entry, **DOS leaves `DS = ES = PSP segment`** (the Program Segment Prefix at
  `load_seg:0000`). `SS:SP` comes from the EXE header, `CS:IP` from `end main`. Nothing
  points at your `.data` yet.
* Every data access in the program — `mov bl,prow`, `mov al,mazes[si]`,
  `mov dist[si],dl`, `mov queue[si]` — is a **`DS:`** access by default. With `DS`
  still holding the PSP, `mov bl,prow` would read a byte from the PSP's command tail area.
* `@data` is a predeclared equate for the data segment (here: the `DGROUP` group). Its
  numeric value is a *relocation*: `mov ax,@data` assembles to `B8 <placeholder>`, a
  `FIXUPP` record patches it at link time, and DOS adds the load segment at load time.
* **`mov ds,imm16` cannot be encoded at all.** The 8086 `MOV` opcode `8E /r`
  (move to a segment register) takes an `r/m16` operand — a *register* or *memory* — and
  there is no immediate form for segment registers. Hence the two-instruction idiom:

| Instruction | Bytes | Why |
|---|---|---|
| `mov ax,@data` | `B8 xx xx` | `mov r16,imm16` exists (`B8+rw`) |
| `mov ds,ax` | `8E D8` | `mov Sreg,r16` exists (`8E /r`, mod=11, reg=DS=011, rm=AX=000 -> `11011000` = D8) |
| `mov ds,1234h` | **illegal** | no opcode: `8E` cannot take an immediate |

(Legal alternatives exist that are *not* used here: `lds si,[dword ptr x]` loads DS from
memory; `push ds / pop ds` moves DS through the stack.)

**Why it matters:** "Segment register initialisation" is the single most common bug class
in hand-written DOS code. The program does it exactly once, at the top of `main`, and
never lets `DS` change afterwards.

### 4.3 What CS / SS / DS / ES each do in THIS program

| Register | Set by | Used for in MAZE.ASM |
|---|---|---|
| **CS** | DOS loader, from the EXE header (`end main`) | All code fetch; every near `call`/`jmp`/`je` is `CS:`-relative automatically. Never touched by the program. |
| **SS:SP** | DOS loader, from `.stack 100h` | Return addresses and saved registers for `call`, `push`, `pop`. Never touched by the program. |
| **DS** | `mov ax,@data / mov ds,ax` at entry | Every variable: `prow`, `pcol`, `level`, `hinton`, all strings, `mazes`, `dist`, `queue`, `bhead/btail/lbase`, `exrow/excol`. |
| **ES** | (DOS leaves it = PSP) — reloaded twice by the program | (a) `vcell`: `mov ax,0B800h / mov es,ax` so `mov es:[di],al` writes video RAM, then `pop es` restores it; (b) `bfsolve`: `push ds / pop es` makes `ES = DS` so `rep stosb` can wipe `dist` with one instruction. |

The two ES uses, verbatim:

```asm
    push es
    push di
    push bx
    push ax                     ; remember character + attribute
    mov ax,0B800h               ; colour text segment
    mov es,ax
```

```asm
    ; ---- dist[] := all 0FFh ---------------------------------------------
    push ds
    pop es                      ; ES = DS for REP STOSB
    mov di,offset dist
    mov cx,MSZ
    mov al,0FFh
    rep stosb
```

Notes:

* `vcell` pushes/pops `ES` (and `DI`, `BX`) so callers never notice that the video
  segment was installed — that is why `drawmaze` can loop calling `vcell` 625 times with
  `ES` still equal to `DS`.
* `rep stosb` writes `AL` to `ES:DI`, `CX` times, incrementing `DI`. It *requires* `ES`;
  since the destination `dist` lives in `DS`, the `push ds / pop es` pair is the cheapest
  way to get `ES = DS` (2 bytes, no `mov ax` needed).
* String instructions (`rep stosb`) and `int` calls do not need `DS` to be `ES`; DOS and
  the BIOS save/restore what they use.

---

## 5. Registers

### 5.1 The 8086 register set

All 16 general registers can be addressed as one 16-bit register or as two independent
8-bit halves (the low halves are the *original* 808 register names):

| 16-bit | High byte | Low byte | Special roles |
|---|---|---|---|
| `AX` | `AH` | `AL` | accumulator: `mul`/`div` operands and results, `int` function numbers in `AH` |
| `BX` | `BH` | `BL` | the only general register usable in some addressing modes; here: **candidate row/col pair** |
| `CX` | `CH` | `CL` | counter: `rep stosb` count, `mul cx` / `mul cl` multiplier |
| `DX` | `DH` | `DL` | `DX:AX` for 32-bit `div`; here: **screen row/column pair** |
| `SI` | — | — | source/index: byte offset into `mazes` / `dist` / `queue`; string pointer in `vstr` |
| `DI` | — | — | destination/index: `rep stosb` target, video offset in `vcell`, queue write index |
| `BP` | — | — | base pointer for stack frames — **unused** in this program (no local variables) |
| `SP` | — | — | stack pointer; changed only by `push`/`pop`/`call`/`ret`/`int` |
| `CS` | — | — | code segment base (where instructions come from) |
| `DS` | — | — | data segment base (variables) |
| `SS` | — | — | stack segment base |
| `ES` | — | — | extra segment: video RAM (`vcell`) or `= DS` (`bfsolve`) |
| `IP` | — | — | instruction pointer (offset inside `CS`) |
| `FLAGS` | — | — | condition flags (CF, ZF, SF, OF, PF, AF, IF, DF, TF) |

### 5.2 The role of each register in MAZE.ASM

| Register | Role in this program | Representative lines |
|---|---|---|
| `AX`/`AH`/`AL` | `@data` load; `AH` = **video attribute** for `vcell`, `AH` = **BIOS/DOS function** for `int`; `AL` = character / digit / distance; `AX` = `mul` and `div` results; `AX` = packed queue entry | `mov ax,@data`; `mov ah,A_WIN`; `mov al,mazes[si]`; `mul cx` |
| `BX`/`BH`/`BL` | **`BL` = candidate row, `BH` = candidate column** — the pair moved around *before* being committed to `prow`/`pcol`; also a scratch register inside `vcell` (`mov bl,80`) and `vnum3` (`mov bl,100`) | `mov bl,prow` / `mov bh,pcol`; `dec bl`; `mov bl,al` (BFS row) |
| `CX`/`CH`/`CL` | `mov cx,MSZ` = `mul cx` (625) and `rep stosb` count; `mov cl,MW` = `mul cl` (25); `vstr`/`vnum2`/`vnum3` park the attribute in `cl` and a digit in `ch` across a `vcell` call | `mov cx,MSZ`; `mov cl,MW`; `push cx` ... `mov cl,ah` |
| `DX`/`DH`/`DL` | **`DH` = screen row, `DL` = screen column** for `vcell`/`vstr`/`vnum2`/`vnum3` and the `drawmaze` loops; also `DL` = BFS distance being written, `DX` = column out of `div cx` | `mov dh,bl` / `mov dl,bh`; `inc dl`; `div cx` -> `mov excol,dl` |
| `SI` | **index into `mazes`/`dist`** (`mov al,mazes[si]`), index into `queue` after `shl si,1`, and **string cursor** in `vstr` (`mov al,[si]`) | `mov si,ax`; `inc si`; `mov al,[si]` |
| `DI` | `rep stosb` destination (`mov di,offset dist`), scan limit in `bfsolve` (`add di,MSZ`), **queue write index** (`mov queue[di],ax`), **video cell offset** in `vcell` | `mov di,ax`; `mov es:[di],al` |
| `BP` | never used — procedures are leaf-like with fixed `push` counts, so no frame pointer is needed | — |
| `SP` | implicit: every `call`/`push`/`pop`/`ret` | `push bx` / `pop bx` |
| `CS` | implicit code fetch; `je game` etc. are `CS:`-relative | — |
| `DS` | base of all variables; loaded once | `mov ax,@data` / `mov ds,ax` |
| `SS`/`SP` | `.stack 100h` block | — |
| `ES` | video segment in `vcell`, `= DS` for `rep stosb` | `mov es,ax` (B800h); `push ds` / `pop es` |
| `IP` | implicit target of `call`/`ret`/`jmp` | `ret` |
| `FLAGS` | `ZF` from every `cmp` drives `je/jne/jb/jae`; `CF` carries the boolean result of `findnb` | `stc` / `clc` / `jc rh_got` |

The BX "candidate" pattern, verbatim — the position is only committed after the target
cell is proven walkable:

```asm
game:
    mov ah,0
    int 16h                     ; wait for a key: AL = ASCII, AH = scancode
    mov bl,prow                 ; candidate position (only committed if legal)
    mov bh,pcol
```

```asm
    call clearhint              ; repaint the old highlighted cell
    call clrplayer              ; repaint the cell the player leaves
    mov prow,bl                 ; commit the new position
    mov pcol,bh
```

### 5.3 Flags you actually need to know

| Flag | Set by | Consumed by (in this program) |
|---|---|---|
| `ZF` (zero) | `cmp`, `sub`, `and`, `or`, `test` | `je hintkey`, `je game`, `jne tn_done`, `je vs_done` |
| `CF` (carry) | `cmp` (unsigned), `clc`, `stc` | `jb start_level`, `jae bs_loop`, `jc rh_got` |
| `CF`+`OF` | `mul` (CF=1 if AH/DX != 0) | not tested here |
| `SF`, `OF`, `PF`, `AF` | arithmetic | not tested here |

`findnb` is a procedure whose *result* is a flag — the assembly-level equivalent of
returning a `bool`:

```asm
    mov al,dist[si]
    cmp al,wantd
    jne fn_no
    mov hrow,bl
    mov hcol,bh
    stc
    ret
fn_no:
    clc
    ret
```

and the caller branches on it with `jc rh_got`.

---

## 6. Addressing modes present in the code

Every operand form that appears in MAZE.ASM, with a real line and (where the bytes do not
depend on a linker-assigned offset) the exact encoding an 8086 emits. `xx xx` marks a
16-bit field that the linker/loader fills in.

| Mode | Form | Verbatim example | Bytes | Effective address / meaning |
|---|---|---|---|---|
| Immediate | `reg, imm` | `mov cx,MSZ` | `B9 71 02` | constant 625 (625 = 0271h, low byte first) |
| Immediate | `reg, imm` | `mov ah,4Ch` | `B4 4C` | constant 4Ch |
| Immediate | `reg, imm` | `mov al,0FFh` | `B0 FF` | constant FFh |
| Immediate | `mem, imm` | `mov prow,1` | `C6 06 xx xx 01` | store 1 into `prow` |
| Immediate | `mem, imm` | `mov dist[si],0` | `C6 84 xx xx 00` | store 0 into `dist[SI]` |
| Immediate arithmetic | `reg, imm` | `or al,20h` | `0C 20` | AL OR 20h (case fold) |
| Immediate arithmetic | `reg, imm` | `add al,'1'` | `04 31` | AL + 31h (level -> ASCII digit) |
| Register | `reg, reg` | `inc bl` | `FE C7` | BL = BL + 1 |
| Register | `reg, reg` | `xor ah,ah` | `32 E4` (or the equivalent `30 E4`) | AH = 0, clears CF/ZF |
| Register | `reg, reg` | `shl si,1` | `D1 E6` | SI <<= 1 (index -> byte offset in `queue`) |
| Register | `reg, reg` | `mov bl,al` | `88 C3` | BL = AL |
| Direct | `[disp16]` | `mov bl,prow` | `8A 1E xx xx` | DS:`offset(prow)` (mod=00, rm=110 = direct address) |
| Direct | `[disp16]` | `cmp hinton,0` | `80 3E xx xx 00` | compare memory byte against 0 |
| Direct | `moffs` | `mov al,level` | `A0 xx xx` (MASM family; generic form `8A 06 xx xx` is equivalent) | load AL from `offset(level)` |
| Register indirect | `[SI]` | `mov al,[si]` (in `vstr`) | `8A 04` | DS:`SI` — the string cursor |
| Direct-indexed | `[disp16+SI]` | `mov al,mazes[si]` | `8A 84 xx xx` | DS:`offset(mazes) + SI` — **the maze lookup** |
| Direct-indexed | `[disp16+SI]` | `mov dist[si],dl` | `88 94 xx xx` | DS:`offset(dist) + SI` — store a distance |
| Direct-indexed | `[disp16+SI]` | `mov ax,queue[si]` | `8B 84 xx xx` | DS:`offset(queue) + SI` — dequeue a word |
| Direct-indexed | `[disp16+SI]` | `cmp mazes[si],'E'` | `80 BC xx xx 45` | compare a maze byte with `'E'` (45h) |
| Direct-indexed | `[disp16+DI]` | `mov queue[di],ax` | `89 85 xx xx` | DS:`offset(queue) + DI` — enqueue a word |
| Direct-indexed | `[disp16+DI]` | `cmp mazes[di],'#'` | `80 BD xx xx 23` | wall test in `trynb` (`mod=10`, `rm=101` = `[DI]` -> BD) |
| Based + disp8 | `[DI+1]` | `mov es:[di+1],ah` | `26 88 65 01` | ES:`DI+1` — the attribute byte of a video cell |
| Segment override | `ES:[DI]` | `mov es:[di],al` | `26 88 05` | **ES:** (not DS) `DI` — character byte in video RAM |
| `offset` operator | `imm` | `mov si,offset msgwin` | `BE xx xx` | address of the string, not its contents |
| `offset` operator | `imm` | `mov di,offset dist` | `BF xx xx` | address of `dist` (needed by `stosb`) |
| Implied | none | `ret` | `C3` | operand is the stack |
| Implied | none | `stc` / `clc` | `F9` / `F8` | operand is `CF` |
| Implied | none | `aam` | `D4 0A` | divides AL by 10 into AH:AL |
| Implied prefix | none | `rep stosb` | `F3 AA` | `CX` times, `ES:DI` |

Notes on the two "star" modes:

* **`mazes[si]`** (and `dist[si]`, `queue[si]`) is `disp16 + index` addressing: the
  assembler emits `mod=10` (16-bit displacement follows the ModRM byte) with `rm=100`
  (`[SI]`). It is one instruction doing `load byte at DS + offset(mazes) + SI` — the
  whole game is built on it, in `getcell`, `drawmaze`, `bfsolve`, `trynb`, `findnb` and
  `refreshhint`.
* **`es:[di]`** carries the one-byte `26h` *segment override prefix*. Without it, the
  CPU would use `DS` and scribble over program data instead of video RAM.

Which segment each memory operand uses, by default:

| Operand | Default segment | Why it is correct |
|---|---|---|
| `prow`, `pcol`, `hinton`, ... | `DS` | plain variables in `.data` |
| `mazes[si]`, `dist[si]`, `queue[si]` | `DS` | tables in `.data` |
| `[si]` in `vstr` | `DS` | string labels are in `.data` |
| instruction fetch (`game:`, `ret`) | `CS` | code segment, automatic |
| `[bp]` (would be `SS`) | — | not used: no stack-frame locals |
| `es:[di]` in `vcell` | **`ES` (overridden)** | video RAM lives at `B800h` |

---

## 7. ASCII and character handling

### 7.1 The ASCII facts the program relies on

| Character(s) | Code(s) | Where in MAZE.ASM |
|---|---|---|
| `'0'..'9'` | `30h..39h` | `add ax,3030h` (`vnum2`), `add al,30h` (`vnum3`) |
| `'1'..'3'` | `31h..33h` | `add al,'1'` (level number in the HUD) |
| `'A'..'Z'` | `41h..5Ah` | key handling before case folding |
| `'a'..'z'` | `61h..7Ah` | `cmp al,'h'`, `cmp al,'w'`, `cmp al,'s'`, `cmp al,'a'`, `cmp al,'d'` |
| space | `20h` | maze path cells; also what `AL = 0` becomes after `or al,20h` |
| `'#'` | `23h` | wall test in `paintcell`, `drawmaze`, `move`, `trynb` |
| `'$'` | `24h` | string terminator tested by `vstr` |
| `'-'` | `2Dh` | `msgdash db '---$'` |
| `'@'` | `40h` | the player, drawn by `putplayer` |
| `'E'` | `45h` | exit cell, found by `bfsolve` (`cmp mazes[si],'E'`) |
| ESC | `1Bh` (ASCII) / `01h` (scancode) | `cmp al,27` and `cmp ah,01h` |
| arrow keys | scancodes `48h/50h/4Bh/4Dh`, ASCII `00h` | `cmp ah,48h` etc. |
| control chars `00h..1Fh` | NUL, BEL, BS, TAB, LF, CR, ESC, ... | only ESC (1Bh) is used |

### 7.2 Why strings still end in `'$'`

```asm
msglvl  db 'Level $'
msgdst  db 'Dist  $'
msgkey  db 'Arrows/WASD = Move$'
msgdash db '---$'
msgwin  db 'You finished all 3 levels!$'
msgany  db 'Press any key to exit$'
```

* MASM emits **exactly the bytes you type** — `'Level $'` is 7 bytes
  (`4C 65 76 65 6C 20 24`), with **no** terminating `00h` (unlike C strings). The
  assembler therefore has no automatic way to know where a string ends.
* Historically the terminator was `'$'` because DOS function `09h` (`int 21h` with
  `AH=09h`) printed until it met a `$`. **This program no longer calls DOS for text at
  all** — every character goes straight to video RAM — but the strings keep their `'$'`
  and the terminator role was handed to `vstr`:

```asm
vs_next:
    mov al,[si]
    cmp al,'$'
    je vs_done
    inc si
    mov ah,cl
    call vcell                  ; leaves DH, DL alone
    inc dl
    jmp vs_next
vs_done:
```

* Because `'/'`, letters, digits and spaces are legal inside a string and `'$'` never
  appears in any of the texts, the marker is unambiguous. A string *without* the `'$'`
  would make `vstr` run off into the next label's bytes and spray garbage across the HUD.
* `msgdash db '---$'` is a whole "string" of 4 bytes used as the *value* printed when the
  distance is unknown (`updhud` prints it instead of `vnum3`).

**Why it matters:** the terminator convention is data, not instruction — changing `vstr`
to stop on `00h` would require re-editing all 11 strings.

### 7.3 Case folding with `or al,20h`

```asm
    cmp al,27                   ; ESC as ASCII
    je quit
    cmp ah,01h                  ; ESC as scancode
    je quit

    or al,20h                   ; force lower case: 'W' and 'w' both become 'w'
```

Uppercase and lowercase letters differ in **exactly one bit — bit 5**:

```
'W' = 57h = 0101 0111          'w' = 77h = 0111 0111
'A' = 41h = 0100 0001          'a' = 61h = 0110 0001
                                OR 20h (0010 0000) sets bit 5
```

so a single `or al,20h` folds any letter, and the code only has to compare lowercase:

```asm
    cmp al,'h'
    je hintkey
    ...
    cmp al,'w'
    je goup
```

Ordering subtleties (why the ESC checks sit *above* the fold):

* `1Bh OR 20h = 3Bh` (`';'`) — if folding happened first, `cmp al,27` could never match.
* For arrow keys `AL = 00h`, so after the fold `AL = 20h` (space). Harmless: `' '`
  matches none of `'h'/'w'/'s'/'a'/'d'`, so execution falls through to
  `jmp game` unless the *scancode* in `AH` (`48h` etc.) matched first.
* The checks of `AH` (scancodes) are unaffected by folding because only `AL` is folded.

### 7.4 Converting numbers to characters

Digits are ASCII, not binary, so printing means adding `30h`:

```asm
    mov ah,0
    aam                         ; AH = tens, AL = units
    add ax,3030h                ; both digits become ASCII
```

`aam` (ASCII-adjust AX after multiply, `D4 0A`) divides `AL` by 10: quotient into `AH`,
remainder stays in `AL`. Worked example — `prow = 7`:

```
AX = 0007h -> aam -> AH=00, AL=07 -> add ax,3030h -> AH='0' (30h), AL='7' (37h)
```

`vnum3` does the same by hand with `div`, because distances can exceed 99 (the file's own
comment: *"the BFS distance, which can pass 99 (e.g. level 3 = 140)"*). Worked example —
`hintd = 140`:

```
xor ah,ah ; AX = 140
mov bl,100 ; div bl -> AL = 1 (hundreds), AH = 40 (remainder)
add al,30h ; AL = '1'
...
mov al,ch ; 40
div bl ; BL = 10 -> AL = 4, AH = 0 -> '4'
...
mov al,bl ; 0 -> '0'
```

Result on screen: `140`. And the HUD level digit:

```asm
    mov al,level
    add al,'1'                  ; 0 -> '1', 2 -> '3'
```

`level = 0` -> `00h + 31h = 31h = '1'`; `level = 2` -> `33h = '3'`.

---

## 8. Encoding facts: what bytes come out

All numbers below are for **real-mode 8086 encodings**. Where a field depends on where
the linker puts a segment, `xx xx` marks it. (MASM-family assemblers may pick between
two *equivalent* encodings of the same register-to-register operation — e.g. `30 E4` vs
`32 E4` for `xor ah,ah` — both are 2 bytes and semantically identical.)

### 8.1 Instructions from the program

| Source line | Bytes | Explanation |
|---|---|---|
| `nop` (reference) | `90` | 1 byte |
| `ret` | `C3` | pop IP |
| `inc si` | `46` | one-byte form for `inc r16` |
| `dec bl` | `FE CB` | `FE /1`, mod=11, reg=001, rm=011 -> CB |
| `inc bl` | `FE C7` | `FE /0` with rm=BL -> C7 |
| `xor ah,ah` | `32 E4` | 2 bytes; also clears CF/ZF |
| `mov ah,0` | `B4 00` | `mov r8,imm8` = `B0+rw` |
| `mov bl,80` | `B3 50` | `B0 + BL(011)` = B3 |
| `mov cl,MW` | `B1 19` | MW = 25 = 19h |
| `mov cx,MSZ` | `B9 71 02` | 625 = 0271h, **little-endian**: 71 then 02 |
| `mov ax,0003h` | `B8 03 00` | 80x25 text mode number |
| `mov ax,0B800h` | `B8 00 B8` | video segment, immediate low byte first |
| `mov ax,4C00h` (in `mov ah,4Ch` form) | `B4 4C` | AH = 4Ch |
| `mov ds,ax` | `8E D8` | `8E /r`, mod=11, reg=DS(011), rm=AX(000) -> D8 |
| `mov es,ax` | `8E C0` | reg=ES(000) -> C0 |
| `mov ax,@data` | `B8 xx xx` | xx xx = DGROUP paragraph, **relocated** |
| `mov si,offset msgwin` | `BE xx xx` | `mov r16,imm16`, SI = 0000+rw(110) = BE |
| `mov al,mazes[si]` | `8A 84 xx xx` | `8A /r`, mod=10 (disp16), rm=100 ([SI]) -> 84 |
| `mov bl,prow` | `8A 1E xx xx` | mod=00, rm=110 -> direct address, reg=BL(011) -> 1E |
| `mov dist[si],dl` | `88 94 xx xx` | `88 /r`, mod=10, reg=DL(010), rm=100 -> 94 |
| `mov es:[di],al` | `26 88 05` | `26` = ES override, `88 /r`, rm=101 ([DI]) -> 05 |
| `mov es:[di+1],ah` | `26 88 65 01` | mod=01 (disp8=01), reg=AH(100), rm=101 -> 65 |
| `or al,20h` | `0C 20` | `OR AL,imm8` |
| `add al,'1'` | `04 31` | `ADD AL,imm8`, 31h = `'1'` |
| `add ax,3030h` | `05 30 30` | `ADD AX,imm16` |
| `cmp al,'h'` | `3C 68` | `CMP AL,imm8` |
| `cmp al,27` | `3C 1B` | 27 **decimal** = 1Bh (ESC) |
| `cmp bl,MH` | `80 FB 19` | `80 /7`, mod=11, rm=BL(011) -> FB |
| `mul cl` | `F6 E1` | `F6 /4`, reg=100, rm=CL(001) -> E1; `AX = AL*CL` |
| `mul cx` | `F7 E1` | `DX:AX = AX*CX` |
| `div bl` | `F6 F3` | `F6 /6`, rm=BL(011) -> F3; `AL=quot, AH=rem` |
| `shl ax,1` | `D1 E0` | `D1 /4`, rm=AX(000) -> E0 |
| `shl si,1` | `D1 E6` | rm=SI(110) -> E6 |
| `push ax` / `pop ax` | `50` / `58` | one-byte opcodes |
| `push ds` / `pop es` | `1E` / `07` | segment push/pop |
| `stc` / `clc` | `F9` / `F8` | flag manipulation |
| `aam` | `D4 0A` | 2 bytes, fixed divisor 10 |
| `rep stosb` | `F3 AA` | `F3` prefix + `AA` |
| `int 10h` / `int 16h` / `int 21h` | `CD 10` / `CD 16` / `CD 21` | `CD ib` |
| `jmp game` (near target) | `EB xx` | short jump, `rel8` |
| `je game` | `74 xx` | short conditional, `rel8` |
| `je quit` (target ~180 bytes away) | `75 xx  E9 xx xx` (5 bytes) | **promoted**: `jne temp / jmp quit / temp:` — see below |
| `call getcell` | `E8 xx xx` | near call, `rel16` (3 bytes) |

**Jump promotion.** All 8086 conditional jumps are `rel8` (target within -128..+127
bytes of the *next* instruction). In this program the ESC checks sit near the top of the
input loop while `quit:` is far below the move/exit/hint code — roughly 180 bytes away —
so a plain `74 xx` could not reach it. MASM 6 and JWasm fix this automatically by
inverting the condition and following it with an unconditional near jump:

```asm
    je quit                     ; source line
    ...
; assembler emits:
    jne temp$1                  ; 2 bytes: skip if NOT equal
    jmp quit                    ; 3 bytes: E9 rel16 (or EB rel8 if close enough)
temp$1:
```

The result is 5 bytes instead of 2, but the branch always lands. (Older toolchains —
MASM 5.1, TASM without jump promotion, NASM at a plain 8086 CPU level — reject the
statement instead; see Section 9.)

### 8.2 Byte/word sizes of the data

| Declaration | Element size | Total bytes | Consequence in code |
|---|---|---|---|
| `level db 0` ... `wantd db 0` | 1 | 9 | plain `mov al,prow` — no scaling |
| 11 strings `db '...$'` | 1 per char | 130 | `vstr` walks bytes with `inc si` |
| `mazes db '####...'` (75 rows) | 1 | 1875 = 3 x 625 | `mov al,mazes[si]`, index = `level*625 + row*25 + col` |
| `dist db MSZ dup(0FFh)` | 1 | 625 | `mov dl,dist[si]` — **byte** index, no shifting |
| `queue dw MSZ dup(0)` | 2 | 1250 | `mov ax,queue[si]` — needs `shl si,1` first |
| `bhead dw 0`, `btail dw 0`, `lbase dw 0` | 2 | 6 | word compares (`cmp ax,btail`) |
| `exrow db 0`, `excol db 0` | 1 | 2 | loaded separately into AL / AH |

The asymmetry between `dist` (bytes) and `queue` (words) is visible in the code:

```asm
    mov si,ax
    shl si,1                    ; queue holds words
    mov ax,queue[si]
```

### 8.3 Little-endian, and the packed queue word

x86 stores multi-byte values **least-significant byte first** (low address = low byte):

* `mov cx,625` -> `B9 71 02` (0271h stored as 71, then 02)
* `mov ax,0B800h` -> `B8 00 B8` (0B800h stored as 00, then B8)

The BFS queue exploits this by packing **two bytes into one word**: the row in the low
byte, the column in the high byte.

```asm
    mov al,exrow
    mov ah,excol                ; queue entry: AL = row, AH = col
    xor di,di
    mov queue[di],ax
```

Worked example — exit at row 3, column 17:

```
AL = 03h (row), AH = 11h (col) -> AX = 1103h
store of the word writes:  memory[queue+0] = 03h   memory[queue+1] = 11h
                                     low byte             high byte
```

and the consumer splits it back:

```asm
    mov ax,queue[si]
    inc bhead
    mov bl,al                   ; row of the cell being expanded
    mov bh,ah                   ; column
```

`BL = 03h` (row), `BH = 11h` (column) — exactly the `(BL,BH)` convention used by
`getcell`, `trynb` and `findnb`.

**Why it matters:** row/col both fit in 0..24, so one word per queue entry halves the
queue footprint and lets a single `mov ax,queue[si]` transfer both coordinates. The
little-endian layout is what makes `AL = low byte` work — on a big-endian machine the
roles would be swapped.

---

## 9. Common assembler errors and how to read them

MASM and JWasm report errors as:

```text
MAZE.ASM(184): error A2xxx: <message>
```

i.e. **file(line), severity, code, text**. The line number is your first hint; the code
is stable across MASM-family versions (wording can differ slightly between MASM, JWasm
and TASM). Warnings (`warning A4xxx`) do not stop the build but usually point at real
suspicion, so treat them as errors.

| Error | Typical message | Cause with a realistic MAZE.ASM typo | Fix |
|---|---|---|---|
| Undefined symbol | `error A2006: undefined symbol : PROWW` | `mov bl,proww` (spelling), or a label used before *any* definition in a two-pass edge case | correct the spelling; check the listing |
| Symbol redefinition | `symbol redefinition` / `already defined` | defining `level db 0` twice, or a label named `move` on two lines | rename one; remember `equ` is immutable too |
| Phase error | `phase error between passes` | a symbol's value changed between pass 1 and pass 2 — e.g. an `EQU` redefined inside a macro/conditional so an instruction grew/shrank between passes | define constants once, before use; avoid redefining symbols between passes |
| Immediate to segment register | `invalid use of segment register` / `operand must have size` | `mov ds,1234h` — the CPU has no encoding for it | `mov ax,1234h` then `mov ds,ax` |
| Jump out of range | `relative jump out of range` / `jump to label too far` | a `je`/`jne`/`jb` target more than **127 bytes** away — all 8086 conditional jumps are `rel8` | branch over an inverted jump: `jne skip` / `jmp far_target` / `skip:` (MASM 6 and JWasm do this promotion automatically; MASM 5.1, TASM without its jump-promotion option, and NASM at an 8086 CPU level report the error) |
| Operand size not specified | `operand must have size` / `operation size not specified` | `mov [si],1` — a memory destination with no register to imply width | `mov byte ptr [si],1` (or `word ptr`) |
| Wrong operand size | `operand type mismatch` | `mov al,bhead` (loading a `dw` into a byte register) | use `mov ax,bhead`, or declare the data `db` |
| Illegal immediate for memory | `immediate data too large` | `mov ax,12345h` (doesn't fit 16 bits) | keep immediates within the operand size |
| Missing `end` | `no end directive found` | accidental deletion of `end main` | restore it — it also carries the entry point |
| Junk after statement | `extra characters on line` | `mov ax,1 2` or a missing comma | fix the syntax |

How to diagnose, in order:

1. **Read the reported line, then the line *before* it** — a missing `endp` or an
   unclosed string usually surfaces one line later.
2. **Generate a listing** (`masm /Fl MAZE.ASM;` or `jwasm -FlMAZE.LST MAZE.ASM`). Each
   line shows the assigned offset and the emitted bytes:

   ```text
   00000000 B4 4C              mov ah,4Ch
   00000002 CD 21              int 21h
   ```

   Offsets are segment-relative, so gaps reveal missing/extra bytes and you can see
   exactly which instruction grew.
3. **Check the symbol table at the end of the listing** — if `PROWW` is absent and
   `PROW` is present, it is a typo; if a symbol has the wrong size (` dword` vs `byte`),
   it is a redefinition.
4. For "jump out of range", look at the distance between the `je` and its label in the
   listing (offset difference > 007Fh).

**Why it matters:** assembler errors are almost always *local and mechanical* — unlike a
compiler's cascading type errors, one wrong byte size is usually the whole story.

---

## 10. Debugging tools

### 10.1 `DEBUG.COM` — the debugger that ships with DOS

One-letter commands at the `-` prompt:

| Command | Meaning | Example use here |
|---|---|---|
| `N file` | name the file to load | `N MAZE.EXE` |
| `L` | load it (applies EXE relocations) | |
| `R` | show/modify registers (also shows `CS:IP` = entry point) | check that `DS` is *not* your data segment yet |
| `U [addr]` | unassemble into mnemonics | verify bytes match the listing (`B4 4C CD 21`) |
| `D [addr]` | hex + ASCII dump | `D DS:0100` to inspect `prow`/`pcol` |
| `E addr bytes` | enter/patch bytes | |
| `T [=addr]` | single-step one instruction, showing registers | trace `mov ax,@data` / `mov ds,ax` |
| `P [=addr]` | proceed (step *over* `int`/`call`) | step over `int 16h` without entering the BIOS |
| `G [=addr]` | run, optionally to an address (a breakpoint) | `G CS:0032` to stop at `game:` |
| `Q` | quit | |

Minimal session:

```text
C:\> DEBUG MAZE.EXE
-R
AX=0000 BX=0000 CX=0000 DX=0000 SP=0100 BP=0000 SI=0000 DI=0000
DS=xxxx ES=xxxx SS=xxxx CS=xxxx IP=0000 NV UP EI PL NZ NA PO NC
-U CS:0000
-T
-D DS:0000 40
```

(Exact segment values depend on where DOS loaded the image; that is precisely why the
program loads `DS` itself instead of assuming anything.)

### 10.2 Turbo Debugger (`TD.EXE`)

Borland's graphical debugger (bundled with TASM) gives source-level views:

```dos
TD MAZE.EXE
```

* **Breakpoints:** move the cursor to a line in the code window, press `F2` (toggles a
  red breakpoint). `F5` runs, `F7` steps into, `F8` steps over, `F11`/`F10` equivalent.
* **Windows:** Code, Data (variables), Registers, CPU (flags + disassembly), Stack —
  from the menu or `Alt+<key>`.
* **Symbols:** to see names like `prow` instead of addresses, assemble with debug info
  (`masm /Zi MAZE.ASM;` or `ML /Zi`) and link with `LINK /CO MAZE.OBJ` — the OMF debug
  records travel through the OBJ into the EXE, and TD reads them.
* **Watch:** add `prow`, `pcol`, `hintd` to a watch window and they refresh on every
  stop — ideal for watching the player's position change one keypress at a time.

### 10.3 The DOSBox debugger

Vanilla DOSBox only has a debugger if it was compiled with debug support;
**DOSBox-X** and debug builds expose it directly (DOSBox-X even starts the program *and*
stops at the entry point with `DEBUGBOX MAZE.EXE`). Commands are typed in the bottom
command line; all values are hex:

| Command / key | Action |
|---|---|
| `BP <seg>:<off>` | set an execute breakpoint (also `F9` at the cursor) |
| `BPINT <int> [ah]` | break when an interrupt is called (e.g. `BPINT 21 4C`) |
| `BPM <seg>:<off>` | break whenever that memory location *changes* |
| `BPLIST` / `BPDEL <n>` / `BPDEL *` | list / delete breakpoints |
| `C <seg>:<off>` / `D <seg>:<off>` | jump the **code** / **data** view to an address |
| `INT <nr>` | execute an interrupt and return |
| `F5` | resume emulation |
| `F10` / `F11` | step over / step into one instruction |
| `Alt+D` / `Alt+E` / `Alt+S` | point the data view at `DS:SI` / `ES:DI` / `SS:SP` |
| Tab | cycle between code / data / register / log windows |

### 10.4 Worked recipe: break at `game:` and inspect `prow`, `pcol`, `dist`

1. **Build with a listing and (optionally) debug info:**

   ```dos
   JWASM -MZ -FlMAZE.LST MAZE.ASM        (or: ML /Zi /Fl MAZE.ASM, then LINK /CO)
   ```

2. **Look up the offsets in `MAZE.LST`.** The listing prints the segment-relative offset
   of every label — find the line `game:` and note its offset (call it `GGGG`), and the
   offsets of `prow`, `pcol` and `dist` in `_DATA` (call them `PPPP`, `CCCC`, `DDDD`).
   Absolute offsets differ per build, which is why you read them instead of guessing.

3. **Start under the DOSBox debugger:**

   ```text
   DEBUGBOX MAZE.EXE            (DOSBox-X: stops at the entry point of main)
   ```

   The register window now shows the runtime `CS` and `DS` — these change on every run,
   so combine them with the listing offsets:

   ```text
   BP <CS>:GGGG                 breakpoint at the 'game:' label
   BPINT 21 4C                  bonus: break on the clean DOS exit in 'quit'
   F5                            resume; press any key in the game window
   ```

   The emulator stops with `IP = GGGG`, i.e. immediately before `mov ah,0 / int 16h`.

4. **Inspect the state:**

   ```text
   D <DS>:PPPP 1                prow (1 byte) — e.g. 01
   D <DS>:CCCC 1                pcol (1 byte) — e.g. 01
   D <DS>:DDDD 271              dist[] — 625 bytes: FFh for untouched cells,
                                00h at the exit, increasing rings outwards
   ```

   In the register window you can also read `BL`/`BH`: at the top of `game:` they are
   loaded with `prow`/`pcol` (`mov bl,prow` / `mov bh,pcol`), so `BL` is the candidate
   row for the key you are about to press.

5. **Watch a single move:** set `BPM <DS>:PPPP` (break when `prow` changes), press an
   arrow key, and stop *exactly* on the instruction that commits the move
   (`mov prow,bl` in the `move:` block). Step with `F11` into `putplayer` ->
   `vcell` and watch `ES` change to `B800h` and `DI` become
   `(row*80+col)*2`.

6. **Cross-check against the listing:** if the debugger's disassembly does not match
   `MAZE.LST`, you are looking at a stale `MAZE.EXE` — rebuild.

**Why it matters:** with direct video RAM writes there is no BIOS call to "print" your
way into the middle of a routine; the debugger's memory view of `B800h` *is* the screen,
so `D B8000:0000 50` shows the first 40 cells (character/attribute pairs) of row 0.

---

## 11. Glossary

| Term | Meaning |
|---|---|
| **Assembler** | Program translating mnemonics into machine-code bytes (MASM, JWasm, NASM, TASM). |
| **Directive** | Source line addressed to the assembler, not the CPU (`.model`, `db`, `equ`, `proc`, `end`). |
| **Instruction** | A CPU operation with an opcode (`mov`, `int`, `mul`). |
| **Mnemonic** | The textual name of an instruction, e.g. `stosb`. |
| **Macro** | Named source template expanded at assembly time; not used in MAZE.ASM. |
| **Object file (.OBJ)** | Assembler output: code/data bytes plus fixups, not runnable. |
| **OMF** | Object Module Format: the record-based container (`SEGDEF`, `FIXUPP`, `MODEND`...) MASM/JWasm write. |
| **Linker** | Combines objects, resolves externals, applies fixups, writes the executable header. |
| **Relocation / fixup** | A field patched at link time (internal symbols) or load time (segment base) because its final value is not known when the code is assembled. |
| **Entry point** | Address execution starts at; given to the linker by `end main`, stored in the EXE header as `CS:IP`. |
| **Segment** | A 64 KB block addressed by a segment register; in real mode its base is `segment * 16`. |
| **Offset** | A 16-bit position inside a segment; physical address = `segment * 16 + offset`. |
| **Paragraph** | An address that is a multiple of 16 bytes; segment bases are paragraph-aligned. |
| **Memory model** | The declared arrangement of code/data segments and pointer sizes (`.model small` here). |
| **DGROUP** | The group bundling data (and stack) segments so one segment register addresses them all; `@data` refers to it. |
| **Near / far pointer** | 16-bit offset only (near, used here) vs 16:16 segment:offset (far). |
| **Register** | On-CPU storage: `AX BX CX DX SI DI BP SP` plus `CS DS SS ES IP FLAGS`. |
| **Addressing mode** | How an operand's location is expressed: immediate, register, direct, `[SI]`, `disp[SI]`, `ES:[DI]`, `offset`. |
| **Little-endian** | Multi-byte values stored low byte first (`625 = 0271h` -> `71 02`); also makes `AL`=row, `AH`=col packing work. |
| **MZ header** | The `MZ`-signature DOS executable header holding `CS:IP`, `SS:SP` and the relocation table. |
| **Listing file (.LST)** | Assembler report with offsets and emitted bytes per source line; used for debugging and error hunting. |
| **Scancode** | Keyboard hardware code returned in `AH` by `int 16h` (e.g. `48h` = up arrow); ASCII is returned in `AL`. |
| **ASCII** | Character encoding: `'0'` = 30h, `'A'` = 41h, `'a'` = 61h, ESC = 1Bh. |
| **Attribute byte** | The high byte of a video cell: high nibble background, low nibble foreground (e.g. `A_WALL = 1Fh`). |
| **Video RAM** | Memory at `B800h:0000` in text mode; 2 bytes per cell (character, attribute). |
| **Software interrupt** | `INT n` — a call into a BIOS/DOS handler vector (`int 10h`, `int 16h`, `int 21h`). |
| **BFS** | Breadth-first search: fills `dist[]` with the shortest cell-to-exit distances, run once per level. |
