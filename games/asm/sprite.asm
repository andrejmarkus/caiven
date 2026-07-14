; Sprite viewer — shows the sprite sheet at center of screen
; Open sprite editor (F2) to paint sprites, switch back to see them live

.CONST SPR_X = 60
.CONST SPR_Y = 60

init:
    PAL 0   0   0   0    ; black background
    PAL 1 255  80  80    ; red
    PAL 2  80 255  80    ; green
    PAL 3  80  80 255    ; blue
    PAL 4 255 255  80    ; yellow
    PAL 5 255 160  40    ; orange
    PAL 6 200  80 255    ; purple
    PAL 7 255 255 255    ; white
    JMP loop

loop:
    CLS
    FILL 0

    MOV R0, 0            ; sprite id 0
    MOV R1, SPR_X
    MOV R2, SPR_Y
    MOV R3, 0            ; no flip
    SPR R0 R1 R2 R3

    WAIT
    JMP loop

.BEGIN_SPRITE_SHEET
sprite_a:
    .DB 0,0,1,1,1,1,0,0
    .DB 0,1,1,1,1,1,1,0
    .DB 1,1,2,1,1,2,1,1
    .DB 1,1,1,1,1,1,1,1
    .DB 1,1,3,3,3,1,1,1
    .DB 1,1,1,3,1,1,1,1
    .DB 0,1,1,1,1,1,1,0
    .DB 0,0,1,1,1,0,0,0
.END_SPRITE_SHEET
