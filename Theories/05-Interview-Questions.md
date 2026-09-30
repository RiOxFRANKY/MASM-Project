# 05 — Interview Questions: MAZE.ASM (16-bit DOS maze game)

A large, exam-style question bank for the current, upgraded `Code/MAZE.ASM`.
Every answer below was written against that file as the single source of truth
(the README line-by-line commentary is not authoritative).

**Programme at a glance**

| Item | Value in `MAZE.ASM` |
|---|---|
| Target | 16-bit real-mode DOS `.EXE` (MZ) |
| Directives | `.model small`, `.stack 100h`, `.data`, `.code`, `end main` |
| Maze | `MW equ 25`, `MH equ 25`, `MSZ equ 625`, `NLEV equ 3` (1875 data bytes) |
| Attributes | `A_WALL 1Fh`, `A_PATH 07h`, `A_EXIT 2Eh`, `A_PLAY 4Eh`, `A_HINT 3Fh`, `A_HUD 0Bh`, `A_WIN 2Fh`, `A_MSG 0Fh` |
| Screen output | direct stores into video RAM `B800h`, `offset = (row*80+col)*2` |
| INT 10h | only `AX=0003h` (mode 03h), twice: startup and win screen |
| INT 16h | only `AH=00h`, twice: game loop and win screen |
| INT 21h | only `AH=4Ch` (terminate) |
| Input | `AL`=ASCII, `AH`=scancode; ESC = `cmp al,27` **or** `cmp ah,01h`; arrows `48h/50h/4Bh/4Dh` on `AH`; WASD on `AL` after `or al,20h`; `H` toggles the hint |
| Search | BFS once per level from the exit `'E'`, byte `dist[]` sentinel `0FFh`, word queue packed `AL`=row/`AH`=col |
| Verified distances | level 0 start = **100** (299 walkable cells), level 1 = **68** (292), level 2 = **140** (287); every exit at **(23,23)**; all walkable cells reachable |
| HUD | column 30 labels, values at column 36; rows 2/3/4/5 = Level/Dist/Row/Col, rows 7/8/9/10 = keys/help |
| Cost per move | at most 4 repaints of maze cells + the three HUD fields (3 + 2 + 2 = 7 characters); full maze draw only at level start |
| Build | `jwasm -mz MAZE.ASM` → 0 errors, 0 warnings |

---

## A. MASM and assembler fundamentals

### A1. List every assembly directive used in `MAZE.ASM` and say what each one does.

**Answer.**

| Directive | Where | Meaning |
|---|---|---|
| `.model small` | line 15 | small memory model: one near code segment (≤64K) + one DGROUP data segment (≤64K); all calls/jumps inside the module are near |
| `.stack 100h` | line 16 | declares a 256-byte stack; the EXE header makes DOS set `SS:SP` from it |
| `.data` / `.code` | lines 39, 156 | switch the location counter to the data segment / code segment |
| `equ` | `MW`, `MH`, `MSZ`, `NLEV`, `A_*` | assemble-time constant; occupies **no** storage and generates no relocation |
| `db` / `dw` | throughout | define byte / word in the current segment |
| `dup` | `db MSZ dup(0FFh)`, `dw MSZ dup(0)` | repeat an initialiser *n* times: 625 bytes of `0FFh`, 625 words of `0` |
| `proc` / `endp` | every procedure | named block, enables `call name`, exports a symbol for the linker |
| `offset` | `mov si,offset msgwin` etc. | 16-bit offset of a symbol inside its segment (no segment selector) |
| `end main` | last line | ends the source **and** names the entry point, so the loader starts at `main` |
| `seg`, `assume`, `PTR`, `STRUCT` | not used | the model directives plus JWasm defaults handle segment assumptions automatically |

**Go deeper.** `equ` is folded by the assembler: `MSZ equ MW * MH` becomes `625`, so `mov cx,MSZ` assembles as `mov cx,625` and costs exactly the same bytes as the literal.

### A2. What exactly does `.model small` buy this program?

**Answer.** Code and data each fit in one 64K segment, so:

* every `call`/`jmp` in the module is **near** — a 16-bit `IP` is pushed, `ret` (`C3`) rather than `retf` (`CB`);
* no far pointers, no `seg` overrides, no `assume` juggling;
* the whole data set (`mazes` 1875 + `dist` 625 + `queue` 1250 + strings + state ≈ 3.9KB, plus 256 bytes of stack) is far below the 64K DGROUP limit, so `SI`/`DI` index it directly.

**Go deeper.** In `.model large` the data would need `DS` switching or a second segment register for anything past 64K, and the flat `mazes[si]` form would no longer reach the whole three-level table.

### A3. Why can't the prologue be written as `mov ds,@data`?

**Answer.** `MOV <segment register>, imm16` has **no encoding on the 8086** (a mov *to* `DS` accepts only an r/m16 source), so the value must travel through a general register:

```asm
main proc
    mov ax,@data
    mov ds,ax
```

`@data` is MASM's predefined symbol for the start of DGROUP. Loading it via `AX` costs one extra instruction but is the only legal 8086 form.

**Go deeper.** `push @data / pop ds` is an equally legal 2-byte idiom (versus 4 bytes for `mov ax,@data / mov ds,ax`), and neither form affects flags.

### A4. What is "the pipeline" here — the assembler's, or the CPU's?

**Answer.** Both readings are worth separating:

1. **Assembler passes.** MASM/JWasm are multi-pass: pass 1 assigns labels, offsets and sizes; later passes emit machine code. Forward references such as `je quit` (line 188, `quit` at line 285) or `jb start_level` (line 251, label at 169) resolve because symbol values are known before the final encoding (short `74h`/`EBh` vs near form) is chosen.
2. **CPU pipeline.** The 8086 has only a 6-byte prefetch queue, no out-of-order execution and no hazards beyond flags. Nothing here depends on it: `or al,20h` is followed by `cmp al,'h'`, which overwrites the flags the `or` produced — harmless, because no branch sits between them. There is no self-modifying code and no timing assumption.

**Go deeper.** The only performance property that matters is *how many video cells are written per keystroke*, which is a software decision (see E5, I4), not a pipeline effect.

### A5. MASM vs NASM — what would change if this file were ported?

**Answer.**

| Feature | This file (MASM/JWasm) | NASM equivalent |
|---|---|---|
| Model | `.model small` | `bits 16` plus explicit sections and linker control |
| Memory operand | `mazes[si]`, `dist[si]`, `queue[di]` | `[mazes + si]`, `[dist + si]` |
| `offset` | `mov si,offset msgwin` | `mov si,msgwin` |
| Operand size | usually inferred from the label | often needs `byte`/`word` qualifiers |
| Procedures | `name proc` / `name endp` | plain labels + `call name` |
| Constants | `MW equ 25` | `MW equ 25` (identical) |
| Entry | `end main` | linker/`_start` convention |
| Build | `jwasm -mz MAZE.ASM` or `masm` + `link` | `nasm -f obj` + link, or `nasm -f bin` |

**Go deeper.** The algorithm, register discipline and screen layout are untouched by a port; only syntax differs.

### A6. Explain little-endianness using this program.

**Answer.** x86 stores a word low byte first. The BFS queue exploits exactly that:

```asm
    mov al,exrow
    mov ah,excol                ; queue entry: AL = row, AH = col
    xor di,di
    mov queue[di],ax
```

In memory `queue[0]` holds `row` at the even offset and `col` at the odd one, and reading the word back yields `AL=row, AH=col` — precisely how `bs_loop` unpacks it:

```asm
    mov bl,al                   ; row of the cell being expanded
    mov bh,ah                   ; column
```

The same rule explains why `vnum2` prints digits **one at a time** instead of storing `AX`:

```asm
    aam                         ; AH = tens, AL = units
    add ax,3030h                ; both digits become ASCII
    mov bl,al                   ; units character
    mov al,ah                   ; tens character
```

If that word were written straight to screen memory the low byte (units) would land in the left cell and the number would read backwards.

**Go deeper.** Single cells are byte-addressed, so endianness never shows on screen for characters — it only matters where 16-bit values are packed (the queue, `add ax,3030h`).

### A7. What do `dup` and `equ` do, and what would break without them?

**Answer.**

```asm
MSZ     equ MW * MH             ; bytes per level (625)
dist    db MSZ dup(0FFh)        ; 625 bytes, all FFh
queue   dw MSZ dup(0)           ; 625 words, all 0
```

* `equ` = symbolic constant, zero storage, zero relocations. Changing `MW` to 30 would retune every `mul cl`, every `cmp …,MW` and the table sizes in one edit.
* `dup(n) value` emits *n* copies. Without it you would hand-write 625 initialisers (or clear `queue` at run time too — only `dist` is re-cleared).

**Go deeper.** `db MSZ dup(0FFh)` makes `dist` correct even before the first `bfsolve`; the run-time `rep stosb` exists because `bfsolve` runs again on every level change and must reset the array.

### A8. What is JWasm, and why `-mz`?

