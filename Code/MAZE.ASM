; ---------------------------------------------------------------------------
; MAZE.ASM - 16-bit DOS random maze game, 1 player or 2 player (PvP)
;
;   * 3 levels: 15x15, 24x24 and 36x36 grids, generated at run time
;   * randomised depth-first search carves a perfect maze, then extra walls
;     are knocked out so there is ALWAYS more than one route to the exit
;   * 1 player : arrow keys or WASD, H toggles the shortest-path hint
;   * 2 players: the same maze twice, side by side
;                player 1 = left maze, W A S D
;                player 2 = right maze, arrow keys
;                time limit 30 / 45 / 60 seconds; first to the exit wins,
;                at time-out the player closer to the exit wins
;   * after every level: optimal path in yellow, walked path in red,
;     squares on both in orange
;   * every grid square is one "pixel": the half-block character (DFh)
;     holds two square pixels per text cell (top = foreground colour,
;     bottom = background colour), so a 36x36 grid needs only 18 text rows
;
; build:  masm MAZE.ASM;  link MAZE.OBJ;      (MASM + LINK, inside DOS)
;         jwasm -mz MAZE.ASM                  (JWasm)
; ---------------------------------------------------------------------------

.model small
.stack 200h

; ---------------------------------------------------------------------------
; constants
; ---------------------------------------------------------------------------
MAXN    equ 36                  ; largest grid side
MAXC    equ MAXN * MAXN         ; largest number of grid squares (1296)
NLEV    equ 3                   ; number of levels

ORGY    equ 2                   ; maze origin, in pixels (text row 1)
LEFTX   equ 2                   ; x of the left (or only) maze
RIGHTX  equ 42                  ; x of the right maze in PvP mode
HUDC    equ 44                  ; text column of the 1 player sidebar

; pixel colours (0-15; 16 background colours because blinking is turned off)
C_FLOOR equ 0                   ; black
C_WALL  equ 1                   ; blue
C_HINT  equ 3                   ; cyan
C_BOTH  equ 6                   ; orange/brown = optimal AND walked
C_PLAY  equ 10                  ; bright green  = player 1
C_PLAY2 equ 11                  ; bright cyan   = player 2
C_TAKEN equ 12                  ; light red     = walked by the player
C_EXIT  equ 13                  ; light magenta = exit
C_OPT   equ 14                  ; yellow        = optimal path

; text attributes: high nibble = background, low nibble = foreground
A_TITLE equ 1Fh                 ; bright white on blue
A_HUD   equ 0Bh                 ; cyan on black
A_MSG   equ 0Fh                 ; bright white on black
A_WIN   equ 2Fh                 ; bright white on green
A_LOSE  equ 4Fh                 ; bright white on red
A_P1    equ 0Ah                 ; bright green on black
A_P2    equ 0Bh                 ; bright cyan on black
A_DIM   equ 07h                 ; grey on black

; ---------------------------------------------------------------------------
; data
; ---------------------------------------------------------------------------
.data
; ---- game state --------------------------------------------------------
players db 1                    ; 1 = single player, 2 = PvP
level   db 0                    ; 0 .. NLEV-1 (shown as 1 .. 3)
msize   dw 0                    ; side of the current grid
cells   dw 0                    ; msize * msize
spos    dw 0                    ; start square  (row 1, column 1)
epos    dw 0                    ; exit square
who     db 0                    ; player being moved / drawn (0 or 1)
ppos    dw 2 dup(0)             ; square of player 1 and player 2
steps   dw 2 dup(0)             ; moves made by each player
orgx    db LEFTX, RIGHTX        ; x origin of each player's maze
score   db 2 dup(0)             ; levels won by each player
winner  db 0                    ; 0 = player 1, 1 = player 2, 2 = draw
reason  db 0                    ; 0 = reached the exit, 1 = time out
pathlen dw 0                    ; start -> exit distance before braiding
best    dw 0                    ; scratch: best row+column while placing exit
extra   dw 0                    ; number of extra loops to add
hinton  db 0                    ; 1 = hint path is shown (1 player only)
mode    db 0                    ; 0 = playing, 1 = showing the level result
strict  db 0                    ; 1 = checkwall only accepts shortcuts
tries   db 0                    ; maze generation attempts
seed    dw 0                    ; random number generator state
dirv    dw 0                    ; direction being tried by the generator
pick    dw 0                    ; scratch: which candidate wall to open
qhead   dw 0                    ; BFS queue read index
qtail   dw 0                    ; BFS queue write index
dtop    dw 0                    ; DFS stack depth
tstart  dw 0                    ; PvP: clock tick when the level started
tlimit  dw 0                    ; PvP: ticks allowed for this level
lastsec dw 0                    ; PvP: seconds shown on screen

deltas  dw 4 dup(0)             ; index steps: up, down, left, right
cand    dw 4 dup(0)             ; DFS: neighbours that may be carved

sizes   db 15, 24, 36           ; grid side per level
limits  dw 546, 819, 1092       ; 30, 45, 60 seconds at 18.2 ticks/second

numbuf  db '0000$'              ; vnum4 output buffer

