.model small
.stack 100h

.data
    prompt    db 'Enter bracket expression: $'
    msg_bal   db 10, 13, 'Balanced$', 0
    msg_unbal db 10, 13, 'Non-Balanced$', 0
    
    ; Simulated Stack
    stack_arr db 100 dup(0)
    top       dw 0

.code
main proc
    ; Initialize data segment
    mov ax, @data
    mov ds, ax

    ; Print prompt
    lea dx, prompt
    mov ah, 09h
    int 21h

read_loop:
    ; Read a single character from keyboard
    mov ah, 01h
    int 21h
    
    cmp al, 0Dh     ; Is it the Enter key (Carriage Return)?
    je end_input
    
    ; Check for opening brackets (Push)
    cmp al, '('
    je push_char
    cmp al, '{'
    je push_char
    cmp al, '['
    je push_char
    
    ; Check for closing brackets (Pop and Compare)
    cmp al, ')'
    je check_paren
    cmp al, '}'
    je check_brace
    cmp al, ']'
    je check_bracket
    
    ; Ignore any other characters (like spaces or letters)
    jmp read_loop

push_char:
    mov bx, top
    mov stack_arr[bx], al   ; Push character onto simulated stack
    inc top                 ; Increment stack pointer
    jmp read_loop

check_paren:
    call pop_char
    cmp ah, 0               ; Check status flag: was stack empty?
    je unbalanced           ; If empty, it's unbalanced
    cmp al, '('             ; Did we pop the matching opening bracket?
    jne unbalanced          ; If not, it's unbalanced
    jmp read_loop

check_brace:
    call pop_char
    cmp ah, 0
    je unbalanced
    cmp al, '{'
    jne unbalanced
    jmp read_loop

check_bracket:
    call pop_char
    cmp ah, 0
    je unbalanced
    cmp al, '['
    jne unbalanced
    jmp read_loop

end_input:
    ; The user finished typing. We must check if the stack is completely empty.
    cmp top, 0
    jne unbalanced          ; If top > 0, there are unclosed opening brackets left
    
    ; If empty, the expression is perfectly balanced
    lea dx, msg_bal
    mov ah, 09h
    int 21h
    jmp exit_prog

unbalanced:
    ; Output Non-Balanced message
    lea dx, msg_unbal
    mov ah, 09h
    int 21h

exit_prog:
    ; Terminate program
    mov ah, 4Ch
    int 21h
main endp

; ---------------------------------------------------------
; Subroutine: pop_char
; Pops a character from the simulated stack into AL.
; Returns AH = 1 on success.
; Returns AH = 0 on failure (stack underflow).
; ---------------------------------------------------------
pop_char proc
    cmp top, 0
    je pop_empty            ; Check for stack underflow
    
    dec top                 ; Decrement pointer
    mov bx, top
    mov al, stack_arr[bx]   ; Retrieve character
    mov ah, 1               ; Set success flag
    ret
    
pop_empty:
    mov ah, 0               ; Set failure flag
    ret
pop_char endp

end main