**Answer.** JWasm is an open-source assembler that understands the MASM dialect used here. `-mz` makes it emit a 16-bit DOS **MZ executable** directly, so this source builds in one step:

```
jwasm -mz MAZE.ASM
```

The classic route documented in the file header (`masm MAZE.ASM` then `link MAZE.OBJ`) reaches the same program in two steps.

**Go deeper.** The MZ header stores the entry point taken from `end main`, the initial `SS:SP` derived from `.stack 100h`, and the relocation entries — that is how DOS knows where to set `SS:SP` and where to start executing before the first instruction of `main` runs.

### A9. Who sets `SS:SP`, and is 256 bytes of stack enough?

**Answer.** DOS loads the EXE, initialises `SS:SP` from the header (DGROUP + `100h` bytes) and sets `DS`/`ES` to the PSP segment — which is exactly why `main` must reload `DS`. The program has no recursion; the deepest chain is roughly `main → move → refreshhint → findnb`, four or five frames of a few words each, plus whatever `INT 10h`/`INT 16h`/`INT 21h` push on our stack. 256 bytes is comfortable for that shape.

**Go deeper.** A stack overflow here would be silent (it would walk into DGROUP data). Standard protection: fill the stack area with `0CDh,020h` (`int 20h`) at start-up and inspect it before exit, or simply allocate a much larger `.stack`.

### A10. Which references in this file are forward references?

**Answer.** Every branch whose target is defined later in the source: `je quit` (line 188 → 285), `je hintkey` (195 → 270), `je doexit` (233 → 248), `jb start_level` (251 → 169) and the `je goup/godown/goleft/goright` chain. Data references such as `mov si,offset msgwin` work the same way: pass 1 records `msgwin`'s offset in `.data`, pass 2 emits it.

**Go deeper.** Because every code label lives in the same 64K code segment, all these branches fit the 8-bit short form; a target more than +127/−128 bytes away would force the near form (or a "phase error" if the assembler had already fixed the size — it does not, it sizes in the final pass).

### A11. What does `call getcell` push, and what does `ret` restore?

**Answer.** In the small model all code is in one segment, so `call` pushes a **16-bit offset only** (2 bytes) and `ret` pops it — no `CS`, and **no flags**. Because `call`/`ret` do not touch flags, `findnb` can return its answer in `CF`:

```asm
    stc
    ret
...
fn_no:
    clc
    ret
```

and the caller's `jc rh_got` (line 833) still sees the flag set inside the callee.

**Go deeper.** `int`/`iret` *do* save and restore FLAGS (and CS:IP) — a different contract, and the reason an interrupt handler can never leak flag state to the interrupted code.

### A12. Name and classify every global symbol in the file.

**Answer.**

| Kind | Symbols |
|---|---|
| Procedures | `main`, `getcell`, `vcell`, `vstr`, `vnum2`, `vnum3`, `paintcell`, `clrplayer`, `putplayer`, `clearhint`, `drawmaze`, `drawhud`, `updhud`, `bfsolve`, `trynb`, `findnb`, `refreshhint` |
| Labels inside `main` | `start_level`, `game`, `goup`, `godown`, `goleft`, `goright`, `move`, `doexit`, `hintkey`, `quit` |
| Inner labels | `vs_next/vs_done`, `pc_notwall/pc_notexit`, `ch_done`, `dm_row/dm_col/dm_notwall/dm_notexit`, `uh_dash/uh_row`, `bs_find/bs_found/bs_loop/bs_done`, `tn_done`, `fn_no`, `rh_none/rh_got`, `mv_hud`, `hk_off/hk_done` |
| Constants | `MW MH MSZ NLEV`, `A_WALL A_PATH A_EXIT A_PLAY A_HINT A_HUD A_WIN A_MSG` |
| Variables | `level prow pcol hinton hintvis hrow hcol hintd wantd`, `dist queue bhead btail lbase exrow excol` |
| Strings | `msglvl msgdst msgrow msgcol msgkey mghint msgend msgesc msgdash msgwin msgany` |
| Table | `mazes` — 3 × 25 × 25 = 1875 bytes |

**Go deeper.** `game`, `move`, `goup` … are module-global symbols even though only `main` jumps to them; MASM has no per-procedure scope for labels (anonymous `@@` labels exist, but this file does not use them).

---

## B. Memory, segments and addressing

### B1. Explain segment:offset addressing with numbers taken from this program.

**Answer.** Real-mode physical address = `segment × 16 + offset` (20 bits, 1 MB):

| Access | Segment:Offset | Physical address |
|---|---|---|
| HUD label `'L'` of `'Level $'` at (row 2, col 30) | `B800:017Ch` | `0B8000h + 017Ch = 0B817Ch` |
| Player stamp at (row 1, col 1) | `B800:00A2h` | `0B80A2h` |
| Exit cell (row 23, col 23) | `B800:0E8Eh` | `0B8E8Eh` |
| HUD "Row" value at (row 4, col 36) | `B800:02C8h` | `0B82C8h` |
| First byte of level 1 (`level=0`) | `DS:(offset mazes)` | data-segment dependent |

`DS` addresses all game data, `ES` is loaded with `0B800h` inside `vcell` and with `DS` inside `bfsolve`, `SS:SP` addresses the stack, `CS:IP` the code.

**Go deeper.** A segment register only shifts by 4 bits, so segments overlap freely; the video window at `B800h` is 32K/64K wide while mode 03h only uses the first 4000 bytes (80 × 25 × 2).

### B2. Why does only `DS` get initialised by the program itself?

**Answer.**

```asm
main proc
    mov ax,@data
    mov ds,ax
```

* **SS:SP** — already correct: the EXE header carries `.stack 100h`, and DOS installs it before `main` runs.
* **CS:IP** — set from the `end main` entry point.
* **ES** — starts out pointing at the PSP, useless here, so each routine loads it when needed (`vcell` → `0B800h`, `bfsolve` → `push ds / pop es`) and `vcell` restores it before returning.
* **DS** — also starts at the PSP, so *without* those two instructions every `mazes[si]` would read DOS's environment block instead of the maze.

**Go deeper.** In a COM file `CS=DS=ES=SS` at entry, which is why the missing-`DS` bug is invisible there and only bites EXE programs like this one.

### B3. Where is `ES` used, and why?

**Answer.** Exactly two places.

1. `bfsolve`, to give `rep stosb` a destination segment:

```asm
    push ds
    pop es                      ; ES = DS for REP STOSB
    mov di,offset dist
    mov cx,MSZ
    mov al,0FFh
    rep stosb
```

2. `vcell`, as the video segment:

```asm
    push es
    push di
    push bx
    push ax                     ; remember character + attribute
    mov ax,0B800h               ; colour text segment
    mov es,ax
    ...
    mov es:[di],al              ; even byte = character
    mov es:[di+1],ah            ; odd  byte = attribute
    pop bx
    pop di
    pop es
    ret
```

`vcell` pushes and pops `ES` so callers never observe the temporary video selector — important because `bfsolve` deliberately leaves `ES = DS`.

**Go deeper.** `DS` is never changed after start-up, so data accesses need no override; an `es:` prefix on data would only cost an extra byte and a cycle.

### B4. Catalogue every addressing mode that appears in `MAZE.ASM`.

**Answer.**

| Mode | Example | Notes |
|---|---|---|
| Register | `mov bl,al`, `mov ah,cl` | no memory access |
| Immediate | `mov cx,MSZ`, `mov ah,A_HINT`, `cmp al,'#'` | `equ` values folded at assembly time |
| Register indirect | `mov al,[si]` in `vstr` | `SI` walks the string |
| Based + displacement | `mazes[si]`, `dist[si]`, `queue[di]` | base register + 16-bit symbol offset |
| Based + displacement (byte) | `es:[di+1]` | attribute right after the character |
| Direct (absolute in `DS`) | `mov prow,bl`, `cmp hinton,0`, `mov level,…` | named variables with no base register |
| No scale addressing | whole file | `shl si,1` is used instead of `queue[si*2]`, because scale addressing is 80186+ |

**Go deeper.** `mov al,mazes[si]` needs no `DS` prefix but does need the 16-bit `mazes` displacement — it encodes as `mod=10`, i.e. `[SI] + disp16`.

### B5. Work out the row-major index for `level=1, row=5, col=7`, then show how `getcell` computes it.

**Answer.** The documented formula (line 292) is

```
index = level*MSZ + row*MW + col = 1*625 + 5*25 + 7 = 625 + 125 + 7 = 757 = 02F5h
```

and `getcell` builds exactly that in `SI`:

```asm
getcell proc
    mov al,level
    mov ah,0
    mov cx,MSZ
    mul cx                      ; AX = level * 625
    mov si,ax
    mov al,bl
    mov cl,MW
    mul cl                      ; AX = row * 25
    add si,ax
    mov al,bh
    mov ah,0
    add si,ax                   ; SI = level*625 + row*25 + col
    mov al,mazes[si]
    ret
getcell endp
```

So the byte read is `mazes + 757` — row 5, column 7 of the **second** maze block (the level index is 0-based).