; ---- text ('$' terminated) ---------------------------------------------
msgtitl db ' RANDOM MAZE $'
msgm1   db '1   One player$'
msgm1a  db 'Three random mazes: 15x15, 24x24 and 36x36.$'
msgm1b  db 'Arrow keys or W A S D to move, H shows a hint.$'
msgm2   db '2   Two players (PvP)$'
msgm2a  db 'Both players get the same maze, side by side.$'
msgm2b  db 'Player 1: W A S D (left)   Player 2: arrow keys (right)$'
msgm2c  db 'Time limit 30 / 45 / 60 seconds. First to the exit wins;$'
msgm2d  db 'at time-out the player closer to the exit wins.$'
msgm3   db 'ESC Exit to DOS$'
msgm4   db 'Press 1 or 2$'
msglvl  db 'Level $'
msgof   db ' of 3$'
msggrid db 'Grid  $'
msgx    db ' x $'
msgmove db 'Moves  $'
msgdist db 'Dist   $'
msgdash db '----$'
msgkey1 db 'Arrows/WASD = Move$'
msgkey2 db 'H = Hint path$'
msgkey3 db 'ESC = Menu$'
msgyou  db ' You$'
msgexit db ' Exit$'
msghint db ' Hint$'
msgdone db ' LEVEL COMPLETE! $'
msgtakn db ' Your path  $'
msgopt  db ' Optimal    $'
msgboth db ' On both paths$'
msgmv   db ' moves$'
msgperf db 'You matched the optimal path!$'
msgnext db 'ENTER = next level   ESC = menu$'
msglast db 'ENTER = finish       ESC = menu$'
msgwin  db 'You finished all 3 levels!$'
msgany  db 'Press any key for the menu$'
msgp1   db ' PLAYER 1  W A S D $'
msgp2   db ' PLAYER 2  ARROW KEYS $'
msgtime db 'Time left  $'
msgsec  db ' s$'
msgmv2  db 'Moves $'
msgds2  db '  Dist $'
msgscr  db 'Score   P1  $'
msgscr2 db '  :  $'
msgscr3 db '  P2$'
msghelp db 'First to the exit wins. At time-out the closer player wins.  ESC = menu$'
msgw1   db ' PLAYER 1 WINS THE LEVEL $'
msgw2   db ' PLAYER 2 WINS THE LEVEL $'
msgdraw db ' DRAW $'
msgrx   db ' - reached the exit first$'
msgrt   db ' - time up, closer to the exit$'
msgrd   db ' - time up, both equally far from the exit$'
msglegd db ' optimal   $'
msglegt db ' own path   $'
msglegb db ' both    ENTER = continue  ESC = menu$'
msgfin  db 'FINAL SCORE$'
msgfp1  db 'P1  $'
msgfw1  db ' PLAYER 1 WINS THE MATCH! $'
msgfw2  db ' PLAYER 2 WINS THE MATCH! $'
msgfdr  db ' THE MATCH IS A DRAW $'

; ---- grids (only the first msize*msize entries are used) ---------------
grid    db MAXC dup(0)          ; 0 = wall, 1 = open
mark    db MAXC dup(0)          ; 1 = on the traced shortest path
trail   db 2 * MAXC dup(0)      ; per player: 1 = walked here
dstart  dw MAXC dup(0)          ; BFS distance from the start
dexit   dw MAXC dup(0)          ; BFS distance from the exit
queue   dw MAXC dup(0)          ; BFS queue of square indices
dstack  dw MAXC dup(0)          ; DFS stack of square indices

; ---------------------------------------------------------------------------
; code
; ---------------------------------------------------------------------------
.code

; ===========================================================================
; main - menu, level loop
; ===========================================================================
main proc
    mov ax,@data
    mov ds,ax
    mov ax,0B800h
    mov es,ax                   ; ES = video RAM for the whole program
    cld
    xor ah,ah
    int 1Ah                     ; CX:DX = clock ticks since midnight
    xor dx,cx
    or dx,1
    mov seed,dx                 ; a different maze every run

menu:
    call initscreen
    call drawmenu
mn_key:
    mov ah,0
    int 16h                     ; wait for a key
    cmp al,27
    je exit_dos
    cmp al,'1'
    je mn_one
    cmp al,'2'
    je mn_two
    jmp mn_key
mn_one:
    mov players,1
    jmp mn_play
mn_two:
    mov players,2
mn_play:
    mov level,0
    mov score[0],0
    mov score[1],0

next_level:
    call setup                  ; size and step table for this level
    call buildmaze              ; random maze with several routes
    call initlevel              ; players, trails, screen, timer
    cmp players,1
    jne nl_duel
    call solo
    jmp nl_played
nl_duel:
    call duel
nl_played:
    or al,al
    jnz menu                    ; ESC pressed during the level
    call showresult
    or al,al
    jnz menu                    ; ESC pressed on the result screen
    inc level
    cmp level,NLEV
    jb next_level
    call finalscreen
    jmp menu

exit_dos:
    mov ax,0003h                ; restore a normal text screen and cursor
    int 10h
    mov ax,4C00h
    int 21h                     ; return to DOS
main endp

; ===========================================================================
; initscreen - 80x25 colour text, no blinking (16 backgrounds), no cursor
; ===========================================================================
initscreen proc
    mov ax,0003h
    int 10h
    mov ax,1003h
    xor bl,bl
    int 10h
    mov ah,01h
    mov cx,2000h
    int 10h
    ret
initscreen endp

; ===========================================================================
; drawmenu - the mode selection screen
; ===========================================================================
drawmenu proc
    mov dh,2
    mov dl,33
    mov si,offset msgtitl
    mov ah,A_TITLE
    call vtext
    mov dh,6
    mov dl,12
    mov si,offset msgm1
    mov ah,A_P1
    call vtext
    mov dh,7
    mov dl,16
    mov si,offset msgm1a
    mov ah,A_DIM
    call vtext
    mov dh,8
    mov dl,16
    mov si,offset msgm1b
    call vtext
    mov dh,11
    mov dl,12
    mov si,offset msgm2
    mov ah,A_P2
    call vtext
    mov dh,12
    mov dl,16
    mov si,offset msgm2a
    mov ah,A_DIM
    call vtext
    mov dh,13
    mov dl,16
    mov si,offset msgm2b
    call vtext
    mov dh,14
    mov dl,16
    mov si,offset msgm2c
    call vtext
    mov dh,15
    mov dl,16
    mov si,offset msgm2d
    call vtext
    mov dh,18
    mov dl,12
    mov si,offset msgm3
    mov ah,A_MSG
    call vtext
    mov dh,21
    mov dl,34
    mov si,offset msgm4
    mov ah,A_HUD
    call vtext
    ret
drawmenu endp

; ===========================================================================
; setup - load the size and neighbour steps of the current level
; ===========================================================================
setup proc
    mov bl,level
    xor bh,bh
    mov al,sizes[bx]
    xor ah,ah
    mov msize,ax
    mul ax                      ; AX = side * side
    mov cells,ax
    shl bx,1
    mov ax,limits[bx]
    mov tlimit,ax
    mov ax,msize
    mov deltas[2],ax            ; down  = +side
    neg ax
    mov deltas[0],ax            ; up    = -side
    mov deltas[4],0FFFFh        ; left  = -1
    mov deltas[6],1             ; right = +1
    mov ax,msize
    inc ax
    mov spos,ax                 ; start = row 1, column 1
    mov ax,msize
    shr ax,1
    mov extra,ax                ; side/2 extra loops
    ret
