; R0 = x, R1 = y, R2 = left input, R3 = right input
; Init state in RAM
MOV r0 10
STM 100 r0
MOV r1 10
STM 101 r1

loop:
    CLS
    ; Load state from RAM
    LDM r0 100
    LDM r1 101
    
    ; Read left and right input
    IN r2 2
    IN r3 3
    
    ; Check if left button is pressed
    JNZ r2 move_left
    
    ; Check if right button is pressed
    JNZ r3 move_right
    
    JMP draw

move_left:
    DEC r0
    STM 100 r0
    JMP draw

move_right:
    ADD r0 1
    STM 100 r0

draw:
    ; draw pixel at (R0, R1) with color red (255, 0, 0)
    DPXR r0 r1 255,0,0
    
    WAIT
    JMP loop
