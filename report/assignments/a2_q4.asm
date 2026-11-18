.model small
.stack 100h

.data
    prompt_n     db 'Enter the number of elements (n): $'
    prompt_elem  db 'Enter element: $'
    msg_sorted   db 10, 13, 'Sorted array: $'
    newline      db 10, 13, '$'
    
    n            dw ?
    array        dw 100 dup(0)

.code
main proc
    ; Initialize data segment
    mov ax, @data
    mov ds, ax

    ; Prompt for n
    lea dx, prompt_n
    mov ah, 09h
    int 21h

    call read_num
    mov n, ax

    ; Print newline
    lea dx, newline
    mov ah, 09h
    int 21h

    ; Loop to read n elements
    mov cx, n
    lea si, array
read_loop:
    push cx         ; Preserve loop counter
    
    lea dx, prompt_elem
    mov ah, 09h
    int 21h
    
    call read_num
    mov [si], ax    ; Store element in array
    add si, 2       ; Move to next word (2 bytes)
    
    lea dx, newline
    mov ah, 09h
    int 21h
    
    pop cx          ; Restore loop counter
    loop read_loop

    ; Bubble sort
    mov cx, n
    dec cx          ; outer loop counter = n - 1
    jz skip_sort    ; if n=1, no sort needed

outer_loop:
    mov bx, cx      ; inner loop counter
    lea si, array
inner_loop:
    mov ax, [si]
    cmp ax, [si+2]
    jle no_swap     ; if array[i] <= array[i+1] (SIGNED), do nothing
    
    ; Swap elements
    mov dx, [si+2]
    mov [si+2], ax
    mov [si], dx
    
no_swap:
    add si, 2       ; move to next element
    dec bx
    jnz inner_loop
    loop outer_loop

skip_sort:
    ; Print sorted array message
    lea dx, msg_sorted
    mov ah, 09h
    int 21h

    ; Loop to print n elements
    mov cx, n
    lea si, array
print_loop:
    mov ax, [si]
    call print_num  ; Print the number
    
    ; Print space
    mov dl, ' '
    mov ah, 02h
    int 21h
    
    add si, 2       ; Move to next word
    loop print_loop

    ; Print newline
    lea dx, newline
    mov ah, 09h
    int 21h

    ; Exit program
    mov ah, 4Ch
    int 21h
main endp

; Subroutine: Read a multi-digit number (handles negatives) into AX
read_num proc
    push bx
    push cx
    push dx
    push di
    
    mov bx, 0       ; Result accumulator
    mov di, 0       ; Flag for negative (0 = pos, 1 = neg)

read_first_char:
    mov ah, 01h
    int 21h
    cmp al, 0Dh     ; Check for Enter
    je end_read
    cmp al, ' '     ; Check for Space
    je end_read
    cmp al, '-'     ; Check for negative sign
    jne process_digit
    mov di, 1       ; Set negative flag
    jmp read_char   ; Go read the actual digits

read_char:
    mov ah, 01h
    int 21h
    cmp al, 0Dh
    je end_read
    cmp al, ' '
    je end_read

process_digit:
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
    cmp di, 1       ; Was it negative?
    jne return_read
    neg bx          ; If yes, two's complement negate the result

return_read:
    mov ax, bx      ; Store final result in AX
    pop di
    pop dx
    pop cx
    pop bx
    ret
read_num endp

; Subroutine: Print a number in AX (handles negatives)
print_num proc
    push ax
    push bx
    push cx
    push dx
    
    ; Check if negative
    cmp ax, 0
    jge print_positive
    
    ; If negative, print minus sign and negate AX
    push ax
    mov dl, '-'
    mov ah, 02h
    int 21h
    pop ax
    neg ax          ; Make it positive for division

print_positive:
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
