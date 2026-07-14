//! Integration tests for the assembler: instruction encoding, label
//! resolution, sections, macros, constants and error reporting.

#![allow(clippy::unwrap_used)]

use fc_asm::{AsmError, assemble, assemble_with_sections};

#[test]
fn encodes_basic_instructions() {
    // CLS = 0x00 (1 byte)
    assert_eq!(assemble("CLS").unwrap(), vec![0x00]);
    // MOV = 0x01, register byte, address as u16 LE
    assert_eq!(assemble("MOV R0 5").unwrap(), vec![0x01, 0x00, 0x05, 0x00]);
    assert_eq!(
        assemble("MOV R3 0x1234").unwrap(),
        vec![0x01, 0x03, 0x34, 0x12]
    );
}

#[test]
fn resolves_forward_labels() {
    // JMP (0x10, 3 bytes) + CLS (1 byte) => "end" sits at pc 4
    let bytes = assemble("JMP end\nCLS\nend:\nRET").unwrap();
    assert_eq!(bytes, vec![0x10, 0x04, 0x00, 0x00, 0x14]);
}

#[test]
fn resolves_scoped_local_labels() {
    // @done is scoped to the enclosing "main" label
    let bytes = assemble("main:\nJMP @done\nCLS\n@done:\nRET").unwrap();
    assert_eq!(bytes, vec![0x10, 0x04, 0x00, 0x00, 0x14]);
}

#[test]
fn const_values_usable_in_operands() {
    let bytes = assemble(".CONST X = 0x10 + 2\nMOV R1 X").unwrap();
    assert_eq!(bytes, vec![0x01, 0x01, 0x12, 0x00]);
}

#[test]
fn macros_expand_to_instructions() {
    let src = ".MACRO ZERO reg\nMOV reg 0\n.ENDM\nZERO R3";
    assert_eq!(assemble(src).unwrap(), vec![0x01, 0x03, 0x00, 0x00]);
}

#[test]
fn db_directive_emits_raw_bytes() {
    assert_eq!(assemble("CLS\n.DB 1 2 3").unwrap(), vec![0x00, 1, 2, 3]);
}

#[test]
fn sprite_sheet_section_is_split_from_program() {
    let src = ".BEGIN_SPRITE_SHEET\n.DB 1 2 3\n.END_SPRITE_SHEET\nCLS";
    let out = assemble_with_sections(src).unwrap();
    assert_eq!(out.program, vec![0x00]);
    // 0x0002 = SectionKind::SpriteSheet wire id
    assert_eq!(out.extra_sections, vec![(0x0002, vec![1, 2, 3])]);
}

#[test]
fn unknown_instruction_reports_line_number() {
    let err = assemble("CLS\nFROB R0").unwrap_err();
    let AsmError::Syntax { line, message, .. } = err;
    assert_eq!(line, 2);
    assert!(message.contains("unknown instruction"), "{message}");
}

#[test]
fn wrong_arg_count_is_syntax_error() {
    let err = assemble("MOV R0").unwrap_err();
    let AsmError::Syntax { message, .. } = err;
    assert!(message.contains("wrong arg count"), "{message}");
}

#[test]
fn undefined_symbol_is_error() {
    assert!(assemble("MOV R0 NOPE").is_err());
}

#[test]
fn logv_encodes_16bit_operand() {
    // LOGV = 0x71 with a u16 LE operand — the VM reads a full word, so the
    // encoded size must be 3 bytes (regression test for the old size/args
    // mismatch that desynced label addresses).
    assert_eq!(assemble("LOGV 300").unwrap(), vec![0x71, 0x2C, 0x01]);
    let bytes = assemble("LOGV 1\nend:\nJMP end").unwrap();
    assert_eq!(bytes, vec![0x71, 0x01, 0x00, 0x10, 0x03, 0x00]);
}

#[test]
fn txtz_is_assemblable() {
    // TXTZ = 0x3B, four register operands (previously VM-only).
    assert_eq!(
        assemble("TXTZ R1 R2 R3 R4").unwrap(),
        vec![0x3B, 1, 2, 3, 4]
    );
}
