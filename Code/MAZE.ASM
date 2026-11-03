; ---------------------------------------------------------------------------
; MAZE.ASM - 16-bit DOS random maze game: you vs the computer, or 2 players
;
;   * 3 levels: 15x15, 24x24 and 36x36 grids, generated at run time
;   * randomised depth-first search carves a perfect maze, then extra walls
;     are knocked out so there is ALWAYS more than one route to the exit
;   * the same maze is shown twice, side by side, and both sides race:
;       1 player : you (left, arrows or W A S D) against an A* bot (right)
;       2 players: player 1 = left maze, W A S D
;                  player 2 = right maze, arrow keys
;   * time limit 30 / 45 / 60 seconds; first to the exit wins,
;     at time-out the one closer to the exit wins
;   * the bot is fair: it only knows the border, the exit position and the
;     squares it has seen (2 / 3 / 4 squares around it by level). It plans with
;     A* treating unseen squares as open, replans when it sees a new wall,
;     and moves at a fixed speed. The screen shows its whole maze to you.
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
LEFTX   equ 2                   ; x of the left maze (player 1 / you)
RIGHTX  equ 42                  ; x of the right maze (player 2 / computer)

; pixel colours (0-15; 16 background colours because blinking is turned off)
C_FLOOR equ 0                   ; black
C_WALL  equ 1                   ; blue
C_HINT  equ 3                   ; cyan          = hint path
C_BOTH  equ 6                   ; orange/brown = optimal AND walked
C_OLD   equ 8                   ; dark grey     = walked in an earlier try
C_PLAY  equ 10                  ; bright green  = player 1
C_PLAY2 equ 11                  ; bright cyan   = player 2 / computer
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
players db 1                    ; 1 = you vs computer, 2 = PvP, 3 = study
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
mode    db 0                    ; 0 = playing, 1 = showing the level result
hinton  db 0                    ; 1 = your hint path is on screen
strict  db 0                    ; 1 = checkwall only accepts shortcuts
tries   db 0                    ; maze generation attempts
seed    dw 0                    ; random number generator state
dirv    dw 0                    ; direction being tried by the generator
pick    dw 0                    ; scratch: which candidate wall to open
qhead   dw 0                    ; BFS queue read index
qtail   dw 0                    ; BFS queue write index
dtop    dw 0                    ; DFS stack depth
tstart  dw 0                    ; clock tick when the level started
tlimit  dw 0                    ; ticks allowed for this level
lastsec dw 0                    ; seconds shown on screen

; ---- computer player ---------------------------------------------------
botlast dw 0                    ; clock tick of the bot's last step
botdly  dw 0                    ; ticks between two bot steps
botsee  dw 0                    ; how far the bot can see (squares)
bplen   dw 0                    ; squares left on the bot's planned path
erow    dw 0                    ; exit row    (A* heuristic target)
ecol    dw 0                    ; exit column
newwall db 0                    ; 1 = botlook discovered a new wall
rmin    dw 0                    ; botlook: rows and columns in sight
rmax    dw 0
cmin    dw 0
cmax    dw 0
curf    dw 0                    ; A*: f value of the current bucket
curp    dw 0                    ; A*: offset of the current bucket
curc    dw 0                    ;     number of squares in it
nxtp    dw 0                    ; A*: offset of the f+2 bucket
nxtc    dw 0                    ;     number of squares in it

deltas  dw 4 dup(0)             ; index steps: up, down, left, right
cand    dw 4 dup(0)             ; DFS: neighbours that may be carved

sizes   db 15, 24, 36           ; grid side per level
limits  dw 546, 819, 1092       ; 30, 45, 60 seconds at 18.2 ticks/second
botspd  db 6, 5, 4              ; bot: clock ticks per step (3, 3.6, 4.5 /s)
botview db 2, 3, 4              ; bot: sight radius per level

; ---- cognitive study (mode 3) ------------------------------------------
MAXTRY  equ 99                  ; safety limit on tries per stage
stview  db 1, 1, 1              ; bot sight radius per stage in the study
fast    db 0                    ; 1 = no delay between bot steps
trynum  dw 0                    ; try being run on this stage (1, 2, ...)
optlen  dw 0                    ; true optimal length (the bot never reads it)
runner  db 0                    ; who is learning: 0 = the bot, 1 = you
histp   dw 0                    ; offset of the runner's try history
hist    dw MAXTRY dup(0)        ; length of every bot try on this stage
hhist   dw MAXTRY dup(0)        ; length of every one of your tries
srtry   db 3 dup(0)             ; bot tries needed on each stage
srhum   db 3 dup(0)             ; your tries needed on each stage
srwin   db 3 dup(0)             ; who learned faster: 0 bot, 1 you, 2 tie
sropt   dw 3 dup(0)             ; optimal length of each stage

numbuf  db '0000$'              ; vnum4 output buffer

