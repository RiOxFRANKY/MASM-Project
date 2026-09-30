# Theory 05 — Interview Questions (with Answers)

Questions you can realistically be asked about this project, grouped by topic.
Format: **Q** → short answer → "how to go deeper".

---

## A. MASM & assembler basics

**Q1. What is MASM and how does it differ from a compiler?**
MASM is Microsoft's x86 *assembler*: it converts mnemonics to machine code 1:1.
A compiler translates a high-level language and may optimise. Pipeline:
`ASM --masm--> OBJ --link--> EXE`.

**Q2. What is the difference between a directive and an instruction?**
A directive (`.model`, `db`, `proc`, `end`) is addressed to the **assembler** and
produces no runtime code by itself. An instruction (`mov`, `int`) is executed by the CPU.

**Q3. What does `.model small` mean?**
One 64 KB code segment + one 64 KB data segment; near (16-bit) pointers are enough.

**Q4. Why `.stack 100h`?**
Reserves 256 bytes for SS:SP — used by `push/pop` and `call/ret`.

**Q5. What does `end main` do?**
Marks end of source **and** sets the EXE entry point to `main`.

**Q6. What is `@data`?**
A linker-provided symbol equal to the segment address of `.data`. Used as
`mov ax,@data / mov ds,ax`.

**Q7. Why can't you write `mov ds,@data` directly?**
Segment registers don't accept immediate operands; you must go through a general
register (or `pop ds` from the stack).

**Q8. What's the difference between MASM, TASM, NASM and JWasm?**
MASM/TASM/JWasm share the MASM-ish syntax (`.model`, `proc`, `mazes[si]`);
NASM uses a different syntax (`section .data`, `mov al, [si + mazes]`). JWasm is a
free MASM-compatible assembler (`jwasm -mz MAZE.ASM`).

---

## B. Memory model & segments

**Q9. What are the segments of a DOS EXE?**
CS (code), DS (data), SS (stack), ES (extra — string ops / DOS buffers).

**Q10. What is the physical address of `segment:offset`?**
`segment × 16 + offset` (a 20-bit address → 1 MB in real mode).

**Q11. What happens if you forget `mov ds,ax`?**
Every variable access uses a garbage DS → crash or memory corruption. It's the #1
"works on my machine" bug in DOS programs.

**Q12. Which registers must a DOS program preserve across an `INT 21h` call?**
`SI, DI, BP, DS, ES` are preserved; `AX, BX, CX, DX` may be destroyed. Our code
therefore reloads what it needs after each DOS call.

---

## C. Interrupts

**Q13. What is the difference between BIOS and DOS interrupts?**
BIOS (`INT 10h`, `16h`, `1Ah`) talks to the hardware and is present even without DOS;
DOS (`INT 21h`) is the operating system's API (files, strings, memory).

**Q14. Which interrupt reads a key and what does it return?**
`INT 16h / AH=00h` blocks until a key is pressed → `AL`=ASCII, `AH`=scancode.

**Q15. What is the difference between ASCII and a scancode?**
ASCII is the *character* the key would type; a scancode identifies the *physical key*
regardless of layout. Arrow keys have no ASCII (AL=0), so we match them on `AH`.

**Q16. List the scancodes used in this program.**
↑ `48h`, ↓ `50h`, ← `4Bh`, → `4Dh`, ESC `01h`.

**Q17. Why does the code do `cmp al,27` *and* `cmp ah,01h`?**
To catch ESC either way — some BIOSes/keys report the ASCII, others (or the E0
prefix path) are easier to catch via the scancode.

**Q18. What does "handling the keyboard interrupt" mean here?**
BIOS's hardware ISR (INT 09h) already converts the scancode into a buffer entry; our
program *consumes* that buffer with INT 16h. True "handling"/hooking would mean
replacing the INT 09h vector with our own ISR — an add-on.

**Q19. What BIOS interrupt sets the cursor position and what are its arguments?**
`INT 10h / AH=02h`, `BH`=page, `DH`=row, `DL`=column.

**Q20. What does `INT 10h / AX=0003h` do?**
Sets 80×25 colour text mode — which also **clears** the screen (why our `draw`
flickers).

**Q21. How would you print a `$`-terminated string?**
`LEA DX,msg / MOV AH,9 / INT 21h`. The `$` is not printed, it just ends the string.

**Q22. How does the program terminate?**
`MOV AH,4Ch / INT 21h` — returns control (and an exit code in AL) to DOS.

---

## D. Program logic

**Q23. Where is the player's position stored?**
`prow`, `pcol` bytes in `.data`. The candidate move lives in `BL`/`BH` until it's
validated, then copied into `prow`/`pcol`.

**Q24. How is collision detection done?**
Look up the target cell with `getcell`; if it equals `'#'` the candidate is discarded
and the real position is untouched.

**Q25. How does the program know the player reached the exit?**
The target cell equals `'E'` → `inc level`, `cmp level,3 / jb newlevel`, otherwise
show the win message and exit.

**Q26. Explain the address formula in `getcell`.**
`SI = level×625 + row×25 + col`, then `mov al,mazes[si]`. This is row-major indexing
for `char mazes[3][25][25]`.

**Q27. Why 625?**
25 × 25 — the byte size of one maze; multiplying by the level number selects which of
the three concatenated mazes.

**Q28. Why are there three `db` blocks of 25 strings?**
They are stored back-to-back as one 1875-byte array; there's no per-level pointer —
the level number *is* the multiplier.

**Q29. Why does `draw` print the maze before `@`?**
The maze comes from static data (it never contains `@`); the player glyph is stamped
on top at `prow/pcol` afterwards.

**Q30. How does the HUD know where to print?**
Explicit cursor placement: `DH`=row, `DL`=30 (right of the 25-wide maze), then
`INT 21h/09h` or `/02h`.

