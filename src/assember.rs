use std::collections::HashMap;

pub fn assemble(source: &str) -> Vec<u8> {
    let mut labels: HashMap<String, u16> = HashMap::new();
    let mut pc: u16 = 0;

    for line in source.lines() {
        let line = clean(line);
        if line.is_empty() || line.starts_with(';') {
            continue;
        }

        if line.ends_with(':') {
            let name = line.trim_end_matches(':').trim().to_string();
            labels.insert(name, pc);
        } else {
            pc += instr_size(line) as u16;
        }
    }

    let mut bytecode = Vec::new();

    for line in source.lines() {
        let line = clean(line);
        if line.is_empty() || line.ends_with(':') {
            continue;
        }

        emit_instruction(&line, &labels, &mut bytecode);
    }

    bytecode
}

fn clean(line: &str) -> String {
    line.split(';').next().unwrap().trim().to_string()
}

fn reg(s: &str) -> u8 {
    s.trim_start_matches(|c| c == 'r' || c == 'R')
        .parse::<u8>()
        .unwrap()
}

fn num(s: &str) -> u8 {
    s.parse::<u8>().unwrap()
}
fn instr_size(line: String) -> usize {
    let op = line.split_whitespace().next().unwrap();
    match op {
        "CLS" | "WAIT" => 1,
        "DEC" => 2,
        "MOV" | "ADD" | "LDM" | "STM" | "IN" | "JMP" => 3,
        "JNZ" | "JZ" => 4,
        "DPXR" | "DPX" => 6,
        _ => panic!("Unknown instruction: {}", line),
    }
}

fn emit_instruction(line: &str, labels: &HashMap<String, u16>, bytecode: &mut Vec<u8>) {
    let parts: Vec<&str> = line.split_whitespace().collect();
    match parts[0] {
        "CLS" => bytecode.extend([0x00]),
        "MOV" => bytecode.extend([0x01, reg(parts[1]), num(parts[2])]),
        "ADD" => bytecode.extend([0x02, reg(parts[1]), num(parts[2])]),
        "DPX" => {
            let color_parts: Vec<&str> = parts[3].split(',').collect();
            bytecode.extend([
                0x03,
                reg(parts[1]),
                reg(parts[2]),
                num(color_parts[0]),
                num(color_parts[1]),
                num(color_parts[2]),
            ]);
        }
        "DPXR" => {
            let color_parts: Vec<&str> = parts[3].split(',').collect();
            bytecode.extend([
                0x04,
                reg(parts[1]),
                reg(parts[2]),
                num(color_parts[0]),
                num(color_parts[1]),
                num(color_parts[2]),
            ]);
        }
        "DEC" => bytecode.extend([0x05, reg(parts[1])]),
        "JMP" => {
            let (low, high) = parse_address(parts[1], labels);
            bytecode.extend([0x10, low, high]);
        }
        "JNZ" => {
            let (low, high) = parse_address(parts[2], labels);
            bytecode.extend([0x11, reg(parts[1]), low, high]);
        }
        "JZ" => {
            let (low, high) = parse_address(parts[2], labels);
            bytecode.extend([0x12, reg(parts[1]), low, high]);
        }
        "IN" => bytecode.extend([0x20, reg(parts[1]), num(parts[2])]),
        "LDM" => bytecode.extend([0x30, reg(parts[1]), num(parts[2])]),
        "STM" => bytecode.extend([0x31, num(parts[1]), reg(parts[2])]),
        "WAIT" => bytecode.extend([0xFF]),
        _ => panic!("Unknown instruction: {}", parts[0]),
    }
}

fn parse_address(s: &str, labels: &HashMap<String, u16>) -> (u8, u8) {
    let addr = if let Some(&a) = labels.get(s) {
        a
    } else {
        s.parse::<u16>()
            .unwrap_or_else(|_| panic!("Invalid address or label: {}", s))
    };
    ((addr & 0xFF) as u8, (addr >> 8) as u8)
}