setup endp

; ===========================================================================
; initlevel - reset players and trails, draw the level, start the clock
; ===========================================================================
initlevel proc
    mov ax,spos
    mov ppos[0],ax
    mov ppos[2],ax
    mov steps[0],0
    mov steps[2],0
    mov hinton,0
    mov mode,0
    mov who,0
    mov di,offset trail
    mov cx,2 * MAXC
    xor al,al
    call memfill                ; nothing walked yet ...
    mov bx,spos
    mov trail[bx],1             ; ... except the start square
    mov trail[bx+MAXC],1
    call cls
    call drawmazes
    cmp players,1
    jne il_duel
    call drawhud
    ret
il_duel:
    call drawhud2
    xor ah,ah
    int 1Ah
    mov tstart,dx               ; the PvP clock starts now
    mov lastsec,0FFFFh
    ret
initlevel endp

; ===========================================================================
; solo - 1 player input loop
;        OUT: AL = 0 level finished, AL = 1 ESC
; ===========================================================================
solo proc
so_key:
    mov ah,0
    int 16h                     ; AH = scan code, AL = character
    cmp al,27
    je so_esc
    mov who,0
    call keydir                 ; any movement key moves the player
    jc so_move
    or al,20h
    cmp al,'h'
    jne so_key
    xor hinton,1                ; toggle the hint
    cmp hinton,0
    je so_off
    call refreshhint
    jmp so_hud
so_off:
    call drawall
so_hud:
    call updhud
    jmp so_key
so_move:
    call trymove
    jnc so_key
    xor al,al
    ret
so_esc:
    mov al,1
    ret
solo endp

; ===========================================================================
; duel - 2 player input loop with time limit
;        OUT: AL = 0 level finished (winner, reason set), AL = 1 ESC
; ===========================================================================
duel proc
du_loop:
    call timeleft               ; AX = seconds left, CF = 1 time is up
    jc du_timeout
    cmp ax,lastsec
    je du_poll
    mov lastsec,ax
    call showtime
du_poll:
    mov ah,01h
    int 16h                     ; ZF = 0 when a key is waiting
    jnz du_key
    sti
    hlt                         ; sleep until the next interrupt
    jmp du_loop
du_key:
    mov ah,0
    int 16h
    cmp al,27
    je du_esc
    call keydir                 ; DL = player who owns the key
    jnc du_loop
    mov who,dl
    call trymove
    jnc du_loop
    mov al,who                  ; this player reached the exit first
    mov winner,al
    mov reason,0
    xor al,al
    ret
du_timeout:
    xor ax,ax
    call showtime
    mov reason,1
    mov bx,ppos[0]
    shl bx,1
    mov ax,dexit[bx]            ; player 1 distance to the exit
    mov bx,ppos[2]
    shl bx,1
    mov dx,dexit[bx]            ; player 2 distance to the exit
    mov winner,2
    cmp ax,dx
    je du_done
    mov winner,0
    jb du_done
    mov winner,1
du_done:
    xor al,al
    ret
du_esc:
    mov al,1
    ret
duel endp

; ===========================================================================
; keydir - is key AX a movement key?
;          OUT: CF = 1 yes, BX = offset into deltas[],
;               DL = 0 for W A S D (player 1), 1 for arrow keys (player 2)
; ===========================================================================
keydir proc
    cmp al,0
    je kd_arrow
    cmp al,0E0h
    je kd_arrow
    mov dl,al
    or dl,20h                   ; lower case
    mov bx,0
    cmp dl,'w'
    je kd_p1
    mov bx,2
    cmp dl,'s'
    je kd_p1
    mov bx,4
    cmp dl,'a'
    je kd_p1
    mov bx,6
    cmp dl,'d'
    je kd_p1
    clc
    ret
kd_p1:
    mov dl,0
    stc
    ret
kd_arrow:
    mov bx,0
    cmp ah,48h
    je kd_p2
    mov bx,2
    cmp ah,50h
    je kd_p2
    mov bx,4
    cmp ah,4Bh
    je kd_p2
    mov bx,6
    cmp ah,4Dh
    je kd_p2
    clc
    ret
kd_p2:
    mov dl,1
    stc
    ret
keydir endp

; ===========================================================================
; trymove - move player 'who' one square in direction BX
;           OUT: CF = 1 when the player has reached the exit
; ===========================================================================
trymove proc
    mov al,who
    xor ah,ah
    shl ax,1
    mov di,ax                   ; DI = who * 2
    mov ax,ppos[di]
    add ax,deltas[bx]           ; AX = square we want to step into
    mov si,ax
    cmp grid[si],1
    jne tm_no                   ; wall: ignore the key
    mov dx,ppos[di]             ; DX = square being left
    mov ppos[di],ax
    inc steps[di]
    call tbase
    mov trail[bx+si],1          ; remember the walked path
    mov ax,dx
    call drawcell               ; repaint the old square
    mov ax,si
    call drawcell               ; draw the player on the new one
    cmp hinton,0
    je tm_hud
    call refreshhint            ; the shortest path starts somewhere new
tm_hud:
    push si
    call updstats
    pop si
    cmp si,epos
    jne tm_no
    stc
    ret
tm_no:
    clc
    ret
trymove endp

; ===========================================================================
; tbase - BX = offset of player 'who' inside trail[]
; ===========================================================================
tbase proc
    xor bx,bx
    cmp who,0
    je tb_done
    mov bx,MAXC
tb_done:
    ret
tbase endp