**Go deeper.** Sanity check for the start position: `level=0, row=1, col=1` → index 26 → `mazes[26]`, the second byte of the second border row, which is a space — the floor tile the player begins on.

### B6. Why `row*80 + col` for the screen but `row*25 + col` for the maze?

**Answer.** Two different two-dimensional layouts:

* the **maze** uses `MW` bytes per row (`mul cl` with `cl = MW = 25`);
* the **screen** in mode 03h has 80 columns of 2-byte cells, so `vcell` computes `row*80`, adds the column, then doubles:

```asm
    mov bl,80                   ; 80 cells per row
    mul bl                      ; AX = row * 80
    xor bh,bh
    mov bl,dl                   ; column
    add ax,bx                   ; AX = row*80 + col
    shl ax,1                    ; 2 bytes per cell
    mov di,ax
```

The mismatch is deliberate: the maze only occupies screen columns 0..24, the HUD starts at column 30 (`drawhud`), and nothing overlaps.

**Go deeper.** Worked check for the HUD "Row" value at (4,36): `(4*80 + 36)*2 = 356*2 = 712 = 02C8h` → written to `B800:02C8`.

### B7. Why is the maze one flat array rather than an array of row pointers?

**Answer.** A single `db 1875` block keeps every row contiguous, so:

* a cell costs one 16-bit addition (`level*625 + row*25 + col`) with no indirection and no far pointer;
* rows are *not* independently addressable anyway — the index formula simply runs past a row boundary, which is exactly why a mis-sized row (question I5) shifts every cell after it;
* the whole table lives in DGROUP with 16-bit `DS`-relative offsets;
* linear scans work naturally: `cmp mazes[si],'E'` in `bs_find`, `inc si` walking one level at a time.

**Go deeper.** An array of 75 near row pointers (3 × 25) would save 75 bytes but add one memory fetch per cell and complicate `bs_find`.

### B8. What physical address is written when the hint cell at (row 6, col 9) is highlighted?

**Answer.** `(6*80 + 9)*2 = (480 + 9)*2 = 489*2 = 978 = 03D2h`, so the character byte goes to `B800:03D2h` (physical `0B83D2h`) and the attribute to `B803D3h`. That is what `refreshhint` ends up doing after re-reading the real character:

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
```

**Go deeper.** `vcell` derives `row*80` with `mul bl`; the largest product is `24*80 = 1920`, far inside 16 bits, so no overflow is possible in this mode.

### B9. Which segment overrides are explicit and which are implicit?

**Answer.**

* **Explicit:** `es:[di]` and `es:[di+1]` in `vcell` (because `ES = 0B800h` there).
* **Implicit `DS:`:** `mazes[si]`, `dist[si]`, `queue[si]`, `prow`, `level`, and `mov al,[si]` in `vstr`.
* **Implicit `SS:`:** every `push`, `pop`, `call`, `ret`.
* **Implicit `CS:`:** every near jump and procedure call.

**Go deeper.** The `es:` prefix costs one byte plus a cycle on an 8086, so keeping `DS` permanently on the data is both simpler and faster.

### B10. How much of DGROUP does this program use, and what happens if it overflows 64K?

**Answer.**

| Item | Bytes |
|---|---|
| game state (`level` … `wantd`) | 11 |
| HUD strings | ≈ 130 |
| `mazes` (3 × 625) | 1875 |
| `dist` | 625 |
| `queue` (625 words) | 1250 |
| `bhead`, `btail`, `lbase`, `exrow`, `excol` | 7 |
| stack (`.stack 100h`) | 256 |
| **≈ total** | **≈ 3.9 KB of the 64 KB limit** |

**Go deeper.** Overflow would mean `SI`/`DI` offsets wrapping silently. Remedies: put `mazes` in its own segment and load it into a spare segment register around accesses, keep only the current level resident and page the rest in with `INT 21h AH=3Fh`, or move to 32-bit protected mode where segments are flat.

---

## C. Interrupts and the BIOS/DOS API

### C1. What physically happens when this program executes `int 16h`?

**Answer.**

1. The CPU pushes **FLAGS**, then **CS**, then **IP** (the return state).
2. `IF` and `TF` are cleared.
3. The CPU reads the vector at `0000:(16h × 4) = 0000:0058h` — four bytes holding the CS:IP of the BIOS keyboard routine.
4. Execution transfers there, using **our** stack.
5. The BIOS returns with `iret`, which pops IP, CS and FLAGS, so flags the BIOS changed are restored to their pre-`int` values.

Our side then reads the result:

```asm
game:
    mov ah,0
    int 16h                     ; wait for a key: AL = ASCII, AH = scancode
```

**Go deeper.** Because `iret` restores FLAGS, a BIOS call can never be relied on to leave a flag behind for us — which is fine here: nothing branches on flags across an `int`.

### C2. What is the interrupt vector table, and which vectors does this program touch?

**Answer.** The IVT is 256 × 4 bytes at `0000:0000`, one CS:IP pair per interrupt number, installed by BIOS/DOS at boot. This program **reads** three vectors and never writes one:

| Vector | Vector address | Used at | Purpose |
|---|---|---|---|
| `INT 10h` | `0000:0040h` | start-up and win screen (`mov ax,0003h`) | set video mode 03h |
| `INT 16h` | `0000:0058h` | game loop and win screen (`mov ah,0`) | blocking keyboard read |
| `INT 21h` | `0000:0084h` | `quit` (`mov ah,4Ch`) | terminate to DOS |

**Go deeper.** Hooking a vector (read it, write your handler's CS:IP with `INT 21h AH=25h`) is how TSRs and custom keyboard drivers work; this game deliberately installs nothing.

### C3. BIOS services versus DOS services — which does this program use, and why?

**Answer.** BIOS services (`INT 10h` video, `INT 16h` keyboard) talk to hardware through ROM and work on any IBM-compatible machine regardless of the operating system. DOS services (`INT 21h`) belong to the operating system.

`MAZE.ASM` uses BIOS for the two things it cannot do alone — **set the display mode once** and **read a keystroke** — and DOS only to **terminate** (`AH=4Ch`). Every character, colour and digit on screen is produced without any interrupt at all, by storing bytes into `B800h`.

**Go deeper.** The labels are still `'$'`-terminated (`msglvl db 'Level $'`), a leftover convention from `INT 21h AH=09h`; here the `$` is consumed by `vstr`, not by DOS.

### C4. Give an exhaustive table of every `int` instruction in `MAZE.ASM`.

**Answer.**

| # | Approx. line | Instructions | Registers | Effect |
|---|---|---|---|---|
| 1 | 165–166 | `mov ax,0003h` / `int 10h` | `AX = 0003h` | 80×25 colour text mode; **clears the screen once** at start-up |
| 2 | 182–183 | `mov ah,0` / `int 16h` | `AH = 00h` | blocks until a key; `AL` = ASCII, `AH` = scancode |
| 3 | 253–254 | `mov ax,0003h` / `int 10h` | `AX = 0003h` | wipes the maze for the win screen |
| 4 | 265–266 | `mov ah,0` / `int 16h` | `AH = 00h` | "Press any key to exit" wait |
| 5 | 286–287 | `mov ah,4Ch` / `int 21h` | `AH = 4Ch`, `AL` = return code | return to DOS |

No other `int` exists in the file: no `INT 21h AH=09h`, no `AH=02h`, no `INT 10h AH=02h/01h/0Eh`.

**Go deeper.** The `AH=4Ch` return code is whatever `AL` last held — typically `27` after ESC, since ESC is detected as `cmp al,27`. To guarantee `ERRORLEVEL 0` you would insert `mov al,0` before the `int 21h`.

### C5. Why are there no `INT 21h` text calls left in the program?

**Answer.** Because they were the source of both the flicker and the colour limits of the old version:

* `INT 21h AH=09h`/`AH=02h` go DOS → BIOS teletype: they advance the cursor, may scroll the page, and use a single "current" attribute — painting one cell a different colour meant positioning the cursor first (`INT 10h AH=02h`) and then writing.
* Each call is an interrupt dispatch plus DOS plus BIOS — thousands of cycles per character.
* Worst of all, the old `draw` re-set the video mode (`AX=0003h`) on **every** keystroke, which blanks the display.

The new code replaces all of it with one or two `mov` stores per cell in `vcell`, so a move touches at most 4 maze cells and the 7 HUD characters; nothing else on screen is rewritten.

**Go deeper.** The only full redraw left is `drawmaze`'s 625 cells, and it runs solely at `start_level`.

### C6. Hardware IRQ1 / `INT 09h` versus this program's `INT 16h` — explain the whole chain.

**Answer.**

| Stage | What happens | Who runs it |
|---|---|---|
| Key press | the keyboard controller raises **IRQ1**, whose vector is `INT 09h` | hardware |
| `INT 09h` | the BIOS handler reads port `60h`, translates the scancode, applies shift/E0 rules and stores the ASCII+scancode word into the BIOS ring buffer at `0040:001Eh`, then sends EOI | BIOS, asynchronously, even with no program waiting |
| `INT 16h AH=00h` | our **software** call: blocks until the ring buffer is non-empty, then dequeues a word into `AX` | BIOS, called *by* us |
| `INT 16h AH=01h` | "is a key waiting?" — sets `ZF` without removing the entry | **not used** in this file |

So the game never polls hardware: it blocks inside `INT 16h`, which merely waits for a buffer that `INT 09h` fills by interrupt. True polling would be `in al,60h` loops with `cli`, which appear nowhere here.

**Go deeper.** This is also why holding a key moves the player repeatedly: typematic repeat feeds the same word back into the ring buffer and `INT 16h` returns it again.

### C7. Why does the win screen re-set video mode 03h?

**Answer.**

```asm
doexit:
    inc level
    cmp level,NLEV
    jb start_level
    ; all levels done: win screen, wait for a key, leave
    mov ax,0003h
    int 10h
