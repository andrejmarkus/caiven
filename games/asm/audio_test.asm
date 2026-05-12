; Audio test — press buttons to trigger sounds
; UP:    high square wave
; DOWN:  low square wave
; LEFT:  noise burst
; RIGHT: noise sweep

; String RAM layout (12-13 chars each, no overlap)
.CONST ADDR_UP  = 200
.CONST ADDR_DN  = 212
.CONST ADDR_LT  = 226
.CONST ADDR_RT  = 238

init:
    PAL 0  10  10  20
    PAL 1 255 255 255

    ; 32 = ASCII space
    CPY ADDR_UP, str_up,    12
    CPY ADDR_DN, str_down,  13
    CPY ADDR_LT, str_left,  12
    CPY ADDR_RT, str_right, 12
    JMP loop

loop:
    CLS
    FILL 0
    NOSND
    NSTOP

    MOV R0, 4
    MOV R1, 20
    MOV R2, 1
    MOV R3, ADDR_UP
    TXT R0 R1 R2 R3 12

    MOV R1, 36
    MOV R3, ADDR_DN
    TXT R0 R1 R2 R3 13

    MOV R1, 52
    MOV R3, ADDR_LT
    TXT R0 R1 R2 R3 12

    MOV R1, 68
    MOV R3, ADDR_RT
    TXT R0 R1 R2 R3 12

    IN R0, 0
    JZ R0, @chk_down
    SNDV 880 15 3
    JMP @end

@chk_down:
    IN R0, 1
    JZ R0, @chk_left
    SNDV 220 15 3
    JMP @end

@chk_left:
    IN R0, 2
    JZ R0, @chk_right
    NSNDV 1000 15 3
    JMP @end

@chk_right:
    IN R0, 3
    JZ R0, @end
    NSNDV 200 10 3

@end:
    WAIT
    JMP loop

; 32 = ASCII space (space char literal ' ' breaks the tokenizer)
str_up:
    .DB 'U','P',':',32,'H','I','G','H',32,'S','N','D'
str_down:
    .DB 'D','O','W','N',':',32,'L','O','W',32,'S','N','D'
str_left:
    .DB 'L','E','F','T',':',32,'N','O','I','S','E',32
str_right:
    .DB 'R','I','G','H','T',':',32,'N','O','I','S','E'
