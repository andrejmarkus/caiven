use crate::instruction::ArgType;
use crate::instruction_set::InstructionSet;
use std::collections::HashMap;

pub fn assemble(source: &str, instruction_set: &InstructionSet) -> Vec<u8> {
    let mut labels: HashMap<String, u16> = HashMap::new();
    let mut pc: u16 = 0;

    for line in source.lines() {
        let line = clean(line);
        if line.is_empty() {
            continue;
        }

        if line.ends_with(':') {
            let name = line.trim_end_matches(':').trim().to_string();
            labels.insert(name, pc);
        } else {
            pc += instr_size(&line, instruction_set) as u16;
        }
    }

    let mut bytecode = Vec::new();

    for line in source.lines() {
        let line = clean(line);
        if line.is_empty() || line.ends_with(':') {
            continue;
        }

        emit_instruction(&line, &mut bytecode, instruction_set, &labels);
    }

    bytecode
}

fn clean(line: &str) -> String {
    line.split(';').next().unwrap().trim().to_string()
}

fn instr_size(line: &str, instruction_set: &InstructionSet) -> usize {
    let name = line.split_whitespace().next().unwrap();
    instruction_set
        .get_by_name(name)
        .unwrap_or_else(|| panic!("Unknown instruction name {name}"))
        .size
}

fn emit_instruction(
    line: &str,
    bytecode: &mut Vec<u8>,
    instruction_set: &InstructionSet,
    labels: &HashMap<String, u16>,
) {
    let tokens: Vec<&str> = line
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|s| !s.is_empty())
        .collect();

    let name = tokens[0];
    let instruction = instruction_set
        .get_by_name(name)
        .unwrap_or_else(|| panic!("Unknown instruction name {name}"));

    bytecode.push(instruction.opcode);

    if tokens.len() - 1 != instruction.args.len() {
        panic!(
            "Instruction {} expects {} arguments, but got {}",
            name,
            instruction.args.len(),
            tokens.len() - 1
        );
    }

    for (i, arg_type) in instruction.args.iter().enumerate() {
        let token = tokens[i + 1];
        match arg_type {
            ArgType::Register => bytecode.push(reg(token)),
            ArgType::Value => bytecode.push(num(token)),
            ArgType::Address => {
                let (low, high) = parse_address(token, labels);
                bytecode.extend([low, high]);
            }
        }
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

fn reg(s: &str) -> u8 {
    s.trim_start_matches(|c| c == 'r' || c == 'R')
        .parse::<u8>()
        .unwrap()
}

fn num(s: &str) -> u8 {
    s.parse::<u8>().unwrap()
}
