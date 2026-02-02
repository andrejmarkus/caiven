use crate::buttons::Button;

pub fn moving_pixel() -> Vec<u8> {
    let mut p = Vec::new();

    // R0 = x, R1 = y, R2 = left input, R3 = right input
    // Init state in RAM
    p.extend([0x01, 0x00, 10]); // 0-2: MOV R0, 10
    p.extend([0x31, 100, 0x00]); // 3-5: STM 100, R0 (Store x to RAM[100])
    p.extend([0x01, 0x01, 10]); // 6-8: MOV R1, 10
    p.extend([0x31, 101, 0x01]); // 9-11: STM 101, R1 (Store y to RAM[101])

    // loop: (position 12)
    p.push(0x00); // 12: CLS

    // Load state from RAM
    p.extend([0x30, 0x00, 100]); // 13-15: LDM R0, 100 (Load x from RAM[100])
    p.extend([0x30, 0x01, 101]); // 16-18: LDM R1, 101 (Load y from RAM[101])

    // Read left and right input
    p.extend([0x20, 0x02, Button::Left as u8]); // 19-21: IN R2, left
    p.extend([0x20, 0x03, Button::Right as u8]); // 22-24: IN R3, right

    // Check if left button is pressed
    p.extend([0x11, 0x02, 36, 0x00]); // 25-28: JNZ R2, move_left (jump to 36)

    // Check if right button is pressed
    p.extend([0x11, 0x03, 44, 0x00]); // 29-32: JNZ R3, move_right (jump to 44)
    p.extend([0x10, 50, 0x00]); // 33-35: JMP draw (jump to 50)

    // move_left: (position 36)
    p.extend([0x05, 0x00]); // 36-37: DEC R0
    p.extend([0x31, 100, 0x00]); // 38-40: STM 100, R0 (Update RAM[100])
    p.extend([0x10, 50, 0x00]); // 41-43: JMP draw (jump to 50)

    // move_right: (position 44)
    p.extend([0x02, 0x00, 0x01]); // 44-46: ADD R0, 1
    p.extend([0x31, 100, 0x00]); // 47-49: STM 100, R0 (Update RAM[100])

    // draw: (position 50)
    // draw pixel at (R0, R1)
    p.extend([0x04, 0x00, 0x01, 255, 0, 0]); // 50-55: DPXR R0, R1, red

    // WAIT
    p.push(0xFF); // 56: WAIT

    // JMP loop
    p.extend([0x10, 12, 0x00]); // 57-59: JMP loop (jump to 12)

    p
}
