use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgType {
    Register,
    Value,
    Address,
}

pub struct OpcodeSpec {
    pub name: &'static str,
    pub opcode: u8,
    pub size: usize,
    pub args: Vec<ArgType>,
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

pub fn default_isa() -> IsaTable {
    use ArgType::*;
    IsaTable::new(vec![
        OpcodeSpec {
            name: "CLS",
            opcode: 0x00,
            size: 1,
            args: vec![],
        },
        OpcodeSpec {
            name: "MOV",
            opcode: 0x01,
            size: 4,
            args: vec![Register, Address],
        },
        OpcodeSpec {
            name: "ADD",
            opcode: 0x02,
            size: 4,
            args: vec![Register, Address],
        },
        OpcodeSpec {
            name: "DEC",
            opcode: 0x03,
            size: 2,
            args: vec![Register],
        },
        OpcodeSpec {
            name: "DPX",
            opcode: 0x04,
            size: 6,
            args: vec![Value, Value, Value, Value, Value],
        },
        OpcodeSpec {
            name: "DPXR",
            opcode: 0x05,
            size: 6,
            args: vec![Register, Register, Value, Value, Value],
        },
        OpcodeSpec {
            name: "SPT",
            opcode: 0x06,
            size: 4,
            args: vec![Register, Register, Register],
        },
        OpcodeSpec {
            name: "PAL",
            opcode: 0x07,
            size: 5,
            args: vec![Value, Value, Value, Value],
        },
        OpcodeSpec {
            name: "TIL",
            opcode: 0x08,
            size: 7,
            args: vec![Register, Register, Register, Register, Value, Value],
        },
        OpcodeSpec {
            name: "PRN",
            opcode: 0x09,
            size: 5,
            args: vec![Register, Register, Register, Register],
        },
        OpcodeSpec {
            name: "SUB",
            opcode: 0x0A,
            size: 4,
            args: vec![Register, Address],
        },
        OpcodeSpec {
            name: "RND",
            opcode: 0x0B,
            size: 4,
            args: vec![Register, Address],
        },
        OpcodeSpec {
            name: "MOVR",
            opcode: 0x0C,
            size: 3,
            args: vec![Register, Register],
        },
        OpcodeSpec {
            name: "SLT",
            opcode: 0x0D,
            size: 4,
            args: vec![Register, Register, Register],
        },
        OpcodeSpec {
            name: "FILL",
            opcode: 0x0E,
            size: 2,
            args: vec![Value],
        },
        OpcodeSpec {
            name: "JMP",
            opcode: 0x10,
            size: 3,
            args: vec![Address],
        },
        OpcodeSpec {
            name: "JNZ",
            opcode: 0x11,
            size: 4,
            args: vec![Register, Address],
        },
        OpcodeSpec {
            name: "JZ",
            opcode: 0x12,
            size: 4,
            args: vec![Register, Address],
        },
        OpcodeSpec {
            name: "JSR",
            opcode: 0x13,
            size: 3,
            args: vec![Address],
        },
        OpcodeSpec {
            name: "RET",
            opcode: 0x14,
            size: 1,
            args: vec![],
        },
        OpcodeSpec {
            name: "ADDR",
            opcode: 0x15,
            size: 3,
            args: vec![Register, Register],
        },
        OpcodeSpec {
            name: "SUBR",
            opcode: 0x16,
            size: 3,
            args: vec![Register, Register],
        },
        OpcodeSpec {
            name: "IN",
            opcode: 0x20,
            size: 3,
            args: vec![Register, Value],
        },
        OpcodeSpec {
            name: "LDM",
            opcode: 0x30,
            size: 4,
            args: vec![Register, Address],
        },
        OpcodeSpec {
            name: "STM",
            opcode: 0x31,
            size: 4,
            args: vec![Address, Register],
        },
        OpcodeSpec {
            name: "LDMI",
            opcode: 0x32,
            size: 3,
            args: vec![Register, Register],
        },
        OpcodeSpec {
            name: "STMI",
            opcode: 0x33,
            size: 3,
            args: vec![Register, Register],
        },
        OpcodeSpec {
            name: "CPY",
            opcode: 0x34,
            size: 7,
            args: vec![Address, Address, Address],
        },
        OpcodeSpec {
            name: "LDMW",
            opcode: 0x35,
            size: 4,
            args: vec![Register, Address],
        },
        OpcodeSpec {
            name: "STMW",
            opcode: 0x36,
            size: 4,
            args: vec![Address, Register],
        },
        OpcodeSpec {
            name: "TAT",
            opcode: 0x40,
            size: 6,
            args: vec![Register, Register, Register, Register, Value],
        },
        OpcodeSpec {
            name: "TSD",
            opcode: 0x41,
            size: 4,
            args: vec![Register, Register, Register],
        },
        OpcodeSpec {
            name: "TXT",
            opcode: 0x42,
            size: 6,
            args: vec![Register, Register, Register, Register, Value],
        },
        OpcodeSpec {
            name: "NUM",
            opcode: 0x43,
            size: 5,
            args: vec![Register, Register, Register, Register],
        },
        OpcodeSpec {
            name: "POSC",
            opcode: 0x60,
            size: 3,
            args: vec![Register, Register],
        },
        OpcodeSpec {
            name: "MOVC",
            opcode: 0x61,
            size: 3,
            args: vec![Register, Register],
        },
        OpcodeSpec {
            name: "LOGR",
            opcode: 0x70,
            size: 2,
            args: vec![Register],
        },
        OpcodeSpec {
            name: "LOGV",
            opcode: 0x71,
            size: 3,
            args: vec![Value],
        },
        OpcodeSpec {
            name: "SND",
            opcode: 0x80,
            size: 4,
            args: vec![Register, Register, Register],
        },
        OpcodeSpec {
            name: "SNDV",
            opcode: 0x81,
            size: 5,
            args: vec![Address, Value, Value],
        },
        OpcodeSpec {
            name: "NOSND",
            opcode: 0x82,
            size: 1,
            args: vec![],
        },
        OpcodeSpec {
            name: "NSND",
            opcode: 0x83,
            size: 4,
            args: vec![Register, Register, Register],
        },
        OpcodeSpec {
            name: "NSNDV",
            opcode: 0x84,
            size: 5,
            args: vec![Address, Value, Value],
        },
        OpcodeSpec {
            name: "SSTOP",
            opcode: 0x85,
            size: 1,
            args: vec![],
        },
        OpcodeSpec {
            name: "NSTOP",
            opcode: 0x86,
            size: 1,
            args: vec![],
        },
        OpcodeSpec {
            name: "WAIT",
            opcode: 0xFF,
            size: 1,
            args: vec![],
        },
    ])
}