; ===========================================================================
; timeleft - OUT: AX = whole seconds left (rounded up), CF = 1 time is up
; ===========================================================================
timeleft proc
    push bx
    push cx
    push dx
    xor ah,ah
    int 1Ah                     ; DX = low word of the tick count
    mov ax,dx
    sub ax,tstart               ; AX = ticks since the level started
    cmp ax,tlimit
    jae tl_over
    mov bx,tlimit
    sub bx,ax                   ; BX = ticks left
    mov ax,bx
    mov cx,10
    mul cx
    add ax,181
    adc dx,0
    mov cx,182
    div cx                      ; AX = ceil(ticks * 10 / 182) seconds
    clc
    jmp tl_out
tl_over:
    xor ax,ax
    stc
tl_out:
    pop dx
    pop cx
    pop bx
    ret
timeleft endp

; ===========================================================================
; buildmaze - generate a maze that has more than one way to the exit
;   1. randomised DFS carves a perfect maze (exactly one route)
;   2. the exit is put on the open square closest to the bottom-right
;   3. a wall between two squares of that route is opened -> a second route
;      (if the route has no such wall the maze is generated again)
;   4. a few more random walls are opened for extra loops
;   5. dexit[] = distance of every square to the exit (used by the hint,
;      by the PvP time-out rule and by the optimal path at the end)
; ===========================================================================
buildmaze proc
    mov tries,0
bm_gen:
    call genmaze
    call placeexit
    mov ax,spos
    mov di,offset dstart
    call bfs
    mov ax,epos
    mov di,offset dexit
    call bfs
    mov bx,spos
    shl bx,1
    mov ax,dexit[bx]
    mov pathlen,ax
    mov strict,1
    call countwalls             ; CX = walls that would make a shortcut
    or cx,cx
    jnz bm_short
    inc tries
    cmp tries,50
    jb bm_gen
    jmp bm_loops
bm_short:
    call openrandom             ; guaranteed second route to the exit
bm_loops:
    mov strict,0
    mov cx,extra
bm_more:
    push cx
    call countwalls             ; CX = walls between two open squares
    or cx,cx
    jz bm_none
    call openrandom
    pop cx
    loop bm_more
    jmp bm_bfs
bm_none:
    pop cx
bm_bfs:
    mov ax,epos
    mov di,offset dexit
    call bfs
    ret
buildmaze endp

; ===========================================================================
; genmaze - randomised depth-first search
;           a square may be carved when it is inside the border, still a
;           wall, touches only the square we come from, and the two squares
;           diagonally in front of it are walls (keeps corridors 1 wide)
; ===========================================================================
genmaze proc
    mov di,offset grid
    mov cx,cells
    xor al,al
    call memfill                ; everything is wall
    mov ax,spos
    mov bx,ax
    mov grid[bx],1
    mov dstack,ax
    mov dtop,1
gm_loop:
    mov si,dtop
    or si,si
    jz gm_done
    dec si
    shl si,1
    mov ax,dstack[si]           ; AX = square on top of the stack
    xor cx,cx                   ; CX = number of candidates
    xor bx,bx                   ; BX = direction offset 0, 2, 4, 6
gm_dir:
    mov dx,deltas[bx]
    mov dirv,dx
    add dx,ax                   ; DX = neighbour in this direction
    call carvable
    jnc gm_next
    mov di,cx
    shl di,1
    mov cand[di],dx
    inc cx
gm_next:
    add bx,2
    cmp bx,8
    jb gm_dir
    or cx,cx
    jz gm_pop                   ; dead end: backtrack
    mov ax,cx
    call random                 ; AX = 0 .. CX-1
    mov di,ax
    shl di,1
    mov dx,cand[di]
    mov bx,dx
    mov grid[bx],1              ; carve it ...
    mov si,dtop
    shl si,1
    mov dstack[si],dx           ; ... and continue from there
    inc dtop
    jmp gm_loop
gm_pop:
    dec dtop
    jmp gm_loop
gm_done:
    ret
genmaze endp

; ===========================================================================
; carvable - may square DX be carved when moving in direction dirv?
;            OUT: CF = 1 yes, CF = 0 no      preserves all registers
; ===========================================================================
carvable proc
    push ax
    push bx
    push cx
    push dx
    push si
    mov bx,dx                   ; BX = candidate square
    mov ax,dx
    xor dx,dx
    div msize                   ; AX = row, DX = column
    mov cx,msize
    dec cx                      ; CX = index of the far border
    cmp ax,1
    jb cv_no
    cmp ax,cx
    jae cv_no
    cmp dx,1
    jb cv_no
    cmp dx,cx
    jae cv_no
    cmp grid[bx],0
    jne cv_no
    xor al,al                   ; count open orthogonal neighbours
    mov si,bx
    sub si,msize
    add al,grid[si]
    mov si,bx
    add si,msize
    add al,grid[si]
    add al,grid[bx-1]
    add al,grid[bx+1]
    cmp al,1
    jne cv_no
    mov cx,1                    ; CX = step across the moving direction
    mov dx,dirv
    cmp dx,1
    je cv_side
    cmp dx,0FFFFh
    jne cv_diag
cv_side:
    mov cx,msize
cv_diag:
    mov si,bx
    add si,dirv                 ; SI = square straight ahead
    add si,cx
    cmp grid[si],0
    jne cv_no
    sub si,cx
    sub si,cx
    cmp grid[si],0
    jne cv_no
    stc
    jmp cv_out
cv_no:
    clc
cv_out:
    pop si
    pop dx
    pop cx
    pop bx
    pop ax
    ret
carvable endp

; ===========================================================================
; placeexit - exit = open square with the largest row + column
; ===========================================================================
placeexit proc
    mov ax,spos
    mov epos,ax
    mov best,0
    xor bx,bx
pe_loop:
    cmp bx,cells
    jae pe_done
    cmp grid[bx],1
    jne pe_next
    mov ax,bx
    xor dx,dx
    div msize
    add ax,dx                   ; AX = row + column
    cmp ax,best
    jbe pe_next
    mov best,ax
    mov epos,bx
pe_next:
    inc bx
    jmp pe_loop
pe_done:
    ret
placeexit endp

; ===========================================================================
; bfs - breadth first search over open squares
;       IN : AX = source square, DI = offset of a word distance array
;       OUT: [DI + 2*i] = steps from the source, FFFFh = unreachable
; ===========================================================================
bfs proc
    mov bx,di
    mov cx,cells
