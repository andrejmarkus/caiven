use crate::directive::{DirectiveSet, default_directives};
use crate::error::AsmError;
use crate::isa::{ArgType, IsaTable, default_isa};
use crate::source_map::{ItemInfo, SourceMap};
use std::collections::HashMap;

pub struct Assembler {
    isa: IsaTable,
    directives: DirectiveSet,
}

impl Default for Assembler {
    fn default() -> Self {
        Self::new()
    }
}

impl Assembler {
    pub fn new() -> Self {
        Self {
            isa: default_isa(),
            directives: default_directives(),
        }
    }

    pub fn assemble(&self, source: &str) -> Result<Vec<u8>, AsmError> {
        let (bytecode, _) = self.assemble_with_source_map(source)?;
        Ok(bytecode)
    }

    pub fn assemble_with_source_map(&self, source: &str) -> Result<(Vec<u8>, SourceMap), AsmError> {
        let labels = self.collect_labels(source)?;
        self.emit(&labels, source)
    }

    pub fn generate_source_map(&self, bytecode: &[u8]) -> SourceMap {
        let mut source_map = SourceMap::new();
        let mut pc = 0;
        while pc < bytecode.len() {
            let opcode = bytecode[pc];
            if let Some(spec) = self.isa.get_by_opcode(opcode) {
                source_map.insert_item(
                    pc,
                    ItemInfo::Instruction {
                        name: spec.name.to_string(),
                        opcode: spec.opcode,
                        size: spec.size,
                    },
                );
                pc += spec.size;
            } else {
                pc += 1;
            }
        }
        source_map
    }

    fn collect_labels(&self, source: &str) -> Result<HashMap<String, u16>, AsmError> {
        let mut labels = HashMap::new();
        let mut pc: u16 = 0;

        for (i, line) in source.lines().enumerate() {
            let line_number = i + 1;
            let cleaned = clean(line);
            if cleaned.is_empty() {
                continue;
            }

            if cleaned.ends_with(':') {
                let name = cleaned.trim_end_matches(':').trim().to_string();
                labels.insert(name, pc);
            } else {
                let tokens = tokenize(&cleaned);
                let name = tokens[0];

                if name.starts_with('.') {
                    let directive = self.directives.get_by_name(name).ok_or_else(|| {
                        AsmError::syntax(
                            line_number,
                            &cleaned,
                            format!("Unknown directive {}", name),
                        )
                    })?;
                    pc += (directive.size)(&tokens[1..], pc) as u16;
                } else {
                    let spec = self.isa.get_by_name(name).ok_or_else(|| {
                        AsmError::syntax(
                            line_number,
                            &cleaned,
                            format!("Unknown instruction {}", name),
                        )
                    })?;
                    pc += spec.size as u16;
                }
            }
        }
        Ok(labels)
    }

    fn emit(
        &self,
        labels: &HashMap<String, u16>,
        source: &str,
    ) -> Result<(Vec<u8>, SourceMap), AsmError> {
        let mut bytecode = Vec::new();
        let mut source_map = SourceMap::new();

        for (i, line) in source.lines().enumerate() {
            let line_number = i + 1;
            let cleaned = clean(line);
            if cleaned.is_empty() {
                continue;
            }

            if cleaned.ends_with(':') {
                let name = cleaned.trim_end_matches(':').trim().to_string();
                source_map.insert_label(bytecode.len(), name);
                continue;
            }

            let tokens = tokenize(&cleaned);
            let name = tokens[0];
            let current_pc = bytecode.len();

            if name.starts_with('.') {
                let directive = self.directives.get_by_name(name).ok_or_else(|| {
                    AsmError::syntax(line_number, &cleaned, format!("Unknown directive {}", name))
                })?;
                let data = (directive.emit)(&tokens[1..], labels, current_pc as u16)
                    .map_err(|e| AsmError::syntax(line_number, &cleaned, e))?;

                source_map.insert_item(
                    current_pc,
                    ItemInfo::Directive {
                        name: name.to_string(),
                        size: data.len(),
                    },
                );
                bytecode.extend(data);
            } else {
                let spec = self.isa.get_by_name(name).ok_or_else(|| {
                    AsmError::syntax(
                        line_number,
                        &cleaned,
                        format!("Unknown instruction {}", name),
                    )
                })?;

                source_map.insert_item(
                    current_pc,
                    ItemInfo::Instruction {
                        name: name.to_string(),
                        opcode: spec.opcode,
                        size: spec.size,
                    },
                );

                self.emit_instruction(&tokens, line_number, &cleaned, &mut bytecode, labels)?;
            }
        }

        Ok((bytecode, source_map))
    }

