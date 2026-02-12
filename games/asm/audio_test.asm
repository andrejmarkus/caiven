; Audio Test
; Frequency: 440Hz (A4)
; Volume: 10 (10%)
; Duration: 60 (1 second at 60fps)

MOV R0, 440
MOV R1, 10
MOV R2, 60
SND R0, R1, R2

; Test noise
MOV R2, 1000 ; Rate
MOV R3, 5    ; Volume
MOV R0, 30   ; Duration (0.5s)
NSND R2, R3, R0

; Test SSTOP / NSTOP (optional, but let's just keep them playing)

LOOP:
    JMP LOOP