bf_fill:
    mov word ptr [bx],0FFFFh
    add bx,2
    loop bf_fill
    mov bx,ax
    shl bx,1
    mov word ptr [di+bx],0      ; distance(source) = 0
    mov queue,ax
    mov qhead,0
    mov qtail,1
bf_loop:
    mov si,qhead
    cmp si,qtail
    jae bf_done                 ; queue empty
    shl si,1
    mov ax,queue[si]            ; AX = square being expanded
    inc qhead
    mov bx,ax
    shl bx,1
    mov dx,[di+bx]
    inc dx                      ; DX = distance for its neighbours
    xor cx,cx
bf_dir:
    mov bx,cx
    mov si,ax
    add si,deltas[bx]           ; SI = neighbour
    cmp grid[si],1
    jne bf_next
    mov bx,si
    shl bx,1
    cmp word ptr [di+bx],0FFFFh
    jne bf_next                 ; already reached
    mov [di+bx],dx
    mov bx,qtail
    shl bx,1
    mov queue[bx],si
    inc qtail
bf_next:
    add cx,2
    cmp cx,8
    jb bf_dir
    jmp bf_loop
bf_done:
    ret
bfs endp

; ===========================================================================
; countwalls - CX = number of walls accepted by checkwall
; openrandom - open a random one of the CX walls accepted by checkwall
; ===========================================================================
countwalls proc
    xor cx,cx
    xor bx,bx
cw_loop:
    cmp bx,cells
    jae cw_done
    call checkwall
    jnc cw_next
    inc cx
cw_next:
    inc bx
    jmp cw_loop
cw_done:
    ret
countwalls endp

openrandom proc
    mov ax,cx
    call random
    mov pick,ax                 ; open the pick-th accepted wall
    xor bx,bx
or_loop:
    cmp bx,cells
    jae or_done
    call checkwall
    jnc or_next
    cmp pick,0
    je or_open
    dec pick
or_next:
    inc bx
    jmp or_loop
or_open:
    mov grid[bx],1
or_done:
    ret
openrandom endp

; ===========================================================================
; checkwall - may wall square BX be opened?
;   it must be an inner wall with open squares on two opposite sides and
;   walls on the other two sides. With strict = 1 both open squares must
;   lie on the start -> exit route at least 4 steps apart, so opening the
;   wall creates a second, shorter route.
;   OUT: CF = 1 yes, CF = 0 no      preserves all registers
; ===========================================================================
checkwall proc
    push ax
    push dx
    push si
    push di
    cmp grid[bx],0
    jne ck_no
    mov ax,bx
    xor dx,dx
    div msize                   ; AX = row, DX = column
    mov si,msize
    dec si
    cmp ax,1
    jb ck_no
    cmp ax,si
    jae ck_no
    cmp dx,1
    jb ck_no
    cmp dx,si
    jae ck_no
    lea si,[bx-1]               ; left / right pair
    lea di,[bx+1]
    mov ax,msize
    call pairok
    jc ck_yes
    mov si,bx                   ; up / down pair
    sub si,msize
    mov di,bx
    add di,msize
    mov ax,1
    call pairok
    jc ck_yes
ck_no:
    clc
    jmp ck_out
ck_yes:
    stc
ck_out:
    pop di
    pop si
    pop dx
    pop ax
    ret
checkwall endp

; ===========================================================================
; pairok - IN: SI, DI = squares on two opposite sides of wall BX,
;              AX = step to the two other sides       OUT: CF
; ===========================================================================
pairok proc
    push cx
    push dx
    push si
    cmp grid[si],1
    jne pk_no
    cmp grid[di],1
    jne pk_no
    mov si,bx
    sub si,ax
    cmp grid[si],0
    jne pk_no
    mov si,bx
    add si,ax
    cmp grid[si],0
    jne pk_no
    pop si
    push si
    cmp strict,0
    je pk_yes
    mov cx,si
    call onpath
    jnc pk_no
    mov cx,di
    call onpath
    jnc pk_no
    mov cx,si
    call getdexit
    push dx
    mov cx,di
    call getdexit
    pop cx                      ; CX = dist(SI), DX = dist(DI)
    sub cx,dx
    jns pk_pos
    neg cx
pk_pos:
    cmp cx,4
    jb pk_no
pk_yes:
    stc
    jmp pk_out
pk_no:
    clc
pk_out:
    pop si
    pop dx
    pop cx
    ret
pairok endp

; ===========================================================================
; onpath   - CF = 1 when square CX lies on a shortest start -> exit route
; getdexit - DX = dexit[CX]          getdstart - DX = dstart[CX]
; ===========================================================================
onpath proc
    push ax
    push dx
    call getdstart
    mov ax,dx
    call getdexit
    add ax,dx
    cmp ax,pathlen
    jne op_no
    stc
    jmp op_out
op_no:
    clc
op_out:
    pop dx
    pop ax
    ret
onpath endp

getdexit proc
    push bx
    mov bx,cx
    shl bx,1
    mov dx,dexit[bx]
    pop bx
    ret
getdexit endp

getdstart proc
    push bx
    mov bx,cx
    shl bx,1
    mov dx,dstart[bx]
    pop bx
    ret
getdstart endp

; ===========================================================================
; tracepath - mark[] := one shortest path from square AX to the exit
;             (always step to a neighbour that is one closer to the exit)
; ===========================================================================
tracepath proc
    push ax
    push bx
    push cx
    push dx
    push si
    push di
    push ax
    mov di,offset mark
    mov cx,cells
    xor al,al
    call memfill
    pop ax
tp_loop:
    mov bx,ax
    mov mark[bx],1
    shl bx,1
    mov dx,dexit[bx]
    or dx,dx
    jz tp_done                  ; reached the exit
    cmp dx,0FFFFh
    je tp_done                  ; unreachable (cannot happen)
    dec dx                      ; DX = distance the next square must have
    xor cx,cx
tp_dir:
    mov bx,cx
    mov si,ax
    add si,deltas[bx]
    cmp grid[si],1
    jne tp_next
    mov bx,si
    shl bx,1
    cmp dexit[bx],dx
    jne tp_next
    mov ax,si
    jmp tp_loop
