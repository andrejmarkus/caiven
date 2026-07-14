//! Debugger support for [`Vm`]: disassembly and snapshot/restore.

use super::sfx::{MusicPlayer, SfxPlayer};
use super::{Vm, VmFault};
use fc_core::Color;

impl Vm {
    pub fn disassemble(&self, pc: usize) -> String {
        let program = self.get_program();
        let source_map = self.get_source_map();

        if pc >= program.len() {
            return "OUT OF BOUNDS".to_string();
        }

        let mut info_parts = Vec::new();
        if let Some(address_info) = source_map.get(pc) {
            for label in &address_info.labels {
                info_parts.push(format!("[{}]", label.to_uppercase()));
            }

            if let Some(item) = &address_info.item {
                match item {
                    fc_asm::ItemInfo::Instruction {
                        name: _,
                        opcode: _,
                        size,
                    } => {
                        let opcode = program[pc];
                        let instruction = self.get_instruction_by_opcode(opcode);
                        if let Some(instr) = instruction {
                            let end = (pc + size).min(program.len());
                            let bytes = &program[pc..end];
                            info_parts.push(instr.spec.format(bytes));
                        } else {
                            info_parts.push(format!("UNKNOWN OPCODE: 0X{:02X}", opcode));
                        }
                    }
                    fc_asm::ItemInfo::Directive { name, size } => {
                        let end = (pc + size).min(program.len());
                        let bytes = &program[pc..end];
                        let hex_string: Vec<String> =
                            bytes.iter().map(|b| format!("{:02X}", b)).collect();
                        info_parts.push(format!("{} {}", name, hex_string.join(" ")));
                    }
                }
            }
        }

        if info_parts.is_empty() {
            format!(".DB 0X{:02X}", program[pc])
        } else {
            info_parts.join(" ")
        }
    }

    pub fn snapshot(&self) -> VmSnapshot {
        VmSnapshot {
            pc: self.cpu.get_pc(),
            registers: self.cpu.get_registers().to_vec(),
            memory: self.memory.get_ram().to_vec(),
            tables: self.tables.clone(),
            camera_x: self.camera.get_x(),
            camera_y: self.camera.get_y(),
            palette: self.palette.get_colors().to_vec(),
            waiting: self.waiting,
            fault: self.fault,
            frame_count: self.frame_count,
            world: self.world.get_pixels().to_vec(),
            ui: self.ui.get_pixels().to_vec(),
            sfx_player: self.sfx_player.clone(),
            music_player: self.music_player.clone(),
        }
    }

    pub fn restore(&mut self, snapshot: &VmSnapshot) {
        self.cpu.set_pc(snapshot.pc);
        for (i, val) in snapshot.registers.iter().enumerate() {
            self.cpu.set_register(i, *val);
        }
        self.memory.set_ram(snapshot.memory.clone());
        self.tables = snapshot.tables.clone();
        self.camera
            .set_position(snapshot.camera_x, snapshot.camera_y);
        self.palette.set_colors(snapshot.palette.clone());
        self.waiting = snapshot.waiting;
        self.fault = snapshot.fault;
        self.frame_count = snapshot.frame_count;
        self.world.set_pixels(snapshot.world.clone());
        self.ui.set_pixels(snapshot.ui.clone());
        self.sfx_player = snapshot.sfx_player.clone();
        self.music_player = snapshot.music_player.clone();
    }
}

#[derive(Clone)]
pub struct VmSnapshot {
    pub pc: usize,
    pub registers: Vec<u32>,
    pub memory: Vec<u8>,
    pub tables: super::TableStore,
    pub camera_x: u32,
    pub camera_y: u32,
    pub palette: Vec<Color>,
    pub waiting: bool,
    pub fault: Option<VmFault>,
    pub frame_count: u32,
    pub world: Vec<u8>,
    pub ui: Vec<u8>,
    pub sfx_player: SfxPlayer,
    pub music_player: MusicPlayer,
}
