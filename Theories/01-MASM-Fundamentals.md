# Theory 01 — MASM Fundamentals

What MASM is, how a `.ASM` file becomes a running `.EXE`, and every directive used in `MAZE.ASM`.

---

## 1. What is MASM?

**MASM** = **M**icrosoft **A**ssembler. It is an assembler for the x86 family: it translates
mnemonic assembly (`mov`, `cmp`, `int`...) into machine code (the raw bytes the CPU executes).

| Term | Meaning |
|---|---|
| Assembler | Translates ASM → object code (`.OBJ`) |
| Linker (`LINK`) | Combines `.OBJ` files, resolves external symbols, produces `.EXE` |
| Compiler | Translates a high-level language (C, ...) → ASM/object |
| Directive | A command to the **assembler**, not the CPU (`.model`, `db`, `proc`) |
| Instruction | A real CPU operation (`mov`, `add`, `int`) |

Assembly is a **1:1 mapping** to machine instructions — one line ≈ one CPU instruction
(with a few pseudo-ops that expand into several).

### Build pipeline

```
MAZE.ASM ──masm──▶ MAZE.OBJ ──link──▶ MAZE.EXE ──DOS/DOSEMU──▶ runs
```

```
masm MAZE.ASM;
link MAZE.OBJ;
MAZE
```

Alternative modern free assembler: **JWasm** (`jwasm -mz MAZE.ASM`) — the `-mz` switch
selects the classic DOS MZ executable format. NASM is a *different* assembler with a
different syntax (no `.model`, no `proc`).

---

## 2. The directives used in MAZE.ASM

```asm
.model small
.stack 100h
.data
.code
main proc ... main endp
end main
```

### `.model small` — memory model

Real-mode DOS programs must declare how they lay out code and data in the 1 MB address
space (segment:offset, 20-bit addresses from 16-bit registers).

| Model | Code | Data | Used for |
|---|---|---|---|
| **`small`** | 1 segment (64 KB) | 1 segment (64 KB) | ← our program |
| `tiny` | 1 (shared CS=DS) | same | .COM files |
| `medium` | many | 1 | large code |
| `compact` | 1 | many | large data |
| `large` / `huge` | many | many | big programs |

`small` means:
- all code fits in **64 KB** (CS = code segment)
- all data fits in **64 KB** (DS = data segment)
- near pointers (16-bit offsets) are enough — no `ES:` prefixes needed for data

### `.stack 100h`

Reserves **256 bytes (100h)** for the stack and tells the linker the initial SS:SP.
Used by `push`/`pop` (our `pnum` procedure uses it) and by `call`/`ret`.

### `.data` / `.code`

- `.data` — start of the **data segment**: variables, strings, the maze tables.
- `.code` — start of the **code segment**: procedures and instructions.

### `name db value` — data definition

| Directive | Size | Example in our code |
|---|---|---|
| `db` | 1 byte | `level db 0`, `prow db 1`, `mazes db '#...'` |
| `dw` | 2 bytes | (not used — a table of `dw` offsets is an *upgrade*) |
| `dd` | 4 bytes | — |
| `dq` | 8 bytes | — |
| `dup` | repeat | e.g. `625 dup('#')` — useful for upgrades |

Strings are just consecutive bytes. The `$` in `'Level $'` is **not** part of the string —
it is the **terminator for DOS function 09h**.

### `label proc` / `label endp` — procedures

```asm
getcell proc
    ...
    ret
getcell endp
```

Defines a callable subroutine. `call getcell` pushes the return address and jumps;
`ret` pops it and resumes. MASM checks argument/label names inside the procedure.

### `end main` — end of source + entry point

Tells the assembler "file ends here" **and** marks `main` as where execution starts
(the linker writes that offset into the EXE header).

---

## 3. Segments, and why DS must be initialised

Real mode addresses are `segment:offset`. The CPU adds `seg << 4 + offset`.

```asm
main proc
    mov ax,@data    ; @data = address of the .data segment (a linker-defined symbol)
    mov ds,ax       ; DS cannot be loaded directly — only via a general register
    ...
```