; ---- text ('$' terminated) ---------------------------------------------
msgtitl db ' RANDOM MAZE $'
msgm1   db '1   One player vs the computer$'
msgm1a  db 'Race an A* bot through the same maze. You: arrows or W A S D.$'
msgm1b  db 'The bot only sees 2 / 3 / 4 squares around it (by level) and$'
msgm1c  db 'replans every time it finds a wall. You see its whole maze.$'
msgm2   db '2   Two players (PvP) - not available right now$'
msgm2a  db 'Both players get the same maze, side by side.$'
msgm2b  db 'Player 1: W A S D (left)   Player 2: arrow keys (right)$'
msgm2c  db 'Mazes: 15x15, 24x24, 36x36. Time limit 30 / 45 / 60 seconds.$'
msgm2d  db 'First to the exit wins; at time-out the closer one wins.$'
msgm5   db '3   Cognitive study (the bot learns)$'
msgm5a  db 'The bot runs one maze again and again, keeping what it has$'
msgm5b  db 'seen, until it walks the optimal path. Then you do the same$'
msgm5c  db 'on that maze and both learning curves are compared. 3 stages.$'
msgm3   db 'ESC Exit to DOS$'
msgm4   db 'Press 1 or 3$'
msgstud db ' COGNITIVE STUDY - THE BOT LEARNS ONE MAZE $'
msgstg  db 'Stage $'
msgsopt db 'Optimal path  $'
msgstep db ' steps$'
msgsee  db 'Bot sight     $'
msgsq   db ' square(s) around it$'
msgtry  db 'Try $'
msgtgap db '   $'
msghist db 'Paths walked so far:$'
msgisop db ' OPTIMAL$'
msgnoop db '        $'
msgsleg db ' this try   $'
msgsleo db ' earlier tries   $'
msgsleb db ' bot   $'
msgslee db ' exit$'
msglrn  db 'Learning: every try starts again from the start, with all it has seen so far.$'
msgnopt db 'Not optimal yet - it tries again with what it learned.                       $'
msgftr  db ' TRY(S) $'
msgdash4 db '----$'
msgskey db 'SPACE = slow / fast speed    ESC = menu$'
msgsfst db '   [ FAST ]$'
msgsnrm db '   [ SLOW ]$'
msgsnxt db 'ENTER = next stage       ESC = menu$'
msgslst db 'ENTER = study results    ESC = menu$'
msgsres db ' COGNITIVE STUDY - RESULTS $'
msgrs1  db '   optimal $'
msgrbt  db '   bot $'
msgryu  db '   you $'
msgrs3  db ' try(s)$'
msgwyou db '   you faster$'
msgwbot db '   bot faster$'
msgwtie db '   tie$'
msgov1  db 'Overall: you were faster on $'
msgov2  db ' stage(s), the bot on $'
msgov3  db ', ties $'
msgrsn  db 'The bot never sees the whole maze: it learns only from what it has walked past.$'
msgstuh db ' COGNITIVE STUDY - NOW YOU: SAME MAZE, SAME RULE $'
msgysee db 'You see the whole maze             $'
msgsley db ' you   $'
msglrh  db 'Your turn: walk from the start to the exit, again until your path is optimal.$'
msgykey db 'Arrows / W A S D = move    ESC = menu              $'
msgbfnd db ' THE BOT FOUND THE OPTIMAL PATH AFTER $'
msgsyou db 'ENTER = your turn on the same maze       ESC = menu$'
msgcmph db 'Try        BOT      YOU       $'
msgcgap db '     $'
msgcpad db '      $'
msgblnk db '                              $'
msgcbot db 'BOT : optimal after $'
msgcyou db 'YOU : optimal after $'
msgcmid db ' try(s), $'
msgcend db ' steps walked in total$'
msgvyou db ' YOU LEARNED THE MAZE FASTER THAN THE BOT $'
msgvbot db ' THE BOT LEARNED THE MAZE FASTER THAN YOU $'
msgvtie db ' TIE - SAME TRIES AND SAME STEPS $'
msglvl  db 'Level $'
msgof   db ' of 3$'
msggrid db 'Grid  $'
msgx    db ' x $'
msgany  db 'Press any key for the menu$'
msgp1   db ' PLAYER 1  W A S D $'
msgp2   db ' PLAYER 2  ARROW KEYS $'
msgyou  db ' YOU  ARROWS / W A S D $'
msgcpu  db ' COMPUTER BOT $'
msgcd3  db '    3     $'
msgcd2  db '    2     $'
msgcd1  db '    1     $'
msgcdgo db '  START!  $'
msgcdno db '          $'
msgtime db 'Time left  $'
msgsec  db ' s$'
msgmv2  db 'Moves $'
msgds2  db '  Dist $'
msgscr  db 'Score   P1  $'
msgscr2 db '  :  $'
msgscr3 db '  P2$'
msgscy  db 'Score   YOU  $'
msgscc  db '  CPU$'
msghelp db 'First to the exit wins. At time-out the closer player wins.  ESC = menu$'
msghlpc db 'Race the bot to the exit.  H or ENTER = hint   ESC = menu$'
msgw1   db ' PLAYER 1 WINS THE LEVEL $'
msgw2   db ' PLAYER 2 WINS THE LEVEL $'
msgwy   db ' YOU WIN THE LEVEL $'
msgwc   db ' THE COMPUTER WINS THE LEVEL $'
msgdraw db ' DRAW $'
msgrx   db ' - reached the exit first$'
msgrt   db ' - time up, closer to the exit$'
msgrd   db ' - time up, both equally far from the exit$'
msglegd db ' optimal   $'
msglegt db ' own path   $'
msglegb db ' both    ENTER = continue  ESC = menu$'
msgfin  db 'FINAL SCORE$'
msgfp1  db 'P1  $'
msgfpy  db 'YOU  $'
msgfw1  db ' PLAYER 1 WINS THE MATCH! $'
msgfw2  db ' PLAYER 2 WINS THE MATCH! $'
msgfwy  db ' YOU WIN THE MATCH! $'
msgfwc  db ' THE COMPUTER WINS THE MATCH! $'
msgfdr  db ' THE MATCH IS A DRAW $'

; ---- grids (only the first msize*msize entries are used) ---------------
grid    db MAXC dup(0)          ; 0 = wall, 1 = open
mark    db MAXC dup(0)          ; 1 = on the traced shortest path
trail   db 2 * MAXC dup(0)      ; per player: 1 = walked here
dstart  dw MAXC dup(0)          ; BFS distance from the start
dexit   dw MAXC dup(0)          ; BFS distance from the exit
queue   dw MAXC dup(0)          ; BFS queue of square indices
dstack  dw MAXC dup(0)          ; DFS stack of square indices
known   db MAXC dup(0)          ; bot map: 0 = unseen, 1 = open, 2 = wall
parent  dw MAXC dup(0)          ; A*: square we came from
bpath   dw MAXC dup(0)          ; A*: planned path, exit first
hintmap db MAXC dup(0)          ; 1 = on the hint path shown to you

; the A* search runs during play, when these build-time arrays are free
gcost   equ dstart              ; A*: steps from the bot (FFFFh = not reached)
closed  equ mark                ; A*: 1 = already expanded
bucket0 equ queue               ; A*: squares with f = curf ...
bucket1 equ dstack              ; ... and with f = curf + 2

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
    cmp al,'3'                  ; '2' (PvP) is switched off for now
    jne mn_key
    mov players,3
    call study                  ; mode 3 runs its own three stages
    jmp menu
mn_one:
    mov players,1
mn_play:
    mov level,0
    mov score[0],0
    mov score[1],0

next_level:
    call setup                  ; size and step table for this level
    call buildmaze              ; random maze with several routes
    call initlevel              ; players, trails, screen, bot
    call countdown              ; 3, 2, 1, START! - then the clock runs
    or al,al
    jnz menu                    ; ESC pressed during the countdown
    call duel                   ; race until the exit or time-out
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
    mov dh,1
    mov dl,33
    mov si,offset msgtitl
    mov ah,A_TITLE
    call vtext
    mov dh,3
    mov dl,12
    mov si,offset msgm1
    mov ah,A_P1
    call vtext
    mov ah,A_DIM
    mov si,offset msgm1a
    call menuline
    mov si,offset msgm1b
    call menuline
    mov si,offset msgm1c
    call menuline
    mov dh,8
    mov dl,12
    mov si,offset msgm2
    mov ah,A_DIM                ; PvP is switched off for now
    call vtext
    mov ah,A_DIM
    mov si,offset msgm2a
    call menuline
    mov si,offset msgm2b
    call menuline
    mov si,offset msgm2c
    call menuline
    mov si,offset msgm2d
    call menuline
    mov dh,14
    mov dl,12
    mov si,offset msgm5
    mov ah,A_TITLE
    call vtext
    mov ah,A_DIM
    mov si,offset msgm5a
    call menuline
    mov si,offset msgm5b
    call menuline
    mov si,offset msgm5c
    call menuline
    mov dh,20
    mov dl,12
    mov si,offset msgm3
    mov ah,A_MSG
    call vtext
    mov dh,22
    mov dl,33
    mov si,offset msgm4
    mov ah,A_HUD
    call vtext
    ret
