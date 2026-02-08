use crate::assembler::AssemblerError;
use crate::assembler::directives::DirectiveSet;
use crate::vm::cpu::ArgType;
use crate::vm::cpu::InstructionSet;
use std::collections::HashMap;
use std::sync::Arc;

pub struct Assembler {
    instruction_set: Arc<InstructionSet>,
    directive_set: Arc<DirectiveSet>,
}

impl Assembler {
    pub fn new(instruction_set: Arc<InstructionSet>, directive_set: Arc<DirectiveSet>) -> Self {
        Self {
            instruction_set,
            directive_set,
        }
    }

    pub fn assemble(&self, source: &str) -> Result<Vec<u8>, AssemblerError> {
        let labels = self.collect_labels(source)?;
        self.emit_bytecode(source, &labels)
    }

    fn collect_labels(&self, source: &str) -> Result<HashMap<String, u16>, AssemblerError> {
        let mut labels = HashMap::new();
        let mut pc: u16 = 0;

        for (i, line) in source.lines().enumerate() {
            let line_number = i + 1;
            let cleaned = self.clean(line);
            if cleaned.is_empty() {
                continue;
            }

            if cleaned.ends_with(':') {
                let name = cleaned.trim_end_matches(':').trim().to_string();
                labels.insert(name, pc);
            } else {
                let tokens = self.tokenize(&cleaned);
                let name = tokens[0];

                if name.starts_with('.') {
                    let directive = self.directive_set.get_by_name(name).ok_or_else(|| {
                        self.error(line_number, &cleaned, format!("Unknown directive {}", name))
                    })?;
                    pc += (directive.size)(&tokens[1..], pc) as u16;
                } else {
                    let instruction = self.instruction_set.get_by_name(name).ok_or_else(|| {
                        self.error(
                            line_number,
                            &cleaned,
                            format!("Unknown instruction {}", name),
                        )
                    })?;
                    pc += instruction.size as u16;
                }
            }
        }
        Ok(labels)
    }

    fn emit_bytecode(
        &self,
        source: &str,
        labels: &HashMap<String, u16>,
    ) -> Result<Vec<u8>, AssemblerError> {
        let mut bytecode = Vec::new();

        for (i, line) in source.lines().enumerate() {
            let line_number = i + 1;
            let cleaned = self.clean(line);
            if cleaned.is_empty() || cleaned.ends_with(':') {
                continue;
            }

            let tokens = self.tokenize(&cleaned);
            let name = tokens[0];

            if name.starts_with('.') {
                let directive = self.directive_set.get_by_name(name).unwrap();
                let current_pc = bytecode.len() as u16;
                let data = (directive.emit)(&tokens[1..], labels, current_pc)
                    .map_err(|e| self.error(line_number, &cleaned, e))?;
                bytecode.extend(data);
            } else {
                self.emit_instruction_tokens(
                    &tokens,
                    line_number,
                    &cleaned,
                    &mut bytecode,
                    labels,
                )?;
            }
        }

        Ok(bytecode)
    }

    fn tokenize<'a>(&self, line: &'a str) -> Vec<&'a str> {
        line.split(|c: char| c.is_whitespace() || c == ',')
            .filter(|s| !s.is_empty())
            .collect()
    }

    fn error(&self, line: usize, source: &str, message: String) -> AssemblerError {
        AssemblerError {
            line,
            source: source.to_string(),
            message,
        }
    }

    fn clean(&self, line: &str) -> String {
        line.split(';').next().unwrap().trim().to_string()
    }

    fn emit_instruction_tokens(
        &self,
        tokens: &[&str],
        line_number: usize,
        source: &str,
        bytecode: &mut Vec<u8>,
        labels: &HashMap<String, u16>,
    ) -> Result<(), AssemblerError> {
        let name = tokens[0];
        let instruction = self.instruction_set.get_by_name(name).ok_or_else(|| {
            self.error(
                line_number,
                source,
                format!("Unknown instruction name {}", name),
            )
        })?;

        bytecode.push(instruction.opcode);

        if tokens.len() - 1 != instruction.args.len() {
            return Err(self.error(
                line_number,
                source,
                format!(
                    "Incorrect number of arguments for instruction {}: expected {}, got {}",
                    name,
                    instruction.args.len(),
                    tokens.len() - 1
                ),
            ));
        }

        for (i, arg_type) in instruction.args.iter().enumerate() {
            let token = tokens[i + 1];
            match arg_type {
                ArgType::Register => {
                    let reg = self.reg(token).map_err(|mut e| {
                        e.line = line_number;
                        e.source = source.to_string();
                        e
                    })?;
                    bytecode.push(reg);
                }
                ArgType::Value => {
                    let val = self.num(token).map_err(|mut e| {
                        e.line = line_number;
                        e.source = source.to_string();
                        e
                    })?;
                    bytecode.push(val);
                }
                ArgType::Address => {
                    let (low, high) = self.addr(token, labels).map_err(|mut e| {
                        e.line = line_number;
                        e.source = source.to_string();
                        e
                    })?;
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