**Why this is always the first thing a MASM small-model program does:** every variable
access such as `mov bl,prow` or `mov al,mazes[si]` is really `[DS:prow]`. If DS still
holds whatever DOS left there, every variable read/write goes to random memory.

Facts to remember:
- `mov ds,reg` is legal; `mov ds,1234h` (immediate → segment register) is **not**.
- CS is set by the loader from the EXE header; SS:SP from `.stack`.
- String instructions and `int` calls may require ES to be valid too (`ES` is used by
  DOS for disk buffers, handled by DOS itself).

---

## 4. Two flavours of "interrupts" you must not confuse

1. **`INT n` — software interrupt**: `call int 10h/16h/21h` services (see Theory 02).
2. **Keyboard interrupt (hardware, IRQ1 → INT 09h)** — the problem statement says
   *"handling keyboard interrupts"*. The BIOS already installs an INT 09h handler that
   reads the keyboard port, converts the scancode into a keystroke, and puts it into the
   BIOS keyboard buffer. Our program then **reads that buffer** with `INT 16h`.

   True *hooking* of INT 09h (replacing the vector) is an **add-on**, not what the
   current code does.

---

## 5. Registers used in MAZE.ASM

| Register | Role in this program |
|---|---|
| `AX` | AH = DOS/BIOS function number, AL = data/return. `@data` load, `mul` results |
| `BX` | **BL = candidate row, BH = candidate col** (split 8-bit pair) |
| `CX` | Counter: `625` for level scaling, `25` for `LOOP`, `mul cx` |
| `DX` | DH/DL = cursor row/col for INT 10h; DL = character for INT 21h/02h; DX = string address for 09h |
| `SI` | **Source/index register** — byte offset into `mazes` |
| `DS` | Data segment base (set once at entry) |
| `SP/BP` | Stack pointer / frame pointer (`push ax` in `pnum`) |

Convention: `AX` (and often `BX`, `CX`, `DX`) are **scratch/argument** registers,
`SI`, `DI`, `BP` are preserved across DOS calls. Our procedures deliberately use this.

---

## 6. Addressing modes present in the code

| Mode | Example | Meaning |
|---|---|---|
| Immediate | `mov cx,625`, `mov prow,1` | constant in the instruction |
| Register | `inc bl`, `or al,20h` | value in a register |
| Direct | `mov al,prow` | fixed variable at `[DS:offset]` |
| **Register indirect / based-index** | `mov al,mazes[si]` | `[DS:SI + constant]` — the maze lookup |
| Implied | `inc si`, `ret` | operands fixed by the opcode |

`mazes[si]` is **direct-indexed addressing**: effective address = `offset(mazes) + SI`.
That single instruction is the heart of the whole game.

---

## 7. ASCII facts used

| Char | Code | Note |
|---|---|---|
| `'0'..'9'` | 30h..39h | digits |
| `'A'..'Z'` | 41h..5Ah | upper case |
| `'a'..'z'` | 61h..7Ah | lower case |
| `'#'` | 23h | wall |
| `'@'` | 40h | player |
| `'E'` | 45h | exit |
| ESC | 27 (1Bh) | quit |
| `$` | 24h | end marker for INT 21h/09h |

Case conversion trick used in the code:

```asm
or al,20h     ; sets bit 5 → 'W'(57h)→'w'(77h), 'A'(41h)→'a'(61h)
```

This works because upper- and lower-case letters differ **only in bit 5**. It is why
both `W`/`w` and arrow keys are accepted.

---

## 8. Assembler ≠ CPU

Remember which things are **assembler-time** and which are **run-time**:

| Assembler-time (directives) | Run-time (instructions) |
|---|---|
| `.model`, `.data`, `.code` | `mov`, `add`, `mul` |
| `db 'text$'` (data layout) | `int 21h` (call DOS) |
| `proc`/`endp` (naming/checking) | `call`/`ret` (stack) |
| `end main` (EXE entry) | `je`/`jmp` (control flow) |
| `equ` constants (e.g. `WIDTH equ 25`) | `cmp` (flags) |

Nothing in the first column exists in the final EXE except the bytes it produced.
