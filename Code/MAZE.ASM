; ---------------------------------------------------------------------------
; MAZE.ASM - 16-bit DOS maze game
;
;   * 3 levels of 25x25 cells stored as one flat byte array
;   * arrow keys / WASD movement with wall collision
;   * colour drawn through BIOS text services for emulator compatibility
;   * the screen is cleared once; moves repaint only the affected cells
;   * BFS hinter (key H): computes exit distances once per level, then
;     highlights the complete shortest path from the player's current cell
;
; build:  masm MAZE.ASM;  link MAZE.OBJ;      (MASM + LINK, inside DOS)
;         jwasm -mz MAZE.ASM                  (JWasm)
; ---------------------------------------------------------------------------

.model small
.stack 100h

; ---------------------------------------------------------------------------
; constants
; ---------------------------------------------------------------------------
MW      equ 25                  ; maze width  (columns, bytes per row)
MH      equ 25                  ; maze height (rows)
MSZ     equ MW * MH             ; bytes per level (625)
NLEV    equ 3                   ; number of levels

; text attributes: high nibble = background colour, low nibble = foreground
A_WALL  equ 1Fh                 ; bright white on blue
A_PATH  equ 07h                 ; grey on black
A_EXIT  equ 2Eh                 ; bright yellow on green
A_PLAY  equ 4Eh                 ; bright yellow on red
A_HINT  equ 3Fh                 ; bright white on cyan
A_HUD   equ 0Bh                 ; cyan on black
A_WIN   equ 2Fh                 ; bright white on green
A_MSG   equ 0Fh                 ; bright white on black

; ---------------------------------------------------------------------------
; data
; ---------------------------------------------------------------------------
.data
; ---- game state --------------------------------------------------------
level   db 0                    ; 0 .. NLEV-1 (shown as 1 .. 3)
prow    db 1                    ; player row
pcol    db 1                    ; player column
hinton  db 0                    ; hint mode   0 = off, 1 = on
hintvis db 0                    ; 1 = hint path cells are highlighted
hrow    db 0                    ; current row while tracing the hint path
hcol    db 0                    ; current column while tracing the hint path
hintd   db 0                    ; player -> exit distance (FFh = unknown)
wantd   db 0                    ; distance being searched for in findnb

; ---- HUD text ('$' terminated, as required by vstr) --------------------
msglvl  db 'Level $'
msgdst  db 'Dist  $'
msgrow  db 'Row   $'
msgcol  db 'Col   $'
msgkey  db 'Arrows/WASD = Move$'
mghint  db 'H = Hint$'
msgend  db 'E = Exit$'
msgesc  db 'ESC = Quit$'
msgdash db '---$'
msgwin  db 'You finished all 3 levels!$'
msgany  db 'Press any key to exit$'

