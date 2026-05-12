; Tile maze — walk through a maze with collision detection
; All sprites are in the sprite sheet (edit with F2)
; Sprite 0 (0x4000): player
; Sprite 1 (0x4040): floor tile
; Sprite 2 (0x4080): wall tile

.CONST PLAYER_X  = 5
.CONST PLAYER_Y  = 6
.CONST FLAGS_ADDR = 0
.CONST MAP_ADDR   = 210

init:
    PAL 0   0   0   0    ; transparent
    PAL 1  60  60  60    ; floor
    PAL 2 120 120 120    ; wall outer
    PAL 3 255 100 100    ; player
    PAL 4 200 200 200    ; wall highlight

    CPY FLAGS_ADDR, tile_flags, 2
    CPY MAP_ADDR, map_start, 256

    MOV R0, 16
    STM PLAYER_X, R0
    MOV R0, 16
    STM PLAYER_Y, R0

    SNDV 440 10 30
    JMP loop

loop:
    CLS
    NOSND

    MOV R0, 0
    MOV R1, 0
    MOV R2, tile_sprite
    MOV R3, MAP_ADDR
    TIL R0 R1 R2 R3 16 16

    LDM R0, PLAYER_X
    LDM R1, PLAYER_Y
    MOV R2, player_sprite
    SPT R0 R1 R2

    ; LEFT
    IN R0, 2
    JZ R0, @move_right
    LDM R1, PLAYER_X
    DEC R1
    LDM R2, PLAYER_Y
    MOV R3, MAP_ADDR
    TAT R0 R1 R2 R3 16
    MOV R2, FLAGS_ADDR
    TSD R3 R0 R2
    JNZ R3, @move_right
    LDM R2, PLAYER_Y
    ADD R2, 7
    MOV R3, MAP_ADDR
    TAT R0 R1 R2 R3 16
    MOV R2, FLAGS_ADDR
    TSD R3 R0 R2
    JNZ R3, @move_right
    STM PLAYER_X, R1

@move_right:
    IN R0, 3
    JZ R0, @move_up
    LDM R1, PLAYER_X
    ADD R1, 8
    LDM R2, PLAYER_Y
    MOV R3, MAP_ADDR
    TAT R0 R1 R2 R3 16
    MOV R2, FLAGS_ADDR
    TSD R3 R0 R2
    JNZ R3, @move_up
    LDM R2, PLAYER_Y
    ADD R2, 7
    MOV R3, MAP_ADDR
    TAT R0 R1 R2 R3 16
    MOV R2, FLAGS_ADDR
    TSD R3 R0 R2
    JNZ R3, @move_up
    LDM R1, PLAYER_X
    ADD R1, 1
    STM PLAYER_X, R1

@move_up:
    IN R0, 0
    JZ R0, @move_down
    LDM R1, PLAYER_X
    LDM R2, PLAYER_Y
    DEC R2
    MOV R3, MAP_ADDR
    TAT R0 R1 R2 R3 16
    MOV R1, FLAGS_ADDR
    TSD R3 R0 R1
    JNZ R3, @move_down
    LDM R1, PLAYER_X
    ADD R1, 7
    MOV R3, MAP_ADDR
    TAT R0 R1 R2 R3 16
    MOV R1, FLAGS_ADDR
    TSD R3 R0 R1
    JNZ R3, @move_down
    STM PLAYER_Y, R2

@move_down:
    IN R0, 1
    JZ R0, @end_loop
    LDM R1, PLAYER_X
    LDM R2, PLAYER_Y
    ADD R2, 8
    MOV R3, MAP_ADDR
    TAT R0 R1 R2 R3 16
    MOV R1, FLAGS_ADDR
    TSD R3 R0 R1
    JNZ R3, @end_loop
    LDM R1, PLAYER_X
    ADD R1, 7
    MOV R3, MAP_ADDR
    TAT R0 R1 R2 R3 16
    MOV R1, FLAGS_ADDR
    TSD R3 R0 R1
    JNZ R3, @end_loop
    LDM R2, PLAYER_Y
    ADD R2, 1
    STM PLAYER_Y, R2

@end_loop:
    WAIT
    JMP loop

tile_flags:
    .DB 0, 1

map_start:
    .DB 1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1
    .DB 1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1
    .DB 1,0,1,1,1,1,1,1,1,1,1,1,1,1,0,1
    .DB 1,0,1,0,0,0,0,0,0,0,0,0,0,1,0,1
    .DB 1,0,1,0,1,1,1,1,1,1,1,1,0,1,0,1
    .DB 1,0,1,0,1,0,0,0,0,0,0,1,0,1,0,1
    .DB 1,0,1,0,1,0,1,1,1,1,0,1,0,1,0,1
    .DB 1,0,1,0,1,0,1,0,0,1,0,1,0,1,0,1
    .DB 1,0,1,0,1,0,1,0,0,1,0,1,0,1,0,1
    .DB 1,0,1,0,1,0,1,1,1,1,0,1,0,1,0,1
    .DB 1,0,1,0,1,0,0,0,0,0,0,1,0,1,0,1
    .DB 1,0,1,0,1,1,1,1,1,1,1,1,0,1,0,1
    .DB 1,0,1,0,0,0,0,0,0,0,0,0,0,1,0,1
    .DB 1,0,1,1,1,1,1,1,1,1,1,1,1,1,0,1
    .DB 1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1
    .DB 1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1

.BEGIN_SPRITE_SHEET
player_sprite:
    .DB 0,0,3,3,3,3,0,0
    .DB 0,3,3,3,3,3,3,0
    .DB 3,3,0,3,3,0,3,3
    .DB 3,3,3,3,3,3,3,3
    .DB 3,3,0,3,3,0,3,3
    .DB 3,0,3,3,3,3,0,3
    .DB 0,3,3,0,0,3,3,0
    .DB 0,0,3,3,3,3,0,0

tile_sprite:
    ; Tile 0: floor
    .DB 1,1,1,1,1,1,1,1
    .DB 1,1,1,1,1,1,1,1
    .DB 1,1,1,1,1,1,1,1
    .DB 1,1,1,1,1,1,1,1
    .DB 1,1,1,1,1,1,1,1
    .DB 1,1,1,1,1,1,1,1
    .DB 1,1,1,1,1,1,1,1
    .DB 1,1,1,1,1,1,1,1
    ; Tile 1: wall
    .DB 2,2,2,2,2,2,2,2
    .DB 2,4,4,4,4,4,4,2
    .DB 2,4,2,2,2,2,4,2
    .DB 2,4,2,2,2,2,4,2
    .DB 2,4,2,2,2,2,4,2
    .DB 2,4,2,2,2,2,4,2
    .DB 2,4,4,4,4,4,4,2
    .DB 2,2,2,2,2,2,2,2
.END_SPRITE_SHEET