**Q31. How are the digits of `prow` printed?**
`pnum`: `AAM` splits the value into tens (AH) and units (AL), `add ax,3030h` makes
them ASCII, then two `INT 21h/02h` calls.

**Q32. What is `AAM` and what is its limitation?**
ASCII Adjust after Multiply: implicit ÷10 → AH=quotient, AL=remainder. Only valid for
0–99 (for ≥100 you'd need a divide loop).

**Q33. Why does the code do `or al,20h`?**
To lowercase the ASCII so `'W'` and `'w'` both match `'w'` — upper/lower letters
differ only in bit 5.

**Q34. Why does `je game` appear so often?**
Every path (ignored key, rejected wall move, normal step) ends by redrawing and
waiting for the next key — one central loop.

**Q35. What happens if the player presses an unused key?**
`jmp game` — nothing changes; the screen is redrawn and INT 16h blocks again.

---

## E. Registers, instructions, stack

**Q36. Why is `BL` used for the row and `BH` for the column?**
They're the two halves of `BX`, so the pair holds a (row,col) candidate as one
register; movement is a single `inc`/`dec` on the right half.

**Q37. What does `mul cx` do and which registers does it destroy?**
`DX:AX = AX × CX`. Destroys `AX` (and `DX`). That's why `level` is zero-extended into
`AX` first (`mov ah,0`).

**Q38. Difference between `mul` and `imul`?**
`mul` is unsigned, `imul` signed — and `imul` can also be 2- or 3-operand.

**Q39. What does `LOOP` do?**
`dec CX` then jump to the label while `CX ≠ 0`. Used for the 25 columns per row.

**Q40. What is the difference between `JE` and `JZ`?**
None — same instruction, two mnemonics for ZF=1.

**Q41. `cmp level,3 / jb newlevel` — why `jb` and not `jl`?**
`jb` is *unsigned* below (CF). `level` is 0..3 and never negative, so unsigned is
correct; `jl` would test the sign flag.

**Q42. What is the stack used for in this program?**
`push ax`/`pop ax` in `pnum` to preserve the converted digits across a DOS call, plus
the return addresses of `call`/`ret`.

**Q43. What is the maximum call depth?**
2 (`main → draw → setcur`) — far below the 256-byte stack.

**Q44. Which addressing mode is `mov al,mazes[si]`?**
Register-indirect / based-index (direct-indexed): effective address = `DS:SI + disp`.

**Q45. What does `LEA DX,msglvl` do vs `MOV DX,msglvl`?**
Both usually assemble the same here (offset of a label), but `LEA` computes an address
from registers and is the idiomatic/robust way to load a label's offset.

---

## F. Design, debugging, testing

**Q46. The game flickers. Why and how do you fix it?**
`draw` re-sets video mode `03h` every frame, clearing the screen. Fix: partial redraw
(only old/new cell + changed HUD digits) or double-buffer into `B800h`.

**Q47. Find a bug in this program.**
Best answer: `pnum` — after `pop ax`, `AH` is no longer `2`, so the second `int 21h`
calls an undefined DOS function. Also: no bounds check in `getcell`; the win message
immediately falls into `quit`; `level` never resets.

**Q48. How would you add a 4th level?**
Paste 25 more `db` rows (or make the count `NLEV equ 4`), change `cmp level,3` to
`cmp level,NLEV` (ideally `NLEV`). This is why `EQU` constants matter.

**Q49. How would you make the maze 30×30?**
Change `WIDTH/HEIGHT`, the `625`/`25` literals → `EQU`-derived values, add 5 chars to
each row and 5 rows per level, and re-check the `pnum` range (0–99 is still OK).

**Q50. How would you test a DOS assembly program?**
DOSBox / DOSBox-X with a debugger (`debug.exe`, `TD`, `TDDEBUG`, or DOSBox's built-in
`debugger`), breakpoint at `game`, inspect `prow/pcol/SI` after each move; test wall
edges, exit cell, ESC, and unknown keys.

**Q51. How would you prove the maze is solvable?**
Run a BFS from (1,1) over the static data (a small C/Python script on the same layout)
— or add the in-program BFS hint (Theory 06).

**Q52. Is this program re-entrant / interrupt-safe?**
No — it's a single-threaded blocking loop with no ISR of its own. If you hook INT 09h
you must keep the ISR tiny, only touch a ring buffer, and use `STI`/`CLI` carefully.

**Q53. What is the time complexity of one move?**
O(1) for collision (`getcell`), but O(625) for the redraw — that's the performance
argument for partial redraws.

**Q54. What would you do differently in a rewrite?**
Data-driven levels (`EQU` + pointer table), partial redraw, state machine
(title/playing/paused/win), non-blocking input, colour via `B800h`, and a documented
calling convention per procedure.

---

## G. Rapid-fire true/false

| # | Statement | Answer |
|---|---|---|
| 1 | `mov ds,5` is a legal instruction | **False** — segment regs need a register |
| 2 | Arrow keys can be matched on `AL` alone | **False** — AL is 0, use AH scancode |
| 3 | `INT 10h/AH=02h` moves the cursor | **True** |
| 4 | `AAM` can print 3-digit numbers | **False** — only 0–99 |
| 5 | `LOOP` uses `CX` | **True** |
| 6 | `or al,20h` converts `'a'`→`'A'` | **False** — it forces lower case |
| 7 | The maze is a 3-D array in memory | **False** — flat 1875 bytes |
| 8 | `je` and `jz` are different instructions | **False** — identical |
| 9 | Setting mode 03h clears the screen | **True** — hence the flicker |
| 10 | The program can exit by falling off the end | **False** — must use INT 21h/4Ch |
