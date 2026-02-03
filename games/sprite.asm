; Initialize sprite data in memory at address 128
; We use a loop to generate a pattern
; Registers:
; R0: Address pointer
; R1: Color value
; R2: Loop counter

MOV R0, 128 ; Start memory address for sprite
MOV R1, 1   ; Initial color
MOV R2, 64  ; Loop 64 times (8x8 pixels)

init_loop:
    STMI R0, R1 ; Store val R1 to addr in R0
    ADD R0, 1   ; Next addr
    ADD R1, 1   ; Next color
    DEC R2      ; Decrement loop counter
    JNZ R2, init_loop ; Continue if R2 != 0

; Draw the sprite
CLS
MOV R0, 10  ; Screen X
MOV R1, 10  ; Screen Y
MOV R2, 128 ; RAM Address of sprite

SPT R0, R1, R2

infinite_loop:
    WAIT
    JMP infinite_loop