```

Mode 03h is used as a **one-instruction screen clear**: it reinitialises the CRTC, blanks and refills text memory and homes the cursor, wiping all three mazes, the player stamp, hints and the HUD at once. Writing a clearing routine by hand would take a 4000-byte `rep stosw` (segment load + `DI` + `CX` + `AL`/`AH` + `rep`) — more code for the same effect, and the mode set is the only alternative the program already uses elsewhere (start-up).

**Go deeper.** The cost is that the whole screen flashes once — acceptable exactly because it happens once per program run, not per keystroke (the old bug did it every keystroke).

### C8. Why is `INT 16h AH=00h` (blocking) the right choice instead of `AH=01h` (peek)?

**Answer.** The game is turn-based: there is nothing to animate or compute while idle, so sleeping in the BIOS until a key arrives costs nothing and needs no loop:

```asm
game:
    mov ah,0
    int 16h                     ; wait for a key ...
```

A peek loop would require `INT 16h AH=01h`, a `jz` back to the top, something to do on `ZF=1`, and careful handling so a key is not consumed twice.

**Go deeper.** `AH=01h` becomes mandatory the moment you want frame-based animation, a timer or multi-key state — none of which this program has.

### C9. What is the relationship between the `'$'` in the strings and DOS?

**Answer.** None at run time. `msglvl db 'Level $'` looks like an `INT 21h AH=09h` string, but DOS never sees it: `vstr` implements its own terminator:

```asm
vs_next:
    mov al,[si]
    cmp al,'$'
    je vs_done
```

So `$` means "stop printing" for `vstr` only. Any ASCII character *other* than `$` (including `4Ch` etc.) would need escaping if it had to appear in a label.

**Go deeper.** Using `len equ $ - msglvl` style lengths instead would remove the terminator entirely, at the cost of passing a count in a register to `vstr`.

### C10. What would break if the program restored a vector or hooked the keyboard?

**Answer.** Nothing in the design requires it — the game only *reads* vectors. But if it did hook `INT 16h`/`INT 09h`, it would have to save the original 4 bytes at `0000:0058h` (or `0000:0024h`), chain to it with a far `call`/`jmp`, and restore before `INT 21h AH=4Ch`, otherwise the next program would inherit a dangling handler pointing into unloaded memory (the classic TSR crash).

**Go deeper.** Because this program exits through `AH=4Ch` and installs nothing, DOS reclaims all of its memory and state automatically.

### C11. Why is `INT 10h` needed at all if we can write video RAM directly?

**Answer.** Direct stores can only *modify* cells of the current mode; they cannot select the mode. Mode 03h gives 80×25 cells with a known `B800h` layout, and setting it also guarantees a known initial screen state (blank, attribute `07h`, cursor home) — that is what makes "the screen is cleared once" true (file header, line 7).

**Go deeper.** On a machine left in mode 13h or a weird page, `B800h` would not be the text plane at all and every `vcell` would write garbage.

### C12. If the game used `INT 21h AH=09h` again, which attributes would be impossible?

**Answer.** All of them, individually. `AH=09h` writes characters with the *current* attribute only and provides no way to set per-cell colour; you would need `INT 10h AH=09h` (write char+attribute at the cursor) or `AH=02h` to move the cursor first — two interrupts per cell instead of two `mov`s. The colour scheme (`A_WALL 1Fh` blue walls, `A_PLAY 4Eh` yellow-on-red player, `A_EXIT 2Eh`, `A_HINT 3Fh`, `A_HUD 0Bh`) would collapse into one attribute for the whole screen.

**Go deeper.** That is precisely the design change between the old version (`print` + `setcur` + `pnum`) and this one (`vstr`/`vnum2`/`vnum3` writing `B800h` directly).

---

## D. Keyboard handling

### D1. What is the difference between `AL` and `AH` after `INT 16h AH=00h`?

**Answer.** `AL` holds the ASCII translation (if the key produces one), `AH` holds the **scancode** (the physical key identity). The dispatcher uses both, deliberately:

```asm
    cmp al,27                   ; ESC as ASCII
    je quit
    cmp ah,01h                  ; ESC as scancode
    je quit
    ...
    cmp ah,48h                  ; arrow up
    je goup
    cmp al,'w'
    je goup
```

Arrows and other extended keys put `00h` (or `E0h`) in `AL` and the scancode in `AH`, so they can only be matched on `AH`; letters are matched on `AL` because both `W` and `w` must work.

**Go deeper.** `AX` is a single 16-bit value: the BIOS fills both bytes in one store, which is why one `int` returns everything.

### D2. Which arrow scancodes does the program use, and what happens to them?

**Answer.**

| Key | `AH` scancode | Label reached |
|---|---|---|
| ↑ Up | `48h` | `goup` → `dec bl` |
| ↓ Down | `50h` | `godown` → `inc bl` |
| ← Left | `4Bh` | `goleft` → `dec bh` |
| → Right | `4Dh` | `goright` → `inc bh` |

Each is tested with `cmp ah,NNh` / `je`, before which the candidate position has already been loaded into `BL`/`BH`.

**Go deeper.** Those are the same codes for the dedicated arrow keys and the numeric-keypad arrows; on the 8086-era BIOS both return `AL = 00h`.

### D3. What about the `E0h` prefix on enhanced keyboards?

**Answer.** The 101/102-key "enhanced" keyboard prefixes right-arrow/up-arrow/navigation keys with `E0h` at the *controller* level. The BIOS `INT 09h` handler consumes the prefix and still delivers `AH = 48h/50h/4Bh/4Dh` (with `AL = 00h`) to the `INT 16h` buffer, so this program needs no `E0` handling at all.

**Go deeper.** Programs that read port `60h` themselves (bypassing the BIOS) *would* have to track `E0`, and would also miss the BIOS's shift/ctrl/alt translation — one more reason this code uses `INT 16h`.

### D4. Why `or al,20h`, and why is it placed *after* the ESC tests?

**Answer.**

```asm
    cmp al,27                   ; ESC as ASCII
    je quit
    cmp ah,01h                  ; ESC as scancode
    je quit

    or al,20h                   ; force lower case: 'W' and 'w' both become 'w'
```

`or al,20h` sets bit 5, which turns `A`–`Z` (41h–5Ah) into `a`–`z` (61h–7Ah) while leaving digits `0`–`9` (30h–39h, bit 5 already set) and the already-lowercase letters untouched. It must come **after** the ESC test because `1Bh or 20h = 3Bh` (`';'`), which would make ESC unmatchable — and after `cmp al,'h'`'s placement is safe only because `'h'` is checked *after* the OR, so both `H` and `h` toggle the hint.

**Go deeper.** The OR is destructive for punctuation in general (`'|'`, `'['` …), but nothing else is matched on `AL`, so no harm is done. A `cmp al,'H' / je` + `cmp al,'h' / je` pair would work too — at the cost of two comparisons instead of one OR plus one compare.

### D5. Why is ESC tested twice?

**Answer.** Belt and braces: `cmp al,27` catches the normal ASCII case, `cmp ah,01h` catches any configuration where the ASCII byte is not `1Bh` (some layouts/modes deliver `AL = 00h` with `AH = 01h`, or the code may be entered with a stale `AL`). The first test uses `ZF` from `cmp al,27`, the second from `cmp ah,01h`; both jump to the same `quit`.

**Go deeper.** `01h` is the scancode for the leftmost key of the top row on every PC keyboard — it is ESC's *physical* identity, independent of any ASCII translation.

### D6. Show the full dispatch chain and explain why unmatched keys are free.

**Answer.**

```asm
    cmp al,'h'
    je hintkey
    cmp ah,48h                  ; arrow up
    je goup
    cmp al,'w'
    je goup
    cmp ah,50h                  ; arrow down
    je godown
    cmp al,'s'
    je godown
    cmp ah,4Bh                  ; arrow left
    je goleft
    cmp al,'a'
    je goleft
    cmp ah,4Dh                  ; arrow right
    je goright
    cmp al,'d'
    je goright
    jmp game                    ; any other key: nothing changed, no redraw
