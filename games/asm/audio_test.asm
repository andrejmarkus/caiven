; Audio Test
; Frequency: 440Hz (A4)
; Volume: 10 (10%)

MOV R0, 440
MOV R1, 10
SND R0, R1

; Wait for a bit?
; We don't have a sleep instruction, but we can use a loop or just let it run.
; Since the VM runs in a loop in run_frame, and we are not using WAIT, it will keep running.

LOOP:
    JMP LOOP
