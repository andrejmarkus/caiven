init:
    PAL 1 200 200 200
    PAL 2 160 160 160

    CPY 0, tile, 64
    CPY 64, map, 256

loop:
    CLS
    MOV R0, 0
    MOV R1, 0
    MOV R2, 0
    MOV R3, 64

    TIL R0 R1 R2 R3 16 16

    WAIT
    JMP loop

tile:
    .DB 2,2,2,2,2,2,2,2
    .DB 2,1,1,1,1,1,1,2
    .DB 2,1,1,1,1,1,1,2
    .DB 2,1,1,1,1,1,1,2
    .DB 2,1,1,1,1,1,1,2
    .DB 2,1,1,1,1,1,1,2
    .DB 2,1,1,1,1,1,1,2
    .DB 2,2,2,2,2,2,2,2

map:
    .FILL 256, 0