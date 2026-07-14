; Tile maze — walk through a maze with collision detection
; All sprites are in the sprite sheet (edit with F2)
; Sprite 0: player, sprite 1: floor, sprite 2: wall
; Maze lives in map RAM: cells hold sprite ids, MAPD draws them.
; Wall solidity comes from sprite flags (FSET marks sprite 2 solid).

.CONST PLAYER_X = 5
.CONST PLAYER_Y = 6
.CONST MAZE_BUF = 256   ; staging buffer for maze data in work RAM
.CONST MAZE_W   = 16
.CONST MAZE_H   = 16
.CONST FLAG_SOLID = 1

init:
    PAL 0   0   0   0    ; transparent
    PAL 1  60  60  60    ; floor
    PAL 2 120 120 120    ; wall outer
    PAL 3 255 100 100    ; player
    PAL 4 200 200 200    ; wall highlight

    ; mark wall sprite (id 2) solid
    MOV R0, 2
    MOV R1, FLAG_SOLID
    FSET R0 R1

    ; stage maze data in work RAM, then write it into map RAM cell by cell
    CPY MAZE_BUF, maze_start, 256
    MOV R5, MAZE_BUF
    MOV R6, MAZE_W
    MOV R1, 0
@copy_row:
    MOV R0, 0
@copy_cell:
    LDMI R2, R5
    MSET R0 R1 R2
    ADD R5, 1
    ADD R0, 1
    SLT R2, R0, R6
    JNZ R2, @copy_cell
    ADD R1, 1
    SLT R2, R1, R6
    JNZ R2, @copy_row

    MOV R0, 8
    STM PLAYER_X, R0
    MOV R0, 8
    STM PLAYER_Y, R0

    SNDV 440 10 30
    JMP loop

loop:
    CLS
    NOSND

    ; draw the maze
    MOV R0, 0
    MOV R1, 0
    MOV R2, 0
    MOV R3, 0
    MOV R4, MAZE_W
    MOV R5, MAZE_H
    MAPD R0 R1 R2 R3 R4 R5

    ; draw the player (sprite id 0)
    MOV R0, 0
    LDM R1, PLAYER_X
    LDM R2, PLAYER_Y
    MOV R3, 0
    SPR R0 R1 R2 R3

    ; LEFT
    IN R0, 2
    JZ R0, @move_right
    LDM R6, PLAYER_X
    DEC R6
    LDM R7, PLAYER_Y
    MOVR R0, R6
    MOVR R1, R7
    JSR solid_at
    JNZ R2, @move_right
    MOVR R0, R6
    MOVR R1, R7
    ADD R1, 7
    JSR solid_at
    JNZ R2, @move_right
    STM PLAYER_X, R6

@move_right:
    IN R0, 3
    JZ R0, @move_up
    LDM R6, PLAYER_X
    ADD R6, 1
    LDM R7, PLAYER_Y
    MOVR R0, R6
    ADD R0, 7
    MOVR R1, R7
    JSR solid_at
    JNZ R2, @move_up
    MOVR R0, R6
    ADD R0, 7
    MOVR R1, R7
    ADD R1, 7
    JSR solid_at
    JNZ R2, @move_up
    STM PLAYER_X, R6

@move_up:
    IN R0, 0
    JZ R0, @move_down
    LDM R6, PLAYER_X
    LDM R7, PLAYER_Y
    DEC R7
    MOVR R0, R6
    MOVR R1, R7
    JSR solid_at
    JNZ R2, @move_down
    MOVR R0, R6
    ADD R0, 7
    MOVR R1, R7
    JSR solid_at
    JNZ R2, @move_down
    STM PLAYER_Y, R7

@move_down:
    IN R0, 1
    JZ R0, @end_loop
    LDM R6, PLAYER_X
    LDM R7, PLAYER_Y
    ADD R7, 1
    MOVR R0, R6
    MOVR R1, R7
    ADD R1, 7
    JSR solid_at
    JNZ R2, @end_loop
    MOVR R0, R6
    ADD R0, 7
    MOVR R1, R7
    ADD R1, 7
    JSR solid_at
    JNZ R2, @end_loop
    STM PLAYER_Y, R7

@end_loop:
    WAIT
    JMP loop

; solid_at: R0 = pixel x, R1 = pixel y → R2 = solid flag of the map cell there
solid_at:
    MOVR R2, R0
    SHR R2, 3
    MOVR R4, R1
    SHR R4, 3
    MGET R5 R2 R4
    FGET R2 R5
    RET

maze_start:
    ; cells are sprite ids: 1 = floor, 2 = wall
    .DB 2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2
    .DB 2,1,1,1,1,1,1,1,1,1,1,1,1,1,1,2
    .DB 2,1,2,2,2,2,2,2,2,2,2,2,2,2,1,2
    .DB 2,1,2,1,1,1,1,1,1,1,1,1,1,2,1,2
    .DB 2,1,2,1,2,2,2,2,2,2,2,2,1,2,1,2
    .DB 2,1,2,1,2,1,1,1,1,1,1,2,1,2,1,2
    .DB 2,1,2,1,2,1,2,2,2,2,1,2,1,2,1,2
    .DB 2,1,2,1,2,1,2,1,1,2,1,2,1,2,1,2
    .DB 2,1,2,1,2,1,2,1,1,2,1,2,1,2,1,2
    .DB 2,1,2,1,2,1,2,2,2,2,1,2,1,2,1,2
    .DB 2,1,2,1,2,1,1,1,1,1,1,2,1,2,1,2
    .DB 2,1,2,1,2,2,2,2,2,2,2,2,1,2,1,2
    .DB 2,1,2,1,1,1,1,1,1,1,1,1,1,2,1,2
    .DB 2,1,2,2,2,2,2,2,2,2,2,2,2,2,1,2
    .DB 2,1,1,1,1,1,1,1,1,1,1,1,1,1,1,2
    .DB 2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2

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
    ; Tile 1: floor
    .DB 1,1,1,1,1,1,1,1
    .DB 1,1,1,1,1,1,1,1
    .DB 1,1,1,1,1,1,1,1
    .DB 1,1,1,1,1,1,1,1
    .DB 1,1,1,1,1,1,1,1
    .DB 1,1,1,1,1,1,1,1
    .DB 1,1,1,1,1,1,1,1
    .DB 1,1,1,1,1,1,1,1
    ; Tile 2: wall
    .DB 2,2,2,2,2,2,2,2
    .DB 2,4,4,4,4,4,4,2
    .DB 2,4,2,2,2,2,4,2
    .DB 2,4,2,2,2,2,4,2
    .DB 2,4,2,2,2,2,4,2
    .DB 2,4,2,2,2,2,4,2
    .DB 2,4,4,4,4,4,4,2
    .DB 2,2,2,2,2,2,2,2
.END_SPRITE_SHEET
