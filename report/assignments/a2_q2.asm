.model small
.stack 100h

.data
    prompt    db 10, 13, 'MYSHELL> $'
    err_msg   db 10, 13, 'Invalid command.$'
    err_file  db 10, 13, 'File error (does it exist?).$'
    msg_dir   db 10, 13, 'Directory Listing:', 10, 13, \
                         '  FILE1.TXT', 10, 13, \
                         '  NOTES.TXT', 10, 13, \
                         '  DATA.BIN$'
    msg_copy  db 10, 13, '1 file(s) copied.$'

    ; Input buffer for AH=0Ah (Max 60 chars, actual length, data)
    input_buf db 60, 0, 60 dup(0)
    
    ; Buffers to hold parsed filenames (ASCIIZ strings)
    file1     db 30 dup(0)
    file2     db 30 dup(0)
    
    ; File handles
    handle1   dw ?
    handle2   dw ?
    
    ; Buffer for reading file contents (128 bytes)
    read_buf  db 128 dup(0)

.code
main proc
    ; Initialize data segment
    mov ax, @data
    mov ds, ax
    mov es, ax

shell_loop:
    ; 1. Print Prompt
    lea dx, prompt
    mov ah, 09h
    int 21h
    
    ; 2. Read user input using buffered input (AH=0Ah)
    lea dx, input_buf
    mov ah, 0Ah
    int 21h

    ; 3. Check input length (byte at offset 1 = actual chars read)
    mov cl, input_buf[1]
    mov ch, 0
    cmp cx, 0
    je shell_loop           ; If empty input, reprint prompt

    ; 4. Null-terminate the string (replace trailing 0Dh with 00h)
    lea si, input_buf[2]    ; SI -> start of input data
    add si, cx              ; SI -> position of the 0Dh byte
    mov byte ptr [si], 0    ; Replace 0Dh with null terminator

    ; 5. Reset SI to beginning of input, skip any leading spaces
    lea si, input_buf[2]
    call skip_sp            ; Skip leading whitespace

    ; 6. Check if remaining input is empty (all spaces entered)
    cmp byte ptr [si], 0
    je shell_loop

parse_command:
    ; --- Check for EXIT ---
    ; Must match exactly "EXIT" followed by space or null
    cmp byte ptr [si], 'E'
    jne not_exit
    cmp byte ptr [si+1], 'X'
    jne not_exit
    cmp byte ptr [si+2], 'I'
    jne not_exit
    cmp byte ptr [si+3], 'T'
    jne not_exit
    mov al, [si+4]
    cmp al, 0              ; Null terminator -> valid
    je do_exit
    cmp al, ' '            ; Space -> valid (trailing args ignored)
    je do_exit
    jmp not_exit           ; e.g. "EXITCODE" should not match EXIT
do_exit:
    mov ah, 4Ch
    int 21h
not_exit:

    ; --- Check for DIR ---
    ; Must match exactly "DIR" followed by space or null
    cmp byte ptr [si], 'D'
    jne not_dir
    cmp byte ptr [si+1], 'I'
    jne not_dir
    cmp byte ptr [si+2], 'R'
    jne not_dir
    mov al, [si+3]
    cmp al, 0
    je do_dir
    cmp al, ' '
    je do_dir
    jmp not_dir
do_dir:
    lea dx, msg_dir
    mov ah, 09h
    int 21h
    jmp shell_loop
not_dir:

    ; --- Check for TYPE ---
    ; Must match "TYPE" followed by a space (filename required)
    cmp byte ptr [si], 'T'
    jne not_type
    cmp byte ptr [si+1], 'Y'
    jne not_type
    cmp byte ptr [si+2], 'P'
    jne not_type
    cmp byte ptr [si+3], 'E'
    jne not_type
    cmp byte ptr [si+4], ' '  ; Must be followed by a space
    jne not_type
    add si, 4               ; SI now points to the space after "TYPE"
    jmp do_type
not_type:

    ; --- Check for COPY ---
    ; Must match "COPY" followed by a space (arguments required)
    cmp byte ptr [si], 'C'
    jne not_copy
    cmp byte ptr [si+1], 'O'
    jne not_copy
    cmp byte ptr [si+2], 'P'
    jne not_copy
    cmp byte ptr [si+3], 'Y'
    jne not_copy
    cmp byte ptr [si+4], ' '  ; Must be followed by a space
    jne not_copy
    add si, 4               ; SI now points to the space after "COPY"
    jmp do_copy
not_copy:

invalid_cmd:
    ; Command not recognized - print error and loop back
    lea dx, err_msg
    mov ah, 09h
    int 21h
    jmp shell_loop

; =========================================
; TYPE <filename>
; Opens a file and prints its contents to screen
; =========================================
do_type:
    ; Skip space(s) after "TYPE" to get to the filename
    call skip_sp
    
    ; Check if a filename was actually provided
    cmp byte ptr [si], 0
    je invalid_cmd          ; No filename given

    ; Copy filename from input buffer into file1 (stop at space or null)
    lea di, file1
