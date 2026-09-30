# Theory 03 — How MAZE.ASM Works (Code Walkthrough)

A complete, line-by-line explanation of `Code/MAZE.ASM`.

---

## 1. Program skeleton

```asm
.model small          ; 64 KB code + 64 KB data
.stack 100h           ; 256-byte stack
.data ...             ; level, prow, pcol, messages, 3×625 maze bytes
.code                 ; main + 5 procedures
main proc ... endp
getcell / pnum / print / setcur / draw
end main              ; entry point
```

Execution order: **`main` → (loop) → `INT 21h/4Ch`**.

---

## 2. The data segment

```asm
level   db 0        ; current level 0..2  (HUD shows level+1)
prow    db 1        ; player row (0-based, maze coords)  — start = (1,1)
pcol    db 1        ; player column
msglvl  db 'Level $'   ; '$' = terminator for INT 21h / AH=09h
...
mazes   db '#########################'   ; level 0 : rows 0..24
        db '#       #             # #'   ; level 1 : rows 25..49
        ...                              ; level 2 : rows 50..74
        db '#########################'
```

Key points:

- The three mazes are **one contiguous 1875-byte array** (3 × 625), not three
  separate arrays. Level *n* simply starts at byte `n × 625`.
- Each maze row is exactly **25 characters** and there are **25 rows** → 625 bytes.
- All four borders are solid `#`, which is what makes the missing bounds check safe.
- `@` start is `(1,1)` in every level; the exit `E` is on the bottom row.

---

## 3. `main` — setup

```asm
main proc
    mov ax,@data     ; @data = segment address of .data (linker symbol)
    mov ds,ax        ; DS must be valid before ANY variable access
```

Then it falls into `newlevel`.

---

## 4. `newlevel` — level reset

```asm
newlevel:
    mov prow,1       ; reset player to the start corner
    mov pcol,1
game:                ; <-- normal re-entry point of the loop
```

Called when a level is completed (and at startup, since `main` falls into it).

---

## 5. `game` — the input loop

```asm
game:
    call draw        ; 1. render everything (screen + HUD + player)
    mov ah,0
    int 16h          ; 2. BLOCK until a key is pressed → AL=ASCII, AH=scancode
    mov bl,prow      ; 3. candidate position in BL/BH (proposed, not committed)
    mov bh,pcol
```

**Why a *candidate*?** The player's real position (`prow`,`pcol`) is only updated
*after* the move has been validated. `BL/BH` hold the row/col we *want* to move to.

### Key decoding

```asm
    cmp al,27        ; ESC (ASCII 27)?
    je quit
    cmp ah,01h       ; ESC (scancode 01h)?
    je quit

    or al,20h        ; force lower case → 'W'/'A'/'S'/'D' all match

    cmp ah,48h       ; Arrow Up      \  arrows have no ASCII,
    je goup          ;                |  so they are matched on AH
    cmp al,'w'       ; 'w' has ASCII, so it is matched on AL
    je goup

    cmp ah,50h       ; Arrow Down
    je godown
    cmp al,'s'
    je godown

    cmp ah,4Bh       ; Arrow Left
    je goleft
    cmp al,'a'
    je goleft

    cmp ah,4Dh       ; Arrow Right
    je goright
    cmp al,'d'
    je goright

    jmp game         ; any other key → ignore, redraw, wait again
```

Each direction label only tweaks the candidate:

```asm
goup:    dec bl      ; row - 1
         jmp move
godown:  inc bl      ; row + 1
         jmp move
goleft:  dec bh      ; col - 1
         jmp move
goright: inc bh      ; col + 1
         jmp move
```

---

## 6. `move` — collision detection & commit

```asm
move:
    call getcell     ; AL = maze[bl][bh]  (the cell we want to enter)
    cmp al,'#'
    je game          ; WALL → discard candidate, position unchanged

    mov prow,bl      ; free cell → COMMIT the move
    mov pcol,bh

    cmp al,'E'
    jne game         ; ordinary path → redraw and wait for next key

    inc level        ; EXIT reached → next level
    cmp level,3
    jb newlevel      ; levels 0,1,2 exist → reset position, reload

    ; level == 3 → finished all levels
    mov ax,0003h
    int 10h          ; reset/clear the screen
    lea dx,msgwin
    call print       ; "You finished all 3 levels!"
                     ; (falls through into quit)

quit:
    mov ah,4Ch
    int 21h          ; terminate → return to DOS
main endp
```

**The whole game logic in one sentence:** compute the next cell; if it isn't `#`, keep
it; if it's `E`, advance the level; otherwise keep playing.

---

## 7. `getcell` — the 3-D → 1-D address calculation

```asm
getcell proc
    mov al,level
    mov ah,0         ; AX = level
    mov cx,625
    mul cx           ; AX = level * 625        (which of the 3 mazes?)
    mov si,ax

    mov al,bl        ; candidate row
    mov cl,25
    mul cl           ; AX = row * 25           (which row inside the maze?)
    add si,ax        ; SI = level*625 + row*25

    mov al,bh        ; candidate column
    mov ah,0
    add si,ax        ; SI = level*625 + row*25 + col

    mov al,mazes[si] ; AL = that byte  (direct-indexed addressing)
    ret
getcell endp
```

