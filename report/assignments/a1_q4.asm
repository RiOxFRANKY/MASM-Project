; =============================================================================
; Q4: Basic Calculator for Arithmetic Operations (+, -, *, /)
;     Reads two single-digit numbers and an operator from the user.
;     Performs the operation and displays the result.
; =============================================================================
; Assembler: MASM (8086)
; =============================================================================

.MODEL SMALL
.STACK 100H

.DATA

    MSG_NUM1  DB 0DH, 0AH, 'Enter first number (0-9): $'
    MSG_OP    DB 0DH, 0AH, 'Enter operator (+, -, *, /): $'
    MSG_NUM2  DB 0DH, 0AH, 'Enter second number (0-9): $'
    MSG_RES   DB 0DH, 0AH, 'Result: $'
    MSG_ERR   DB 0DH, 0AH, 'Error: Invalid operator!$'
    MSG_DIV0  DB 0DH, 0AH, 'Error: Division by zero!$'
    NEWLINE   DB 0DH, 0AH, '$'

    NUM1 DB ?
    NUM2 DB ?
    OPER DB ?

.CODE
MAIN PROC
    MOV AX, @DATA
    MOV DS, AX

    ; --- Read first number ---
    LEA DX, MSG_NUM1
    MOV AH, 09H
    INT 21H

    MOV AH, 01H             ; Read character
    INT 21H
    SUB AL, 30H              ; Convert ASCII to number
    MOV NUM1, AL

    ; --- Read operator ---
    LEA DX, MSG_OP
    MOV AH, 09H
    INT 21H

    MOV AH, 01H
    INT 21H
    MOV OPER, AL

    ; --- Read second number ---
    LEA DX, MSG_NUM2
    MOV AH, 09H
    INT 21H

    MOV AH, 01H
    INT 21H
    SUB AL, 30H
    MOV NUM2, AL

    ; --- Display "Result: " ---
    LEA DX, MSG_RES
    MOV AH, 09H
    INT 21H

    ; --- Perform operation based on operator ---
    MOV AL, NUM1
    MOV BL, NUM2

    CMP OPER, '+'
    JE DO_ADD
    CMP OPER, '-'
    JE DO_SUB
    CMP OPER, '*'
    JE DO_MUL
    CMP OPER, '/'
    JE DO_DIV

    ; Invalid operator
    LEA DX, MSG_ERR
    MOV AH, 09H
    INT 21H
    JMP EXIT_PROG

DO_ADD:
    ADD AL, BL               ; AL = NUM1 + NUM2
    XOR AH, AH              ; Clear AH (result in AX)
    JMP DISPLAY_RESULT

DO_SUB:
    SUB AL, BL               ; AL = NUM1 - NUM2
    XOR AH, AH
    JMP DISPLAY_RESULT

DO_MUL:
    MUL BL                   ; AX = AL * BL
    JMP DISPLAY_RESULT

DO_DIV:
    CMP BL, 0
    JE DIV_BY_ZERO

    XOR AH, AH              ; Clear AH for division
    DIV BL                   ; AL = AX / BL, AH = remainder
    XOR AH, AH              ; We display only quotient
    JMP DISPLAY_RESULT

DIV_BY_ZERO:
    LEA DX, MSG_DIV0
    MOV AH, 09H
    INT 21H
    JMP EXIT_PROG

; -------------------------------------------------
; DISPLAY_RESULT: Display number in AX as decimal
; Handles numbers 0-99 for single-digit inputs
; -------------------------------------------------
DISPLAY_RESULT:
    ; AX contains the result (0 to 81 max for mul, 0-18 for add)
    ; Convert to decimal and display
    MOV BX, AX               ; Save result

    ; Check if negative (only for subtraction)
    TEST BX, 8000H
    JZ NOT_NEGATIVE
    ; Display minus sign
    MOV DL, '-'
    MOV AH, 02H
    INT 21H
    NEG BX                   ; Make positive

NOT_NEGATIVE:
    MOV AX, BX

    ; Divide by 100 to handle 3 digits (max result is 81*1=81)
    ; Simple approach: divide by 10 repeatedly
    XOR CX, CX              ; Digit counter
    MOV BX, 10

PUSH_DIGITS:
    XOR DX, DX
    DIV BX                   ; AX = quotient, DX = remainder
    PUSH DX                  ; Push remainder (digit) on stack
    INC CX
    CMP AX, 0
    JNE PUSH_DIGITS

PRINT_DIGITS:
    POP DX                   ; Pop digit
    ADD DL, 30H              ; Convert to ASCII
    MOV AH, 02H
    INT 21H
    LOOP PRINT_DIGITS

EXIT_PROG:
    MOV AH, 4CH
    INT 21H

MAIN ENDP
END MAIN
