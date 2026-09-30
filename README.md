# MASM Maze

`Code/MAZE.ASM` is a 16-bit DOS maze game with 3 levels on a 25x25 grid.

- `#` is a wall, `@` is the player, `E` is the exit.
- Move with the arrow keys (or W A S D). Press ESC to quit.
- Reach `E` to go to the next level.

## Problem Statement
Write a MASM program to draw a simple maze using the # character to represent walls and empty spaces for paths. Place a player character (e.g., @) at a defined starting point in the maze. Allow the user to navigate the maze using the arrow keys (Up, Down, Left, Right) by handling keyboard interrupts. Ensure the player cannot move through walls (#) and can only move along valid paths. Continuously update and store the player's current position, and refresh the maze view after each move.

## Build and run

### MASM + LINK (in DOSBox)

Run these from inside the `Code` folder.

```
masm MAZE.ASM;
link MAZE.OBJ;
MAZE
```

### JWasm + DOSBox on Linux

A DOS `.EXE` cannot run directly on Linux, so it is run inside DOSBox. On Ubuntu or Debian, from the repository root:

```
sudo apt install dosbox build-essential git
git clone https://github.com/Baron-von-Riedesel/JWasm.git
make -C JWasm -f GccUnix.mak
cd Code
../JWasm/build/GccUnixR/jwasm -mz MAZE.ASM
dosbox MAZE.EXE
```

Click inside the DOSBox window before pressing keys, so it receives them.

---

# How the code works, line by line

A few terms come up often:

- **Registers:** `AX`, `BX`, `CX` and `DX` are 16-bit. Each can be used as two 8-bit halves, such as `AH` (high byte) and `AL` (low byte).
- **Interrupts:** `INT 10h` calls the BIOS video services, `INT 16h` the BIOS keyboard services and `INT 21h` the DOS services. The number put in `AH` before the call picks the service.

Lines that repeat the same pattern are explained once.

## 1. Header

```asm
.model small
.stack 100h
```

| Line | Meaning |
|---|---|
| `.model small` | The program uses one code segment and one data segment, each up to 64 KB. |
| `.stack 100h` | Reserves 256 bytes of stack. `call`, `ret`, `push` and `pop` use it. |

## 2. Data section

```asm
.data
level   db 0
prow    db 1
pcol    db 1
```

| Line | Meaning |
|---|---|
| `.data` | Everything below is data, not instructions. |
| `level db 0` | One byte for the current level: 0, 1 or 2. It shows on screen as 1, 2 or 3. |
| `prow db 1` | The player's current **row**. It starts at 1. |
| `pcol db 1` | The player's current **column**. It starts at 1. These two bytes are where the position is stored. |

```asm
msglvl  db 'Level $'
msgrow  db 'Row   $'
msgcol  db 'Col   $'
msgkey  db 'Arrows/WASD = Move$'
msgend  db 'E = Exit$'
msgesc  db 'ESC = Quit$'
msgwin  db 'You finished all 3 levels!$'
```

- Each line stores a text message.
- Every message ends with `$`, because the DOS print service (`AH=09h`) prints characters until it reaches a `$`.

```asm
mazes   db '#########################'
        db '#       #             # #'
        ...
```

- These are 75 lines of 25 characters each: three mazes of 25 rows.
- In memory they form one long run of bytes, stored row after row and level after level.
- One level is 25 × 25 = **625 bytes**.
- So the character at (level, row, col) is at byte `level*625 + row*25 + col` of `mazes`. That formula is the heart of the program.
- `#` is a wall, a space is a path and `E` is the exit. The `@` is never stored in the maze; it is drawn on top of it.

## 3. Program start

```asm
.code
main proc
    mov ax,@data
    mov ds,ax
```

| Line | Meaning |
|---|---|
| `.code` | Instructions start here. |
| `main proc` | Starts the procedure `main`, where the program begins. |
| `mov ax,@data` | Puts the address of the data segment into `AX`. |
| `mov ds,ax` | Copies it into `DS` so the program can reach its variables. It goes through `AX` because a constant can't be moved into `DS` directly. |

```asm
newlevel:
    mov prow,1
    mov pcol,1
```

- `newlevel:` is a label that the code jumps to whenever a level starts.
- The two `mov` lines put the player back at the start, (1,1).

