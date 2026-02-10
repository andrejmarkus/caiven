init:
    ; Palette setup
    PAL 0 0 0 0       ; Transparent
    PAL 1 60 60 60    ; Floor
    PAL 2 120 120 120 ; Wall
    PAL 3 255 100 100 ; Player
    PAL 4 200 200 200 ; Wall Highlight

    ; Data layout in RAM:
    ; 0: Flags (2 bytes), 5: Player X, 6: Player Y
    ; 10: Player Sprite (64 bytes), 80: Tileset, 210: Map

    CPY 0, tile_flags, 2
    CPY 10, player_sprite, 64
    CPY 80, tileset, 128
    CPY 210, map_start, 256
    CPY 500, score_text, 5

    ; Initial position
    MOV R0, 16
    STM 5, R0
    MOV R0, 16
    STM 6, R0

loop:
    CLS
    NOSND
    
    ; Draw text
    MOV R0, 2
    MOV R1, 2
    MOV R2, 0
    MOV R3, 500
    TXT R0 R1 R2 R3 5

    ; Draw Map
    MOV R0, 0   ; X=0
    MOV R1, 0   ; Y=0
    MOV R2, 80  ; Tileset Address
    MOV R3, 210 ; Map Address
    TIL R0 R1 R2 R3 16 16

    ; Draw Player
    LDM R0, 5   ; Player X
    LDM R1, 6   ; Player Y
    MOV R2, 10  ; Player Sprite Address
    SPT R0 R1 R2

    ; --- Movement LEFT ---
    IN R0, 2
    JZ R0, move_right
    LDM R1, 5 ; R1 = X
    DEC R1    ; R1 = X - 1
    LDM R2, 6 ; R2 = Y
    MOV R3, 210 ; Map Addr
    TAT R0 R1 R2 R3 16 ; R0 = Tile Index
    MOV R2, 0 ; Flags Addr
    TSD R3 R0 R2 ; R3 = Solid? (0/1)
    JNZ R3, move_right_bump ; Collision
    ; Check bottom-left
    LDM R2, 6
    ADD R2, 7 ; Y + 7
    MOV R3, 210
    TAT R0 R1 R2 R3 16 ; R0 = Tile
    MOV R2, 0
    TSD R3 R0 R2
    JNZ R3, move_right_bump
    ; Success - Update X
    STM 5, R1
    SNDV 220 5
    JMP move_right

move_right_bump:
    SNDV 880 10

move_right:
    IN R0, 3
    JZ R0, move_up
    LDM R1, 5
    ADD R1, 8 ; X + 8
    LDM R2, 6
    MOV R3, 210
    TAT R0 R1 R2 R3 16
    MOV R2, 0
    TSD R3 R0 R2
    JNZ R3, move_up_bump
    LDM R2, 6
    ADD R2, 7
    MOV R3, 210
    TAT R0 R1 R2 R3 16
    MOV R2, 0
    TSD R3 R0 R2
    JNZ R3, move_up_bump
    ; Update X
    LDM R1, 5
    ADD R1, 1
    STM 5, R1
    SNDV 220 5
    JMP move_up

move_up_bump:
    SNDV 880 10

move_up:
    IN R0, 0
    JZ R0, move_down
    LDM R1, 5
    LDM R2, 6
    DEC R2 ; Y - 1
    MOV R3, 210
    TAT R0 R1 R2 R3 16
    MOV R1, 0
    TSD R3 R0 R1
    JNZ R3, move_down_bump
    LDM R1, 5
    ADD R1, 7 ; X + 7
    MOV R3, 210
    TAT R0 R1 R2 R3 16
    MOV R1, 0
    TSD R3 R0 R1
    JNZ R3, move_down_bump
    ; Update Y
    STM 6, R2
    SNDV 220 5
    JMP move_down

move_down_bump:
    SNDV 880 10

move_down:
    IN R0, 1
    JZ R0, end_loop
    LDM R1, 5
    LDM R2, 6
    ADD R2, 8 ; Y + 8
    MOV R3, 210
    TAT R0 R1 R2 R3 16
    MOV R1, 0
    TSD R3 R0 R1
    JNZ R3, end_loop_bump
    LDM R1, 5
    ADD R1, 7 ; X + 7
    MOV R3, 210
    TAT R0 R1 R2 R3 16
    MOV R1, 0
    TSD R3 R0 R1
    JNZ R3, end_loop_bump
    ; Update Y
    LDM R2, 6
    ADD R2, 1
    STM 6, R2
    SNDV 220 5
    JMP end_loop

end_loop_bump:
    SNDV 880 10

end_loop:
    WAIT
    JMP loop

tile_flags:
    .DB 0, 1

player_sprite:
    .DB 0,0,3,3,3,3,0,0
    .DB 0,3,3,3,3,3,3,0
    .DB 3,3,0,3,3,0,3,3
    .DB 3,3,3,3,3,3,3,3
    .DB 3,3,0,3,3,0,3,3
    .DB 3,0,3,3,3,3,0,3
    .DB 0,3,3,0,0,3,3,0
    .DB 0,0,3,3,3,3,0,0

tileset:
    ; Tile 0: Floor
    .DB 1,1,1,1,1,1,1,1
    .DB 1,1,1,1,1,1,1,1
    .DB 1,1,1,1,1,1,1,1
    .DB 1,1,1,1,1,1,1,1
    .DB 1,1,1,1,1,1,1,1
    .DB 1,1,1,1,1,1,1,1
    .DB 1,1,1,1,1,1,1,1
    .DB 1,1,1,1,1,1,1,1
    ; Tile 1: Wall
    .DB 2,2,2,2,2,2,2,2
    .DB 2,4,4,4,4,4,4,2
    .DB 2,4,2,2,2,2,4,2
    .DB 2,4,2,2,2,2,4,2
    .DB 2,4,2,2,2,2,4,2
    .DB 2,4,2,2,2,2,4,2
    .DB 2,4,4,4,4,4,4,2
    .DB 2,2,2,2,2,2,2,2

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

score_text:
    .DB 'S', 'C', 'O', 'R', 'E'
