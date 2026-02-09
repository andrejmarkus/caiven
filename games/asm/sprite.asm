init:
    PAL 1 255 0 0
    PAL 2 0 255 0
    PAL 3 0 0 255

    CPY 32, sprite, 64

    JMP start

start:
    CLS
    MOV R0, 40
    MOV R1, 40
    MOV R2, 32

    SPT R0 R1 R2

infinite_loop:
    WAIT
    JMP infinite_loop

sprite:
    .DB 0,0,0,0,0,0,0,0
    .DB 0,0,1,1,1,1,0,0
    .DB 0,1,1,1,1,1,1,0
    .DB 0,1,1,1,1,1,1,0
    .DB 0,1,1,1,1,1,1,0
    .DB 0,1,1,1,1,1,1,0
    .DB 0,0,1,1,1,1,0,0
    .DB 0,0,0,0,0,0,0,0