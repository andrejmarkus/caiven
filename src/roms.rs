use crate::settings::HEIGHT;

pub fn test_rom() -> Vec<u8> {
    let mut program: Vec<u8> = Vec::new();

    program.push(0x00);

    for i in 0..HEIGHT {
        program.push(0x01);
        program.push(i as u8); // x
        program.push(i as u8); // y
        program.push(255); // r
        program.push(255); // g
        program.push(255); // b
    }

    program
}
