# DOSBox-RS

A small DOSBox-style 8086 PC emulator written in Rust, with a built-in MASM assembler and LINK linker. It has no external crates.

It mounts a host folder as drive `C:` and gives you a DOS prompt, where you can assemble, link and run real 16-bit programs:

```
C:\>masm MAZE.ASM;
C:\>link MAZE.OBJ;
C:\>MAZE
```

## Build and run

```
cargo build --release
target/release/dosbox ../Code
```

On Windows, if the default MSVC toolchain reports that `link.exe` is missing, build with the GNU toolchain instead:

```
cargo +stable-x86_64-pc-windows-gnu build --release
```

Options:

| Option | Meaning |
|---|---|
| `DIRECTORY` | Host folder mounted as `C:` (default: the current folder) |
| `-c COMMAND` | Run a DOS command at startup. Can be repeated. |
| `--exit` | Quit after the `-c` commands |
| `--headless` | No window. The final screen is printed to stdout. |
| `--keys SCRIPT` | Keys typed in headless mode, e.g. `"ab{down}{esc}"` |
| `--screenshot FILE` | In headless mode, also save the final screen as a coloured HTML page |

Example that builds and plays one move of the maze without a window:

```
dosbox ../Code --headless --exit -c "masm MAZE.ASM;" -c "link MAZE.OBJ;" -c MAZE --keys "{right}h"
```

Keys: **Ctrl+C** stops the running program, **Ctrl+F9** quits the emulator.

## DOS commands

`DIR`, `CD`, `CLS`, `TYPE`, `DEL`, `REN`, `MD`, `RD`, `ECHO`, `VER`, `HELP` and `EXIT`, plus:

- `MASM source[.ASM][,object][;]` assembles to an `.OBJ` file
- `LINK object[.OBJ][,exe][,map][;] [/T] [/M]` links to an `.EXE`. `/T` makes a `.COM` file and `/M` writes a `.MAP` file.
- Typing a program's name runs its `.COM`, `.EXE` or `.BAT` file.

## How the source is organised

| Folder | What it does |
|---|---|
| `src/main.rs`, `src/config.rs` | Command-line options; starts the shell |
| `src/cpu/` | The 8086 CPU (with the common 80186 additions) |
| ↳ `registers.rs`, `flags.rs` | Register file and the FLAGS word |
| ↳ `modrm.rs` | Decoding of ModR/M bytes and effective addresses |
| ↳ `alu.rs`, `muldiv.rs`, `bcd.rs` | Arithmetic, shifts, multiply/divide, BCD adjust |
| ↳ `stack.rs`, `string.rs` | PUSH/POP and interrupts; MOVS/CMPS/STOS/LODS/SCAS with REP |
| ↳ `groups.rs`, `execute.rs` | The opcode dispatcher |
| `src/hardware/` | 1 MB memory, 80×25 text video at `B800:0000`, keyboard buffer, I/O ports, clock |
| `src/machine/` | Ties CPU, memory, devices and terminal together; runs programs and routes interrupts |
| `src/bios/` | INT 10h (video, including turning blinking off for 16 background colours), INT 16h (keyboard), INT 15h and 1Ah (system and time) |
| `src/dos/` | INT 21h: console I/O, files, directories, memory, the program loader and the PSP |
| `src/formats/` | The MZ `.EXE` header and the object-file format |
| `src/masm/` | The assembler |
| ↳ `lexer.rs`, `expr.rs`, `parser.rs`, `statement.rs` | Source text → tokens → expressions → statements |
| ↳ `symbols.rs`, `segments.rs`, `eval.rs`, `operand.rs` | Symbols, segment/group layout, expression values, operands |
| ↳ `assembler.rs`, `directives.rs`, `data.rs` | The multi-pass driver, directives, and `DB`/`DW`/`DD` |
| ↳ `encoder/` | Machine-code encoding: arithmetic, moves, branches, misc |
| ↳ `output.rs` | Builds the object file |
| `src/linker/` | Object file → `.EXE` or `.COM`, plus the `.MAP` file |
| `src/shell/` | The DOS prompt: internal commands, MASM/LINK commands, programs and batch files |
| `src/terminal/` | Screen output with ANSI colours and the CP437 character set; keyboard input for Windows, Unix and headless mode |

## Supported MASM subset

- Simplified segments (`.MODEL TINY/SMALL/MEDIUM/COMPACT/LARGE`, `.CODE`, `.DATA`, `.DATA?`, `.CONST`, `.STACK`, `.STARTUP`, `.EXIT`)
- Full segment syntax (`SEGMENT`/`ENDS`, `GROUP`, `ASSUME`)
- `PROC`/`ENDP` (NEAR and FAR), labels, `LABEL`, `EQU`, `=`, `ORG`, `EVEN`, `ALIGN`, `END`
- `DB`/`DW`/`DD`/`DQ`/`DT` with strings, `DUP` and `?`
- Expressions: `OFFSET`, `SEG`, `PTR`, `SHORT`, `HIGH`, `LOW`, `TYPE`, `LENGTH`, `SIZE`, arithmetic and logic operators, `$`
- The full 8086 instruction set, plus `PUSHA`/`POPA`, `PUSH imm`, `IMUL r,rm,imm`, shift by immediate, and `ENTER`/`LEAVE`

Out-of-range conditional jumps are extended automatically.

Not supported: macros, structures, conditional assembly, `INCLUDE`, and linking more than one object module (`EXTRN`/`PUBLIC`). The object file uses this project's own format, not Microsoft OMF.