```

The chain is a linear sequence of compare/branch pairs — at most 12 comparisons for a match. An unmatched key falls into `jmp game`, which re-reads the keyboard without touching video RAM, the HUD or any variable, so the screen cannot change and no redraw is needed.

**Go deeper.** A `table db …` + `xlat`/`mov` dispatch would be smaller if the key set grew (e.g. adding `Q`, function keys); for eight keys the compare chain is faster (no memory fetch) and clearer.

### D7. Why is the candidate position loaded *before* the key is identified?

**Answer.**

```asm
game:
    mov ah,0
    int 16h                     ; wait for a key: AL = ASCII, AH = scancode
    mov bl,prow                 ; candidate position (only committed if legal)
    mov bh,pcol
```

`BL`/`BH` are the parameter registers for `getcell` and are only used by the `goup/godown/goleft/goright` paths, so loading them early costs two instructions on every key (including ESC and unknown keys) but removes four redundant loads from the movement paths. Crucially they are only *committed* to `prow`/`pcol` after validation in `move`.

**Go deeper.** Loading them early is safe even when they are never used: `BL`/`BH` are scratch for the dispatcher, and nothing between `game` and the branches depends on them.

### D8. What would change if the game needed key *state* (pressed / not pressed) rather than key *events*?

**Answer.** `INT 16h AH=00h` reports events, not state. For state you would either poll `INT 16h AH=01h` in a tight loop plus a timer (BIOS tick at `0040:006Ch`, refreshed by `INT 1Ch`), or bypass the BIOS entirely: `cli`, read port `60h`/`64h`, keep your own bitmap of down keys, `sti`. Both break the current turn-based design, where the CPU may simply sleep inside `INT 16h`.

**Go deeper.** Holding a key currently *looks* like continuous movement only thanks to typematic repeat (question C6), which is a BIOS/keyboard feature, not something the program implements.

---

## E. Video and graphics

### E1. Why is video RAM at `B800h` and not `A000h`?

**Answer.** Because the program runs in **colour text mode 03h**, whose memory window starts at segment `B800h`. `A000h` is the VGA **graphics** aperture (mode 13h and friends), where bytes are pixels, not characters. `B000h` would be the monochrome (MDA/Hercules) text window.

```asm
    mov ax,0B800h               ; colour text segment
    mov es,ax
```

**Go deeper.** The mode number decides which aperture is decoded by the video hardware — writing to `B800h` while in mode 13h would do nothing visible.

### E2. How is one screen cell laid out in memory?

**Answer.** Two bytes per cell, character first:

| Byte | Content |
|---|---|
| even offset | ASCII character (`'@'`, `'#'`, `'E'`, `' '` …) |
| odd offset | attribute byte: bits 0–3 foreground, bits 4–6 background, bit 7 blink |

```asm
    mov es:[di],al              ; even byte = character
    mov es:[di+1],ah            ; odd  byte = attribute
```

A full 80×25 screen is `80*25*2 = 4000` bytes, all inside the first 4K of the `B800h` window.

**Go deeper.** Character *and* attribute are updated together, which is why colouring a cell never needs a second pass or a separate attribute map.

### E3. Decode every attribute constant used in the file.

**Answer.** High nibble = background (bits 4–6, bit 7 = blink), low nibble = foreground (bit 3 = intensity):

| Equate | Value | Background (high) | Foreground (low) | As used for |
|---|---|---|---|---|
| `A_WALL` | `1Fh` | 1 = blue | F = bright white | `'#'` walls |
| `A_PATH` | `07h` | 0 = black | 7 = light grey | `' '` corridors, restored cells |
| `A_EXIT` | `2Eh` | 2 = green | E = bright yellow | `'E'` exit |
| `A_PLAY` | `4Eh` | 4 = red | E = bright yellow | `'@'` player |
| `A_HINT` | `3Fh` | 3 = cyan | F = bright white | highlighted hint cell (character unchanged) |
| `A_HUD` | `0Bh` | 0 = black | B = light cyan | all sidebar labels and numbers |
| `A_WIN` | `2Fh` | 2 = green | F = bright white | "You finished all 3 levels!" |
| `A_MSG` | `0Fh` | 0 = black | F = bright white | "Press any key to exit" |

All eight have bit 7 clear, so no cell blinks.

**Go deeper.** The comment at line 26 states the rule verbatim: "high nibble = background colour, low nibble = foreground". Backgrounds are limited to 8 colours unless blink is disabled (`INT 10h AX=1003h`), which the program never does — but it never selects a background above 4 anyway.

### E4. Work out the video offset for three specific cells in this program.

**Answer.** Formula: `offset = (row*80 + col)*2`.

| Cell | Calculation | Offset | Segment:Offset |
|---|---|---|---|
| player start (1,1) | `(80+1)*2` | 162 = `00A2h` | `B800:00A2h` |
| HUD `'Level $'` first char (2,30) | `(160+30)*2` | 380 = `017Ch` | `B800:017Ch` |
| HUD "Col" value (5,36) | `(400+36)*2` | 872 = `0368h` | `B800:0368h` |
| exit (23,23) | `(1840+23)*2` | 3726 = `0E8Eh` | `B800:0E8Eh` |

**Go deeper.** The maze occupies screen rows 0..24 and columns 0..24; the HUD occupies columns 30..~49 on rows 2..10 — the 5-column gap at 25..29 is untouched, which is why a full-screen repaint is never needed during play.

### E5. Why does direct video writing remove flicker completely?

**Answer.** Flicker in text programs comes from **clearing or redrawing the whole screen** between frames. This program:

* sets the mode (and thus clears) **once** at start-up and **once** on the win screen;
* draws the whole maze only at `start_level` (`drawmaze` is called there and never inside the move loop);
* on a move, rewrites at most four cells (`clearhint`, `clrplayer`, `putplayer`, `refreshhint`) plus the three HUD fields (3 distance characters + 2 row digits + 2 col digits = 7 characters).

Since every write is a plain `mov` into a cell that is about to change anyway, no cell ever goes blank-then-filled, so there is nothing to see flash.

**Go deeper.** The old version's `draw` did `mov ax,0003h / int 10h` per keystroke — a hardware mode reset blanks the screen for milliseconds; that, not the character output, was the flicker.

### E6. Compare the cost of a BIOS call with a direct `mov` for one cell.

**Answer.**

| Approach | Work per cell |
|---|---|
| `INT 10h AH=0Eh` (teletype) | push FLAGS/CS/IP, vector fetch, ROM entry, register save, attribute lookup, cursor update, scroll test, `IRET` — typically hundreds to a few thousand cycles, no colour control |
| `INT 10h AH=02h` + `AH=09h` (position + write) | **two** interrupts for one coloured cell |
| `vcell` (this file) | ≈15 instructions: 4 `push`, one segment load, `mul`, `shl`, two `mov` stores, 4 `pop`, `ret` — tens of cycles, exact colour |

For `drawmaze`'s 625 cells the difference is `625 × (thousands)` versus `625 × (tens)` — the difference between a visible repaint and none.

**Go deeper.** `vcell` pushes/pops `ES`, `DI`, `BX`, `AX` to be a good citizen; an unrolled inner loop could keep `ES = B800h` and a running `DI` and be several times faster still (that micro-optimisation is *not* in the current file).

### E7. What does `AX = 0003h / INT 10h` actually do?

**Answer.** Sets BIOS video mode 03: 80×25 colour text, 16 foreground/8 background attributes, page 0 active. Side effects the program relies on:

* text memory is cleared (every cell becomes a space with attribute `07h`);
* the cursor is homed to (0,0);
* the CRTC/sequencer/GC registers are reinitialised, so the visible layout is guaranteed;
* graphics planes/modes are reset to text operation.

The program calls it twice: line 166 (start-up) and line 254 (win screen).

**Go deeper.** Bit 7 of `AL` (`83h`) would *not* clear memory; the program deliberately uses `03h` (clear) — that is why "the screen is cleared once" (line 7) holds.

### E8. How would mode 13h differ for this game?

**Answer.**

| Aspect | mode 03h (used) | mode 13h |
|---|---|---|
| Aperture | `B800h` | `A000h` |
| Cell | 2 bytes: ASCII + attribute | 1 byte per **pixel**, 320×200 = 65536 bytes linear |
| Colour | 16 fg / 8 bg fixed palette | 256 of 262144, palette programmed through DAC ports `3C8h/3C9h` |
| Text | hardware font, free characters | none — you must blit your own glyphs (8×8/8×16) for `@`, `#`, `E`, digits |
| One "cell" | 1 byte char + 1 byte attr | ≈8×16 = 128 pixels to paint, times 625 cells |
| HUD | `vstr`/`vnum2` work as-is | every routine (`vcell`, `vstr`, `vnum2`, `vnum3`, `drawmaze`) would need rewriting for pixels |

**Go deeper.** A 30×30 maze in mode 13h at 8×16 per tile would need 240×480 pixels — taller than the 200-line screen — so the geometry, not just the writes, would have to change.

