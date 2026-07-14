//! Builds the default VM instruction set from the shared fc-asm ISA table.
//!
//! The instruction shape (mnemonic, opcode, operand types) lives solely in
//! `fc_asm::default_specs()`; this module only attaches the VM execute
//! handler to each mnemonic.

use crate::isa::instruction_set::InstructionHandler;
use crate::isa::operations;
use crate::isa::{Instruction, InstructionSet};

fn handler_for(name: &str) -> Option<InstructionHandler> {
    let handler: InstructionHandler = match name {
        "CLS" => operations::clear_screen,
        "MOV" => operations::move_value,
        "ADD" => operations::add_value,
        "DEC" => operations::decrement_value,
        "DPX" => operations::draw_pixel,
        "DPXR" => operations::draw_pixel_from_register,
        "SPT" => operations::sprite,
        "PAL" => operations::palette,
        "TIL" => operations::tilemap,
        "PRN" => operations::print,
        "SUB" => operations::subtract_value,
        "RND" => operations::random_value,
        "MOVR" => operations::move_register,
        "SLT" => operations::set_less_than,
        "FILL" => operations::fill_screen,
        "FILLR" => operations::fill_screen_reg,
        "JMP" => operations::jump,
        "JNZ" => operations::jump_if_not_zero,
        "JZ" => operations::jump_if_zero,
        "JSR" => operations::jump_subroutine,
        "RET" => operations::return_subroutine,
        "ADDR" => operations::add_register,
        "SUBR" => operations::subtract_register,
        "PUSH" => operations::push_register,
        "POP" => operations::pop_register,
        "GETSP" => operations::get_sp,
        "SETSP" => operations::set_sp,
        "MUL" => operations::mul_register,
        "DIV" => operations::div_register,
        "MOD" => operations::mod_register,
        "FMUL" => operations::fmul_register,
        "FDIV" => operations::fdiv_register,
        "IN" => operations::input,
        "INR" => operations::input_reg,
        "INP" => operations::input_pressed,
        "INPR" => operations::input_pressed_reg,
        "RNDR" => operations::random_value_reg,
        "AND" => operations::and_register,
        "OR" => operations::or_register,
        "XOR" => operations::xor_register,
        "NOT" => operations::not_register,
        "SHL" => operations::shl_register,
        "SHR" => operations::shr_register,
        "SAR" => operations::sar_register,
        "NEG" => operations::neg_register,
        "SLTS" => operations::set_less_than_signed,
        "EQ" => operations::set_equal,
        "LDM32" => operations::load_dword_from_memory,
        "STM32" => operations::store_dword_to_memory,
        "LDM32I" => operations::load_dword_from_memory_indirect,
        "STM32I" => operations::store_dword_to_memory_indirect,
        "MOV32" => operations::move_value_32,
        "LDM" => operations::load_from_memory,
        "STM" => operations::store_to_memory,
        "LDMI" => operations::load_from_memory_indirect,
        "STMI" => operations::store_to_memory_indirect,
        "CPY" => operations::copy,
        "LDMW" => operations::load_word_from_memory,
        "STMW" => operations::store_word_to_memory,
        "MATH1" => operations::math1,
        "MAX" => operations::max_register,
        "MIN" => operations::min_register,
        "JREG" => operations::jump_register_subroutine,
        "TXTZ" => operations::text_nullterm,
        "TAT" => operations::tile_at,
        "TSD" => operations::tile_solid,
        "TXT" => operations::text,
        "NUM" => operations::draw_number,
        "PALR" => operations::palette_reg,
        "LINE" => operations::line,
        "RECT" => operations::rect,
        "RECTF" => operations::rect_fill,
        "CIRC" => operations::circ,
        "CIRCF" => operations::circ_fill,
        "PSET" => operations::pset,
        "MGET" => operations::map_get,
        "MSET" => operations::map_set,
        "FGET" => operations::flags_get,
        "FSET" => operations::flags_set,
        "MAPD" => operations::map_draw,
        "SPR" => operations::sprite_by_id,
        "TNEW" => operations::table_new,
        "TGET" => operations::table_get,
        "TSET" => operations::table_set,
        "TLEN" => operations::table_len,
        "TIDX" => operations::table_entry_at,
        "POSC" => operations::set_camera_position,
        "MOVC" => operations::move_camera,
        "LOGR" => operations::log_register,
        "LOGV" => operations::log_value,
        "SND" => operations::play_sound,
        "SNDV" => operations::play_sound_value,
        "NOSND" => operations::stop_sound,
        "NSND" => operations::play_noise,
        "NSNDV" => operations::play_noise_value,
        "SSTOP" => operations::stop_square,
        "NSTOP" => operations::stop_noise,
        "SFX" => operations::play_sfx,
        "MUS" => operations::play_music,
        "NOMUS" => operations::stop_music_opcode,
        "SFXR" => operations::play_sfx_reg,
        "MUSR" => operations::play_music_reg,
        "WAIT" => operations::wait,
        _ => return None,
    };
    Some(handler)
}

pub fn default_instruction_set() -> InstructionSet {
    let mut set = InstructionSet::new();
    for spec in fc_asm::default_specs() {
        let Some(execute) = handler_for(spec.name) else {
            panic!("no VM handler for ISA instruction {}", spec.name);
        };
        set.register(Instruction { spec, execute });
    }
    set
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_isa_spec_has_a_handler() {
        // default_instruction_set panics on a spec without a handler.
        let set = default_instruction_set();
        for spec in fc_asm::default_specs() {
            assert!(
                set.get_by_opcode(spec.opcode).is_some(),
                "missing instruction for {}",
                spec.name
            );
        }
    }

    #[test]
    fn debug_format_comes_from_shared_spec() {
        let set = default_instruction_set();
        let mov = set.get_by_opcode(0x01).expect("MOV registered");
        assert_eq!(mov.spec.format(&[0x01, 1, 0x34, 0x12]), "MOV R1, 0x1234");
    }
}