; ---- maze data: NLEV * MH rows of MW characters = 1875 bytes -----------
; '#' = wall, ' ' = path, 'E' = exit. '@' is never stored here.
mazes   db '#########################'
        db '#       #             # #'
        db '####### # ##### # # # # #'
        db '# #   # # #     # # #   #'
        db '# # # # # # # # # # #####'
        db '#   #   #   # #   #     #'
        db '# ######## #### ####### #'
        db '# #   #       #       # #'
        db '# # # # ##### # ####### #'
        db '# # # #   #     #     # #'
        db '# # # # # # ### # ### # #'
        db '#   # # # #   # #   #   #'
        db '### # ### ### # # # ### #'
        db '#   # #   #   # #   #   #'
        db '### # # ### ### # ##### #'
        db '#   # #   # #   #     # #'
        db '# # # # #   # ##### # # #'
        db '# #     # # # #   # # # #'
        db '# # ### # #   # # ### # #'
        db '#       #   # # # #   # #'
        db '# ####### ##### # # ### #'
        db '#   #   #   #   # # #   #'
        db '# # # # ### # ### # # ###'
        db '# #   #       #     #  E#'
        db '#########################'

        db '#########################'
        db '# #   #           #     #'
        db '# ### # ##### # ### # # #'
        db '# #   #   #   # #   #   #'
        db '# # ### # # ##### ##### #'
        db '# #     #   #   # #   # #'
        db '# ####### # # # # # # # #'
        db '#   #   # # # #     #   #'
        db '### # # # # # # ### # # #'
        db '#   # #   # # #   # # # #'
        db '# ### # ##### ### # # ###'
        db '#   # # #     # # # #   #'
        db '### # ### ##### # # ### #'
        db '# # #     #     #   #   #'
        db '# # ### ####### ##### ###'
        db '# #   #       #     #   #'
        db '# ### ####### # # # ### #'
        db '#   #     # # # #   #   #'
        db '# # ##### # # # ### # ###'
        db '# # #     #     #   #   #'
        db '# ### ################# #'
        db '# #   #         #     # #'
        db '# # ### ####### # ### # #'
        db '#       #         #    E#'
        db '#########################'

        db '#########################'
        db '# #         #     # #   #'
        db '# # ####### # ### # # # #'
        db '# # #     #   #   #   # #'
        db '# # ### ####### ####### #'
        db '#   #   #     #         #'
        db '##### ### # ########### #'
        db '#         #     #     # #'
        db '# ############# # ### # #'
        db '#   #         #   # # # #'
        db '### # ### ######### # # #'
        db '#   #   # #         #   #'
        db '# ### # ### # ###########'
        db '# #   #     #     #     #'
        db '# ########### ### # ### #'
        db '#           #   #   #   #'
        db '########### ### ##### # #'
        db '#   #     # #   #   # # #'
        db '# # ##### # # ### # # # #'
        db '# #     # # # #   # # # #'
        db '# ##### # # # # ### # ###'
        db '# #       # # # #   #   #'
        db '# ######### ### # ##### #'
        db '#               #      E#'
        db '#########################'

; ---- BFS tables (one level at a time) ----------------------------------
dist    db MSZ dup(0FFh)        ; distance of a cell to 'E', FFh = not reached
hintmap db MSZ dup(0)           ; cells currently highlighted by the hint
queue   dw MSZ dup(0)           ; BFS queue, packed: low byte = row, high = col
bhead   dw 0                    ; queue read index
btail   dw 0                    ; queue write index
lbase   dw 0                    ; offset of the current level inside 'mazes'
exrow   db 0                    ; exit row    (found by scanning)
excol   db 0                    ; exit column (found by scanning)

; ---------------------------------------------------------------------------
; code
; ---------------------------------------------------------------------------
.code

; ===========================================================================
; main - program entry, state machine around the input loop
; ===========================================================================
main proc
    mov ax,@data
    mov ds,ax

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

; ---- input loop ----------------------------------------------------------
game:
    mov ah,08h
    int 21h                     ; read a key without echo
    cmp al,27                   ; ESC as ASCII
    je quit
    cmp al,0                    ; DOS extended-key prefix
    je read_scan
    cmp al,0E0h                 ; alternate extended-key prefix
    je read_scan
    mov bl,prow                 ; candidate position (only committed if legal)
    mov bh,pcol

    or al,20h                   ; force lower case: 'W' and 'w' both become 'w'
    cmp al,'h'
    je hintkey
    cmp al,'w'
    je goup
    cmp al,'s'
    je godown
    cmp al,'a'
    je goleft
    cmp al,'d'
    je goright
    jmp game                    ; any other key: nothing changed, no redraw

read_scan:
    mov ah,08h
    int 21h                     ; AL = extended-key scan code
    mov bl,prow
    mov bh,pcol
    cmp al,48h                  ; arrow up
    je goup
    cmp al,50h                  ; arrow down
    je godown
    cmp al,4Bh                  ; arrow left
    je goleft
    cmp al,4Dh                  ; arrow right
    je goright
    jmp game

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