### E9. Why does `vcell` save `AX` on the stack instead of another register?

**Answer.**

```asm
    push ax                     ; remember character + attribute
    mov ax,0B800h               ; colour text segment
    mov es,ax
    mov al,dh                   ; row
    xor ah,ah
    ...
    pop ax                      ; character + attribute back
    mov es:[di],al
    mov es:[di+1],ah
```

`AX` is the only 16-bit register free at that moment: `DH`/`DL` hold the position being decoded, `BX` is used for the `mul bl`/`add ax,bx` arithmetic, `DI` receives the computed offset, and `ES` must hold `B800h`. Pushing `AX` costs 2 bytes and one memory write and lets the same registers do double duty.

**Go deeper.** An alternative is to compute the offset in `SI` and keep the character in `AL`, but `SI` is a caller-owned index in several paths — the stack save is simpler and keeps the documented "preserves DH, DL, BX, DI, ES" contract.

### E10. Why keep `ES` correct across `vcell`?

**Answer.** Because `ES` has two live meanings in this program: `DS` (for `rep stosb` in `bfsolve`) and `B800h` (for video). `vcell` is called from deep inside `vstr`, `vnum2`, `vnum3`, `drawmaze`, `paintcell`, `refreshhint` — including right after `bfsolve` has set `ES = DS`. If `vcell` did not restore `ES`, `drawhud` would later run with `ES = B800h`, and any future `rep stosb`/`mov es,…` assumption would break.

**Go deeper.** The push/pop pair is 4 instructions; the alternative — reloading `ES` at the top of every consumer — is more code and more opportunities to forget one.

### E11. Why does the HUD live at column 30?

**Answer.** `drawmaze` writes screen columns `0..MW-1` = `0..24` (loop `inc dl / cmp dl,MW / jb dm_col`), so columns 25+ are free; the sidebar starts at 30 to leave a visible gap, and all its labels are written at `dl = 30` with the numbers at `dl = 36`:

```asm
drawhud proc
    mov dh,2
    mov dl,30
    mov si,offset msglvl
    mov ah,A_HUD
    call vstr
    mov al,level
    add al,'1'                  ; 0 -> '1', 2 -> '3'
    ...
```

Because every label is exactly 6 characters (`'Level $'`, `'Dist  $'`, `'Row   $'`, `'Col   $'`), the values always land on column 36, which `updhud` rewrites blindly without re-measuring.

**Go deeper.** This fixed geometry is what lets `updhud` hard-code `mov dh,3 / mov dl,36` etc.; widening the maze to 30 columns would force a rethink (question I7).

### E12. Where is the cursor while the game runs, and does it matter?

**Answer.** The program never positions or hides the cursor (`INT 10h AH=02h` and `AH=01h` are unused), so it stays where the last mode set left it — row 0, column 0 — and is simply overwritten by whatever `drawmaze` stores into that cell. Nothing reads the cursor position, and no BIOS text service is used, so its position has no effect on output.

**Go deeper.** If the blinking caret annoyed you, the fix is one `mov ax,0100h / int 10h` (cursor type 0 = invisible) at start-up — still only at start-up, never in the move loop.

---

## F. Program logic walkthrough

### F1. What is the "candidate position" pattern and why does it use `BL`/`BH`?

**Answer.** `BL` holds the *proposed* row, `BH` the *proposed* column; `prow`/`pcol` hold the committed position. The key read loads the candidate:

```asm
    mov bl,prow                 ; candidate position (only committed if legal)
    mov bh,pcol
```

and each direction adjusts a copy, never the real state:

```asm
goup:
    dec bl
    jmp move
godown:
    inc bl
    jmp move
goleft:
    dec bh
    jmp move
goright:
    inc bh
```

`BL`/`BH` are chosen because they are the documented input registers of `getcell`, `paintcell`, `trynb` and `findnb` — the candidate flows straight into the collision test with no extra moves.

**Go deeper.** Using `BX` also keeps `SI`/`DI` free for indices and `DX` free for screen coordinates, which is the register contract the rest of the file follows.

### F2. Where exactly is the position committed, and why there?

**Answer.** Only in `move`, *after* the target cell has been validated:

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
```

Committing earlier would leave `prow`/`pcol` disagreeing with the screen on a rejected move (the `@` would have to be redrawn on the old cell), and would corrupt the HUD.

**Go deeper.** The pattern is "compute → validate → side effects → commit": note that `clearhint`/`clrplayer` also happen only after validation, so a bump into a wall repaints *nothing at all*.

### F3. What is the collision rule, exactly?

**Answer.** Two tests, in order, on the byte read by `getcell`:

1. `cmp al,'#' / je game` — a wall rejects the move and jumps back to the input loop with **zero** side effects (no repaint, no HUD update).
2. `cmp al,'E' / je doexit` — the exit completes the level.
3. Everything else is walkable: in practice `' '` corridors, but *any* byte other than `'#'`/`'E'` would be accepted — the maze data is trusted to contain only `#`, space and `E`.

**Go deeper.** There is no bounds check in `getcell` itself (see K2); the rule "the candidate is never out of range" is enforced by the maze design — every level is fully ringed by walls, so `BL`/`BH` can never reach 0/25, let alone wrap to 255.

### F4. Walk through a level transition.

**Answer.**

```asm
doexit:
    inc level
    cmp level,NLEV
    jb start_level
    ; all levels done: win screen, wait for a key, leave
    mov ax,0003h
    int 10h
```

* Stepping on `'E'` enters `doexit` *without* having moved the player (the commit happens only for non-exit cells), so the `@` stays where it is — irrelevant, because `start_level` resets everything.
* `level` is incremented (0→1→2), compared against `NLEV = 3` with an **unsigned** `jb`, and if still below it, control returns to `start_level`, which resets `prow`/`pcol` to 1,1, clears `hinton`/`hintvis`/`hintd`, re-runs `bfsolve`, redraws the maze and HUD and stamps `@` again.
* When `level` reaches 3, the mode is reset (screen cleared), the two win strings are drawn with `vstr`, a final `INT 16h` waits, and `quit` terminates via `INT 21h AH=4Ch`.

**Go deeper.** `jb` (unsigned) is essential: see K5 for what a signed compare would do once `level` could exceed 127.

### F5. How does the hint toggle work?

**Answer.**

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

* `xor al,1` flips bit 0 — a 0/1 toggle without `cmp`/`je` chains.
* Turning **on**: `refreshhint` clears any stale highlight, computes the next step and lights it; `updhud` then prints `hintd` as three digits.
* Turning **off**: `clearhint` repaints the highlighted cell with its natural colour, `hintd` is zeroed, and `updhud` prints `'---'` because `hinton = 0`.

**Go deeper.** `hintvis` (not `hinton`) is what `clearhint` tests, so pressing `H` twice quickly, or moving with the hint off, never repaints a cell that was not highlighted.

### F6. What is the HUD update strategy?

**Answer.** Split into "once per level" and "per keystroke":

| Routine | When | What it writes |
|---|---|---|
| `drawhud` | at `start_level` only | all labels (`Level`, `Dist`, `Row`, `Col`, `Arrows/WASD = Move`, `H = Hint`, `E = Exit`, `ESC = Quit`) at column 30, plus initial values at column 36 |
| `updhud` | after every legal move and on every hint toggle | **only** the three changing fields: distance (3 characters), row (2 digits), col (2 digits) — all at column 36 |

```asm
updhud proc
    mov dh,3
    mov dl,36
    mov ah,A_HUD
    cmp hinton,0
    je uh_dash
    cmp hintd,0FFh
    je uh_dash
    mov al,hintd
    call vnum3                  ; e.g. '140'
    jmp uh_row
uh_dash:
    mov si,offset msgdash       ; '---'
    call vstr
uh_row:
    mov dh,4
    mov dl,36
    mov al,prow
    mov ah,A_HUD
    call vnum2
    ...
```

The Level field never changes during a level, and the labels never change at all, so they are not rewritten.

**Go deeper.** `updhud` writes `'---'` whenever the hint is off **or** `hintd = 0FFh` (unreachable/unknown) — two independent reasons to hide the number, both handled by the same branch.

### F7. Why does an unknown key cause no redraw — and no HUD update?

**Answer.** The dispatcher ends in `jmp game`, which skips `move`, `doexit` and `hintkey` entirely:

```asm
    jmp game                    ; any other key: nothing changed, no redraw
```

No variable is modified (not even `prow`/`pcol`, since only the candidate `BL`/`BH` was touched) and no `vcell` runs, so the screen is already correct. The comment states the invariant explicitly: "nothing changed, no redraw".

**Go deeper.** The same applies to a rejected wall bump — `je game` skips the HUD too, which is right because row/col/dist cannot have changed.

### F8. Why is `clearhint` called *before* `clrplayer` in `move`?

**Answer.** Order matters only when the two repaint the **same** cell — it is harmless either way because both use `paintcell`, which re-reads the real character from `mazes` and applies its natural colour. The chosen order is: erase the highlight, erase the player, commit, stamp the player, recompute the highlight at the new position.