tp_next:
    add cx,2
    cmp cx,8
    jb tp_dir
tp_done:
    pop di
    pop si
    pop dx
    pop cx
    pop bx
    pop ax
    ret
tracepath endp

refreshhint proc
    mov ax,ppos[0]
    call tracepath
    call drawall
    ret
refreshhint endp

; ===========================================================================
; random - IN: AX = n      OUT: AX = random number 0 .. n-1
;          16-bit linear congruential generator, high part of seed * n
; ===========================================================================
random proc
    push bx
    push dx
    mov bx,ax
    mov ax,seed
    mov dx,25173
    mul dx
    add ax,13849
    mov seed,ax
    mul bx                      ; DX = seed * n / 65536
    mov ax,dx
    pop dx
    pop bx
    ret
random endp

; ===========================================================================
; memfill - fill CX bytes at DS:DI with AL
; ===========================================================================
memfill proc
    push es
    push ds
    pop es
    rep stosb
    pop es
    ret
memfill endp

; ===========================================================================
; cls - blank the whole text screen (grey on black)
; ===========================================================================
cls proc
    push ax
    push cx
    push di
    xor di,di
    mov cx,2000
    mov ax,0720h
    rep stosw
    pop di
    pop cx
    pop ax
    ret
cls endp

; ===========================================================================
; setpix - set one half-block pixel
;          IN: DL = x (0-79), DH = y (0-49), AL = colour     preserves all
;          even y = upper half (foreground nibble)
;          odd  y = lower half (background nibble)
; ===========================================================================
setpix proc
    push ax
    push bx
    push cx
    mov cl,al                   ; CL = colour
    mov al,dh
    shr al,1                    ; text row = y / 2
    mov bl,160
    mul bl
    mov bx,ax
    mov al,dl
    xor ah,ah
    shl ax,1
    add bx,ax                   ; BX = offset of the text cell
    mov byte ptr es:[bx],0DFh   ; upper half block
    mov al,es:[bx+1]
    test dh,1
    jnz sp_lower
    and al,0F0h
    or al,cl
    jmp sp_store
sp_lower:
    and al,0Fh
    shl cl,1
    shl cl,1
    shl cl,1
    shl cl,1
    or al,cl
sp_store:
    mov es:[bx+1],al
    pop cx
    pop bx
    pop ax
    ret
setpix endp

; ===========================================================================
; cellcolour - BL = colour of square SI in the maze of player 'who'
; ===========================================================================
cellcolour proc
    push ax
    push di
    mov al,who
    xor ah,ah
    shl ax,1
    mov di,ax                   ; DI = who * 2
    cmp mode,0
    jne cc_result
    cmp si,ppos[di]
    je cc_player
    cmp si,epos
    je cc_exit
    cmp grid[si],0
    je cc_wall
    cmp hinton,0
    je cc_floor
    test mark[si],1
    jz cc_floor
    mov bl,C_HINT
    jmp cc_out
cc_result:
    cmp si,epos
    je cc_exit
    cmp grid[si],0
    je cc_wall
    cmp si,ppos[di]
    je cc_player                ; where the player stopped (time-out)
    xor bx,bx
    cmp who,0
    je cc_trail
    mov bx,MAXC
cc_trail:
    mov al,trail[bx+si]         ; this player's trail
    mov bl,mark[si]             ; bit 0 = on the optimal path
    cmp al,0
    je cc_kind
    or bl,2                     ; bit 1 = walked by this player
cc_kind:
    cmp bl,3
    je cc_both
    cmp bl,1
    je cc_opt
    cmp bl,2
    je cc_taken
cc_floor:
    mov bl,C_FLOOR
    jmp cc_out
cc_player:
    mov bl,C_PLAY
    cmp who,0
    je cc_out
    mov bl,C_PLAY2
    jmp cc_out
cc_exit:
    mov bl,C_EXIT
    jmp cc_out
cc_wall:
    mov bl,C_WALL
    jmp cc_out
cc_both:
    mov bl,C_BOTH
    jmp cc_out
cc_opt:
    mov bl,C_OPT
    jmp cc_out
cc_taken:
    mov bl,C_TAKEN
cc_out:
    pop di
    pop ax
    ret
cellcolour endp

; ===========================================================================
; drawcell  - paint square AX as one pixel in the maze of player 'who'
; drawall   - paint every square of that maze
; drawmazes - paint the maze of every player
; ===========================================================================
drawcell proc
    push ax
    push bx
    push dx
    push si
    mov si,ax
    call cellcolour             ; BL = colour
    xor dx,dx
    div msize                   ; AX = row, DX = column
    mov dh,al
    add dh,ORGY                 ; DH = pixel row
    push bx
    mov bl,who
    xor bh,bh
    add dl,orgx[bx]             ; DL = pixel column
    pop bx
    mov al,bl
    call setpix
    pop si
    pop dx
    pop bx
    pop ax
    ret
drawcell endp

drawall proc
    push ax
    xor ax,ax
da_loop:
    cmp ax,cells
    jae da_done
    call drawcell
    inc ax
    jmp da_loop
da_done:
    pop ax
    ret
drawall endp

drawmazes proc
    mov who,0
    call drawall
    cmp players,2
    jne dz_done
    mov who,1
    call drawall
    mov who,0
dz_done:
    ret
drawmazes endp

; ===========================================================================
; text output straight into video RAM
;   vaddr - BX = video offset of row DH, column DL
;   vtext - '$' string at SI, attribute AH, at DH/DL (DL advances)
;   vchar - character AL, attribute AH, at DH/DL (DL advances)
;   vnum2 - AL (0-99) as two digits, attribute AH
;   vnum4 - AX (0-9999) as four digits, attribute BL
; ===========================================================================
vaddr proc
    push ax
    mov al,160
    mul dh
    mov bx,ax
    mov al,dl
    xor ah,ah
    shl ax,1
    add bx,ax
    pop ax
    ret
vaddr endp

vtext proc
    push bx
    push si
    call vaddr
