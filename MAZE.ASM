.model small
.stack 100h

.data
level   db 0
prow    db 1
pcol    db 1
msglvl  db 'Level $'
msgrow  db '   Row $'
msgcol  db '   Col $'
msghelp db 'Arrows/WASD = Move   E = Exit   ESC = Quit$'
msgwin  db 'You finished all 10 levels!$'
crlf    db 13,10,'$'

mazes   db '####################'
        db '#     #            #'
        db '# ### # ########## #'
        db '# #   #          # #'
        db '# # ######## ### # #'
        db '# #        # #   # #'
        db '# ######## # # ### #'
        db '#        # # #   # #'
        db '######## #   ### #E#'
        db '####################'

        db '####################'
        db '#   #      #       #'
        db '# # # #### # ##### #'
        db '# #   #    #     # #'
        db '# ##### ######## # #'
        db '#     #        # # #'
        db '##### # ###### # # #'
        db '#     #      #   # #'
        db '# ########## #####E#'
        db '####################'

        db '####################'
        db '# #     #     #    #'
        db '# # ### # ### # ## #'
        db '#   #   #   #    # #'
        db '##### ##### ###### #'
        db '#   #     #      # #'
        db '# # ##### # #### # #'
        db '# #     #   #  # # #'
        db '# ##### ##### ##  E#'
        db '####################'

        db '####################'
        db '#      #     #     #'
        db '###### # ### # ### #'
        db '#    # #   # #   # #'
        db '# ## # ### # ### # #'
        db '# #  #     #   # # #'
        db '# # ##### # ### # ##'
        db '# #     # #        #'
        db '# ##### #   # ####E#'
        db '####################'

        db '####################'
        db '#   #       #      #'
        db '# # # # # # # # # ##'
        db '# # # # # # # # #  #'
        db '# # # # # # # # ## #'
        db '# # # # # # # # #  #'
        db '# # # # # # # # # ##'
        db '# # # # # # # # #  #'
        db '#       #       # E#'
        db '####################'

        db '####################'
        db '#        #         #'
        db '# ###### # ####### #'
        db '# #    # #       # #'
        db '# # ## # ####### # #'
        db '# #  # #       # # #'
        db '# ## # ####### # # #'
        db '#    #             #'
        db '################# E#'
        db '####################'

        db '####################'
        db '# #   #   #   #    #'
        db '# # # # # # # # ## #'
        db '#   # # # # # #  # #'
        db '##### # # # # ## # #'
        db '#     #   #   #  # #'
        db '# ################ #'
        db '#                  #'
        db '################# E#'
        db '####################'

        db '####################'
        db '#    #    #    #   #'
        db '# ## # ## # ## # # #'
        db '# #  # #  # #  # # #'
        db '# # ## # ## # ## # #'
        db '# #    #    #    # #'
        db '# ############### ##'
        db '#   #   #   #   #  #'
        db '# #   #   #   #   E#'
        db '####################'

        db '####################'
        db '#  #     #   #     #'
        db '## # ### # # # ### #'
        db '#  # # #   #   # # #'
        db '# ## # ####### # # #'
        db '#    #       # # # #'
        db '###### ##### # # # #'
        db '#    #   #   # #   #'
        db '# ##   # # ### ###E#'
        db '####################'

        db '####################'
        db '# #    #      #    #'
        db '# # ## # #### # ## #'
        db '#   #  #    # # #  #'
        db '### # ##### # # # ##'
        db '#   #     # #   #  #'
        db '# ####### # ###### #'
        db '#       #          #'
        db '####### # ########E#'
        db '####################'

.code
main proc
    mov ax,@data
    mov ds,ax

newlevel:
    mov prow,1
    mov pcol,1

game:
    call draw
    mov ah,0
    int 16h
    mov bl,prow
    mov bh,pcol
    cmp al,27
    je quit
    cmp ah,01h
    je quit
    or al,20h
    cmp ah,48h
    je goup
    cmp al,'w'
    je goup
    cmp ah,50h
    je godown
    cmp al,'s'
    je godown
    cmp ah,4Bh
    je goleft
    cmp al,'a'
    je goleft
    cmp ah,4Dh
    je goright
    cmp al,'d'
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
move:
    call getcell
    cmp al,'#'
    je game
    mov prow,bl
    mov pcol,bh
    cmp al,'E'
    jne game
    inc level
    cmp level,10
    jb newlevel

    mov ax,0003h
    int 10h
    lea dx,msgwin
    mov ah,9
    int 21h

quit:
    mov ah,4Ch
    int 21h
main endp

getcell proc
    mov al,level
    mov ah,0
    mov cx,200
    mul cx
    mov si,ax
    mov al,bl
    mov cl,20
    mul cl
    add si,ax
    mov al,bh
    mov ah,0
    add si,ax
    mov al,mazes[si]
    ret
getcell endp

pnum proc
    mov ah,0
    aam
    add ax,3030h
    push ax
    mov dl,ah
    mov ah,2
    int 21h
    pop ax
    mov dl,al
    mov ah,2
    int 21h
    ret
pnum endp

print proc
    mov ah,9
    int 21h
    ret
print endp

draw proc
    mov ax,0003h
    int 10h

    lea dx,msglvl
    call print
    mov al,level
    inc al
    call pnum
    lea dx,msgrow
    call print
    mov al,prow
    call pnum
    lea dx,msgcol
    call print
    mov al,pcol
    call pnum
    lea dx,crlf
    call print

    mov al,level
    mov ah,0
    mov cx,200
    mul cx
    mov si,ax
    mov ch,10
nextrow:
    mov cl,20
nextcol:
    mov dl,mazes[si]
    mov ah,2
    int 21h
    inc si
    dec cl
    jnz nextcol
    lea dx,crlf
    call print
    dec ch
    jnz nextrow

    lea dx,msghelp
    call print

    mov ah,2
    mov bh,0
    mov dh,prow
    inc dh
    mov dl,pcol
    int 10h
    mov ah,2
    mov dl,'@'
    int 21h

    mov ah,2
    mov bh,0
    mov dh,13
    mov dl,0
    int 10h
    ret
draw endp

end main