```asm
    call clearhint              ; repaint the old highlighted cell
    call clrplayer              ; repaint the cell the player leaves
    mov prow,bl                 ; commit the new position
    mov pcol,bh
    call putplayer              ; '@' onto the new cell
    cmp hinton,0
    je mv_hud
    call refreshhint            ; highlight the next cell of the shortest path
```

`refreshhint` itself begins with another `clearhint`, which is a no-op at that point because `clearhint` already set `hintvis = 0`.

**Go deeper.** If `clearhint` ran *after* `putplayer` and the hint cell equalled the new player cell, the `@` would be painted over — the current order plus `hintvis` bookkeeping avoids that class of bug entirely.

### F9. Is the maze data ever modified at run time?

**Answer.** No. `mazes` is read-only in practice: `getcell`, `drawmaze`, `paintcell`, `bs_find` and `trynb` all only read `mazes[si]`/`mazes[di]`. The player glyph `'@'` exists **only** in video RAM (written by `putplayer`), and it is erased by `clrplayer` → `paintcell`, which re-reads the original character:

```asm
paintcell proc
    call getcell                ; AL = character
    mov ah,A_PATH
    cmp al,'#'
    jne pc_notwall
    mov ah,A_WALL
pc_notwall:
    cmp al,'E'
    jne pc_notexit
    mov ah,A_EXIT
pc_notexit:
    mov dh,bl
    mov dl,bh
    call vcell
    ret
```

**Go deeper.** Because the data never changes, `bfsolve`'s `dist[]` (the only large mutable table) stays valid for the whole level — one solve serves every step (see G2).

### F10. What exactly is reset at `start_level`, and why each item?

**Answer.**

```asm
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

| Reset | Reason |
|---|---|
| `prow`/`pcol` = 1,1 | every level starts at the top-left corridor cell |
| `hinton` = 0 | the hint is off at the start of a level (the HUD shows `'---'`) |
| `hintvis` = 0 | no cell is highlighted, so `clearhint` will not repaint a stale cell |
| `hintd` = 0 | the distance field starts unknown/off |
| `bfsolve` | the new level needs a fresh `dist[]`, `lbase`, `exrow`/`excol` |
| `drawmaze` | the previous level's pixels must be replaced — the **only** full redraw |
| `drawhud` | labels plus initial Row/Col/Level values |
| `putplayer` | stamp `'@'` at (1,1) |

Note that `hrow`/`hcol`/`wantd`/`hintvis` are *not* all reloaded, because `clearhint` gates on `hintvis`, which is zeroed here.

**Go deeper.** `lbase` is refreshed inside `bfsolve` (level × 625), which is what makes `trynb`'s `mazes[di]` lookups hit the right level.

### F11. Trace one complete successful move, instruction by instruction, with the hint ON.

**Answer.** Player at (1,1) pressing `→` on level 0:

1. `mov ah,0 / int 16h` → `AL = 'd'`, `AH = 4Dh`.
2. `mov bl,prow` / `mov bh,pcol` → `BL = 1`, `BH = 1`.
3. ESC tests fail; `or al,20h` → `'d'`; `'h'` no; `cmp ah,48h` no; `cmp al,'w'` no; `cmp ah,50h` no; `cmp al,'s'` no; `cmp ah,4Bh` no; `cmp al,'a'` no; `cmp ah,4Dh` **yes** → `goright` → `inc bh` (BH = 2) → falls into `move`.
4. `call getcell` → `SI = 0*625 + 1*25 + 2 = 27` → `mazes[27]` = space (not `'#'`, not `'E'`).
5. `call clearhint` → repaints the old hint cell at (1,1)? No — the old hint was the cell *ahead*; `hintvis = 1` so it is repainted via `paintcell` (restoring `' '` with `A_PATH`), `hintvis = 0`.
6. `call clrplayer` → `paintcell(1,1)` restores `' '` with `A_PATH`.
7. `mov prow,bl / mov pcol,bh` → commit (1,2).
8. `call putplayer` → writes `'@'` + `A_PLAY` at (1,2).
9. `hinton ≠ 0` → `call refreshhint` → new `hintd = dist[1,2]`, `wantd = hintd-1`, `findnb` finds the next cell, it is repainted with its real character and `A_HINT`, `hintvis = 1`.
10. `call updhud` → distance (3 characters at row 3 col 36), Row `01` (row 4), Col `02` (row 5).
11. `jmp game`.

Total screen writes: at most 4 maze cells + 7 HUD characters.

**Go deeper.** If the move had been rejected (wall), the trace would end at step 4 with `je game` — zero writes.

### F12. What happens if `getcell` is asked for a cell that is off the level?

**Answer.** It does not check. `SI = level*625 + row*25 + col` is computed blindly and `mov al,mazes[si]` reads whatever byte sits there — the neighbouring level's data (for a too-large `col`), or `dist[]`/`queue` beyond the end of `mazes` (for a large `row`). No fault occurs (real mode has no protection); you just read a wrong byte and may accept a bogus move.

In practice this cannot happen: every level is completely ringed by walls, so the candidate position is always clamped by the wall test before it can leave `0..24`. `trynb` and `findnb`, which *can* be handed `row = 255` after `dec bl`, **do** check (`cmp bl,MH / jae`) — see K2 for the version of this bug that would actually fire.

**Go deeper.** Adding the same two compares to `getcell` would cost 4 bytes and make the routine safe against a borderless maze.

---

## G. BFS and algorithms

### G1. Why breadth-first search rather than DFS or Dijkstra?

**Answer.** Every corridor step costs the same (1 cell), so the shortest path is the one with the fewest steps — exactly what BFS guarantees, because it explores in non-decreasing distance from the source. DFS may find *a* path but no guarantee it is shortest; Dijkstra with uniform weights is correct but needs a priority queue (log factor, and far more code) for no benefit.

| Algorithm | Correct here? | Cost | Code size |
|---|---|---|---|
| BFS | yes | `O(V+E)` | queue + visited array (present in the file) |
| DFS | no (longest/short path only by luck) | `O(V+E)` | stack or recursion |
| Dijkstra (binary heap) | yes | `O((V+E) log V)` | heap, decrease-key |
| A* | yes | better in practice | heap + heuristic |
| Bellman-Ford | yes | `O(V·E)` | absurd here |

**Go deeper.** On an unweighted grid BFS *is* Dijkstra with a FIFO queue — the classic "0-1 BFS" insight generalises this (see G13).

### G2. Why search **from the exit** instead of from the player?

**Answer.** `dist[c]` is defined as *the shortest distance from cell `c` to the exit*, seeded at `'E'`:

```asm
    mov dist[si],0              ; distance(exit, exit) = 0
```

Because the grid is undirected and unweighted, `dist` is symmetric: `dist[player]` is exactly the remaining optimal path length for *any* player position on the level. One solve per level therefore serves every step the player takes — a hint costs `O(1)` at run time (`findnb` looks at 4 cells).

Had the search started at the player, `dist` would be invalidated by every move and `bfsolve` would have to re-run 625-cell work per keystroke.

**Go deeper.** Searching from the exit also means the exit location only has to be found once per level (`bs_find` scans `mazes[lbase .. lbase+625)`), and `exrow`/`excol` are computed by a single `div cx` on the found index.

### G3. Why is BFS run once per level rather than once per program?

**Answer.** `dist[]` is indexed *inside* the level (`dist[level*MSZ + …]` is not used — the index passed to `dist[si]` is `row*MW+col`, level-relative), and `lbase` points `mazes` at the current level. Since `dist` describes one maze's topology, changing levels invalidates all of it, so `start_level` calls `bfsolve` right before `drawmaze`:

```asm
    call bfsolve                ; dist[] = shortest distance of every cell
    call drawmaze               ; full colourised maze (only on level change)
```

Within a level the maze data never changes (question F9), so the table stays valid for the entire level.

**Go deeper.** If levels were stored as an array of `dist` tables (`dist NLEV*MSZ`), all three could be solved up front and `start_level` would skip `bfsolve` — trading 1875 bytes of DGROUP for a slightly snappier level change.

### G4. Describe the queue data structure.

**Answer.**

```asm
queue   dw MSZ dup(0)           ; BFS queue, packed: low byte = row, high = col
bhead   dw 0                    ; queue read index
btail   dw 0                    ; queue write index
```

* A **word** per entry: `AL = row`, `AH = col` (little-endian: `row` at the even byte).
* `bhead`/`btail` are **word** indices into `queue` (byte offset = `index × 2`, hence `shl si,1` / `shl ax,1`).
* FIFO discipline: enqueue at `btail`, dequeue at `bhead`, empty when `bhead >= btail` (unsigned `jae bs_done`).
* Capacity `MSZ = 625` words = 1250 bytes; `trynb` also checks `cmp ax,MSZ / jae tn_done` as a belt-and-braces "queue full" guard (the comment notes it cannot happen).

**Go deeper.** A circular buffer would waste nothing when `btail` wraps, but with ≤625 total enqueues a monotone queue is simpler and never wraps — `bhead`/`btail` are reset to 0 at the top of every `bfsolve`.

### G5. Why pack row and column into one word instead of two arrays?

**Answer.** One `queue[si]` load yields both coordinates in `AX`, then `mov bl,al / mov bh,ah` splits them — a single memory access instead of two, and only one index register to maintain. It also keeps the queue in a single contiguous block (one `dw` declaration) rather than two parallel byte arrays that must stay in lockstep.

**Go deeper.** The alternative of storing the **cell index** (0..624) as a word would be even cheaper for the index computation (`SI` directly), at the cost of dividing by `MW`/`mod MW` to recover row/column — the current code instead multiplies row by `MW` and adds column in three instructions.

### G6. What is the sentinel `0FFh`, and why that value?

**Answer.** `dist db MSZ dup(0FFh)` initialises every cell to `FFh`, meaning *unvisited/unknown*. Real distances are `0 .. 0FDh` (see G8), so `FFh` can never collide with a legitimate distance:

```asm
    mov al,dist[si]
    cmp al,0FFh
    jne tn_done                 ; already visited
    mov dist[si],dl