; ---- validate and apply the move ----------------------------------------
move:
    cmp bl,MH
    jae game                    ; reject an out-of-grid row
    cmp bh,MW
    jae game                    ; reject an out-of-grid column
    call getcell                ; AL = cell we want to step into
    cmp al,'#'
    je game                     ; wall  -> reject, screen untouched
    cmp al,'E'
    je doexit                   ; exit  -> level complete

    push bx                     ; repaint helpers use BL/BH internally
    call clearhint              ; repaint the old highlighted cell
    call clrplayer              ; repaint the cell the player leaves
    pop bx                      ; restore the proposed destination
    mov prow,bl                 ; commit the new position
    mov pcol,bh
    call putplayer              ; '@' onto the new cell
    cmp hinton,0
    je mv_hud
    call refreshhint            ; highlight the next cell of the shortest path
mv_hud:
    call updhud                 ; rewrite only the Row/Col/Dist digits
    jmp game

; ---- level finished ------------------------------------------------------
doexit:
    call clearhint              ; remove the old level's highlighted path
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

; ---- hint key ------------------------------------------------------------
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

quit:
    mov ah,4Ch
    int 21h                     ; return to DOS
main endp

; ===========================================================================
; getcell - AL = maze character at (BL = row, BH = col) of the current level
;           index = level*MSZ + row*MW + col        (row-major layout)
;           clobbers AX, CX, SI - preserves BL, BH
; ===========================================================================
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

; ===========================================================================
; vcell - write ONE character cell using BIOS text services
;         IN : AL = character, AH = attribute, DH = row, DL = column
;         preserves BX, CX, DX   (destroys AX)
; ===========================================================================
vcell proc
    push bx
    push cx
    push dx
    push ax                     ; preserve character and attribute
    mov bh,0
    mov ah,02h                  ; set cursor position on page zero
    int 10h
    pop ax
    pop dx
    mov bl,ah                   ; BIOS function 09h takes attribute in BL
    mov bh,0
    mov cx,1
    mov ah,09h                  ; write one character with its attribute
    push dx
    int 10h
    pop dx
    pop cx
    pop bx
    ret
vcell endp

; ===========================================================================
; vstr - write a '$'-terminated string into video RAM
;        IN : SI = string, AH = attribute, DH/DL = start position
;        advances DL, preserves DH
; ===========================================================================
vstr proc
    push bx
    push cx
    mov cl,ah                   ; keep the attribute
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
    pop cx
    pop bx
    ret
vstr endp

; ===========================================================================
; vnum2 - print AL (0..99) as exactly two digits
;         IN : AL = value, AH = attribute, DH/DL = position (DL += 2)
; ===========================================================================
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

; ===========================================================================
; paintcell - repaint one maze cell with its natural colour
;             IN : BL = row, BH = col
; ===========================================================================
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
paintcell endp

; ===========================================================================
; player cells
; ===========================================================================
clrplayer proc                  ; repaint the cell under the player
    mov bl,prow
    mov bh,pcol
    call paintcell
    ret
clrplayer endp

putplayer proc                  ; stamp '@' on the current cell
    mov al,'@'
    mov ah,A_PLAY
    mov dh,prow
    mov dl,pcol
    call vcell
    ret
putplayer endp

; ===========================================================================
; clearhint - remove the highlight from the previously shown hint cell
; ===========================================================================
clearhint proc
    cmp hintvis,0
    je ch_done
    xor si,si
ch_loop:
    cmp si,MSZ
    jae ch_done
    cmp hintmap[si],0
    je ch_next
    mov hintmap[si],0
    push si
    mov ax,si
    xor dx,dx
    mov cx,MW
    div cx                      ; AX = row, DX = column
    mov bl,al
    mov bh,dl
    call paintcell              ; restore this cell's natural colour
    pop si
ch_next:
    inc si
    jmp ch_loop
ch_done:
    mov hintvis,0
    ret
clearhint endp

; ===========================================================================
; drawmaze - draw the whole current level with colours
;            called only when a level starts (never inside the move loop,
;            which is what keeps the display flicker free)
; ===========================================================================
drawmaze proc
    mov al,level
    xor ah,ah
    mov cx,MSZ
    mul cx
    mov si,ax                   ; SI = first byte of this level
    xor dh,dh                   ; screen row 0 .. MH-1
dm_row:
    xor dl,dl                   ; screen column 0 .. MW-1
dm_col:
    mov al,mazes[si]
    mov ah,A_PATH
    cmp al,'#'
    jne dm_notwall
    mov ah,A_WALL