drawmenu endp

menuline proc                   ; next row, column 16: text SI, attribute AH
    inc dh
    mov dl,16
    call vtext
    ret
menuline endp

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
;             and, against the computer, wake up the bot
; ===========================================================================
initlevel proc
    mov ax,spos
    mov ppos[0],ax
    mov ppos[2],ax
    mov steps[0],0
    mov steps[2],0
    mov mode,0
    mov hinton,0
    mov who,0
    mov di,offset trail
    mov cx,2 * MAXC
    xor al,al
    call memfill                ; nothing walked yet ...
    mov bx,spos
    mov trail[bx],1             ; ... except the start square
    mov trail[bx+MAXC],1
    cmp players,1
    jne il_draw
    call botinit                ; the bot's map: border and exit only
il_draw:
    call cls
    call drawmazes
    call drawhud2
    cmp players,1
    jne il_done
    call botlook                ; first look around the start ...
    call botplan                ; ... and first plan
il_done:
    mov lastsec,0FFFFh
    ret
initlevel endp

; ===========================================================================
; countdown - show 3, 2, 1, START! (one second each) between the level
;             info and the clock, then start the clock and the bot.
;             Keys pressed meanwhile are thrown away.
;             OUT: AL = 1 if ESC was pressed (back to the menu)
; ===========================================================================
countdown proc
    mov si,offset msgcd3
    mov ah,A_TITLE
    mov cx,18                   ; 18 ticks = one second
    call cdstep
    jc cd_esc
    mov si,offset msgcd2
    call cdstep
    jc cd_esc
    mov si,offset msgcd1
    call cdstep
    jc cd_esc
    mov si,offset msgcdgo
    mov ah,A_WIN
    mov cx,9                    ; START! stays up for half a second
    call cdstep
    jc cd_esc
    mov si,offset msgcdno
    mov ah,A_DIM
    mov dh,19
    mov dl,24
    call vtext                  ; clear the countdown
    xor ah,ah
    int 1Ah
    mov tstart,dx               ; the clock starts now ...
    mov botlast,dx              ; ... and so does the bot
    xor al,al
    ret
cd_esc:
    mov al,1
    ret
countdown endp

; ===========================================================================
; cdstep - show text SI (attribute AH) at row 19, column 24 and wait CX
;          clock ticks, throwing keys away. OUT: CF = 1 if ESC was pressed
; ===========================================================================
cdstep proc
    push ax
    push bx
    push dx
    mov dh,19
    mov dl,24
    call vtext
    xor ah,ah
    int 1Ah
    mov bx,dx                   ; BX = tick when this step started
cd_loop:
    mov ah,01h
    int 16h                     ; any key waiting?
    jz cd_idle
    mov ah,0
    int 16h                     ; take it out of the buffer
    cmp al,27
    je cd_quit
    jmp cd_loop
cd_idle:
    xor ah,ah
    int 1Ah
    mov ax,dx
    sub ax,bx
    cmp ax,cx
    jae cd_done
    sti
    hlt                         ; sleep until the next interrupt
    jmp cd_loop
cd_done:
    clc
    jmp cd_out
cd_quit:
    stc
cd_out:
    pop dx
    pop bx
    pop ax
    ret
cdstep endp

; ===========================================================================
; duel - race loop with time limit (player vs player or player vs bot)
;        OUT: AL = 0 level finished (winner, reason set), AL = 1 ESC
; ===========================================================================
duel proc
du_loop:
    call timeleft               ; AX = seconds left, CF = 1 time is up
    jc du_timeout
    cmp ax,lastsec
    je du_bot
    mov lastsec,ax
    call showtime
du_bot:
    cmp players,1
    jne du_poll
    call botturn                ; CF = 1 when the bot reached the exit
    jnc du_poll
    mov who,1
    jmp du_won
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
    cmp players,1
    jne du_dir
    cmp al,13                   ; against the bot: ENTER or H = hint
    je du_hint
    mov dl,al
    or dl,20h
    cmp dl,'h'
    jne du_dir
du_hint:
    call showhint
    jmp du_loop
du_dir:
    call keydir                 ; DL = player who owns the key
    jnc du_loop
    cmp players,1
    jne du_owner
    xor dl,dl                   ; against the bot every movement key is yours
du_owner:
    mov who,dl
    call trymove
    jnc du_loop
du_won:
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
    je tm_go
    clc                         ; wall: ignore the key
    ret
tm_go:
    call applymove
    ret
trymove endp

; ===========================================================================
; applymove - put player 'who' on open square AX
;             OUT: CF = 1 when that square is the exit
; ===========================================================================
applymove proc
    push bx
    push dx
    push si
    push di
    mov si,ax                   ; SI = new square
    mov al,who
    xor ah,ah
    shl ax,1
    mov di,ax                   ; DI = who * 2
    mov dx,ppos[di]             ; DX = square being left
    mov ppos[di],si
    inc steps[di]
    call tbase
    mov trail[bx+si],1          ; remember the walked path
    mov ax,dx
    call drawcell               ; repaint the old square
    mov ax,si
    call drawcell               ; draw the player on the new one
    cmp who,0
    jne am_stats
    cmp hinton,0
    je am_stats
    mov hinton,0                ; your move ends the hint
    call drawall
am_stats:
    call updstats
    cmp si,epos
    je am_exit
    clc
    jmp am_out
am_exit:
    stc
am_out:
    pop di
    pop si
    pop dx
    pop bx
    ret
applymove endp

; ===========================================================================
; showhint - show your shortest path to the exit in cyan until your next
;            move (against the computer only)
; ===========================================================================
showhint proc
    push ax
    push cx
    push si
    push di
    mov ax,ppos[0]
    call tracepath              ; mark[] = shortest path from you
    push es
    push ds
    pop es
    mov si,offset mark
    mov di,offset hintmap
    mov cx,cells
    rep movsb                   ; keep it: the bot reuses mark[] for A*
    pop es
    mov hinton,1
    mov al,who
    push ax
    mov who,0
    call drawall
    pop ax
    mov who,al
    pop di
    pop si
    pop cx
    pop ax
    ret
