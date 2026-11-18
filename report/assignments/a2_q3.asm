.model small
.stack 100h

.data
    prompt_head   db 'Enter initial head position: $'
    prompt_n      db 'Enter number of requests: $'
    prompt_req    db 'Enter track request: $'
    msg_step      db 10, 13, 'Movement to track: $'
    msg_step_mov  db ' | Seek time: $'
    msg_total     db 10, 13, 10, 13, 'Total head movement: $'
    msg_avg       db 10, 13, 'Average head movement: $'
    newline       db 10, 13, '$'
    
    initial_head  dw ?
    n             dw ?
    requests      dw 100 dup(0)
    total_mov     dw 0

.code
main proc
    ; Initialize data segment
    mov ax, @data
    mov ds, ax

    ; Read initial head position
    lea dx, prompt_head
    mov ah, 09h
    int 21h
    call read_num
    mov initial_head, ax
    
    lea dx, newline
    mov ah, 09h
    int 21h

    ; Read number of requests (n)
    lea dx, prompt_n
    mov ah, 09h
    int 21h
    call read_num
    mov n, ax
    
    lea dx, newline
    mov ah, 09h
    int 21h

    ; Loop to read n requests
    mov cx, n
    lea si, requests
read_loop:
    push cx             ; Preserve loop counter
    
    lea dx, prompt_req
    mov ah, 09h
    int 21h
    
    call read_num
    mov [si], ax        ; Store track request in array
    add si, 2           ; Move to next word (16-bit)
    
    lea dx, newline
    mov ah, 09h
    int 21h
    
    pop cx              ; Restore loop counter
    loop read_loop

    ; --- FCFS Calculation ---
    mov cx, n
    lea si, requests
    mov bx, initial_head
    mov total_mov, 0    ; Initialize total movement to 0
    
fcfs_loop:
    push cx             ; Save outer loop counter
    
    mov ax, [si]        ; AX = current_request
    mov di, ax          ; Save current_request in DI for the next iteration
    
    ; Calculate |current_request - previous_position|
    sub ax, bx          ; AX = current_request - previous_position
    jns positive_diff   ; If positive (Sign Flag = 0), skip negation
    neg ax              ; If negative, make it positive (Two's complement)
    
positive_diff:
    ; AX now holds the seek time for this step
    add total_mov, ax   ; Accumulate total head movement
    
    ; --- Print step information ---
    push ax             ; Save the current seek time
    
    lea dx, msg_step
    mov ah, 09h
    int 21h
    
    mov ax, di          ; Load current track
    call print_num      ; Print track number
    
    lea dx, msg_step_mov
    mov ah, 09h
    int 21h
    
    pop ax              ; Restore seek time
    call print_num      ; Print seek time for this step
    ; ------------------------------
    
    mov bx, di          ; previous_position = current_request
    add si, 2           ; Move to next request in array
    pop cx              ; Restore outer loop counter
    loop fcfs_loop

    ; Print total head movement message
    lea dx, msg_total
    mov ah, 09h
    int 21h
    
    ; Print the accumulated total
    mov ax, total_mov
    call print_num
    
    ; --- Calculate and Print Average ---
    lea dx, msg_avg
    mov ah, 09h
    int 21h
    
    mov ax, total_mov
    mov dx, 0           ; Clear high word of dividend (DX:AX)
    mov bx, n
    cmp bx, 0
    je skip_avg         ; Prevent division by zero if n=0
    div bx              ; AX = total_mov / n
    
    call print_num      ; Print average
skip_avg:
    
    ; Print newline
    lea dx, newline
    mov ah, 09h
    int 21h

    ; Exit program
    mov ah, 4Ch
    int 21h
main endp

; Subroutine: Read a multi-digit number into AX
read_num proc
    push bx
    push cx
    mov bx, 0       ; Result accumulator
read_char:
    mov ah, 01h
    int 21h
    cmp al, 0Dh     ; Check for Enter (Carriage Return)
    je end_read
    cmp al, ' '     ; Check for Space
    je end_read
    
    sub al, '0'     ; Convert ASCII to integer
    mov cl, al
    mov ch, 0
    
    mov ax, bx
    mov dx, 10
    mul dx          ; Multiply accumulator by 10
    add ax, cx      ; Add new digit
    mov bx, ax
    jmp read_char
end_read:
    mov ax, bx      ; Store final result in AX
    pop cx
    pop bx
    ret
read_num endp

; Subroutine: Print a number in AX
print_num proc
    push ax
    push bx
    push cx
    push dx
    
    mov cx, 0       ; Digit counter
    mov bx, 10      ; Divisor
divide_loop:
    mov dx, 0
    div bx          ; AX = AX / 10, DX = AX % 10
    push dx         ; Push remainder (digit) onto stack
    inc cx          ; Increment digit count
    cmp ax, 0
    jne divide_loop
    
print_digits:
    pop dx          ; Get digit from stack
    add dl, '0'     ; Convert to ASCII
    mov ah, 02h
    int 21h
    loop print_digits
    
    pop dx
    pop cx
    pop bx
    pop ax
    ret
print_num endp

end main