dm_notwall:
    cmp al,'E'
    jne dm_notexit
    mov ah,A_EXIT
dm_notexit:
    call vcell                  ; one STORE instead of a BIOS call
    inc si
    inc dl
    cmp dl,MW
    jb dm_col
    inc dh
    cmp dh,MH
    jb dm_row
    ret
drawmaze endp

; ===========================================================================
; drawhud - sidebar at column 30 (the maze only occupies columns 0..24)
;           all labels are 6 characters wide, so the numbers sit at column 36
; ===========================================================================
drawhud proc
    mov dh,2
    mov dl,30
    mov si,offset msglvl
    mov ah,A_HUD
    call vstr
    mov al,level
    add al,'1'                  ; 0 -> '1', 2 -> '3'
    mov ah,A_HUD
    mov dh,2
    mov dl,36
    call vcell

    mov dh,3
    mov dl,30
    mov si,offset msgdst
    mov ah,A_HUD
    call vstr
    mov dh,3
    mov dl,36
    mov si,offset msgdash       ; distance is unknown until H is pressed
    mov ah,A_HUD
    call vstr

    mov dh,4
    mov dl,30
    mov si,offset msgrow
    mov ah,A_HUD
    call vstr
    mov dh,4
    mov dl,36
    mov al,prow
    mov ah,A_HUD
    call vnum2

    mov dh,5
    mov dl,30
    mov si,offset msgcol
    mov ah,A_HUD
    call vstr
    mov dh,5
    mov dl,36
    mov al,pcol
    mov ah,A_HUD
    call vnum2

    mov dh,7
    mov dl,30
    mov si,offset msgkey
    mov ah,A_HUD
    call vstr

    mov dh,8
    mov dl,30
    mov si,offset mghint
    mov ah,A_HUD
    call vstr

    mov dh,9
    mov dl,30
    mov si,offset msgend
    mov ah,A_HUD
    call vstr

    mov dh,10
    mov dl,30
    mov si,offset msgesc
    mov ah,A_HUD
    call vstr
    ret
drawhud endp

; ===========================================================================
; updhud - refresh ONLY the changing digits (Row, Col, Dist)
;          direct video RAM writes, so there is nothing to flicker
; ===========================================================================
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
    mov dh,5
    mov dl,36
    mov al,pcol
    mov ah,A_HUD
    call vnum2
    ret
updhud endp

; ===========================================================================
; bfsolve - fill dist[] with the shortest distance of every walkable cell of
;           the current level to its exit 'E'  (breadth first search).
;           Run once per level; hints follow the precomputed distances.
; ===========================================================================
bfsolve proc
    ; ---- locate the exit -------------------------------------------------
    mov al,level
    xor ah,ah
    mov cx,MSZ
    mul cx
    mov lbase,ax                ; start of this level inside 'mazes'
    mov si,ax
    mov di,ax
    add di,MSZ                  ; one past the end
bs_find:
    cmp mazes[si],'E'
    je bs_found
    inc si
    cmp si,di
    jb bs_find
    ret                         ; no exit: dist[] stays 0FFh everywhere

bs_found:
    mov ax,si
    sub ax,lbase                ; AX = index of 'E' inside the level
    xor dx,dx
    mov cx,MW
    div cx                      ; AX = row, DX = column
    mov exrow,al
    mov excol,dl

    ; ---- dist[] := all 0FFh ---------------------------------------------
    push ds
    pop es                      ; ES = DS for REP STOSB
    mov di,offset dist
    mov cx,MSZ
    mov al,0FFh
    rep stosb

    ; ---- empty queue, seed it with the exit ------------------------------
    mov bhead,0
    mov btail,0
    mov al,exrow
    xor ah,ah
    mov cl,MW
    mul cl
    mov si,ax
    xor ah,ah
    mov al,excol
    add si,ax                   ; SI = index of the exit
    mov dist[si],0              ; distance(exit, exit) = 0
    mov al,exrow
    mov ah,excol                ; queue entry: AL = row, AH = col
    xor di,di
    mov queue[di],ax
    mov btail,1

    ; ---- expand ----------------------------------------------------------