```

and the display code treats it as "no number available":

```asm
    cmp hintd,0FFh
    je uh_dash                  ; '---'
```

**Go deeper.** `00h` would be a terrible sentinel because it is the distance of the exit itself; `80h` would work but wastes the useful upper half of the byte for no reason.

### G7. Why is `dist[]` a byte array rather than a word array?

**Answer.** The longest distance in this build is **140** (level 2, from (1,1) to (23,23)); levels 0 and 1 give 100 and 68. All fit in 0..254 with room to spare, so a byte halves the memory (625 bytes vs 1250) and makes every load/store single-byte (`mov dl,dist[si]`).

The real limit is the sentinel: with bytes you get `0..254` usable plus `FFh`; with words you would use `0..0FFFEh` plus `0FFFFh`.

**Go deeper.** A maze whose shortest path exceeds 253 is possible (a 30×30 spiral can approach 900) — then the guard in G8 truncates the search and distant cells keep `FFh`. The fix is a `dw MSZ dup(0FFFFh)` table with 16-bit arithmetic throughout (G8, I7).

### G8. Explain the overflow guard `cmp dl,0FEh / jae`.

**Answer.**

```asm
    mov dl,dist[si]
    inc dl                      ; distance the neighbours will get
    cmp dl,0FEh
    jae bs_loop                 ; byte would overflow: stop expanding
```

`DL` is the distance the *next* ring of cells would receive (`dist[current] + 1`). If it ever reached `0FFh` the sentinel would be written as a real distance (indistinguishable from "unvisited"), and `0FEh` would be a value that `refreshhint`/`trynb` could misread. The unsigned `jae` therefore stops expanding as soon as `DL ≥ 254`, so stored distances never exceed `0FDh` (253) and `0FEh`/`0FFh` remain reserved.

**Go deeper.** Without the guard, a path of length 255 would store `FFh`; `trynb` would then see the cell as unvisited, re-enqueue it and churn — and `refreshhint` would show `'---'` for a perfectly reachable cell. The guard converts a potential corruption into a graceful "no hint that far out".

### G9. Why does the hint step use `dist[player] - 1`?

**Answer.**

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

In BFS from the exit, `dist` partitions the grid into rings. If the player stands in ring `d`, at least one 4-neighbour must lie in ring `d - 1` — namely the predecessor on some shortest path (the BFS tree edge that first reached the player's cell). Every such neighbour is a *legal* next step of an optimal route, so picking the first one found (`findnb`, order up/down/left/right) always yields a shortest path step.

**Go deeper.** It is a greedy walk down the distance gradient; repeated application reaches the exit in exactly `dist[start]` steps — which is why the HUD "Dist" countdown decreases by 1 on every optimal move.

### G10. What are the BFS complexity figures for this program?

**Answer.** Let `V ≤ 625` cells and `E ≤ 4V` directed edges (each walkable cell has up to 4 neighbours; walls are never enqueued).

| Phase | Complexity | Numbers here |
|---|---|---|
| `bfsolve` (once per level) | `O(V + E)` | ≤ 625 dequeues + ≤ ~1200 successful/attempted relaxations → tens of thousands of instructions, well under a tenth of a second even at 4.77 MHz |
| `drawmaze` (once per level) | `O(V)` | 625 `vcell` calls |
| hint refresh (per move) | `O(1)` | ≤ 4 `findnb` probes |
| repaint (per move) | `O(1)` | ≤ 4 cells + 7 HUD characters |

**Go deeper.** Because each cell is enqueued at most once (the `dist = FFh` test gates enqueueing), the queue can never exceed the number of *walkable* cells — 299/292/287 for the three levels, comfortably inside its 625-word capacity.

### G11. How do you know every walkable cell receives a distance?

**Answer.** BFS starts at `'E'` and expands only through non-`'#'` cells, so it reaches exactly the set of cells connected to the exit by corridors. The verified figures for this build are:

| Level | Walkable cells | All reachable? | Distance from start (1,1) to exit |
|---|---|---|---|
| 0 (`level=0`) | 299 | yes | **100** |
| 1 (`level=1`) | 292 | yes | **68** |
| 2 (`level=2`) | 287 | yes | **140** |

All three exits sit at (23,23), and no corridor is sealed off — otherwise its cells would still read `0FFh` and the hint would print `'---'` even with `hinton` on.

**Go deeper.** A cheap runtime assertion: after `bfsolve`, probe `dist` at (1,1); if it is `FFh` the level is unsolvable and `start_level` could refuse to start (the current file does not do this).

### G12. Walk through `trynb` line by line.

**Answer.**

```asm
trynb proc
    cmp bl,MH
    jae tn_done                 ; rows are 0 .. MH-1 (BL may be 255 after DEC)
    cmp bh,MW
    jae tn_done                 ; columns are 0 .. MW-1

    mov al,bl
    xor ah,ah
    mov cl,MW
    mul cl                      ; AX = row * 25
    mov si,ax
    xor ah,ah
    mov al,bh
    add si,ax                   ; SI = index inside the level

    mov ax,si
    add ax,lbase
    mov di,ax
    cmp mazes[di],'#'           ; never walk through a wall
    je tn_done

    mov al,dist[si]
    cmp al,0FFh
    jne tn_done                 ; already visited
    mov dist[si],dl

    mov ax,btail
    cmp ax,MSZ
    jae tn_done                 ; queue full (cannot happen: 625 cells max)
    shl ax,1
    mov di,ax
    mov al,bl
    mov ah,bh
    mov queue[di],ax
    inc btail
tn_done:
    ret
trynb endp
```

Four gates in order: **bounds** (unsigned, because `dec bl` from 0 gives 255) → **not a wall** (using `lbase + index` so the right level is tested) → **not yet visited** (`dist = FFh`) → **queue has room**. Only then is `dist` set to `DL` and the packed coordinate enqueued.

**Go deeper.** `DL` is preserved across the routine (documented in its header), so the caller can reuse it for the remaining three neighbours — that is why `bfsolve` pushes/pops `BX` around each call but never saves `DX`.

### G13. How would you support weighted cells (e.g. mud costs 3, ice costs 1)?

**Answer.** BFS assumes unit weights. Options:

1. **0-1 BFS** (weights only 0 or 1): a deque — push front for weight 0, back for weight 1 — keeps `O(V+E)`.
2. **Dijkstra with a binary heap**: `O((V+E) log V)`; `dist` must become a word (or at least saturate), and relaxation changes from "first visit wins" to `if new < dist[v] then update and re-push`.
3. **Bucket/Dial's algorithm**: integer weights small → array of buckets indexed by distance, still `O(V + E + C)` where `C` is the max cost.

Concretely here: replace the `cmp al,0FFh / jne tn_done` gate with a distance comparison, push the neighbour with `dist[nb] = dist[cur] + weight(cur,nb)`, and allow re-relaxation (at the cost of a larger queue or a visited-set strategy).

**Go deeper.** The hint rule (G9) still works — step to any neighbour with `dist = dist[player] - weight` — but with weights you must check *all four* neighbours rather than taking the first `d-1` match, since different first steps may have different weights.

### G14. How would you highlight the **whole** path instead of one step?

**Answer.** Keep a predecessor table filled during BFS:

```asm
prev    dw MSZ dup(0FFFFh)      ; index of the BFS parent of each cell
```

In `trynb`, when `dist[si]` is first assigned, also store `prev[si] = current index`. Then, from the player's index, walk `prev` until reaching the exit index, painting each cell (or storing them in a list for `refreshhint` to light up).

A cheaper variant **without** extra memory: repeatedly step to the neighbour with `dist = d-1` (the current `findnb` rule) until `dist = 0` — `O(path length)` per rendering, `140` steps worst case here.

**Go deeper.** `prev` costs 1250 bytes; given DGROUP is only ~3.9K used, that fits comfortably — the current file simply does not need it because it shows one step at a time.

<!--CONT-->