Result: **`AL = mazes[level*625 + row*25 + col]`** — the classic row-major index,
exactly what the C compiler would compute for `mazes[level][row][col]`.

Sanity check: max index = `2*625 + 24*25 + 24 = 1874`, and the array is 1875 bytes
→ always in range *because* the border walls stop the player at row/col 0 and 24.

Note `mov cl,25` destroys `CX` (which previously held 625) — harmless because `CX` is
not needed afterwards in this procedure.

---

## 8. `draw` — full-screen render

```asm
draw proc
    mov ax,0003h
    int 10h          ; set 80x25 text mode → ALSO CLEARS the screen

    mov al,level
    mov ah,0
    mov cx,625
    mul cx
    mov si,ax        ; SI = first byte of the current level's maze
    mov dh,0         ; DH = row counter
nextrow:
    mov dl,0         ; cursor to column 0 of this row
    call setcur      ; INT 10h / 02h
    mov cx,25        ; CX = loop counter for LOOP
nextcol:
    mov dl,mazes[si]
    mov ah,2
    int 21h          ; write char (cursor auto-advances)
    inc si
    loop nextcol     ; CX--, repeat while CX≠0
    inc dh           ; next row
    cmp dh,25
    jb nextrow
```

Inner structure: a nested loop — **rows × 25 columns**, printing byte after byte
straight from the maze array. The player is *not* drawn here from `prow/pcol`; it is
overwritten later.

### The HUD sidebar (col 30, safely right of the 25-wide maze)

```asm
    mov dh,2 :  mov dl,30 :  call setcur :  lea dx,msglvl : call print : <print level+1>
    mov dh,4 :  ... "Row   " + prow
    mov dh,5 :  ... "Col   " + pcol
    mov dh,8 :  ... "Arrows/WASD = Move"
    mov dh,9 :  ... "E = Exit"
    mov dh,10:  ... "ESC = Quit"
```

### Player glyph + park the cursor

```asm
    mov dh,prow
    mov dl,pcol
    call setcur      ; cursor to the player's cell
    mov ah,2
    mov dl,'@'
    int 21h          ; stamp '@' over the path cell

    mov dh,12
    mov dl,30
    call setcur      ; park cursor off the maze so the next echo looks tidy
    ret
draw endp
```

Because `prow/pcol` always point at a non-`#` cell, stamping `@` there can never
overwrite a wall.

---

## 9. The two small helpers

```asm
print proc           ; print the $-terminated string at DS:DX
    mov ah,9
    int 21h
    ret
print endp

setcur proc          ; put cursor at (DH,DL) on page 0
    mov ah,2
    mov bh,0
    int 10h
    ret
setcur endp
```

```asm
pnum proc            ; print the value in AL (0..99) as two digits
    mov ah,0
    aam              ; AH=tens, AL=units
    add ax,3030h     ; → ASCII
    push ax          ; save both digits
    mov dl,ah        ; tens first
    mov ah,2
    int 21h
    pop ax
    mov dl,al        ; then units
    int 21h          ; AH is still 2 from before... but is it?
    ret
pnum endp
```

⚠️ Worth noting in review: after `pop ax`, `AH` still holds `2` (the function number
restored from the pushed value `tens+30h`… actually `AH` becomes `tens+30h`). The code
therefore **re-sets nothing** — look closely:

- `push ax` saved `AX = tens/units ASCII`.
- `mov ah,2` then `int 21h` prints the tens digit.
- `pop ax` **restores** `AX` to the ASCII value → `AH` is now `tens+30h`, **not** 2.
- `mov dl,al` then `int 21h` would call the *wrong* DOS function!

**In practice it works** because `int 21h` is called with `AH = tens+30h`… which is an
invalid/undocumented function. This is a genuine latent bug — see Theory 04 (fix:
`mov ah,2` again after `pop ax`). Good interview material: "find the bug in `pnum`."

---

## 10. Complete data-flow summary

```
             ┌────────────── mazes[] (1875 bytes) ──────────────┐
             │  level 0 (625) │ level 1 (625) │ level 2 (625)   │
             └───────────────┬──────────────────────────────────┘
                             │ getcell: level*625 + row*25 + col
   key ──▶ BL/BH ──▶ AL ────┴──► '#' ? reject : commit prow/pcol
                             │
                     'E' ────┴──► inc level ──► newlevel / win
                             │
   draw: SI walks the current 625-byte window, row by row,
         then HUD prints prow/pcol/level, then '@' is stamped.
```

| Variable | Written by | Read by |
|---|---|---|
| `prow`,`pcol` | `main` (commit), `newlevel` (reset) | `draw`, `getcell` (as BL/BH) |
| `level` | `main` (on exit cell) | `getcell`, `draw` |
| `mazes` | never (static) | `getcell`, `draw` |