bs_loop:
    mov ax,bhead
    cmp ax,btail
    jae bs_done                 ; queue empty -> every reachable cell done
    mov si,ax
    shl si,1                    ; queue holds words
    mov ax,queue[si]
    inc bhead
    mov bl,al                   ; row of the cell being expanded
    mov bh,ah                   ; column

    mov al,bl
    xor ah,ah
    mov cl,MW
    mul cl
    mov si,ax
    xor ah,ah
    mov al,bh
    add si,ax                   ; SI = index of this cell
    mov dl,dist[si]
    inc dl                      ; distance the neighbours will get
    cmp dl,0FEh
    jae bs_loop                 ; byte would overflow: stop expanding

    push bx                     ; up
    dec bl
    call trynb
    pop bx
    push bx                     ; down
    inc bl
    call trynb
    pop bx
    push bx                     ; left
    dec bh
    call trynb
    pop bx
    push bx                     ; right
    inc bh
    call trynb
    pop bx
    jmp bs_loop
bs_done:
    ret
bfsolve endp

; ===========================================================================
; trynb - try to enqueue one neighbour
;         IN : BL = row, BH = col, DL = distance to store
;         keeps BL, BH, DL
; ===========================================================================
trynb proc
    cmp bl,MH
    jae tn_done                 ; rows are 0 .. MH-1 (BL may be 255 after DEC)
    cmp bh,MW
    jae tn_done                 ; columns are 0 .. MW-1

    mov al,bl
    xor ah,ah
    mov cl,MW
    mul cl
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

; ===========================================================================
; findnb - does (BL,BH) hold the distance we are looking for?
;          IN : BL = row, BH = col, wantd = required distance
;          OUT: CF = 1 and hrow/hcol set   |   CF = 0
; ===========================================================================
findnb proc
    cmp bl,MH
    jae fn_no
    cmp bh,MW
    jae fn_no
    mov al,bl
    xor ah,ah
    mov cl,MW
    mul cl
    mov si,ax
    xor ah,ah
    mov al,bh
    add si,ax                   ; SI = index inside the level
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
findnb endp

; ===========================================================================
; refreshhint - full shortest path from the player's current cell to 'E'
;               IN : (uses prow, pcol and dist[] from bfsolve)
;               sets hintd, hintvis and highlights every step to the exit
; ===========================================================================
refreshhint proc
    call clearhint              ; drop any previous highlight first

    mov al,prow
    xor ah,ah
    mov cl,MW
    mul cl
    mov si,ax
    xor ah,ah
    mov al,pcol
    add si,ax                   ; SI = index of the player
    mov al,dist[si]
    mov hintd,al                ; remaining optimal distance
    cmp al,0FFh
    je rh_none
    cmp al,0
    je rh_done                  ; standing on the exit: nothing to show
    dec al
    mov wantd,al                ; the next cell must be one step closer

    mov al,prow
    mov hrow,al
    mov al,pcol
    mov hcol,al
rh_search:
    mov bl,hrow
    mov bh,hcol
    dec bl
    call findnb
    jc rh_mark
    mov bl,hrow
    mov bh,hcol
    inc bl
    call findnb
    jc rh_mark
    mov bl,hrow
    mov bh,hcol
    dec bh
    call findnb
    jc rh_mark
    mov bl,hrow
    mov bh,hcol
    inc bh
    call findnb
    jc rh_mark
rh_none:
    mov hintd,0FFh              ; unreachable / not computable: show '---'
    ret

rh_mark:
    mov bl,hrow
    mov bh,hcol
    mov al,bl
    xor ah,ah
    mov cl,MW
    mul cl
    mov si,ax
    xor ah,ah
    mov al,bh
    add si,ax                   ; SI = path cell index
    mov hintmap[si],1
    call getcell                ; keep the real character on the cell
    mov ah,A_HINT               ; ...but light it up in the hint colour
    mov dh,bl
    mov dl,bh
    call vcell
    mov hintvis,1
    cmp wantd,0
    je rh_done
    dec wantd
    jmp rh_search
rh_done:
    ret
refreshhint endp

end main