## 4. Main game loop: reading a key

```asm
game:
    call draw
    mov ah,0
    int 16h
```

| Line | Meaning |
|---|---|
| `game:` | The top of the loop. The program returns here after every key press. |
| `call draw` | Clears the screen and redraws the maze, the `@` and the text. This is the refresh after each move. |
| `mov ah,0` | Selects keyboard service 0, "wait for a key". |
| `int 16h` | Calls the BIOS keyboard interrupt and waits until a key is pressed. It returns the **scan code** (which physical key) in `AH` and the **ASCII code** (which character) in `AL`. Arrow keys have no character, so they are identified by `AH`. |

```asm
    mov bl,prow
    mov bh,pcol
```

- Copies the current position into `BL` (row) and `BH` (column).
- The program changes this copy first and only saves it if the move turns out to be allowed.

```asm
    cmp al,27
    je quit
    cmp ah,01h
    je quit
```

- `cmp` compares two values, and `je` ("jump if equal") jumps when they matched.
- The first pair checks whether the key's character is ESC (ASCII 27).
- The second pair checks whether the key's scan code is ESC (01h).
- Checking both means ESC works whichever way the emulator reports it. Either way the program jumps to `quit`.

```asm
    or al,20h
```

- Sets bit 5 of `AL`, which turns an uppercase letter into lowercase (`'W'` becomes `'w'`).
- This way `W` and `w` need only one comparison.
- Arrow keys give `AL` = 0, so this does nothing harmful for them.

## 5. Deciding the direction

```asm
    cmp ah,48h
    je goup
    cmp al,'w'
    je goup
```

- If the key is the Up arrow (scan code 48h) **or** `w`, it jumps to `goup`.
- The next three pairs work the same way:

| Scan code | Letter | Direction |
|---|---|---|
| `50h` | `s` | Down → `godown` |
| `4Bh` | `a` | Left → `goleft` |
| `4Dh` | `d` | Right → `goright` |

```asm
    jmp game
```

- If the key was none of these, the program ignores it and goes back to the top of the loop.

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

- `dec` subtracts 1 and `inc` adds 1.
- Up means row − 1, Down row + 1, Left column − 1 and Right column + 1.
- Each case then jumps to `move`. `goright` needs no `jmp` because `move:` comes right after it, so it just continues into it.

## 6. Checking and making the move

```asm
move:
    call getcell
    cmp al,'#'
    je game
```

| Line | Meaning |
|---|---|
| `call getcell` | Looks up which character is at the new position (`BL`,`BH`) and returns it in `AL`. |
| `cmp al,'#'` / `je game` | If that cell is a wall, the move is thrown away. `prow`/`pcol` stay the same, so the player **cannot pass through walls**. |

```asm
    mov prow,bl
    mov pcol,bh
```

- The cell is a path, so the new position is saved. This is where the stored position is updated.

```asm
    cmp al,'E'
    jne game
```

- `jne` means "jump if not equal".
- If the player is not on the exit, the program goes back to the loop.

```asm
    inc level
    cmp level,3
    jb newlevel
```

- The player reached `E`, so the program moves to the next level.
- `jb` means "jump if below". If `level` is still under 3, it starts that level at `newlevel`.

```asm
    mov ax,0003h
    int 10h
    lea dx,msgwin
    call print
```

- This runs only after all 3 levels are finished.
- `AX=0003h` with `INT 10h` resets the 80×25 text screen, which also clears it.
- `lea dx,msgwin` puts the address of the win message into `DX`, and `print` shows it.

```asm
quit:
    mov ah,4Ch
    int 21h
main endp
```

- DOS service 4Ch ends the program and returns to DOS.
- `main endp` closes the `main` procedure.

## 7. `getcell`: read the maze cell at (BL, BH)

```asm
getcell proc
    mov al,level
    mov ah,0
    mov cx,625
    mul cx
    mov si,ax
```

- Puts `level` into `AX`. `AH=0` makes sure the full 16-bit value is correct.
- `mul cx` multiplies `AX` by 625, giving the byte where this level starts in `mazes`.
- The result is kept in `SI`.

```asm
    mov al,bl
    mov cl,25
    mul cl
    add si,ax
```

- `mul cl` is an 8-bit multiply: `AX = AL × CL`, so `AX = row × 25`.
- Adding it to `SI` moves to the start of that row.

