use crate::directive::{DirectiveSet, default_directives};
use crate::error::AsmError;
use crate::expr::eval_expr;
use crate::isa::{ArgType, IsaTable, default_isa};
use crate::preprocess::{LineSection, Preprocessor, SourceLine, resolve_local_refs, tokenize};
use crate::source_map::{ItemInfo, SourceMap};
use std::collections::HashMap;
use std::path::Path;

/// SectionKind wire ID for SpriteSheet (matches fc-rom::SectionKind::SpriteSheet).
const SPRITE_SHEET_KIND: u16 = 0x0002;

/// RAM base address where the SpriteSheet section is auto-loaded by the host.
pub const SPRITE_SHEET_RAM_BASE: u16 = fc_core::memory::SPRITE_SHEET_RAM_BASE as u16;

pub struct AssemblerOutput {
    pub program: Vec<u8>,
    pub source_map: SourceMap,
    /// Extra sections as (kind_wire_id, bytes). Excludes the Program section.
    pub extra_sections: Vec<(u16, Vec<u8>)>,
}

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
        Ok(self.assemble_with_sections_str(source)?.program)
    }

    pub fn assemble_with_source_map(&self, source: &str) -> Result<(Vec<u8>, SourceMap), AsmError> {
        let out = self.assemble_with_sections_str(source)?;
        Ok((out.program, out.source_map))
    }

    pub fn assemble_with_sections(&self, source: &str) -> Result<AssemblerOutput, AsmError> {
        self.assemble_with_sections_str(source)
    }

    pub fn assemble_file(&self, path: &Path) -> Result<Vec<u8>, AsmError> {
        Ok(self.assemble_file_with_sections(path)?.program)
    }

    pub fn assemble_file_with_source_map(
        &self,
        path: &Path,
    ) -> Result<(Vec<u8>, SourceMap), AsmError> {
        let out = self.assemble_file_with_sections(path)?;
        Ok((out.program, out.source_map))
    }

    pub fn assemble_file_with_sections(&self, path: &Path) -> Result<AssemblerOutput, AsmError> {
        let mut pp = Preprocessor::new();
        let lines = pp.process_file(path)?;
        self.assemble_inner(&lines, pp.into_constants())
    }

    fn assemble_with_sections_str(&self, source: &str) -> Result<AssemblerOutput, AsmError> {
        let mut pp = Preprocessor::new();
        let lines = pp.process_str(source)?;
        self.assemble_inner(&lines, pp.into_constants())
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
                        size: spec.size(),
                    },
                );
                pc += spec.size();
            } else {
                pc += 1;
            }
        }
        source_map
    }

    fn assemble_inner(
        &self,
        lines: &[SourceLine],
        constants: HashMap<String, u16>,
    ) -> Result<AssemblerOutput, AsmError> {
        let labels = self.collect_labels(lines, &constants)?;
        let mut symbols = constants;
        for (k, v) in &labels {
            symbols.entry(k.clone()).or_insert(*v);
        }
        self.emit(lines, &symbols)
    }

    fn collect_labels(
        &self,
        lines: &[SourceLine],
        symbols: &HashMap<String, u16>,
    ) -> Result<HashMap<String, u16>, AsmError> {
        let mut labels: HashMap<String, u16> = HashMap::new();
        labels.extend(symbols.iter().map(|(k, v)| (k.clone(), *v)));

        let mut program_pc: u16 = 0;
        let mut sprite_pc: u16 = SPRITE_SHEET_RAM_BASE;
        let mut current_scope = String::new();

        for sl in lines {
            let pc = if sl.section == LineSection::SpriteSheet {
                &mut sprite_pc
            } else {
                &mut program_pc
            };
            let line_number = sl.line_number;
            let text = &sl.text;

            if text.ends_with(':') {
                let raw_name = text.trim_end_matches(':').trim();
                if let Some(local) = raw_name.strip_prefix('@') {
                    let mangled = format!("{}@@{}", current_scope, local);
                    labels.insert(mangled, *pc);
                } else {
                    current_scope = raw_name.to_string();
                    labels.insert(current_scope.clone(), *pc);
                }
                continue;
            }

            let tokens = tokenize(text);
            if tokens.is_empty() {
                continue;
            }
            let name_upper = tokens[0].to_uppercase();

            if name_upper.starts_with('.') {
                let directive = self.directives.get_by_name(&name_upper).ok_or_else(|| {
                    AsmError::syntax(
                        line_number,
                        text,
                        format!("unknown directive {}", name_upper),
                    )
                })?;
                let refs: Vec<&str> = tokens[1..].iter().map(|s| s.as_str()).collect();
                *pc += (directive.size)(&refs, *pc, &labels) as u16;
            } else {
                let spec = self.isa.get_by_name(&name_upper).ok_or_else(|| {
                    AsmError::syntax(
                        line_number,
                        text,
                        format!("unknown instruction {}", name_upper),
                    )
                })?;
                *pc += spec.size() as u16;
            }
        }
        Ok(labels)
    }

    fn emit(
        &self,
        lines: &[SourceLine],
        symbols: &HashMap<String, u16>,
    ) -> Result<AssemblerOutput, AsmError> {
        let mut program = Vec::new();
        let mut sprite_sheet: Vec<u8> = Vec::new();
        let mut source_map = SourceMap::new();
        let mut current_scope = String::new();

        for sl in lines {
            let line_number = sl.line_number;
            let text = &sl.text;
            let buf = if sl.section == LineSection::SpriteSheet {
                &mut sprite_sheet
            } else {
                &mut program
            };

            if text.ends_with(':') {
                let raw_name = text.trim_end_matches(':').trim();
                if let Some(local) = raw_name.strip_prefix('@') {
                    let mangled = format!("{}@@{}", current_scope, local);
                    source_map.insert_label(program.len(), mangled);
                } else {
                    current_scope = raw_name.to_string();
                    source_map.insert_label(program.len(), current_scope.clone());
                }
                continue;
            }

            let tokens = tokenize(text);
            if tokens.is_empty() {
                continue;
            }
            let name_upper = tokens[0].to_uppercase();
            let current_pc = buf.len();

            if name_upper.starts_with('.') {
                let directive = self.directives.get_by_name(&name_upper).ok_or_else(|| {
                    AsmError::syntax(
                        line_number,
                        text,
                        format!("unknown directive {}", name_upper),
                    )
                })?;
                let refs: Vec<&str> = tokens[1..].iter().map(|s| s.as_str()).collect();
                // Use program PC for directives in SpriteSheet sections (source map tracks program space)
                let emit_pc = if sl.section == LineSection::SpriteSheet {
                    (SPRITE_SHEET_RAM_BASE as usize) + current_pc
                } else {
                    current_pc
                } as u16;
                let data = (directive.emit)(&refs, symbols, emit_pc)
                    .map_err(|e| AsmError::syntax(line_number, text, e.to_string()))?;
                if sl.section == LineSection::Program {
                    source_map.insert_item(
                        current_pc,
                        ItemInfo::Directive {
                            name: name_upper,
                            size: data.len(),
                        },
                    );
                }
                buf.extend(data);
            } else {
                let spec = self.isa.get_by_name(&name_upper).ok_or_else(|| {
                    AsmError::syntax(
                        line_number,
                        text,
                        format!("unknown instruction {}", name_upper),
                    )
                })?;
                if sl.section == LineSection::Program {
                    source_map.insert_item(
                        current_pc,
                        ItemInfo::Instruction {
                            name: name_upper.clone(),
                            opcode: spec.opcode,
                            size: spec.size(),
                        },
                    );
                }
                self.emit_instruction(&tokens, line_number, text, buf, symbols, &current_scope)?;
            }
        }

        let mut extra_sections = Vec::new();
        if !sprite_sheet.is_empty() {
            extra_sections.push((SPRITE_SHEET_KIND, sprite_sheet));
        }

        Ok(AssemblerOutput {
            program,
            source_map,
            extra_sections,
        })
    }

    fn emit_instruction(
        &self,
        tokens: &[String],
        line_number: usize,
        source_line: &str,
        bytecode: &mut Vec<u8>,
        symbols: &HashMap<String, u16>,
        scope: &str,
    ) -> Result<(), AsmError> {
        let name_upper = tokens[0].to_uppercase();
        let spec = self.isa.get_by_name(&name_upper).ok_or_else(|| {
            AsmError::syntax(
                line_number,
                source_line,
                format!("unknown instruction {}", name_upper),
            )
        })?;

        bytecode.push(spec.opcode);

        if tokens.len() - 1 != spec.args.len() {
            return Err(AsmError::syntax(
                line_number,
                source_line,
                format!(
                    "wrong arg count for {}: expected {}, got {}",
                    name_upper,
                    spec.args.len(),
                    tokens.len() - 1
                ),
            ));
        }

        for (idx, arg_type) in spec.args.iter().enumerate() {
            let token = &tokens[idx + 1];
            match arg_type {
                ArgType::Register => {
                    let reg = parse_register(token)
                        .map_err(|e| AsmError::syntax(line_number, source_line, e))?;
                    bytecode.push(reg);
                }
                ArgType::Value => {
                    let val = self
                        .eval_u8(token, symbols, scope)
                        .map_err(|e| AsmError::syntax(line_number, source_line, e))?;
                    bytecode.push(val);
                }
                ArgType::Address => {
                    let addr = self
                        .eval_address(token, symbols, scope)
                        .map_err(|e| AsmError::syntax(line_number, source_line, e))?;
                    bytecode.extend([addr as u8, (addr >> 8) as u8]);
                }
                ArgType::Dword => {
                    let val = self
                        .eval_dword(token, symbols, scope)
                        .map_err(|e| AsmError::syntax(line_number, source_line, e))?;
                    bytecode.extend([
                        (val & 0xFF) as u8,
                        ((val >> 8) & 0xFF) as u8,
                        ((val >> 16) & 0xFF) as u8,
                        ((val >> 24) & 0xFF) as u8,
                    ]);
                }
            }
        }

        Ok(())
    }

    fn eval_u8(&self, s: &str, symbols: &HashMap<String, u16>, scope: &str) -> Result<u8, String> {
        let resolved = resolve_local_refs(s, scope);
        let val = eval_expr(&resolved, symbols).map_err(|e| e.to_string())?;
        if val > 255 {
            return Err(format!("value {} out of u8 range", val));
        }
        Ok(val as u8)
    }

    fn eval_address(
        &self,
        s: &str,
        symbols: &HashMap<String, u16>,
        scope: &str,
    ) -> Result<u16, String> {
        let resolved = resolve_local_refs(s, scope);
        eval_expr(&resolved, symbols).map_err(|e| format!("invalid address '{}': {}", s, e))
    }

    fn eval_dword(
        &self,
        s: &str,
        symbols: &HashMap<String, u16>,
        scope: &str,
    ) -> Result<u32, String> {
        let resolved = resolve_local_refs(s, scope);
        let t = resolved.trim();
        if t.starts_with("0x") || t.starts_with("0X") {
            u32::from_str_radix(&t[2..], 16).map_err(|e| e.to_string())
        } else if t.starts_with("0b") || t.starts_with("0B") {
            u32::from_str_radix(&t[2..], 2).map_err(|e| e.to_string())
        } else if t
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit() || c == '-')
        {
            t.parse::<i32>()
                .map(|v| v as u32)
                .map_err(|e| e.to_string())
        } else {
            eval_expr(t, symbols)
                .map(|v| v as u32)
                .map_err(|e| format!("invalid dword '{}': {}", s, e))
        }
    }
}

fn parse_register(s: &str) -> Result<u8, String> {
    let upper = s.to_uppercase();
    let digits = upper.trim_start_matches('R');
    digits
        .parse::<u8>()
        .map_err(|_| format!("invalid register: {}", s))
}