showhint endp

; ===========================================================================
; the computer player
;
; botinit - the bot knows only the border and where the exit is
; botturn - called from the race loop: one step every botdly clock ticks
;           OUT: CF = 1 when the bot reached the exit
; botstep - take the next square of the plan, look around, and replan if a
;           new wall came into sight
; ===========================================================================
botinit proc
    mov di,offset known
    mov cx,cells
    xor al,al
    call memfill                ; nothing seen yet
    xor bx,bx
bi_loop:
    cmp bx,cells
    jae bi_done
    mov ax,bx
    xor dx,dx
    div msize                   ; AX = row, DX = column
    mov cx,msize
    dec cx
    or ax,ax
    jz bi_wall
    cmp ax,cx
    je bi_wall
    or dx,dx
    jz bi_wall
    cmp dx,cx
    jne bi_next
bi_wall:
    mov known[bx],2             ; the frame of the maze is visible to all
bi_next:
    inc bx
    jmp bi_loop
bi_done:
    mov ax,epos
    xor dx,dx
    div msize
    mov erow,ax                 ; the exit is visible to all as well
    mov ecol,dx
    mov bl,level
    xor bh,bh
    mov al,botspd[bx]
    xor ah,ah
    mov botdly,ax
    mov al,botview[bx]
    mov botsee,ax
    mov bplen,0
    ret
botinit endp

botturn proc
    push ax
    push dx
    xor ah,ah
    int 1Ah                     ; DX = low word of the tick count
    mov ax,dx
    sub ax,botlast
    cmp ax,botdly
    jb bt_wait
    mov botlast,dx
    call botstep
    jmp bt_out
bt_wait:
    clc
bt_out:
    pop dx
    pop ax
    ret
botturn endp

botstep proc
    push bx
    push si
    cmp bplen,0
    jne bs_go
    call botplan                ; no plan yet (or the last one failed)
    cmp bplen,0
    jne bs_go
    clc
    jmp bs_out
bs_go:
    dec bplen
    mov bx,bplen
    shl bx,1
    mov ax,bpath[bx]            ; next square on the plan
    mov si,ax
    cmp grid[si],1
    je bs_move
    mov bplen,0                 ; the plan ran into a wall: plan again later
    clc
    jmp bs_out
bs_move:
    mov bl,who
    push bx
    mov who,1
    call applymove
    pushf
    call botlook                ; what can the bot see from here?
    cmp newwall,0
    je bs_keep
    call botplan                ; a new wall: find a new shortest path
bs_keep:
    popf
    pop bx
    mov who,bl
bs_out:
    pop si
    pop bx
    ret
botstep endp

; ===========================================================================
; botlook - add every square within botsee rows/columns of the bot to the
;           bot's own map (the screen always shows the whole maze)
;           OUT: newwall = 1 if a wall appeared
; ===========================================================================
botlook proc
    push ax
    push bx
    push cx
    push dx
    push si
    mov newwall,0
    mov ax,ppos[2]
    xor dx,dx
    div msize                   ; AX = bot row, DX = bot column
    mov si,msize
    dec si                      ; SI = last row / column
    mov cx,ax
    sub cx,botsee
    jnc bl_r0
    xor cx,cx
bl_r0:
    mov rmin,cx
    mov cx,ax
    add cx,botsee
    cmp cx,si
    jbe bl_r1
    mov cx,si
bl_r1:
    mov rmax,cx
    mov cx,dx
    sub cx,botsee
    jnc bl_c0
    xor cx,cx
bl_c0:
    mov cmin,cx
    mov cx,dx
    add cx,botsee
    cmp cx,si
    jbe bl_c1
    mov cx,si
bl_c1:
    mov cmax,cx
    mov cx,rmin                 ; CX = row being scanned
bl_row:
    cmp cx,rmax
    ja bl_done
    mov ax,cx
    mul msize
    add ax,cmin
    mov bx,ax                   ; BX = first square of this row in sight
    mov si,cmin                 ; SI = column being scanned
bl_col:
    cmp si,cmax
    ja bl_next
    cmp known[bx],0
    jne bl_skip
    mov known[bx],1
    cmp grid[bx],0
    jne bl_skip
    mov known[bx],2
    mov newwall,1
bl_skip:
    inc bx
    inc si
    jmp bl_col
bl_next:
    inc cx
    jmp bl_row
bl_done:
    pop si
    pop dx
    pop cx
    pop bx
    pop ax
    ret
botlook endp

; ===========================================================================
; botplan - A* from the bot to the exit over the bot's own map
;           (unseen squares are assumed open, known walls are avoided)
;   g = steps from the bot, h = |row - exit row| + |column - exit column|.
;   Every step costs 1 and changes h by exactly 1, so f = g + h of a new
;   square is either the current f or f + 2. Two buckets are therefore
;   enough for the open list: 'current' (f) and 'next' (f + 2).
;   OUT: bpath[0 .. bplen-1] = path from the exit back to the bot
; ===========================================================================
botplan proc
    push ax
    push bx
    push cx
    push dx
    push si
    push di
    mov bplen,0
    mov bx,offset gcost
    mov cx,cells
bp_fill:
    mov word ptr [bx],0FFFFh
    add bx,2
    loop bp_fill
    mov di,offset closed
    mov cx,cells
    xor al,al
    call memfill
    mov curp,offset bucket0
    mov nxtp,offset bucket1
    mov curc,0
    mov nxtc,0
    mov ax,ppos[2]              ; start the search at the bot
    mov bx,ax
    shl bx,1
    mov gcost[bx],0
    push ax
    call hcost
    mov curf,ax                 ; f(bot) = h(bot)
    pop ax
    call pushcur
bp_loop:
    cmp curc,0
    jne bp_pop
    cmp nxtc,0
    jne bp_swap
    jmp bp_done                 ; open list empty: no path known
bp_swap:
    mov ax,curp                 ; move on to the f + 2 bucket
    xchg ax,nxtp
    mov curp,ax
    mov ax,nxtc
    mov curc,ax
    mov nxtc,0
    add curf,2
bp_pop:
    dec curc
    mov bx,curp
    mov si,curc
    shl si,1
    mov ax,[bx+si]              ; AX = square to expand
    mov si,ax
    cmp closed[si],0
    jne bp_loop                 ; already expanded through a shorter way
    mov closed[si],1
    cmp ax,epos
    je bp_found
    mov bx,ax
    shl bx,1
    mov dx,gcost[bx]
    inc dx                      ; DX = g of every neighbour
    xor cx,cx
