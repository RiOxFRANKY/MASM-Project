; =============================================================================
; Q1: Transfer a block of 10 data bytes from memory location 3000H to 4000H.
;     Before transfer, multiply each element by 5, then add 10.
; =============================================================================
; Assembler: MASM (8086)
; =============================================================================

.MODEL SMALL
.STACK 100H

.DATA
    ; Messages for output
    MSG_PROMPT DB 0DH, 0AH, 'Enter 10 numbers (0-9) without spaces: $'
    MSG DB 0DH, 0AH, 'Output Array at 4000H: $'
    SPACE DB ', $'

.CODE
MAIN PROC
    MOV AX, @DATA
    MOV DS, AX

    ; --- Read 10 numbers from user into 3000H ---
    LEA DX, MSG_PROMPT
    MOV AH, 09H
    INT 21H

    MOV SI, 3000H
    MOV CX, 0AH
READ_LOOP:
    MOV AH, 01H             ; Read single character from console
    INT 21H
    SUB AL, 30H             ; Convert ASCII digit to raw number
    MOV [SI], AL            ; Store at 3000H
    INC SI
    LOOP READ_LOOP


    ; --- Transfer and Process Data ---
    MOV SI, 3000H           ; SI -> source block (3000H)
    MOV DI, 4000H           ; DI -> destination block (4000H)
    MOV CX, 0AH             ; Counter = 10 elements

TRANSFER:
    MOV AL, [SI]            ; Load byte from source
    
    ; Multiply by 5
    MOV BL, 05H
    MUL BL                  ; AX = AL * BL
    
    ; Add 10
    ADD AL, 0AH
    
    MOV [DI], AL            ; Store result to destination at 4000H
    
    INC SI
    INC DI
    LOOP TRANSFER


    ; --- Print Results to Screen ---
    LEA DX, MSG
    MOV AH, 09H
    INT 21H

    MOV SI, 4000H           ; Point to destination array
    MOV CX, 0AH             ; 10 elements to print

PRINT_LOOP:
    MOV AL, [SI]            ; Get element
    XOR AH, AH              ; Clear AH so AX = AL
    CALL PRINT_NUM          ; Print the number

    CMP CX, 1               ; Don't print comma after the last element
    JE SKIP_COMMA
    
    LEA DX, SPACE
    MOV AH, 09H
    INT 21H
SKIP_COMMA:

    INC SI
    LOOP PRINT_LOOP

    ; Exit to DOS
    MOV AH, 4CH
    INT 21H
MAIN ENDP

; =============================================================================
; PRINT_NUM: Display a number in AX as decimal
; =============================================================================
PRINT_NUM PROC
    PUSH AX
    PUSH BX
    PUSH CX
    PUSH DX

    XOR CX, CX              ; Digit counter
    MOV BX, 10              ; Divisor

PN_PUSH:
    XOR DX, DX
    DIV BX                   ; AX = quotient, DX = remainder
    PUSH DX                  ; Push remainder (digit) on stack
    INC CX
    CMP AX, 0
    JNE PN_PUSH

PN_PRINT:
    POP DX                   ; Pop digit
    ADD DL, 30H              ; Convert to ASCII
    MOV AH, 02H
    INT 21H
    LOOP PN_PRINT

    POP DX
    POP CX
    POP BX
    POP AX
    RET
PRINT_NUM ENDP

END MAIN