    fn emit_instruction(
        &self,
        tokens: &[&str],
        line_number: usize,
        source_line: &str,
        bytecode: &mut Vec<u8>,
        labels: &HashMap<String, u16>,
    ) -> Result<(), AsmError> {
        let name = tokens[0];
        let spec = self.isa.get_by_name(name).ok_or_else(|| {
            AsmError::syntax(
                line_number,
                source_line,
                format!("Unknown instruction {}", name),
            )
        })?;

        bytecode.push(spec.opcode);

        if tokens.len() - 1 != spec.args.len() {
            return Err(AsmError::syntax(
                line_number,
                source_line,
                format!(
                    "Incorrect number of arguments for {}: expected {}, got {}",
                    name,
                    spec.args.len(),
                    tokens.len() - 1
                ),
            ));
        }

        for (idx, arg_type) in spec.args.iter().enumerate() {
            let token = tokens[idx + 1];
            match arg_type {
                ArgType::Register => {
                    let reg = parse_register(token)
                        .map_err(|e| AsmError::syntax(line_number, source_line, e))?;
                    bytecode.push(reg);
                }
                ArgType::Value => {
                    let val = parse_u8(token).map_err(|e| {
                        AsmError::syntax(
                            line_number,
                            source_line,
                            format!("Invalid number: {} ({})", token, e),
                        )
                    })?;
                    bytecode.push(val);
                }
                ArgType::Address => {
                    let (low, high) = parse_address(token, labels)
                        .map_err(|e| AsmError::syntax(line_number, source_line, e))?;
                    bytecode.extend([low, high]);
                }
            }
        }

        Ok(())
    }
}

fn clean(line: &str) -> String {
    line.split(';').next().unwrap().trim().to_string()
}

fn tokenize(line: &str) -> Vec<&str> {
    line.split(|c: char| c.is_whitespace() || c == ',')
        .filter(|s| !s.is_empty())
        .collect()
}

fn parse_register(s: &str) -> Result<u8, String> {
    s.trim_start_matches(['r', 'R'])
        .parse::<u8>()
        .map_err(|_| format!("Invalid register: {}", s))
}

fn parse_u8(s: &str) -> Result<u8, String> {
    if s.starts_with('\'') && s.ends_with('\'') && s.len() == 3 {
        return Ok(s.as_bytes()[1]);
    }
    if s.starts_with("0x") || s.starts_with("0X") {
        u8::from_str_radix(&s[2..], 16).map_err(|e| e.to_string())
    } else if s.starts_with("0b") || s.starts_with("0B") {
        u8::from_str_radix(&s[2..], 2).map_err(|e| e.to_string())
    } else {
        s.parse::<u8>().map_err(|e| e.to_string())
    }
}

fn parse_u16(s: &str) -> Result<u16, String> {
    if s.starts_with('\'') && s.ends_with('\'') && s.len() == 3 {
        return Ok(s.as_bytes()[1] as u16);
    }
    if s.starts_with("0x") || s.starts_with("0X") {
        u16::from_str_radix(&s[2..], 16).map_err(|e| e.to_string())
    } else if s.starts_with("0b") || s.starts_with("0B") {
        u16::from_str_radix(&s[2..], 2).map_err(|e| e.to_string())
    } else {
        s.parse::<u16>().map_err(|e| e.to_string())
    }
}

fn parse_address(s: &str, labels: &HashMap<String, u16>) -> Result<(u8, u8), String> {
    let addr = if let Some(&a) = labels.get(s) {
        a
    } else {
        parse_u16(s).map_err(|e| format!("Unknown label or invalid address: {} ({})", s, e))?
    };
    Ok(((addr & 0xFF) as u8, (addr >> 8) as u8))
}
