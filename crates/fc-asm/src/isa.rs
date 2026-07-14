use crate::opcodes::*;
use std::collections::HashMap;
use std::fmt::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgType {
    Register,
    Value,
    Address,
    Dword,
}

impl ArgType {
    /// Encoded operand width in bytes.
    pub const fn width(self) -> usize {
        match self {
            ArgType::Register | ArgType::Value => 1,
            ArgType::Address => 2,
            ArgType::Dword => 4,
        }
    }
}

pub struct OpcodeSpec {
    pub name: &'static str,
    pub opcode: u8,
    pub args: Vec<ArgType>,
}

impl OpcodeSpec {
    /// Total encoded size: opcode byte plus operand widths. Derived from
    /// `args` so the two can never disagree.
    pub fn size(&self) -> usize {
        1 + self.args.iter().map(|a| a.width()).sum::<usize>()
    }

    /// Format an encoded instruction (`bytes[0]` is the opcode) as debugger
    /// text, e.g. `MOV R1, 0x0005`. Returns an `(INCOMPLETE)` marker when
    /// fewer than `size()` bytes are supplied.
    pub fn format(&self, bytes: &[u8]) -> String {
        if bytes.len() < self.size() {
            return format!("{} (INCOMPLETE)", self.name);
        }
        let mut out = self.name.to_string();
        let mut off = 1;
        for (i, arg) in self.args.iter().enumerate() {
            out.push_str(if i == 0 { " " } else { ", " });
            let _ = match arg {
                ArgType::Register => write!(out, "R{}", bytes[off]),
                ArgType::Value => write!(out, "{}", bytes[off]),
                ArgType::Address => {
                    let v = u16::from_le_bytes([bytes[off], bytes[off + 1]]);
                    write!(out, "0x{v:04X}")
                }
                ArgType::Dword => {
                    let v = u32::from_le_bytes([
                        bytes[off],
                        bytes[off + 1],
                        bytes[off + 2],
                        bytes[off + 3],
                    ]);
                    write!(out, "0x{v:08X}")
                }
            };
            off += arg.width();
        }
        out
    }
}

pub struct IsaTable {
    by_name: HashMap<&'static str, usize>,
    by_opcode: [Option<usize>; 256],
    specs: Vec<OpcodeSpec>,
}

impl IsaTable {
    pub fn new(specs: Vec<OpcodeSpec>) -> Self {
        let mut by_name = HashMap::new();
        let mut by_opcode = [None; 256];
        for (i, spec) in specs.iter().enumerate() {
            by_name.insert(spec.name, i);
            by_opcode[spec.opcode as usize] = Some(i);
        }
        Self {
            by_name,
            by_opcode,
            specs,
        }
    }

    pub fn get_by_name(&self, name: &str) -> Option<&OpcodeSpec> {
        self.by_name.get(name).map(|&i| &self.specs[i])
    }

    pub fn get_by_opcode(&self, opcode: u8) -> Option<&OpcodeSpec> {
        self.by_opcode[opcode as usize].map(|i| &self.specs[i])
    }
}

fn op(name: &'static str, opcode: u8, args: &[ArgType]) -> OpcodeSpec {
    OpcodeSpec {
        name,
        opcode,
        args: args.to_vec(),
    }
}