vt_loop:
    mov al,[si]
    cmp al,'$'
    je vt_done
    mov es:[bx],al
    mov es:[bx+1],ah
    add bx,2
    inc si
    inc dl
    jmp vt_loop
vt_done:
    pop si
    pop bx
    ret
vtext endp

vchar proc
    push bx
    call vaddr
    mov es:[bx],al
    mov es:[bx+1],ah
    inc dl
    pop bx
    ret
vchar endp

vnum2 proc
    push ax
    push bx
    mov bl,ah                   ; attribute
    xor ah,ah
    aam                         ; AH = tens, AL = units
    add ax,3030h
    push ax
    mov al,ah
    mov ah,bl
    call vchar
    pop ax
    mov ah,bl
    call vchar
    pop bx
    pop ax
    ret
vnum2 endp

vnum4 proc
    push ax
    push cx
    push si
    push di
    push dx
    cmp ax,9999
    jbe vn_ok
    mov ax,9999
vn_ok:
    mov si,offset numbuf+3
    mov di,10
    mov cx,4
vn_digit:
    xor dx,dx
    div di
    add dl,'0'
    mov [si],dl
    dec si
    loop vn_digit
    pop dx
    mov si,offset numbuf
    mov ah,bl
    call vtext
    pop di
    pop si
    pop cx
    pop ax
    ret
vnum4 endp

; ===========================================================================
; levelinfo - 'Level n of 3' and 'Grid  nn x nn' at DH/DL and DH+1/DL
; ===========================================================================
levelinfo proc
    push dx
    mov si,offset msglvl
    mov ah,A_HUD
    call vtext
    mov al,level
    add al,'1'
    call vchar
    mov si,offset msgof
    call vtext
    pop dx
    inc dh
    mov si,offset msggrid
    call vtext
    mov al,byte ptr msize
    call vnum2
    mov si,offset msgx
    call vtext
    mov al,byte ptr msize
    call vnum2
    ret
levelinfo endp

; ===========================================================================
; drawhud - 1 player title line and sidebar
; updhud  - refresh the move counter and the hint distance
; updstats- refresh the numbers of player 'who' (either mode)
; ===========================================================================
drawhud proc
    mov dh,0
    mov dl,LEFTX
    mov si,offset msgtitl
    mov ah,A_TITLE
    call vtext
    mov dh,2
    mov dl,HUDC
    call levelinfo

    mov dh,5
    mov dl,HUDC
    mov si,offset msgmove
    call vtext
    mov dh,6
    mov dl,HUDC
    mov si,offset msgdist
    call vtext

    mov dh,8
    mov dl,HUDC
    mov si,offset msgkey1
    call vtext
    mov dh,9
    mov dl,HUDC
    mov si,offset msgkey2
    call vtext
    mov dh,10
    mov dl,HUDC
    mov si,offset msgkey3
    call vtext

    mov dh,12
    mov dl,HUDC
    mov al,0DBh                 ; full block as a colour swatch
    mov ah,C_PLAY
    call vchar
    mov si,offset msgyou
    mov ah,A_HUD
    call vtext
    mov dh,13
    mov dl,HUDC
    mov al,0DBh
    mov ah,C_EXIT
    call vchar
    mov si,offset msgexit
    mov ah,A_HUD
    call vtext
    mov dh,14
    mov dl,HUDC
    mov al,0DBh
    mov ah,C_HINT
    call vchar
    mov si,offset msghint
    mov ah,A_HUD
    call vtext
    call updhud
    ret
drawhud endp

updhud proc
    mov dh,5
    mov dl,HUDC+7
    mov ax,steps[0]
    mov bl,A_HUD
    call vnum4
    mov dh,6
    mov dl,HUDC+7
    cmp hinton,0
    je uh_dash
    mov bx,ppos[0]
    shl bx,1
    mov ax,dexit[bx]            ; squares left to the exit
    mov bl,A_HUD
    call vnum4
    ret
uh_dash:
    mov si,offset msgdash
    mov ah,A_HUD
    call vtext
    ret
updhud endp

updstats proc
    cmp players,1
    jne us_duel
    call updhud
    ret
us_duel:
    push ax
    push bx
    push dx
    push si
    push di
    mov bl,who
    xor bh,bh
    mov dl,orgx[bx]             ; under this player's maze
    shl bx,1
    mov di,bx                   ; DI = who * 2
    mov dh,21
    mov si,offset msgmv2
    mov ah,A_HUD
    call vtext
    mov ax,steps[di]
    mov bl,A_MSG
    call vnum4
    mov si,offset msgds2
    mov ah,A_HUD
    call vtext
    mov bx,ppos[di]
    shl bx,1
    mov ax,dexit[bx]            ; distance used by the time-out rule
    mov bl,A_MSG
    call vnum4
    pop di
    pop si
    pop dx
    pop bx
    pop ax
    ret
updstats endp

; ===========================================================================
; drawhud2 - PvP labels and bottom panel
;   row 0  : player names over each maze
;   row 20 : level, grid size, time left
;   row 21 : moves and distance of each player
;   row 22 : match score
;   row 23 : rules / level winner
; ===========================================================================
drawhud2 proc
    mov dh,0
    mov dl,LEFTX
    mov si,offset msgp1
    mov ah,A_P1
    call vtext
    mov dh,0
    mov dl,RIGHTX
    mov si,offset msgp2
    mov ah,A_P2
    call vtext
    mov dh,19
    mov dl,LEFTX
    call levelinfo
    mov who,0
    call updstats
    mov who,1
    call updstats
    mov who,0
    call showscore
    mov dh,23
    mov dl,LEFTX
    mov si,offset msghelp
    mov ah,A_DIM
    call vtext
    ret
drawhud2 endp

showtime proc                   ; IN: AX = seconds left
    push ax
    push dx
    push si
    push ax
    mov dh,19
    mov dl,RIGHTX
    mov si,offset msgtime
    mov ah,A_HUD
    call vtext
    pop ax
    mov ah,A_WIN
    cmp al,10
    jae st_num
    mov ah,A_LOSE               ; last 10 seconds in red
