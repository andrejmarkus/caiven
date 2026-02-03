use crate::assembler::AssemblerError;
use crate::instructions::ArgType;
use crate::instructions::InstructionSet;
use std::collections::HashMap;
use std::sync::Arc;

pub struct Assembler {
    instruction_set: Arc<InstructionSet>,
}

impl Assembler {
    pub fn new(instruction_set: Arc<InstructionSet>) -> Self {
        Self { instruction_set }
    }

    pub fn assemble(&self, source: &str) -> Result<Vec<u8>, AssemblerError> {
        let mut labels: HashMap<String, u16> = HashMap::new();
        let mut pc: u16 = 0;

        for (line_number, line) in source.lines().enumerate() {
            let line_number = line_number + 1;
            let line = self.clean(line);

            if line.is_empty() {
                continue;
            }

            if line.ends_with(':') {
                let name = line.trim_end_matches(':').trim().to_string();
                labels.insert(name, pc);
            } else {
                pc += self
                    .instr_size(&line, &self.instruction_set)
                    .ok_or_else(|| AssemblerError {
                        line: line_number,
                        message: format!("Unknown instruction in line {}", line_number),
                        source: line.clone(),
                    })? as u16;
            }
        }

        let mut bytecode = Vec::new();

        for (line_number, line) in source.lines().enumerate() {
            let mut line_number = line_number + 1;
            let line = self.clean(line);
            if line.is_empty() || line.ends_with(':') {
                continue;
            }

            self.emit_instruction(&line, &mut line_number, &mut bytecode, &labels)?;
        }

        Ok(bytecode)
    }

    fn clean(&self, line: &str) -> String {
        line.split(';').next().unwrap().trim().to_string()
    }

    fn instr_size(&self, line: &str, instruction_set: &InstructionSet) -> Option<usize> {
        let name = line.split_whitespace().next().unwrap();
        instruction_set.get_by_name(name).map(|i| i.size)
    }

    fn emit_instruction(
        &self,
        line: &str,
        line_number: &mut usize,
        bytecode: &mut Vec<u8>,
        labels: &HashMap<String, u16>,
    ) -> Result<(), AssemblerError> {
        let tokens: Vec<&str> = line
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter(|s| !s.is_empty())
            .collect();

        let name = tokens[0];
        let instruction = self
            .instruction_set
            .get_by_name(name)
            .ok_or_else(|| AssemblerError {
                line: *line_number,
                message: format!("Unknown instruction name {name}"),
                source: line.to_string(),
            })?;
        bytecode.push(instruction.opcode);

        if tokens.len() - 1 != instruction.args.len() {
            return Err(AssemblerError {
                line: *line_number,
                message: format!(
                    "Incorrect number of arguments for instruction {}: expected {}, got {}",
                    name,
                    instruction.args.len(),
                    tokens.len() - 1
                ),
                source: line.to_string(),
            });
        }

        for (i, arg_type) in instruction.args.iter().enumerate() {
            let token = tokens[i + 1];
            match arg_type {
                ArgType::Register => bytecode.push(self.reg(token)?),
                ArgType::Value => bytecode.push(self.num(token)?),
                ArgType::Address => {
                    let (low, high) = self.addr(token, labels)?;
                    bytecode.extend([low, high]);
                }
            }
        }

        Ok(())
    }

    fn addr(&self, s: &str, labels: &HashMap<String, u16>) -> Result<(u8, u8), AssemblerError> {
        let addr = if let Some(&a) = labels.get(s) {
            a
        } else {
            s.parse::<u16>().map_err(|_| AssemblerError {
                line: 0,
                message: format!("Unknown label or invalid address: {}", s),
                source: s.to_string(),
            })?
        };

        Ok(((addr & 0xFF) as u8, (addr >> 8) as u8))
    }

    fn reg(&self, s: &str) -> Result<u8, AssemblerError> {
        s.trim_start_matches(|c| c == 'r' || c == 'R')
            .parse::<u8>()
            .map_err(|_| AssemblerError {
                line: 0,
                message: format!("Invalid register: {}", s),
                source: s.to_string(),
            })
    }

    fn num(&self, s: &str) -> Result<u8, AssemblerError> {
        s.parse::<u8>().map_err(|_| AssemblerError {
            line: 0,
            message: format!("Invalid number: {}", s),
            source: s.to_string(),
        })
    }
}
