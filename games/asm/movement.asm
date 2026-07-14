; Movement demo — arrow keys move a sprite around the screen
; Open sprite editor (F2) to edit the player sprite live

.CONST PX = 0   ; player x stored at RAM addr 0
.CONST PY = 1   ; player y stored at RAM addr 1

init:
    PAL 0  10  10  30    ; dark blue background
    PAL 1 200 200 255    ; player body (light blue)
    PAL 2 255 220  80    ; player highlight (yellow)
    PAL 3 255  80  80    ; player accent (red)

    MOV R0, 60
    STM PX, R0
    MOV R0, 60
    STM PY, R0
    JMP loop

loop:
    CLS
    FILL 0

    IN R0, 0
    JZ R0, @chk_down
    LDM R1, PY
    DEC R1
    STM PY, R1

@chk_down:
    IN R0, 1
    JZ R0, @chk_left
    LDM R1, PY
    ADD R1, 1
    STM PY, R1

@chk_left:
    IN R0, 2
    JZ R0, @chk_right
    LDM R1, PX
    DEC R1
    STM PX, R1

@chk_right:
    IN R0, 3
    JZ R0, @draw
    LDM R1, PX
    ADD R1, 1
    STM PX, R1

@draw:
    MOV R0, 0            ; sprite id 0
    LDM R1, PX
    LDM R2, PY
    MOV R3, 0            ; no flip
    SPR R0 R1 R2 R3

    WAIT
    JMP loop

.BEGIN_SPRITE_SHEET
player_sprite:
    .DB 0,0,1,1,1,1,0,0
    .DB 0,1,1,2,2,1,1,0
    .DB 1,1,2,1,1,2,1,1
    .DB 1,1,1,1,1,1,1,1
    .DB 1,1,3,3,3,1,1,1
    .DB 1,1,1,3,1,1,1,1
    .DB 0,1,1,1,1,1,1,0
    .DB 0,0,1,1,1,0,0,0
.END_SPRITE_SHEET