type_copy_loop:
    mov al, [si]
    cmp al, ' '
    je type_copy_done
    cmp al, 0
    je type_copy_done
    mov [di], al
    inc si
    inc di
    jmp type_copy_loop
type_copy_done:
    mov byte ptr [di], 0    ; Null-terminate file1

    ; Open file for reading (AH=3Dh, AL=0 -> read-only)
    lea dx, file1
    mov ah, 3Dh
    mov al, 0
    int 21h
    jc file_error           ; Carry set = error (file not found, etc.)
    mov handle1, ax         ; Save the returned file handle

type_read_loop:
    ; Read 1 byte at a time from the file (AH=3Fh)
    mov ah, 3Fh
    mov bx, handle1
    lea dx, read_buf
    mov cx, 1
    int 21h
    jc type_read_error
    cmp ax, 0               ; AX=0 means End-Of-File
    je type_close

    ; Print the byte using DOS character output (AH=02h)
    mov dl, read_buf
    mov ah, 02h
    int 21h
    jmp type_read_loop

type_read_error:
    ; Close handle before reporting error
    mov ah, 3Eh
    mov bx, handle1
    int 21h
    jmp file_error

type_close:
    ; Close file handle (AH=3Eh)
    mov ah, 3Eh
    mov bx, handle1
    int 21h
    jmp shell_loop

; =========================================
; COPY <source> <destination>
; Copies contents of source file to destination
; =========================================
do_copy:
    ; Skip space(s) after "COPY" to get to source filename
    call skip_sp

    ; Check if source filename is present
    cmp byte ptr [si], 0
    je invalid_cmd

    ; Copy source filename into file1
    lea di, file1
copy_src_loop:
    mov al, [si]
    cmp al, ' '
    je copy_src_done
    cmp al, 0
    je invalid_cmd          ; Reached end of string: destination is missing!
    mov [di], al
    inc si
    inc di
    jmp copy_src_loop
copy_src_done:
    mov byte ptr [di], 0    ; Null-terminate source filename

    ; Skip space(s) between source and destination filenames
    call skip_sp

    ; Check if destination filename is present
    cmp byte ptr [si], 0
    je invalid_cmd

    ; Copy destination filename into file2
    lea di, file2
copy_dest_loop:
    mov al, [si]
    cmp al, ' '
    je copy_dest_done
    cmp al, 0
    je copy_dest_done
    mov [di], al
    inc si
    inc di
    jmp copy_dest_loop
copy_dest_done:
    mov byte ptr [di], 0    ; Null-terminate destination filename

    ; 1. Open source file for reading (AH=3Dh, AL=0)
    lea dx, file1
    mov ah, 3Dh
    mov al, 0
    int 21h
    jc file_error
    mov handle1, ax

    ; 2. Create (or overwrite) destination file (AH=3Ch, CX=0 = normal attr)
    lea dx, file2
    mov ah, 3Ch
    mov cx, 0
    int 21h
    jc copy_create_error    ; If dest create fails, close handle1 first
    mov handle2, ax

copy_read_loop:
    ; 3. Read a chunk from source (up to 128 bytes)
    mov ah, 3Fh
    mov bx, handle1
    lea dx, read_buf
    mov cx, 128
    int 21h
    jc copy_read_err
    cmp ax, 0               ; AX=0 -> End-Of-File on source
    je copy_close

    ; 4. Write that chunk to destination
    mov cx, ax              ; Write exactly as many bytes as were read
    mov ah, 40h
    mov bx, handle2
    lea dx, read_buf
    int 21h
    jc copy_write_err
    jmp copy_read_loop

copy_read_err:
    ; Close both handles, then report error
    mov ah, 3Eh
    mov bx, handle2
    int 21h
copy_create_error:
    mov ah, 3Eh
    mov bx, handle1
    int 21h
    jmp file_error

copy_write_err:
    mov ah, 3Eh
    mov bx, handle1
    int 21h
    mov ah, 3Eh
    mov bx, handle2
    int 21h
    jmp file_error

copy_close:
    ; 5. Close source handle
    mov ah, 3Eh
    mov bx, handle1
    int 21h
    ; 6. Close destination handle
    mov ah, 3Eh
    mov bx, handle2
    int 21h
    ; Print success message
    lea dx, msg_copy
    mov ah, 09h
    int 21h
    jmp shell_loop

file_error:
    lea dx, err_file
    mov ah, 09h
    int 21h
    jmp shell_loop

main endp

; =========================================
; Subroutine: skip_sp
; Purpose   : Advances SI past any space characters.
; Registers : SI is modified; AL is clobbered.
; =========================================
skip_sp proc near
sp_loop:
    mov al, [si]
    cmp al, ' '
    jne sp_done
    inc si
    jmp sp_loop
sp_done:
    ret
skip_sp endp

end main
