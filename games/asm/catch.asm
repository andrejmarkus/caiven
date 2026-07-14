; Catch the Fruit — demonstrates F1+F2 assembler features:
;   .CONST, expressions, local labels (@name), .MACRO/.ENDM
;   .BEGIN_SPRITE_SHEET / .END_SPRITE_SHEET (F2 asset sections)

; === RAM layout ===
.CONST PLAYER_X  = 10
.CONST PLAYER_Y  = 11
.CONST FRUIT_X   = 12
.CONST FRUIT_Y   = 13
.CONST SCORE     = 14   ; 16-bit word
.CONST TIMER     = 16   ; 16-bit word

; === String data locations in RAM ===
.CONST STR_SCORE  = 300
.CONST STR_TIMER  = 310

; === UI layout ===
.CONST UI_Y       = 5
.CONST SCORE_X    = 5
.CONST SCORE_VAL_X = 45
.CONST TIMER_X    = 75
.CONST TIMER_VAL_X = 105

; === Fruit spawn range ===
.CONST FRUIT_X_RANGE = 100
.CONST FRUIT_X_MIN   = 10
.CONST FRUIT_Y_RANGE = 80
.CONST FRUIT_Y_MIN   = 20

; === Starting state ===
.CONST INIT_POS   = 60
.CONST INIT_TIMER = 100

; === Collision sprite half-size ===
.CONST HALF_SPR   = 8

; === Palette ===
.CONST COL_BG     = 0
.CONST COL_WHITE  = 1
.CONST COL_RED    = 2
.CONST COL_GREEN  = 3

; === Macros ===
.MACRO LOAD_BYTE dst addr
    LDM dst addr
.ENDM

.MACRO STORE_BYTE addr src
    STM addr src
.ENDM

init:
    PAL COL_BG    0   0   0
    PAL COL_WHITE 255 255 255
    PAL COL_RED   255 50  50
    PAL COL_GREEN 50  255 50

    MOV R0, INIT_POS
    STORE_BYTE PLAYER_X, R0
    STORE_BYTE PLAYER_Y, R0

    MOV R0, 0
    STMW SCORE, R0

    CPY STR_SCORE, score_label, 7
    CPY STR_TIMER, timer_label, 6

    JSR spawn_fruit
    JMP loop

loop:
    CLS
    FILL COL_BG

    ; Score label
    MOV R0, SCORE_X
    MOV R1, UI_Y
    MOV R2, COL_WHITE
    MOV R3, STR_SCORE
    TXT R0 R1 R2 R3 6

    ; Score value
    MOV R0, SCORE_VAL_X
    MOV R1, UI_Y
    MOV R2, COL_GREEN
    LDMW R3, SCORE
    NUM R0 R1 R2 R3

    ; Timer label
    MOV R0, TIMER_X
    MOV R1, UI_Y
    MOV R2, COL_WHITE
    MOV R3, STR_TIMER
    TXT R0 R1 R2 R3 5

    ; Timer value
    MOV R0, TIMER_VAL_X
    MOV R1, UI_Y
    MOV R2, COL_GREEN
    LDMW R3, TIMER
    NUM R0 R1 R2 R3

    ; Draw player (sprite id 0)
    MOV R0, 0
    LOAD_BYTE R1, PLAYER_X
    LOAD_BYTE R2, PLAYER_Y
    MOV R3, 0
    SPR R0 R1 R2 R3

    ; Draw fruit (sprite id 1)
    MOV R0, 1
    LOAD_BYTE R1, FRUIT_X
    LOAD_BYTE R2, FRUIT_Y
    MOV R3, 0
    SPR R0 R1 R2 R3

    ; Input: LEFT
    IN R0, 2
    JZ R0, @check_right
    LDM R1, PLAYER_X
    DEC R1
    STM PLAYER_X, R1
@check_right:
    IN R0, 3
    JZ R0, @check_up
    LDM R1, PLAYER_X
    ADD R1, 1
    STM PLAYER_X, R1
@check_up:
    IN R0, 0
    JZ R0, @check_down
    LDM R1, PLAYER_Y
    DEC R1
    STM PLAYER_Y, R1
@check_down:
    IN R0, 1
    JZ R0, @move_sound
    LDM R1, PLAYER_Y
    ADD R1, 1
    STM PLAYER_Y, R1

@move_sound:
    IN R0, 0
    IN R1, 1
    ADDR R0, R1
    IN R1, 2
    ADDR R0, R1
    IN R1, 3
    ADDR R0, R1
    JZ R0, @stop_move_sound
    NSNDV 400 2 2
    JMP check_collision
@stop_move_sound:
    NSTOP

check_collision:
    LDM R0, PLAYER_X
    LDM R1, FRUIT_X
    MOVR R2, R1
    ADD R2, HALF_SPR
    SLT R3, R0, R2
    JZ R3, timer_logic
    MOVR R2, R0
    ADD R2, HALF_SPR
    SLT R3, R1, R2
    JZ R3, timer_logic

    LDM R0, PLAYER_Y
    LDM R1, FRUIT_Y
    MOVR R2, R1
    ADD R2, HALF_SPR
    SLT R3, R0, R2
    JZ R3, timer_logic
    MOVR R2, R0
    ADD R2, HALF_SPR
    SLT R3, R1, R2
    JZ R3, timer_logic

    ; Collision!
    SNDV 880 15 10
    NSNDV 5000 8 5
    LDMW R0, SCORE
    ADD R0, 1
    STMW SCORE, R0
    JSR spawn_fruit

timer_logic:
    LDMW R0, TIMER
    DEC R0
    STMW TIMER, R0
    JZ R0, game_over
    WAIT
    JMP loop

spawn_fruit:
    RND R0, FRUIT_X_RANGE
    ADD R0, FRUIT_X_MIN
    STM FRUIT_X, R0
    RND R0, FRUIT_Y_RANGE
    ADD R0, FRUIT_Y_MIN
    STM FRUIT_Y, R0
    MOV R0, INIT_TIMER
    STMW TIMER, R0
    RET

game_over:
    NSNDV 50 20 60
    JMP init

score_label:
    .DB 'S', 'C', 'O', 'R', 'E', ':', 0

timer_label:
    .DB 'T', 'I', 'M', 'E', ':', 0

; === Sprite sheet (auto-loaded to 0x4000 by host) ===
.BEGIN_SPRITE_SHEET
player_sprite:
    .DB 0,1,1,1,1,1,1,0
    .DB 1,1,1,1,1,1,1,1
    .DB 1,0,1,1,1,1,0,1
    .DB 1,1,1,1,1,1,1,1
    .DB 1,1,0,0,0,1,1,1
    .DB 1,1,1,0,1,1,1,1
    .DB 0,1,1,1,1,1,1,0
    .DB 0,0,1,1,1,0,0,0

fruit_sprite:
    .DB 0,0,0,3,3,0,0,0
    .DB 0,0,2,2,2,2,0,0
    .DB 0,2,2,2,2,2,2,0
    .DB 2,2,2,2,2,2,2,2
    .DB 2,2,2,2,2,2,2,2
    .DB 0,2,2,2,2,2,2,0
    .DB 0,0,2,2,2,2,0,0
    .DB 0,0,0,0,0,0,0,0
.END_SPRITE_SHEET