bp_dir:
    mov bx,cx
    mov si,ax
    add si,deltas[bx]           ; SI = neighbour
    cmp known[si],2
    je bp_next                  ; a wall the bot has seen
    cmp closed[si],0
    jne bp_next
    mov bx,si
    shl bx,1
    cmp dx,gcost[bx]
    jae bp_next                 ; not shorter than what we had
    mov gcost[bx],dx
    mov parent[bx],ax
    push ax
    mov ax,si
    call hcost
    add ax,dx                   ; AX = f of the neighbour
    cmp ax,curf
    mov ax,si
    je bp_same
    call pushnext
    jmp bp_pushed
bp_same:
    call pushcur
bp_pushed:
    pop ax
bp_next:
    add cx,2
    cmp cx,8
    jb bp_dir
    jmp bp_loop
bp_found:
    mov ax,epos                 ; follow the parents back to the bot
    xor di,di
bp_back:
    cmp ax,ppos[2]
    je bp_done
    mov bpath[di],ax
    add di,2
    inc bplen
    mov bx,ax
    shl bx,1
    mov ax,parent[bx]
    jmp bp_back
bp_done:
    pop di
    pop si
    pop dx
    pop cx
    pop bx
    pop ax
    ret
botplan endp

; ===========================================================================
; hcost    - AX = Manhattan distance from square AX to the exit
; pushcur  - add square AX to the current A* bucket
; pushnext - add square AX to the f + 2 A* bucket
; ===========================================================================
hcost proc
    push dx
    xor dx,dx
    div msize                   ; AX = row, DX = column
    sub ax,erow
    jns hc_row
    neg ax
hc_row:
    sub dx,ecol
    jns hc_col
    neg dx
hc_col:
    add ax,dx
    pop dx
    ret
hcost endp

pushcur proc
    push bx
    push si
    mov si,curc
    cmp si,MAXC
    jae pc_full
    mov bx,curp
    shl si,1
    mov [bx+si],ax
    inc curc
pc_full:
    pop si
    pop bx
    ret
pushcur endp

pushnext proc
    push bx
    push si
    mov si,nxtc
    cmp si,MAXC
    jae pn_full
    mov bx,nxtp
    shl si,1
    mov [bx+si],ax
    inc nxtc
pn_full:
    pop si
    pop bx
    ret
pushnext endp

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
    cmp players,3
    jne cc_race
    jmp cc_study
cc_race:
    cmp mode,0
    jne cc_result
    cmp si,ppos[di]
    je cc_player
    cmp si,epos
    je cc_exit
    cmp hinton,0
    je cc_plain
    cmp who,0
    jne cc_plain
    cmp hintmap[si],0
    je cc_plain
    mov bl,C_HINT               ; your hint path
    jmp cc_out
cc_plain:
    cmp grid[si],0
    je cc_wall
    jmp cc_floor
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
cc_study:                       ; mode 3: this try red, earlier tries grey
    cmp si,epos
    je cc_exit
    cmp grid[si],0
    je cc_wall
    cmp mode,0
    jne cs_found
    cmp si,ppos[2]
    je cs_runner
    cmp trail[si+MAXC],0
    jne cc_taken
cs_old:
    cmp trail[si],0
    je cc_floor
    mov bl,C_OLD
    jmp cc_out
cs_found:                       ; the optimal try in yellow
    cmp trail[si+MAXC],0
    je cs_old
    mov bl,C_OPT
    jmp cc_out
cs_runner:                      ; cyan bot or green you
    mov bl,C_PLAY2
    cmp runner,0
    je cs_done
    mov bl,C_PLAY
cs_done:
    jmp cc_out
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
    mov who,1
    call drawall
    mov who,0
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
; updstats - refresh the moves and exit distance of player 'who'
; ===========================================================================
updstats proc
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
; drawhud2 - labels and bottom panel
;   row 0  : player names over each maze
;   row 19 : level and time left        row 20 : grid size
;   row 21 : moves and distance of each player
;   row 22 : match score
;   row 23 : rules / level winner
; ===========================================================================
drawhud2 proc
    mov dh,0
    mov dl,LEFTX
    mov si,offset msgp1
    mov ah,A_P1
    cmp players,1
    jne dh_left
    mov si,offset msgyou
dh_left:
    call vtext
    mov dh,0
    mov dl,RIGHTX
    mov si,offset msgp2
    mov ah,A_P2
    cmp players,1
    jne dh_right
    mov si,offset msgcpu
dh_right:
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
    cmp players,1
    jne dh_help
    mov si,offset msghlpc
dh_help:
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
    cmp players,1
    jne ss_left
    mov si,offset msgscy
ss_left:
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
    cmp players,1
    jne ss_right
    mov si,offset msgscc
ss_right:
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
; result2 - score the level, show the winner, reason and legend
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
    cmp players,1
    je r2_bot
    mov si,offset msgw1
    mov ah,A_WIN
    or bl,bl
    jz r2_name
    mov si,offset msgw2
    jmp r2_name
r2_bot:
    mov si,offset msgwy
    mov ah,A_WIN
    or bl,bl
    jz r2_name
    mov si,offset msgwc
    mov ah,A_LOSE
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
    mov dh,9
    mov dl,34
    mov si,offset msgfin
    mov ah,A_MSG
    call vtext
    mov dh,11                   ; 'P1  nn  :  nn  P2' / 'YOU  nn  :  nn  CPU'
    mov dl,32
    mov si,offset msgfp1
    cmp players,1
    jne fs_left
    mov si,offset msgfpy
fs_left:
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
    cmp players,1
    jne fs_right
    mov si,offset msgscc
fs_right:
    call vtext
    mov dh,13
    mov dl,27
    mov si,offset msgfdr
    mov ah,A_TITLE
    mov al,score[0]
    cmp al,score[1]
    je fs_show
    ja fs_first
    mov si,offset msgfw2        ; the right-hand side won
    mov ah,A_WIN
    cmp players,1
    jne fs_show
    mov si,offset msgfwc
    mov ah,A_LOSE
    jmp fs_show
fs_first:
    mov si,offset msgfw1        ; the left-hand side won
    mov ah,A_WIN
    cmp players,1
    jne fs_show
    mov si,offset msgfwy
fs_show:
    call vtext
    mov dh,15
    mov dl,27
    mov si,offset msgany
    mov ah,A_DIM
    call vtext
    mov ah,0
    int 16h
    ret
finalscreen endp

; ===========================================================================
; study - mode 3, cognitive study. On each stage the bot first runs the
;   same maze again and again from the start, keeping its own map (known[])
;   between tries, until one try is exactly as long as the optimal path.
;   Then it is your turn on the same maze with the same rule, and the two
;   learning curves are compared.
;   Why the bot always gets there: if a try discovers no new wall, the bot
;   walked its first plan - a shortest path on a map where unseen squares
;   count as open, which is never longer than the true shortest path - so
;   that try was optimal. Every other try uncovers at least one new wall,
;   and there are only so many walls.
; ===========================================================================
study proc
    mov orgx[1],LEFTX           ; one maze only, drawn on the left
    mov fast,1                  ; full speed unless SPACE slows it down
    mov level,0
