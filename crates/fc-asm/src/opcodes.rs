//! Bytecode opcode constants — the single numeric source of truth for the ISA.
//!
//! Used by the assembler ISA table (`isa.rs`), the fc-lang code generator and
//! (via the ISA table) the fc-vm instruction set.

pub const OP_CLS: u8 = 0x00;
pub const OP_MOV: u8 = 0x01;
pub const OP_ADD: u8 = 0x02;
pub const OP_DEC: u8 = 0x03;
pub const OP_DPX: u8 = 0x04;
pub const OP_DPXR: u8 = 0x05;
pub const OP_SPT: u8 = 0x06;
pub const OP_PAL: u8 = 0x07;
pub const OP_TIL: u8 = 0x08;
pub const OP_PRN: u8 = 0x09;
pub const OP_SUB: u8 = 0x0A;
pub const OP_RND: u8 = 0x0B;
pub const OP_MOVR: u8 = 0x0C;
pub const OP_SLT: u8 = 0x0D;
pub const OP_FILL: u8 = 0x0E;
pub const OP_FILLR: u8 = 0x0F;

pub const OP_JMP: u8 = 0x10;
pub const OP_JNZ: u8 = 0x11;
pub const OP_JZ: u8 = 0x12;
pub const OP_JSR: u8 = 0x13;
pub const OP_RET: u8 = 0x14;
pub const OP_ADDR: u8 = 0x15;
pub const OP_SUBR: u8 = 0x16;
pub const OP_PUSH: u8 = 0x17;
pub const OP_POP: u8 = 0x18;
pub const OP_GETSP: u8 = 0x19;
pub const OP_SETSP: u8 = 0x1A;
pub const OP_MUL: u8 = 0x1B;
pub const OP_DIV: u8 = 0x1C;
pub const OP_MOD: u8 = 0x1D;
pub const OP_FMUL: u8 = 0x1E;
pub const OP_FDIV: u8 = 0x1F;

pub const OP_IN: u8 = 0x20;
pub const OP_AND: u8 = 0x21;
pub const OP_OR: u8 = 0x22;
pub const OP_XOR: u8 = 0x23;
pub const OP_NOT: u8 = 0x24;
pub const OP_SHL: u8 = 0x25;
pub const OP_SHR: u8 = 0x26;
pub const OP_SAR: u8 = 0x27;
pub const OP_NEG: u8 = 0x28;
pub const OP_SLTS: u8 = 0x29;
pub const OP_EQ: u8 = 0x2A;
pub const OP_LDM32: u8 = 0x2B;
pub const OP_STM32: u8 = 0x2C;
pub const OP_LDM32I: u8 = 0x2D;
pub const OP_STM32I: u8 = 0x2E;
pub const OP_MOV32: u8 = 0x2F;

pub const OP_LDM: u8 = 0x30;
pub const OP_STM: u8 = 0x31;
pub const OP_LDMI: u8 = 0x32; // byte indirect load
pub const OP_STMI: u8 = 0x33; // byte indirect store
pub const OP_CPY: u8 = 0x34;
pub const OP_LDMW: u8 = 0x35;
pub const OP_STMW: u8 = 0x36;
pub const OP_MATH1: u8 = 0x37;
pub const OP_MAX: u8 = 0x38;
pub const OP_MIN: u8 = 0x39;
pub const OP_JREG: u8 = 0x3A;
pub const OP_TXTZ: u8 = 0x3B;
pub const OP_INR: u8 = 0x3C; // button held, button id from register
pub const OP_INP: u8 = 0x3D; // button just pressed (btnp), literal button id
pub const OP_INPR: u8 = 0x3E; // button just pressed, button id from register
pub const OP_RNDR: u8 = 0x3F; // random with max from register

pub const OP_TAT: u8 = 0x40;
pub const OP_TSD: u8 = 0x41;
pub const OP_TXT: u8 = 0x42;
pub const OP_NUM: u8 = 0x43;
pub const OP_PALR: u8 = 0x44; // palette entry from registers
pub const OP_LINE: u8 = 0x45; // line, palette color
pub const OP_RECT: u8 = 0x46; // rectangle outline
pub const OP_RECTF: u8 = 0x47; // filled rectangle
pub const OP_CIRC: u8 = 0x48; // circle outline
pub const OP_CIRCF: u8 = 0x49; // filled circle
pub const OP_PSET: u8 = 0x4A; // pixel with palette color
pub const OP_MGET: u8 = 0x4B; // read tile at map cell
pub const OP_MSET: u8 = 0x4C; // write tile at map cell
pub const OP_FGET: u8 = 0x4D; // read sprite flags
pub const OP_FSET: u8 = 0x4E; // write sprite flags
pub const OP_MAPD: u8 = 0x4F; // draw map region (implicit bases)
pub const OP_SPR: u8 = 0x50; // draw sprite by id with flip flags

pub const OP_TNEW: u8 = 0x51; // new native table → id
pub const OP_TGET: u8 = 0x52; // table get (0 when missing)
pub const OP_TSET: u8 = 0x53; // table set (insert or overwrite)
pub const OP_TLEN: u8 = 0x54; // table entry count
pub const OP_TIDX: u8 = 0x55; // entry (key, val) at insertion index
pub const OP_CHKHEAP: u8 = 0x56; // fault if candidate heap-top >= live SP

pub const OP_POSC: u8 = 0x60;
pub const OP_MOVC: u8 = 0x61;

pub const OP_LOGR: u8 = 0x70;
pub const OP_LOGV: u8 = 0x71;

pub const OP_SND: u8 = 0x80;
pub const OP_SNDV: u8 = 0x81;
pub const OP_NOSND: u8 = 0x82;
pub const OP_NSND: u8 = 0x83;
pub const OP_NSNDV: u8 = 0x84;
pub const OP_SSTOP: u8 = 0x85;
pub const OP_NSTOP: u8 = 0x86;
pub const OP_SFX: u8 = 0x87;
pub const OP_MUS: u8 = 0x88;
pub const OP_NOMUS: u8 = 0x89;
pub const OP_SFXR: u8 = 0x8A; // play sfx, id from register
pub const OP_MUSR: u8 = 0x8B; // play music, id from register

pub const OP_WAIT: u8 = 0xFF;