st_num:
    call vnum2
    mov si,offset msgsec
    mov ah,A_HUD
    call vtext
    pop si
    pop dx
    pop ax
    ret
showtime endp

showscore proc
    mov dh,22
    mov dl,LEFTX
    mov si,offset msgscr
    mov ah,A_MSG
    call vtext
    mov al,score[0]
    mov ah,A_P1
    call vnum2
    mov si,offset msgscr2
    mov ah,A_MSG
    call vtext
    mov al,score[1]
    mov ah,A_P2
    call vnum2
    mov si,offset msgscr3
    mov ah,A_MSG
    call vtext
    ret
showscore endp

; ===========================================================================
; showresult - draw the level result and wait for ENTER or ESC
;              OUT: AL = 0 continue, AL = 1 ESC
; ===========================================================================
showresult proc
    mov ax,spos
    call tracepath              ; mark[] = a shortest start -> exit path
    mov mode,1
    call drawmazes
    cmp players,1
    jne sr_duel
    call result1
    jmp sr_wait
sr_duel:
    call result2
sr_wait:
    mov ah,0
    int 16h
    cmp al,27
    je sr_esc
    cmp al,13
    jne sr_wait
    xor al,al
    ret
sr_esc:
    mov al,1
    ret
showresult endp

; ===========================================================================
; result1 - 1 player sidebar after a level: legend and move counts
; ===========================================================================
result1 proc
    mov dh,16
    mov dl,HUDC
    mov si,offset msgdone
    mov ah,A_WIN
    call vtext

    mov dh,18
    mov dl,HUDC
    mov al,0DBh
    mov ah,C_TAKEN
    call vchar
    mov si,offset msgtakn
    mov ah,A_MSG
    call vtext
    mov ax,steps[0]
    mov bl,A_MSG
    call vnum4
    mov si,offset msgmv
    mov ah,A_MSG
    call vtext

    mov dh,19
    mov dl,HUDC
    mov al,0DBh
    mov ah,C_OPT
    call vchar
    mov si,offset msgopt
    mov ah,A_MSG
    call vtext
    mov bx,spos
    shl bx,1
    mov ax,dexit[bx]            ; length of the optimal path
    mov bl,A_MSG
    call vnum4
    mov si,offset msgmv
    mov ah,A_MSG
    call vtext

    mov dh,20
    mov dl,HUDC
    mov al,0DBh
    mov ah,C_BOTH
    call vchar
    mov si,offset msgboth
    mov ah,A_MSG
    call vtext

    mov bx,spos
    shl bx,1
    mov ax,dexit[bx]
    cmp ax,steps[0]
    jne r1_keys
    mov dh,22
    mov dl,HUDC
    mov si,offset msgperf
    mov ah,A_WIN
    call vtext
r1_keys:
    mov dh,23
    mov dl,HUDC
    call keyline
    ret
result1 endp

keyline proc                    ; 'ENTER = next level' or 'ENTER = finish'
    mov si,offset msgnext
    cmp level,NLEV-1
    jb kl_show
    mov si,offset msglast
kl_show:
    mov ah,A_HUD
    call vtext
    ret
keyline endp

; ===========================================================================
; result2 - PvP: score the level, show the winner, reason and legend
; ===========================================================================
result2 proc
    mov dh,23                   ; clear the rules line
    call clrrow
    mov dh,24
    call clrrow
    mov dh,23
    mov dl,LEFTX
    mov si,offset msgdraw
    mov ah,A_TITLE
    mov bl,winner
    cmp bl,2
    je r2_name
    xor bh,bh
    inc score[bx]               ; the level winner scores a point
    mov si,offset msgw1
    mov ah,A_WIN
    or bl,bl
    jz r2_name
    mov si,offset msgw2
r2_name:
    call vtext
    mov si,offset msgrx         ; why: exit / closer / equal
    cmp reason,0
    je r2_why
    mov si,offset msgrt
    cmp winner,2
    jne r2_why
    mov si,offset msgrd
r2_why:
    mov ah,A_MSG
    call vtext
    call showscore

    mov dh,24
    mov dl,LEFTX
    mov al,0DBh
    mov ah,C_OPT
    call vchar
    mov si,offset msglegd
    mov ah,A_MSG
    call vtext
    mov al,0DBh
    mov ah,C_TAKEN
    call vchar
    mov si,offset msglegt
    mov ah,A_MSG
    call vtext
    mov al,0DBh
    mov ah,C_BOTH
    call vchar
    mov si,offset msglegb
    mov ah,A_MSG
    call vtext
    ret
result2 endp

clrrow proc                     ; blank text row DH
    push ax
    push cx
    push di
    mov al,160
    mul dh
    mov di,ax
    mov cx,80
    mov ax,0720h
    rep stosw
    pop di
    pop cx
    pop ax
    ret
clrrow endp

; ===========================================================================
; finalscreen - end of the 3 levels, wait for a key
; ===========================================================================
finalscreen proc
    call cls
    cmp players,1
    jne fs_duel
    mov dh,11
    mov dl,27
    mov si,offset msgwin
    mov ah,A_WIN
    call vtext
    jmp fs_wait
fs_duel:
    mov dh,9
    mov dl,34
    mov si,offset msgfin
    mov ah,A_MSG
    call vtext
    mov dh,11                   ; 'P1  nn  :  nn  P2'
    mov dl,32
    mov si,offset msgfp1
    mov ah,A_P1
    call vtext
    mov al,score[0]
    call vnum2
    mov si,offset msgscr2
    mov ah,A_MSG
    call vtext
    mov al,score[1]
    mov ah,A_P2
    call vnum2
    mov si,offset msgscr3
    call vtext
    mov dh,13
    mov dl,27
    mov si,offset msgfdr
    mov ah,A_TITLE
    mov al,score[0]
    cmp al,score[1]
    je fs_show
    mov si,offset msgfw1
    mov ah,A_WIN
    ja fs_show
    mov si,offset msgfw2
fs_show:
    call vtext
fs_wait:
    mov dh,15
    mov dl,27
    mov si,offset msgany
    mov ah,A_DIM
    call vtext
    mov ah,0
    int 16h
    ret
finalscreen endp

end main