sy_stage:
    call setup
    call buildmaze
    call botinit                ; a new maze: the bot knows nothing yet
    mov bl,level
    xor bh,bh
    mov al,stview[bx]
    xor ah,ah
    mov botsee,ax
    mov bx,spos
    shl bx,1
    mov ax,dexit[bx]
    mov optlen,ax               ; for the referee only
    mov runner,0                ; first the bot ...
    mov histp,offset hist
    call learn
    or al,al
    jnz sy_quit
    mov bl,level
    xor bh,bh
    mov al,byte ptr trynum
    mov srtry[bx],al
    shl bx,1
    mov ax,optlen
    mov sropt[bx],ax
    call studydone
    call studyenter
    jc sy_quit
    mov runner,1                ; ... then you, on the same maze
    mov histp,offset hhist
    call learn
    or al,al
    jnz sy_quit
    mov bl,level
    xor bh,bh
    mov al,byte ptr trynum
    mov srhum[bx],al
    call studycmp
    call studyenter
    jc sy_quit
    inc level
    cmp level,NLEV
    jae sy_end
    jmp sy_stage
sy_end:
    call studyfinal
sy_quit:
    mov orgx[1],RIGHTX
    ret
study endp

studyenter proc                 ; wait for ENTER (CF = 0) or ESC (CF = 1)
se_key:
    mov ah,0
    int 16h
    cmp al,27
    je se_esc
    cmp al,13
    jne se_key
    clc
    ret
se_esc:
    stc
    ret
studyenter endp

; ===========================================================================
; learn - the runner (0 = bot, 1 = you) walks the current maze from the
;         start again and again until one try is as long as the optimal
;         path. Every try is stored in the runner's history (histp).
;         OUT: AL = 1 if ESC was pressed
; ===========================================================================
learn proc
    mov di,offset trail
    mov cx,2 * MAXC
    xor al,al
    call memfill                ; no paths walked yet
    mov trynum,0
lr_try:
    inc trynum
    call newtry
    call studyhud
lr_step:
    cmp runner,0
    jne lr_you
    mov cx,1                    ; the bot: one step per clock tick (or none)
    call studywait
    or al,al
    jnz lr_esc
    call botstep                ; CF = 1 when the exit is reached
    jmp lr_moved
lr_you:
    mov ah,0
    int 16h                     ; your move
    cmp al,27
    je lr_esc
    call keydir                 ; arrows and W A S D both move you
    jnc lr_step
    mov who,1
    call trymove
lr_moved:
    pushf
    call studycur
    popf
    jnc lr_step
    call endtry
    mov ax,steps[2]
    cmp ax,optlen
    je lr_done
    cmp trynum,MAXTRY
    jae lr_done
    mov dh,22
    mov dl,LEFTX
    mov si,offset msgnopt
    mov ah,A_MSG
    call vtext
    mov cx,27                   ; show the result for a moment
    call studywait
    or al,al
    jnz lr_esc
    jmp lr_try
lr_done:
    mov mode,1
    mov who,1
    call drawall                ; the optimal try in yellow
    xor al,al
    ret
lr_esc:
    mov al,1
    ret
learn endp

; ===========================================================================
; newtry - back to the start for the next try. The bot keeps its map, so
;          its first plan already uses everything seen in earlier tries.
; ===========================================================================
newtry proc
    mov ax,spos
    mov ppos[2],ax
    mov steps[2],0
    mov bplen,0
    mov mode,0
    mov who,1
    mov di,offset trail + MAXC
    mov cx,MAXC
    xor al,al
    call memfill                ; this try has walked nothing yet
    mov bx,spos
    mov trail[bx+MAXC],1
    call cls
    call drawall
    cmp runner,0
    jne nt_done
    call botlook                ; look around the start ...
    call botplan                ; ... and plan with all it knows
nt_done:
    ret
newtry endp

; ===========================================================================
; endtry - store the length of the try and add its squares to the paths of
;          the earlier tries (shown in grey)
; ===========================================================================
endtry proc
    mov bx,trynum
    dec bx
    cmp bx,MAXTRY
    jae et_merge
    shl bx,1
    mov si,histp
    mov ax,steps[2]
    mov [bx+si],ax
et_merge:
    xor bx,bx
et_loop:
    cmp bx,cells
    jae et_done
    cmp trail[bx+MAXC],0
    je et_next
    mov trail[bx],1
et_next:
    inc bx
    jmp et_loop
et_done:
    mov ax,trynum
    call studyhist
    ret
endtry endp

; ===========================================================================
; studywait - wait CX clock ticks. In fast mode a step does not wait at all
;             and a longer pause is cut to half a second.
;             SPACE toggles fast mode. OUT: AL = 1 if ESC was pressed
; ===========================================================================
studywait proc
    push bx
    push cx
    push dx
    cmp fast,0
    je sw_start
    cmp cx,1
    ja sw_pause
    xor cx,cx                   ; a step: no delay
    jmp sw_start
sw_pause:
    mov cx,9                    ; between tries: half a second
sw_start:
    xor ah,ah
    int 1Ah
    mov bx,dx                   ; BX = tick when the wait started
sw_loop:
    mov ah,01h
    int 16h                     ; any key waiting?
    jz sw_time
    mov ah,0
    int 16h
    cmp al,27
    je sw_esc
    cmp al,' '
    jne sw_loop
    cmp runner,0
    jne sw_loop                 ; SPACE only changes the bot's speed
    xor fast,1
    call studykeys
    cmp fast,0
    je sw_loop
    cmp cx,1
    ja sw_loop
    xor cx,cx                   ; fast mode: stop waiting now
    jmp sw_loop
sw_time:
    jcxz sw_go
    xor ah,ah
    int 1Ah
    mov ax,dx
    sub ax,bx
    cmp ax,cx
    jae sw_go
    sti
    hlt                         ; sleep until the next interrupt
    jmp sw_loop
sw_go:
    xor al,al
    jmp sw_out
sw_esc:
    mov al,1
sw_out:
    pop dx
    pop cx
    pop bx
    ret
studywait endp

; ===========================================================================
; study screen
;   left : the maze (rows 1-18), legend (row 20), moves (row 21),
;          status (row 22), keys (row 23), ENTER prompt (row 24)
;   right: stage, grid, optimal length, sight (rows 2-6),
;          current try (row 8), history of tries (rows 10-18)
; ===========================================================================
studyhud proc
    mov dh,0
    mov dl,LEFTX
    mov si,offset msgstud
    cmp runner,0
    je sh_title
    mov si,offset msgstuh
