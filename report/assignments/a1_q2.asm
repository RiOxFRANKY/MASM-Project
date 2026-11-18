; =============================================================================
; Q2: Store an array of 20 numbers in zig-zag (alternating less-than and
;     greater-than) order in memory location 4000H.
;     Zig-zag: a[0] < a[1] > a[2] < a[3] > a[4] ...
; =============================================================================
; Assembler: MASM (8086) - EXE format
; Approach: Bubble-sort style single pass with conditional swaps.
;   - At even indices (0, 2, 4...): ensure a[i] < a[i+1], else swap
;   - At odd indices (1, 3, 5...):  ensure a[i] > a[i+1], else swap
; =============================================================================

.MODEL SMALL
.STACK 100H

.DATA
    MSG_PROMPT DB 0DH, 0AH, 'Enter 20 numbers (0-9) without spaces: $'
    ARR DB 20 DUP(0)
    COUNT EQU 20

    ; Messages for output
    MSG DB 0DH, 0AH, 'Zig-Zag Array at 4000H: $'
    SPACE DB ', $'

.CODE
MAIN PROC
    MOV AX, @DATA
    MOV DS, AX

    ; --- Read 20 numbers from user into ARR ---
    LEA DX, MSG_PROMPT
    MOV AH, 09H
    INT 21H

    LEA SI, ARR
    MOV CX, COUNT
READ_LOOP:
    MOV AH, 01H             ; Read single character from console
    INT 21H
    SUB AL, 30H             ; Convert ASCII digit to raw number
    MOV [SI], AL
    INC SI
    LOOP READ_LOOP

    ; --- Step 1: Copy array to destination at 4000H ---
    ; We manually point DI to 4000H instead of using ORG in .DATA
    ; to avoid creating massive executable files filled with empty padding.
    LEA SI, ARR
    MOV DI, 4000H
    MOV CX, COUNT
COPY_LOOP:
    MOV AL, [SI]
    MOV [DI], AL
    INC SI
    INC DI
    LOOP COPY_LOOP

    ; --- Step 2: Zig-zag sort in-place at 4000H ---
    MOV SI, 4000H
    MOV CX, COUNT - 1       ; 19 pairs to check (i=0 to 18)
    XOR BX, BX              ; BX = index counter (0 = even position)

ZIGZAG_LOOP:
    MOV AL, [SI]             ; a[i]
    MOV AH, [SI + 1]        ; a[i+1]

    TEST BX, 01H            ; Check if index is even or odd
    JNZ ODD_INDEX            ; If odd index, jump

EVEN_INDEX:
    ; Even index: need a[i] < a[i+1]
    CMP AL, AH
    JBE NO_SWAP              ; Already a[i] <= a[i+1], skip
    JMP DO_SWAP

ODD_INDEX:
    ; Odd index: need a[i] > a[i+1]
    CMP AL, AH
    JAE NO_SWAP              ; Already a[i] >= a[i+1], skip

DO_SWAP:
    MOV [SI], AH             ; Swap a[i] and a[i+1]
    MOV [SI + 1], AL

NO_SWAP:
    INC SI                   ; Move to next element
    INC BX                   ; Increment index counter
    LOOP ZIGZAG_LOOP


    ; --- Step 3: Print Results to Screen ---
    LEA DX, MSG
    MOV AH, 09H
    INT 21H

    MOV SI, 4000H           ; Point to sorted array
    MOV CX, COUNT           ; 20 elements to print

PRINT_LOOP:
    MOV AL, [SI]            ; Get element
    XOR AH, AH              ; Clear AH so AX = AL
    CALL PRINT_NUM          ; Print the number

    CMP CX, 1               ; Don't print comma after the last element
    JE SKIP_COMMA
    
    LEA DX, SPACE           ; Print comma and space
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