```asm
    mov al,bh
    mov ah,0
    add si,ax
```

- Adds the column, so `SI = level*625 + row*25 + col`.

```asm
    mov al,mazes[si]
    ret
getcell endp
```

- Reads that byte from the maze into `AL` and returns to the caller.

## 8. `pnum`: print a number 0–99 as two digits

```asm
pnum proc
    mov ah,0
    aam
```

- `aam` divides `AL` by 10. The tens digit goes into `AH` and the units digit into `AL`.
- Example: 23 becomes `AH=2`, `AL=3`.

```asm
    add ax,3030h
```

- Adds 30h to both bytes, which turns the digits into their characters (`'0'` is 30h).
- So 2 and 3 become `'2'` and `'3'`.

```asm
    push ax
    mov dl,ah
    mov ah,2
    int 21h
```

- `push ax` saves both digits on the stack, because the next line overwrites `AH`.
- DOS service 2 prints the single character in `DL`, so this prints the tens digit.

```asm
    pop ax
    mov dl,al
    mov ah,2
    int 21h
    ret
pnum endp
```

- `pop ax` restores the saved digits.
- The rest prints the units digit and returns.

## 9. `print` and `setcur`: small helpers

```asm
print proc
    mov ah,9
    int 21h
    ret
print endp
```

- Prints the `$`-terminated text whose address is in `DX`.

```asm
setcur proc
    mov ah,2
    mov bh,0
    int 10h
    ret
setcur endp
```

- BIOS service 2 moves the cursor to row `DH`, column `DL`.
- `BH=0` means screen page 0.

## 10. `draw`: refresh the whole screen

```asm
draw proc
    mov ax,0003h
    int 10h
```

- Clears the screen by resetting 80×25 text mode.

```asm
    mov al,level
    mov ah,0
    mov cx,625
    mul cx
    mov si,ax
```

- Same calculation as in `getcell`: `SI` now points to the first character of the current level.

```asm
    mov dh,0
nextrow:
    mov dl,0
    call setcur
```

- `DH` counts rows, starting at 0.
- For each row, the cursor moves to (row, column 0).
- Moving the cursor for each row, instead of printing a new-line, stops the screen from scrolling after the 25th row. The screen has exactly 25 rows.

```asm
    mov cx,25
nextcol:
    mov dl,mazes[si]
    mov ah,2
    int 21h
    inc si
    loop nextcol
```

- Prints 25 characters of this row, one at a time, moving `SI` to the next character each time.
- `loop` subtracts 1 from `CX` and jumps back while `CX` is not 0.

```asm
    inc dh
    cmp dh,25
    jb nextrow
```

- Moves to the next row and repeats until all 25 rows are drawn.

```asm
    mov dh,2
    mov dl,30
    call setcur
    lea dx,msglvl
    call print
    mov al,level
    inc al
    call pnum
```

- Moves the cursor to row 2, column 30, to the right of the maze, and prints `Level `.
- Then it prints `level + 1`, so the screen shows 01–03 instead of 0–2.

The next five blocks follow the same pattern: move the cursor, then print.

| Screen row | Prints |
|---|---|
| 4 | `Row   ` + `prow` (the stored row) |
| 5 | `Col   ` + `pcol` (the stored column) |
| 8 | `Arrows/WASD = Move` |
| 9 | `E = Exit` |
| 10 | `ESC = Quit` |

```asm
    mov dh,prow
    mov dl,pcol
    call setcur
    mov ah,2
    mov dl,'@'
    int 21h
```

- Moves the cursor to the player's stored position and prints `@` over the maze.
- Maze row 0 is on screen row 0, so the position needs no adjustment.

```asm
    mov dh,12
    mov dl,30
    call setcur
    ret
draw endp
```

- Moves the blinking cursor to an empty spot so it doesn't sit on top of the `@`, then returns to the game loop.

## 11. End of file

```asm
end main
```

- Marks the end of the source and tells the assembler the program starts at `main`.

---

## Summary

```
start → reset position → ┌─ draw screen
                         │  wait for key (INT 16h)
                         │  ESC? → quit
                         │  work out new row/col in BL/BH
                         │  getcell: wall? → ignore move
                         │  save new position in prow/pcol
                         └─ on 'E'? → next level (or win after 3)
```