/// The full instruction-set shape (mnemonic, opcode, operand types) — the
/// single source of truth shared by the assembler and the fc-vm interpreter.
pub fn default_specs() -> Vec<OpcodeSpec> {
    use ArgType::*;
    vec![
        op("CLS", OP_CLS, &[]),
        op("MOV", OP_MOV, &[Register, Address]),
        op("ADD", OP_ADD, &[Register, Address]),
        op("DEC", OP_DEC, &[Register]),
        op("DPX", OP_DPX, &[Value, Value, Value, Value, Value]),
        op("DPXR", OP_DPXR, &[Register, Register, Value, Value, Value]),
        op("SPT", OP_SPT, &[Register, Register, Register]),
        op("PAL", OP_PAL, &[Value, Value, Value, Value]),
        op(
            "TIL",
            OP_TIL,
            &[Register, Register, Register, Register, Value, Value],
        ),
        op("PRN", OP_PRN, &[Register, Register, Register, Register]),
        op("SUB", OP_SUB, &[Register, Address]),
        op("RND", OP_RND, &[Register, Address]),
        op("MOVR", OP_MOVR, &[Register, Register]),
        op("SLT", OP_SLT, &[Register, Register, Register]),
        op("FILL", OP_FILL, &[Value]),
        op("JMP", OP_JMP, &[Address]),
        op("JNZ", OP_JNZ, &[Register, Address]),
        op("JZ", OP_JZ, &[Register, Address]),
        op("JSR", OP_JSR, &[Address]),
        op("RET", OP_RET, &[]),
        op("ADDR", OP_ADDR, &[Register, Register]),
        op("SUBR", OP_SUBR, &[Register, Register]),
        op("PUSH", OP_PUSH, &[Register]),
        op("POP", OP_POP, &[Register]),
        op("GETSP", OP_GETSP, &[Register]),
        op("SETSP", OP_SETSP, &[Register]),
        op("MUL", OP_MUL, &[Register, Register]),
        op("DIV", OP_DIV, &[Register, Register]),
        op("MOD", OP_MOD, &[Register, Register]),
        op("FMUL", OP_FMUL, &[Register, Register]),
        op("FDIV", OP_FDIV, &[Register, Register]),
        op("IN", OP_IN, &[Register, Value]),
        op("AND", OP_AND, &[Register, Register]),
        op("OR", OP_OR, &[Register, Register]),
        op("XOR", OP_XOR, &[Register, Register]),
        op("NOT", OP_NOT, &[Register]),
        op("SHL", OP_SHL, &[Register, Value]),
        op("SHR", OP_SHR, &[Register, Value]),
        op("SAR", OP_SAR, &[Register, Value]),
        op("NEG", OP_NEG, &[Register]),
        op("SLTS", OP_SLTS, &[Register, Register, Register]),
        op("EQ", OP_EQ, &[Register, Register, Register]),
        op("LDM32", OP_LDM32, &[Register, Address]),
        op("STM32", OP_STM32, &[Address, Register]),
        op("LDM32I", OP_LDM32I, &[Register, Register]),
        op("STM32I", OP_STM32I, &[Register, Register]),
        op("MOV32", OP_MOV32, &[Register, Dword]),
        op("LDM", OP_LDM, &[Register, Address]),
        op("STM", OP_STM, &[Address, Register]),
        op("LDMI", OP_LDMI, &[Register, Register]),
        op("STMI", OP_STMI, &[Register, Register]),
        op("CPY", OP_CPY, &[Address, Address, Address]),
        op("LDMW", OP_LDMW, &[Register, Address]),
        op("STMW", OP_STMW, &[Address, Register]),
        op("MATH1", OP_MATH1, &[Register, Register, Value]),
        op("MAX", OP_MAX, &[Register, Register]),
        op("MIN", OP_MIN, &[Register, Register]),
        op("JREG", OP_JREG, &[Register]),
        op("TXTZ", OP_TXTZ, &[Register, Register, Register, Register]),
        op(
            "TAT",
            OP_TAT,
            &[Register, Register, Register, Register, Value],
        ),
        op("TSD", OP_TSD, &[Register, Register, Register]),
        op(
            "TXT",
            OP_TXT,
            &[Register, Register, Register, Register, Value],
        ),
        op("NUM", OP_NUM, &[Register, Register, Register, Register]),
        op("POSC", OP_POSC, &[Register, Register]),
        op("MOVC", OP_MOVC, &[Register, Register]),
        op("LOGR", OP_LOGR, &[Register]),
        op("LOGV", OP_LOGV, &[Address]),
        op("SND", OP_SND, &[Register, Register, Register]),
        op("SNDV", OP_SNDV, &[Address, Value, Value]),
        op("NOSND", OP_NOSND, &[]),
        op("NSND", OP_NSND, &[Register, Register, Register]),
        op("NSNDV", OP_NSNDV, &[Address, Value, Value]),
        op("SSTOP", OP_SSTOP, &[]),
        op("NSTOP", OP_NSTOP, &[]),
        op("SFX", OP_SFX, &[Value]),
        op("MUS", OP_MUS, &[Value]),
        op("NOMUS", OP_NOMUS, &[]),
        op("WAIT", OP_WAIT, &[]),
    ]
}

pub fn default_isa() -> IsaTable {
    IsaTable::new(default_specs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_duplicate_opcodes_or_names() {
        let specs = default_specs();
        let mut seen_op = [false; 256];
        let mut seen_name = std::collections::HashSet::new();
        for spec in &specs {
            assert!(
                !seen_op[spec.opcode as usize],
                "duplicate opcode 0x{:02X}",
                spec.opcode
            );
            seen_op[spec.opcode as usize] = true;
            assert!(seen_name.insert(spec.name), "duplicate name {}", spec.name);
        }
    }

    #[test]
    fn size_derived_from_args() {
        let isa = default_isa();
        let mov = isa.get_by_name("MOV").expect("MOV spec");
        assert_eq!(mov.size(), 4);
        let mov32 = isa.get_by_name("MOV32").expect("MOV32 spec");
        assert_eq!(mov32.size(), 6);
        let cls = isa.get_by_name("CLS").expect("CLS spec");
        assert_eq!(cls.size(), 1);
    }

    #[test]
    fn format_renders_operand_types() {
        let isa = default_isa();
        let mov = isa.get_by_name("MOV").expect("MOV spec");
        assert_eq!(mov.format(&[OP_MOV, 1, 0x34, 0x12]), "MOV R1, 0x1234");
        let fill = isa.get_by_name("FILL").expect("FILL spec");
        assert_eq!(fill.format(&[OP_FILL, 7]), "FILL 7");
        let mov32 = isa.get_by_name("MOV32").expect("MOV32 spec");
        assert_eq!(
            mov32.format(&[OP_MOV32, 2, 0x78, 0x56, 0x34, 0x12]),
            "MOV32 R2, 0x12345678"
        );
        let cls = isa.get_by_name("CLS").expect("CLS spec");
        assert_eq!(cls.format(&[OP_CLS]), "CLS");
        assert_eq!(mov.format(&[OP_MOV, 1]), "MOV (INCOMPLETE)");
    }
}
