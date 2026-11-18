.model small
.stack 100h

.data
    prompt_msg      db 'Enter password: $'
    success_msg     db 10, 13, 'Access Granted! System Booting...', 10, 13, '$'
    fail_msg        db 10, 13, 'Access Denied! Incorrect Password.', 10, 13, '$'
    lock_msg        db 10, 13, 'SYSTEM LOCKED! Max retries exceeded.', 10, 13, '$'
    
    hardcoded_pass  db 'secret'
    pass_len        equ $ - hardcoded_pass
    
    input_buffer    db 20 dup(0)
    attempts        db 3

.code
main proc
    ; Initialize data segments
    mov ax, @data
    mov ds, ax
    mov es, ax      ; ES needed for string comparison (cmpsb)

retry_loop:
    ; Check if attempts reached 0
    cmp attempts, 0
    je system_lock

    ; Display prompt
    mov ah, 09h
    lea dx, prompt_msg
    int 21h

    ; Initialize buffer pointer and counter
    lea di, input_buffer
    mov cx, 0       ; Character count

input_loop:
    ; Read character using BIOS interrupt 16h
    mov ah, 00h
    int 16h         ; AL = ASCII character

    cmp al, 0Dh     ; Check if ENTER key is pressed (Carriage Return)
    je check_password

    cmp al, 08h     ; Handle backspace (optional, but good practice)
    je handle_backspace

    ; Store character in buffer and increment pointer & counter
    mov [di], al
    inc di
    inc cx
    jmp input_loop

handle_backspace:
    cmp cx, 0       ; If nothing to backspace, ignore
    je input_loop
    dec di          ; Move pointer back
    dec cx          ; Decrement count
    jmp input_loop

check_password:
    ; Check if lengths match first
    cmp cx, pass_len
    jne password_incorrect

    ; Compare strings byte by byte
    lea si, hardcoded_pass
    lea di, input_buffer
    mov cx, pass_len
    cld             ; Clear direction flag (auto-increment SI and DI)
    repe cmpsb      ; Compare CX bytes at DS:SI and ES:DI
    jne password_incorrect

    ; Passwords match -> Success
    mov ah, 09h
    lea dx, success_msg
    int 21h
    jmp exit_program

password_incorrect:
    ; Display fail message
    mov ah, 09h
    lea dx, fail_msg
    int 21h

    ; Decrement attempts and try again
    dec attempts
    jmp retry_loop

system_lock:
    ; Display lock message
    mov ah, 09h
    lea dx, lock_msg
    int 21h

    ; Simulate system lock (infinite loop)
lock_halt:
    cli             ; Disable interrupts
    hlt             ; Halt processor
    jmp lock_halt   ; Loop indefinitely to "lock"

exit_program:
    ; Terminate program normally
    mov ah, 4Ch
    int 21h
main endp
end main