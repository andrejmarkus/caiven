use crate::assembler::{
    AssemblerError, DirectiveSet, InstructionSet, SourceMap,
    item::{ArgType, AssemblyItem},
};
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
        let (bytecode, _) = self.assemble_with_source_map(source)?;
        Ok(bytecode)
    }

    pub fn assemble_with_source_map(
        &self,
        source: &str,
    ) -> Result<(Vec<u8>, SourceMap), AssemblerError> {
        let labels = self.collect_labels(source)?;
        self.emit_bytecode_with_source_map(source, &labels)
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

    fn emit_bytecode_with_source_map(
        &self,
        source: &str,
        labels: &HashMap<String, u16>,
    ) -> Result<(Vec<u8>, SourceMap), AssemblerError> {
        let mut bytecode = Vec::new();
        let mut source_map = SourceMap::new();

        for (i, line) in source.lines().enumerate() {
            let line_number = i + 1;
            let cleaned = self.clean(line);
            if cleaned.is_empty() {
                continue;
            }

            if cleaned.ends_with(':') {
                let name = cleaned.trim_end_matches(':').trim().to_string();
                source_map.insert_label(bytecode.len(), name);
                continue;
            }

            let tokens = self.tokenize(&cleaned);
            let name = tokens[0];
            let current_pc = bytecode.len();

            if name.starts_with('.') {
                let directive = self.directive_set.get_by_name(name).unwrap();
                let data = (directive.emit)(&tokens[1..], labels, current_pc as u16)
                    .map_err(|e| self.error(line_number, &cleaned, e))?;

                source_map.insert_item(
                    current_pc,
                    AssemblyItem::Directive {
                        name: name.to_string(),
                        size: data.len(),
                    },
                );

                bytecode.extend(data);
            } else {
                let instruction = self.instruction_set.get_by_name(name).unwrap();
                source_map.insert_item(
                    current_pc,
                    AssemblyItem::Instruction {
                        name: name.to_string(),
                        opcode: instruction.opcode,
                        size: instruction.size,
                    },
                );

                self.emit_instruction_tokens(
                    &tokens,
                    line_number,
                    &cleaned,
                    &mut bytecode,
                    labels,
                )?;
            }
        }

        Ok((bytecode, source_map))
    }

    fn emit_bytecode(
        &self,
        source: &str,
        labels: &HashMap<String, u16>,
    ) -> Result<Vec<u8>, AssemblerError> {
        let (bytecode, _) = self.emit_bytecode_with_source_map(source, labels)?;
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
            self.parse_u16(s).map_err(|e| AssemblerError {
                line: 0,
                message: format!("Unknown label or invalid address: {} ({})", s, e),
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
        self.parse_u8(s).map_err(|e| AssemblerError {
            line: 0,
            message: format!("Invalid number: {} ({})", s, e),
            source: s.to_string(),
        })
    }

    fn parse_u8(&self, s: &str) -> Result<u8, String> {
        if s.starts_with('\'') && s.ends_with('\'') && s.len() == 3 {
            return Ok(s.as_bytes()[1]);
        }
        if s.starts_with("0x") {
            u8::from_str_radix(&s[2..], 16).map_err(|e| e.to_string())
        } else if s.starts_with("0b") {
            u8::from_str_radix(&s[2..], 2).map_err(|e| e.to_string())
        } else {
            s.parse::<u8>().map_err(|e| e.to_string())
        }
    }

    fn parse_u16(&self, s: &str) -> Result<u16, String> {
        if s.starts_with('\'') && s.ends_with('\'') && s.len() == 3 {
            return Ok(s.as_bytes()[1] as u16);
        }
        if s.starts_with("0x") {
            u16::from_str_radix(&s[2..], 16).map_err(|e| e.to_string())
        } else if s.starts_with("0b") {
            u16::from_str_radix(&s[2..], 2).map_err(|e| e.to_string())
        } else {
            s.parse::<u16>().map_err(|e| e.to_string())
        }
    }
}