sh_title:
    mov ah,A_TITLE
    call vtext
    mov dh,2
    mov dl,RIGHTX
    mov si,offset msgstg
    mov ah,A_HUD
    call vtext
    mov al,level
    add al,'1'
    call vchar
    mov si,offset msgof
    call vtext
    mov dh,3
    mov dl,RIGHTX
    mov si,offset msggrid
    call vtext
    mov al,byte ptr msize
    call vnum2
    mov si,offset msgx
    call vtext
    mov al,byte ptr msize
    call vnum2
    mov dh,5
    mov dl,RIGHTX
    mov si,offset msgsopt
    mov ah,A_MSG
    call vtext
    mov ax,optlen
    mov bl,A_MSG
    call vnum4
    mov si,offset msgstep
    call vtext
    mov dh,6
    mov dl,RIGHTX
    mov ah,A_HUD
    cmp runner,0
    je sh_sight
    mov si,offset msgysee
    call vtext
    jmp sh_list
sh_sight:
    mov si,offset msgsee
    call vtext
    mov al,byte ptr botsee
    add al,'0'
    call vchar
    mov si,offset msgsq
    call vtext
sh_list:
    mov dh,10
    mov dl,RIGHTX
    mov si,offset msghist
    mov ah,A_DIM
    call vtext

    mov dh,20
    mov dl,LEFTX
    mov al,0DBh
    mov ah,C_TAKEN
    call vchar
    mov si,offset msgsleg
    mov ah,A_MSG
    call vtext
    mov al,0DBh
    mov ah,C_OLD
    call vchar
    mov si,offset msgsleo
    mov ah,A_MSG
    call vtext
    mov al,0DBh
    mov ah,C_PLAY2
    mov si,offset msgsleb
    cmp runner,0
    je sh_legend
    mov ah,C_PLAY
    mov si,offset msgsley
sh_legend:
    call vchar
    mov ah,A_MSG
    call vtext
    mov al,0DBh
    mov ah,C_EXIT
    call vchar
    mov si,offset msgslee
    mov ah,A_MSG
    call vtext

    mov dh,22
    mov dl,LEFTX
    mov si,offset msglrn
    cmp runner,0
    je sh_status
    mov si,offset msglrh
sh_status:
    mov ah,A_DIM
    call vtext
    cmp runner,0
    jne sh_youkeys
    call studykeys
    jmp sh_rest
sh_youkeys:
    mov dh,23
    mov dl,LEFTX
    mov si,offset msgykey
    mov ah,A_DIM
    call vtext
sh_rest:
    call studycur
    mov ax,trynum
    dec ax                      ; finished tries so far
    call studyhist
    ret
studyhud endp

studykeys proc                  ; bot speed keys and the fast/slow indicator
    push ax
    push dx
    push si
    mov dh,23
    mov dl,LEFTX
    mov si,offset msgskey
    mov ah,A_DIM
    call vtext
    mov si,offset msgsnrm
    cmp fast,0
    je sk_show
    mov si,offset msgsfst
    mov ah,A_WIN
sk_show:
    call vtext
    pop si
    pop dx
    pop ax
    ret
studykeys endp

studycur proc                   ; 'Try nn   nnnn steps' for the running try
    push ax
    push bx
    push dx
    push si
    mov bl,A_P2
    cmp runner,0
    je sc_show
    mov bl,A_P1
sc_show:
    mov dh,8
    mov dl,RIGHTX
    mov si,offset msgtry
    mov ah,bl
    call vtext
    mov al,byte ptr trynum
    call vnum2
    mov si,offset msgtgap
    call vtext
    mov ax,steps[2]
    call vnum4
    mov si,offset msgstep
    mov ah,bl
    call vtext
    pop si
    pop dx
    pop bx
    pop ax
    ret
studycur endp

; ===========================================================================
; studyhist - list the last 8 finished tries of the runner
;             IN: AX = number of finished tries
; ===========================================================================
studyhist proc
    push ax
    push bx
    push cx
    push dx
    push si
    mov bx,ax                   ; BX = number of tries to list up to
    mov cx,ax
    sub cx,8
    jnc sh_from
    xor cx,cx
sh_from:                        ; CX = first try shown (0-based)
    mov dh,11
sh_loop:
    cmp cx,bx
    jae sh_done
    cmp cx,MAXTRY
    jae sh_done
    push bx
    mov dl,RIGHTX
    mov si,offset msgtry
    mov ah,A_DIM
    call vtext
    mov al,cl
    inc al
    call vnum2
    mov si,offset msgtgap
    call vtext
    mov bx,histp
    mov si,cx
    shl si,1
    mov ax,[bx+si]              ; length of this try
    push ax
    mov bl,A_MSG
    call vnum4
    mov si,offset msgstep
    mov ah,A_MSG
    call vtext
    pop ax
    mov si,offset msgnoop
    cmp ax,optlen
    jne sh_mark
    mov si,offset msgisop
    mov ah,A_WIN
sh_mark:
    call vtext
    pop bx
    inc cx
    inc dh
    jmp sh_loop
sh_done:
    pop si
    pop dx
    pop cx
    pop bx
    pop ax
    ret
studyhist endp

studydone proc                  ; the bot is done: hand over to you
    mov dh,22
    call clrrow
    mov dh,22
    mov dl,LEFTX
    mov si,offset msgbfnd
    mov ah,A_WIN
    call vtext
    mov al,byte ptr trynum
    call vnum2
    mov si,offset msgftr
    call vtext
    mov dh,24
    mov dl,LEFTX
    mov si,offset msgsyou
    mov ah,A_HUD
    call vtext
    ret
studydone endp

; ===========================================================================
; histsum - AX = sum of the first CX words at DS:SI
; ===========================================================================
histsum proc
    push cx
    push si
    xor ax,ax
    jcxz hs_done
hs_loop:
    add ax,[si]
    add si,2
    loop hs_loop
hs_done:
    pop si
    pop cx
    ret
histsum endp

; ===========================================================================
; studycmp - both have found the optimal path: show the two histories side
;            by side, the totals, and who learned the maze faster
; ===========================================================================
studycmp proc
    mov dh,19
cp_clear:
    call clrrow
    inc dh
    cmp dh,24
    jbe cp_clear

    mov bl,level
    xor bh,bh
    mov al,srtry[bx]
    xor ah,ah
    mov di,ax                   ; DI = bot tries
    mov al,srhum[bx]
    mov cx,ax                   ; CX = your tries
    cmp cx,di
    jae cp_max
    mov cx,di
cp_max:                         ; CX = rows needed (the longer list)
    mov dh,10
    mov dl,RIGHTX
    mov si,offset msgcmph
    mov ah,A_DIM
    call vtext
    mov bx,cx
    sub bx,8
    jnc cp_first
    xor bx,bx
