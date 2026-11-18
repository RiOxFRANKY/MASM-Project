; =============================================================================
; Q3: Convert a hexadecimal number into its equivalent ASCII string.
;     Example: 4Bh -> displays "4B"
; =============================================================================
; Assembler: MASM (8086) - EXE format
; Approach: Extract each nibble (4 bits) of the hex byte, convert to ASCII
;           character ('0'-'9' or 'A'-'F'), and display using DOS INT 21H.
; =============================================================================

.MODEL SMALL
.STACK 100H

.DATA
    MSG_PROMPT DB 'Enter a 2-character Hex number (e.g. 4B): $'
    HEX_NUM DB 0            ; Store the converted byte here
    MSG2 DB 0DH, 0AH, 'ASCII Character equivalent: $'

.CODE
MAIN PROC
    MOV AX, @DATA
    MOV DS, AX

    ; --- Read Input ---
    LEA DX, MSG_PROMPT
    MOV AH, 09H
    INT 21H

    ; Read first character (high nibble)
    MOV AH, 01H
    INT 21H
    CALL ASCII_TO_NIBBLE    ; Convert to raw 0-F
    MOV CL, 4
    SHL AL, CL              ; Shift to high nibble position
    MOV HEX_NUM, AL

    ; Read second character (low nibble)
    MOV AH, 01H
    INT 21H
    CALL ASCII_TO_NIBBLE    ; Convert to raw 0-F
    OR HEX_NUM, AL          ; Combine with high nibble


    ; --- Process Output ---
    ; Display the result message
    LEA DX, MSG2
    MOV AH, 09H
    INT 21H

    ; Display the actual ASCII character
    MOV DL, HEX_NUM
    MOV AH, 02H
    INT 21H

    ; Exit to DOS
    MOV AH, 4CH
    INT 21H
MAIN ENDP

; -------------------------------------------------
; ASCII_TO_NIBBLE: Convert ASCII char to 0-F
; -------------------------------------------------
ASCII_TO_NIBBLE PROC
    CMP AL, '9'
    JBE ATN_DIGIT
    ; If > '9', assume 'A'-'F' (or 'a'-'f')
    ; We can force uppercase by clearing the lowercase bit
    AND AL, 0DFH
    SUB AL, 37H             ; 'A' (41H) - 37H = 0AH
    RET
ATN_DIGIT:
    SUB AL, 30H             ; '0' (30H) - 30H = 00H
    RET
ASCII_TO_NIBBLE ENDP

END MAIN
