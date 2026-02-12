; Catch the Fruit Game
; RAM Addresses:
; 10: Player X (1 byte)
; 11: Player Y (1 byte)
; 12: Fruit X (1 byte)
; 13: Fruit Y (1 byte)
; 14: Score (16-bit)
; 16: Timer (16-bit)

init:
    ; Palette setup
    PAL 0 0 0 0       ; Black Background
    PAL 1 255 255 255 ; White (Labels/Player)
    PAL 2 255 50 50   ; Red (Fruit)
    PAL 3 50 255 50   ; Green (Score)

    ; Initial state
    MOV R0, 60
    STM 10, R0
    STM 11, R0

    MOV R0, 0
    STMW 14, R0

    ; Copy sprites and labels to RAM
    CPY 100, player_sprite, 64
    CPY 200, fruit_sprite, 64
    CPY 300, score_label, 7
    CPY 310, timer_label, 6

    ; Spawn first fruit
    JSR spawn_fruit
    JMP loop

loop:
    CLS
    FILL 0 ; Background
    
    ; --- Draw UI ---
    ; Score text
    MOV R0, 5
    MOV R1, 5
    MOV R2, 1 ; White
    MOV R3, 300 ; score_label
    TXT R0 R1 R2 R3 6
    
    ; Score Value
    MOV R0, 45
    MOV R1, 5
    MOV R2, 3 ; Green
    LDMW R3, 14
    NUM R0 R1 R2 R3
    
    ; Timer text
    MOV R0, 75
    MOV R1, 5
    MOV R2, 1 ; White
    MOV R3, 310 ; timer_label
    TXT R0 R1 R2 R3 5

    ; Timer Value
    MOV R0, 105
    MOV R1, 5
    MOV R2, 3 ; Green
    LDMW R3, 16
    NUM R0 R1 R2 R3

    ; --- Draw Player ---
    LDM R0, 10
    LDM R1, 11
    MOV R2, 100
    SPT R0 R1 R2

    ; --- Draw Fruit ---
    LDM R0, 12
    LDM R1, 13
    MOV R2, 200
    SPT R0 R1 R2

    ; --- Input / Movement ---
    IN R0, 2 ; LEFT
    JZ R0, check_right
    LDM R1, 10
    DEC R1
    STM 10, R1
check_right:
    IN R0, 3 ; RIGHT
    JZ R0, check_up
    LDM R1, 10
    ADD R1, 1
    STM 10, R1
check_up:
    IN R0, 0 ; UP
    JZ R0, check_down
    LDM R1, 11
    DEC R1
    STM 11, R1
check_down:
    IN R0, 1 ; DOWN
    JZ R0, move_sound
    LDM R1, 11
    ADD R1, 1
    STM 11, R1

move_sound:
    IN R0, 0
    IN R1, 1
    ADDR R0, R1
    IN R1, 2
    ADDR R0, R1
    IN R1, 3
    ADDR R0, R1
    JZ R0, stop_move_sound
    NSNDV 400 2 2 ; Step sound
    JMP check_collision

stop_move_sound:
    NSTOP

check_collision:
    LDM R0, 10 ; PX
    LDM R1, 12 ; FX
    ; X overlap check
    MOVR R2, R1
    ADD R2, 8
    SLT R3, R0, R2
    JZ R3, timer_logic
    MOVR R2, R0
    ADD R2, 8
    SLT R3, R1, R2
    JZ R3, timer_logic
    
    ; Y overlap check
    LDM R0, 11 ; PY
    LDM R1, 13 ; FY
    MOVR R2, R1
    ADD R2, 8
    SLT R3, R0, R2
    JZ R3, timer_logic
    MOVR R2, R0
    ADD R2, 8
    SLT R3, R1, R2
    JZ R3, timer_logic
    
    ; Collision!
    SNDV 880 15 10 ; Square catch
    NSNDV 5000 8 5 ; Noise crunch
    LDMW R0, 14
    ADD R0, 1
    STMW 14, R0
    JSR spawn_fruit

timer_logic:
    LDMW R0, 16
    DEC R0
    STMW 16, R0
    JZ R0, game_over
    WAIT
    JMP loop

spawn_fruit:
    RND R0, 100
    ADD R0, 10
    STM 12, R0 ; New FX
    RND R0, 80
    ADD R0, 20
    STM 13, R0 ; New FY
    MOV R0, 100
    STMW 16, R0 ; Reset Timer
    RET

game_over:
    NSNDV 50 20 60
    JMP init

score_label:
    .DB 'S', 'C', 'O', 'R', 'E', ':', 0
timer_label:
    .DB 'T', 'I', 'M', 'E', ':', 0

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

score_label:
    .DB 'S', 'C', 'O', 'R', 'E', ':', 0
timer_label:
    .DB 'T', 'I', 'M', 'E', ':', 0

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