cp_first:                       ; BX = first try shown (0-based)
    mov dh,11
cp_row:
    cmp dh,18
    ja cp_totals
    mov dl,RIGHTX
    cmp bx,cx
    jb cp_entry
    mov si,offset msgblnk       ; nothing left to list on this row
    mov ah,A_DIM
    call vtext
    jmp cp_next
cp_entry:
    mov si,offset msgtry
    mov ah,A_DIM
    call vtext
    mov al,bl
    inc al
    call vnum2
    mov si,offset msgcgap
    call vtext
    push bx
    mov si,offset hist
    mov ax,di                   ; bot tries
    call cpvalue                ; bot column
    pop bx
    mov si,offset msgcgap
    mov ah,A_DIM
    call vtext
    push bx
    mov si,offset hhist
    mov ax,cx                   ; your tries
    call cpvalue                ; your column
    pop bx
    mov si,offset msgcpad       ; wipe the rest of the old history line
    mov ah,A_DIM
    call vtext
cp_next:
    inc bx
    inc dh
    jmp cp_row

cp_totals:
    mov dh,19
    mov dl,LEFTX
    mov si,offset msgcbot
    mov ah,A_P2
    call vtext
    mov ax,di
    mov ah,A_P2
    call vnum2
    mov si,offset msgcmid
    call vtext
    push cx
    mov cx,di
    mov si,offset hist
    call histsum
    pop cx
    push ax                     ; bot total steps
    mov bl,A_P2
    call vnum4
    mov si,offset msgcend
    mov ah,A_P2
    call vtext
    mov dh,20
    mov dl,LEFTX
    mov si,offset msgcyou
    mov ah,A_P1
    call vtext
    mov ax,cx
    mov ah,A_P1
    call vnum2
    mov si,offset msgcmid
    call vtext
    mov si,offset hhist
    call histsum
    push ax                     ; your total steps
    mov bl,A_P1
    call vnum4
    mov si,offset msgcend
    mov ah,A_P1
    call vtext
    pop dx                      ; DX = your total steps
    pop ax                      ; AX = bot total steps

    mov bl,level
    xor bh,bh
    cmp cx,di                   ; fewer tries learned faster ...
    jb cp_you
    ja cp_bot
    cmp dx,ax                   ; ... then fewer steps in total
    jb cp_you
    ja cp_bot
    mov srwin[bx],2
    mov si,offset msgvtie
    mov ah,A_TITLE
    jmp cp_verdict
cp_you:
    mov srwin[bx],1
    mov si,offset msgvyou
    mov ah,A_WIN
    jmp cp_verdict
cp_bot:
    mov srwin[bx],0
    mov si,offset msgvbot
    mov ah,A_LOSE
cp_verdict:
    mov dh,22
    mov dl,LEFTX
    call vtext
    mov dh,24
    mov dl,LEFTX
    mov si,offset msgsnxt
    cmp level,NLEV-1
    jb cp_keys
    mov si,offset msgslst
cp_keys:
    mov ah,A_HUD
    call vtext
    ret
studycmp endp

; ===========================================================================
; cpvalue - one cell of the comparison table: try BX of the history at SI
;           (AX = number of tries in it), or '----' when there is none.
;           The optimal try is shown in green.
; ===========================================================================
cpvalue proc
    cmp bx,ax
    jb cv_have
    mov si,offset msgdash4
    mov ah,A_DIM
    call vtext
    ret
cv_have:
    shl bx,1
    mov ax,[bx+si]
    mov bl,A_MSG
    cmp ax,optlen
    jne cv_show
    mov bl,A_WIN
cv_show:
    call vnum4
    ret
cpvalue endp

; ===========================================================================
; studyfinal - every stage: optimal length, tries of the bot and of you,
;              who was faster; then the overall count
; ===========================================================================
studyfinal proc
    call cls
    mov dh,3
    mov dl,26
    mov si,offset msgsres
    mov ah,A_TITLE
    call vtext
    xor bx,bx
    mov dh,7
sf_loop:
    cmp bx,NLEV
    jb sf_stage
    jmp sf_overall
sf_stage:
    mov dl,2
    mov si,offset msgstg
    mov ah,A_HUD
    call vtext
    mov al,bl
    add al,'1'
    call vchar
    mov si,offset msgtgap
    call vtext
    mov al,sizes[bx]
    call vnum2
    mov si,offset msgx
    call vtext
    mov al,sizes[bx]
    call vnum2
    mov si,offset msgrs1
    mov ah,A_MSG
    call vtext
    push bx
    shl bx,1
    mov ax,sropt[bx]
    mov bl,A_MSG
    call vnum4
    pop bx
    mov si,offset msgrbt
    mov ah,A_P2
    call vtext
    mov al,srtry[bx]
    call vnum2
    mov si,offset msgrs3
    call vtext
    mov si,offset msgryu
    mov ah,A_P1
    call vtext
    mov al,srhum[bx]
    call vnum2
    mov si,offset msgrs3
    call vtext
    mov si,offset msgwtie
    mov ah,A_TITLE
    cmp srwin[bx],2
    je sf_who
    mov si,offset msgwyou
    mov ah,A_WIN
    cmp srwin[bx],1
    je sf_who
    mov si,offset msgwbot
    mov ah,A_LOSE
sf_who:
    call vtext
    inc bx
    add dh,2
    jmp sf_loop
sf_overall:
    xor cx,cx                   ; CL = your stages, CH = bot stages
    xor dx,dx                   ; DL = ties
    xor bx,bx
sf_count:
    mov al,srwin[bx]
    cmp al,1
    jne sf_notyou
    inc cl
sf_notyou:
    cmp al,0
    jne sf_notbot
    inc ch
sf_notbot:
    cmp al,2
    jne sf_nottie
    inc dl
sf_nottie:
    inc bx
    cmp bx,NLEV
    jb sf_count
    mov bl,dl                   ; BL = ties
    mov dh,14
    mov dl,2
    mov si,offset msgov1
    mov ah,A_MSG
    call vtext
    mov al,cl
    add al,'0'
    call vchar
    mov si,offset msgov2
    call vtext
    mov al,ch
    add al,'0'
    call vchar
    mov si,offset msgov3
    call vtext
    mov al,bl
    add al,'0'
    call vchar
    mov al,'.'
    call vchar
    mov dh,17
    mov dl,1
    mov si,offset msgrsn
    mov ah,A_DIM
    call vtext
    mov dh,20
    mov dl,27
    mov si,offset msgany
    mov ah,A_DIM
    call vtext
    mov ah,0
    int 16h
    ret
studyfinal endp

end main
