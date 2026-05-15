use std::collections::HashMap;
use crate::ast::*;
use crate::error::{LangError, Result};
use fc_asm::SourceMap;

const GLOBALS_BASE: u16 = 0x0000;
const SCRATCH_BASE: u16 = 0x3FF0;
const SCRATCH_STEP: u16 = 4;
const FP_SAVE_ADDR: u16 = 0x3FEC;
const STRING_POOL_BASE: u16 = 0x3800;

// Heap allocator (bump pointer)
const HEAP_BASE: u32      = 0x6000;
const HEAP_TOP_ADDR: u16  = 0x5000; // u32 at 0x5000: current heap top
// Runtime scratch (non-reentrant; table ops don't nest)
const RT_TMP0: u16 = 0x5004;
const RT_TMP1: u16 = 0x5008;
const RT_TMP2: u16 = 0x500C;
const RT_TMP3: u16 = 0x5010;
const RT_TMP4: u16 = 0x5014; // iteration counter for __rt_settab probe loop
// Table layout constants
const TABLE_CAP: u32    = 8;         // fixed capacity (power-of-2 → bitmask works)
const TABLE_ENTRY_SZ: u32 = 8;       // key(u32) + val(u32)
const TABLE_HDR_SZ: u32 = 8;         // cap(u32) + count(u32)
const TABLE_ALLOC_SZ: u32 = TABLE_HDR_SZ + TABLE_CAP * TABLE_ENTRY_SZ; // 72
const TABLE_SENTINEL: u32 = 0xFFFFFFFF; // marks empty slot key

// Opcode constants
const OP_MOV: u8     = 0x01;
const OP_DPX: u8     = 0x04;
const OP_SPT: u8     = 0x06;
const OP_PAL: u8     = 0x07;
const OP_TIL: u8     = 0x08;

const OP_RND: u8     = 0x0B;
const OP_MOVR: u8    = 0x0C;
const OP_FILL: u8    = 0x0E;
const OP_JMP: u8     = 0x10;
const OP_JNZ: u8     = 0x11;
const OP_JZ: u8      = 0x12;
const OP_JSR: u8     = 0x13;
const OP_RET: u8     = 0x14;
const OP_ADDR: u8    = 0x15;
const OP_SUBR: u8    = 0x16;
const OP_PUSH: u8    = 0x17;
const OP_POP: u8     = 0x18;
const OP_GETSP: u8   = 0x19;
const OP_SETSP: u8   = 0x1A;
const OP_MUL: u8     = 0x1B;
const OP_DIV: u8     = 0x1C;
const OP_MOD: u8     = 0x1D;
const OP_AND: u8     = 0x21;
const OP_NEG: u8     = 0x28;
const OP_SLTS: u8    = 0x29;
const OP_EQ: u8      = 0x2A;
const OP_LDM32: u8   = 0x2B;
const OP_STM32: u8   = 0x2C;
const OP_LDM32I: u8  = 0x2D;
const OP_STM32I: u8  = 0x2E;
const OP_MOV32: u8   = 0x2F;
const OP_IN: u8      = 0x20;
const OP_CPY: u8     = 0x34;
const OP_TXT: u8     = 0x42;
const OP_NUM: u8     = 0x43;
const OP_SFX: u8     = 0x87;
const OP_MUS: u8     = 0x88;
const OP_NOMUS: u8   = 0x89;
const OP_WAIT: u8    = 0xFF;
const OP_CLS: u8     = 0x00;

#[derive(Clone, Debug)]
enum VarLoc {
    Const(u32),
    Global(u16),     // absolute RAM address (4-byte cell)
    Local(usize),    // slot index (FP - slot*4)
    Param(usize),    // param index (FP + (idx+1)*4, stored by caller)
}

struct BreakTarget {
    end_label: String,
    slots_at_entry: usize,
}

struct FnCtx {
    params: Vec<String>,
    scopes: Vec<HashMap<String, usize>>,
    next_slot: usize,
    break_targets: Vec<BreakTarget>,
}

impl FnCtx {
    fn new(params: Vec<String>) -> Self {
        FnCtx {
            params,
            scopes: vec![HashMap::new()],
            next_slot: 0,
            break_targets: Vec::new(),
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) -> usize {
        let scope = self.scopes.pop().unwrap_or_default();
        let freed = scope.len();
        self.next_slot -= freed;
        freed
    }

    fn alloc_local(&mut self, name: String) -> usize {
        let slot = self.next_slot;
        self.next_slot += 1;
        self.scopes.last_mut().unwrap().insert(name, slot);
        slot
    }

    fn lookup(&self, name: &str) -> Option<VarLoc> {
        // Check locals (inner-most scope first)
        for scope in self.scopes.iter().rev() {
            if let Some(&slot) = scope.get(name) {
                return Some(VarLoc::Local(slot));
            }
        }
        // Check params
        for (i, p) in self.params.iter().enumerate() {
            if p == name {
                return Some(VarLoc::Param(i));
            }
        }
        None
    }
}

pub struct Compiler {
    code: Vec<u8>,
    source_map: SourceMap,
    consts: HashMap<String, u32>,
    globals: HashMap<String, u16>,
    next_global: u16,
    fn_ctx: Option<FnCtx>,
    top_break_targets: Vec<BreakTarget>,
    patches: Vec<(usize, String)>,
    labels: HashMap<String, usize>,
    label_counter: usize,
    string_pool: Vec<u8>,
    string_offsets: HashMap<String, u16>,
    cpy_src_patch: usize,
    cpy_len_patch: usize,
}

impl Compiler {
    pub fn new() -> Self {
        Compiler {
            code: Vec::new(),
            source_map: SourceMap::new(),
            consts: HashMap::new(),
            globals: HashMap::new(),
            next_global: GLOBALS_BASE,
            fn_ctx: None,
            top_break_targets: Vec::new(),
            patches: Vec::new(),
            labels: HashMap::new(),
            label_counter: 0,
            string_pool: Vec::new(),
            string_offsets: HashMap::new(),
            cpy_src_patch: 0,
            cpy_len_patch: 0,
        }
    }

    pub fn finish(mut self) -> Result<(Vec<u8>, SourceMap)> {
        self.apply_patches()?;
        // Patch CPY src/len and append string pool to ROM
        let pool_src = self.code.len();
        let pool_len = self.string_pool.len();
        self.code[self.cpy_src_patch]     = (pool_src & 0xFF) as u8;
        self.code[self.cpy_src_patch + 1] = ((pool_src >> 8) & 0xFF) as u8;
        self.code[self.cpy_len_patch]     = (pool_len & 0xFF) as u8;
        self.code[self.cpy_len_patch + 1] = ((pool_len >> 8) & 0xFF) as u8;
        self.code.extend_from_slice(&self.string_pool);
        Ok((self.code, self.source_map))
    }

    fn apply_patches(&mut self) -> Result<()> {
        for (offset, label) in &self.patches {
            if let Some(&target) = self.labels.get(label) {
                let lo = (target & 0xFF) as u8;
                let hi = ((target >> 8) & 0xFF) as u8;
                self.code[*offset] = lo;
                self.code[*offset + 1] = hi;
            } else {
                return Err(crate::error::LangError::UnresolvedLabel { label: label.clone() });
            }
        }
        Ok(())
    }

    fn fresh_label(&mut self, prefix: &str) -> String {
        let n = self.label_counter;
        self.label_counter += 1;
        format!("__{}__{}", prefix, n)
    }

    fn emit_label(&mut self, name: &str) {
        self.labels.insert(name.to_string(), self.code.len());
    }

    fn emit_jmp(&mut self, label: &str) {
        self.code.push(OP_JMP);
        self.patches.push((self.code.len(), label.to_string()));
        self.code.push(0);
        self.code.push(0);
    }

    fn emit_jz(&mut self, reg: u8, label: &str) {
        self.code.push(OP_JZ);
        self.code.push(reg);
        self.patches.push((self.code.len(), label.to_string()));
        self.code.push(0);
        self.code.push(0);
    }

    fn emit_jnz(&mut self, reg: u8, label: &str) {
        self.code.push(OP_JNZ);
        self.code.push(reg);
        self.patches.push((self.code.len(), label.to_string()));
        self.code.push(0);
        self.code.push(0);
    }

    fn emit_jsr(&mut self, label: &str) {
        self.code.push(OP_JSR);
        self.patches.push((self.code.len(), label.to_string()));
        self.code.push(0);
        self.code.push(0);
    }

    fn emit_addr16(&mut self, addr: u16) {
        self.code.push((addr & 0xFF) as u8);
        self.code.push(((addr >> 8) & 0xFF) as u8);
    }

    // MOV Rd, imm16 (address-sized immediate)
    fn emit_mov(&mut self, rd: u8, imm16: u16) {
        self.code.push(OP_MOV);
        self.code.push(rd);
        self.emit_addr16(imm16);
    }

    // MOV32 Rd, imm32
    fn emit_mov32(&mut self, rd: u8, imm32: u32) {
        self.code.push(OP_MOV32);
        self.code.push(rd);
        self.code.push((imm32 & 0xFF) as u8);
        self.code.push(((imm32 >> 8) & 0xFF) as u8);
        self.code.push(((imm32 >> 16) & 0xFF) as u8);
        self.code.push(((imm32 >> 24) & 0xFF) as u8);
    }

    // Emit load of u32 value into register, choosing MOV vs MOV32
    fn emit_mov_r0_imm(&mut self, val: u32) {
        if val <= 0xFFFF {
            self.emit_mov(0, val as u16);
        } else {
            self.emit_mov32(0, val);
        }
    }

    fn emit_movr(&mut self, rd: u8, rs: u8) {
        self.code.push(OP_MOVR);
        self.code.push(rd);
        self.code.push(rs);
    }

    fn emit_addr(&mut self, rd: u8, rs: u8) {
        self.code.push(OP_ADDR);
        self.code.push(rd);
        self.code.push(rs);
    }

    fn emit_subr(&mut self, rd: u8, rs: u8) {
        self.code.push(OP_SUBR);
        self.code.push(rd);
        self.code.push(rs);
    }

    fn emit_push(&mut self, reg: u8) {
        self.code.push(OP_PUSH);
        self.code.push(reg);
    }

    fn emit_pop(&mut self, reg: u8) {
        self.code.push(OP_POP);
        self.code.push(reg);
    }

    fn emit_ldm32(&mut self, rd: u8, addr: u16) {
        self.code.push(OP_LDM32);
        self.code.push(rd);
        self.emit_addr16(addr);
    }

    fn emit_stm32(&mut self, addr: u16, rs: u8) {
        self.code.push(OP_STM32);
        self.emit_addr16(addr);
        self.code.push(rs);
    }

    fn emit_ldm32i(&mut self, rd: u8, raddr: u8) {
        self.code.push(OP_LDM32I);
        self.code.push(rd);
        self.code.push(raddr);
    }

    fn emit_stm32i(&mut self, raddr: u8, rs: u8) {
        self.code.push(OP_STM32I);
        self.code.push(raddr);
        self.code.push(rs);
    }

    fn emit_getsp(&mut self, rd: u8) {
        self.code.push(OP_GETSP);
        self.code.push(rd);
    }

    // Intern a string literal into the pool; returns its RAM address after CPY.
    fn intern_string(&mut self, s: &str) -> u16 {
        if let Some(&off) = self.string_offsets.get(s) {
            return STRING_POOL_BASE + off;
        }
        let off = self.string_pool.len() as u16;
        self.string_offsets.insert(s.to_string(), off);
        self.string_pool.extend_from_slice(s.as_bytes());
        self.string_pool.push(0); // null terminator
        STRING_POOL_BASE + off
    }

    fn emit_setsp(&mut self, rs: u8) {
        self.code.push(OP_SETSP);
        self.code.push(rs);
    }

    fn emit_mul_reg(&mut self, rd: u8, rs: u8) {
        self.code.push(OP_MUL);
        self.code.push(rd);
        self.code.push(rs);
    }

    fn emit_and_reg(&mut self, rd: u8, rs: u8) {
        self.code.push(OP_AND);
        self.code.push(rd);
        self.code.push(rs);
    }

    // Load local slot i into R0: R0 = mem[FP - (slot+1)*4]
    // slot 0 → FP-4, slot 1 → FP-8, ...
    fn emit_load_local(&mut self, slot: usize) {
        // R1 = FP
        self.emit_movr(1, 3);
        // R2 = (slot+1)*4
        self.emit_mov(2, ((slot + 1) * 4) as u16);
        // R1 = R1 - R2
        self.emit_subr(1, 2);
        // R0 = mem[R1]
        self.emit_ldm32i(0, 1);
    }

    // Store R0 into local slot i: mem[FP - (slot+1)*4] = R0
    fn emit_store_local(&mut self, slot: usize) {
        // R1 = FP
        self.emit_movr(1, 3);
        // R2 = (slot+1)*4
        self.emit_mov(2, ((slot + 1) * 4) as u16);
        // R1 = R1 - R2
        self.emit_subr(1, 2);
        // mem[R1] = R0
        self.emit_stm32i(1, 0);
    }

    // Load param i into R0: R0 = mem[FP + 6 + i*4]
    // Stack layout (JSR pushes 2 bytes): [old_FP(4), ret(2), arg0(4), arg1(4), ...]
    fn emit_load_param(&mut self, param_idx: usize) {
        // R1 = FP
        self.emit_movr(1, 3);
        // R2 = 6 + param_idx*4
        self.emit_mov(2, (6 + param_idx * 4) as u16);
        // R1 = R1 + R2
        self.emit_addr(1, 2);
        // R0 = mem[R1]
        self.emit_ldm32i(0, 1);
    }

    // Store R0 into param slot i: mem[FP + 6 + i*4] = R0
    fn emit_store_param(&mut self, param_idx: usize) {
        // R1 = FP
        self.emit_movr(1, 3);
        // R2 = 6 + param_idx*4
        self.emit_mov(2, (6 + param_idx * 4) as u16);
        // R1 = R1 + R2
        self.emit_addr(1, 2);
        // mem[R1] = R0
        self.emit_stm32i(1, 0);
    }

    fn lookup_var(&self, name: &str) -> Option<VarLoc> {
        if let Some(ctx) = &self.fn_ctx {
            if let Some(loc) = ctx.lookup(name) {
                return Some(loc);
            }
        }
        if let Some(&val) = self.consts.get(name) {
            return Some(VarLoc::Const(val));
        }
        if let Some(&addr) = self.globals.get(name) {
            return Some(VarLoc::Global(addr));
        }
        None
    }

    fn alloc_global(&mut self, name: &str) -> u16 {
        let addr = self.next_global;
        self.globals.insert(name.to_string(), addr);
        self.next_global += 4;
        addr
    }

    // Save FP (R3) to FP_SAVE_ADDR if we are inside a function
    fn save_fp_if_needed(&mut self) {
        if self.fn_ctx.is_some() {
            self.emit_stm32(FP_SAVE_ADDR, 3);
        }
    }

    // Restore FP (R3) from FP_SAVE_ADDR if we are inside a function
    fn restore_fp_if_needed(&mut self) {
        if self.fn_ctx.is_some() {
            self.emit_ldm32(3, FP_SAVE_ADDR);
        }
    }

    /// Emit __rt_newtable, __rt_gettab, __rt_settab subroutines.
    /// Must be called between JMP __start_ and fn bodies.
    fn emit_rt_helpers(&mut self) {
        // ── __rt_newtable ────────────────────────────────────────────
        // In:  (nothing)  Out: R0 = ptr to new 72-byte table in heap
        // Clobbers: R0–R2; saves/restores R3 (FP)
        self.emit_label("__rt_newtable");
        self.emit_push(3); // save FP

        // R0 = HEAP_TOP
        self.emit_ldm32(0, HEAP_TOP_ADDR);
        self.emit_stm32(RT_TMP0, 0); // save ptr

        // Write cap=TABLE_CAP at [ptr+0]
        self.emit_mov32(1, TABLE_CAP);
        self.emit_stm32i(0, 1);
        // Write count=0 at [ptr+4]
        self.emit_mov(2, 4);
        self.emit_addr(0, 2); // R0 = ptr+4
        self.emit_mov32(1, 0);
        self.emit_stm32i(0, 1);

        // Loop: fill TABLE_CAP entries with sentinel key + 0 val
        // R2 = ptr+8 (start of entries), R1 = sentinel
        self.emit_ldm32(0, RT_TMP0);
        self.emit_mov(2, TABLE_HDR_SZ as u16);
        self.emit_addr(0, 2); // R0 = ptr + HDR_SZ (entry[0])
        self.emit_stm32(RT_TMP1, 0); // RT_TMP1 = entry ptr cursor
        self.emit_mov(2, TABLE_CAP as u16); // R2 = loop counter (8)
        self.emit_stm32(RT_TMP2, 2);

        let fill_loop = self.fresh_label("nt_fill");
        let fill_end  = self.fresh_label("nt_fill_end");
        self.emit_label(&fill_loop);
        self.emit_ldm32(2, RT_TMP2);
        self.emit_jz(2, &fill_end);

        // Write sentinel key
        self.emit_ldm32(0, RT_TMP1);
        self.emit_mov32(1, TABLE_SENTINEL);
        self.emit_stm32i(0, 1);
        // Write 0 val at [ptr+4]
        self.emit_mov(2, 4);
        self.emit_addr(0, 2);
        self.emit_mov32(1, 0);
        self.emit_stm32i(0, 1);
        // cursor += 8
        self.emit_ldm32(0, RT_TMP1);
        self.emit_mov(1, TABLE_ENTRY_SZ as u16);
        self.emit_addr(0, 1);
        self.emit_stm32(RT_TMP1, 0);
        // counter--
        self.emit_ldm32(0, RT_TMP2);
        self.emit_mov(1, 1);
        self.emit_subr(0, 1);
        self.emit_stm32(RT_TMP2, 0);
        self.emit_jmp(&fill_loop);
        self.emit_label(&fill_end);

        // HEAP_TOP += TABLE_ALLOC_SZ
        self.emit_ldm32(0, HEAP_TOP_ADDR);
        self.emit_mov32(1, TABLE_ALLOC_SZ);
        self.emit_addr(0, 1);
        self.emit_stm32(HEAP_TOP_ADDR, 0);

        // R0 = ptr (return value)
        self.emit_ldm32(0, RT_TMP0);
        self.emit_pop(3); // restore FP
        self.code.push(OP_RET);

        // ── __rt_gettab ──────────────────────────────────────────────
        // In: R0=ptr, R1=key  Out: R0=value (0 if not found)
        self.emit_label("__rt_gettab");
        self.emit_push(3);
        self.emit_stm32(RT_TMP0, 0); // save ptr
        self.emit_stm32(RT_TMP1, 1); // save key

        // hash = key & (TABLE_CAP-1)
        self.emit_mov32(2, TABLE_CAP - 1);
        self.emit_and_reg(1, 2); // R1 = hash

        // probe = ptr + HDR_SZ + hash*8
        self.emit_mov32(2, TABLE_ENTRY_SZ);
        self.emit_mul_reg(1, 2);    // R1 = hash * 8
        self.emit_ldm32(0, RT_TMP0);
        self.emit_mov(2, TABLE_HDR_SZ as u16);
        self.emit_addr(0, 2);       // R0 = ptr + HDR_SZ
        self.emit_addr(0, 1);       // R0 = ptr + HDR_SZ + hash*8
        self.emit_stm32(RT_TMP2, 0); // RT_TMP2 = probe

        // probe_limit = ptr + TABLE_ALLOC_SZ
        self.emit_ldm32(0, RT_TMP0);
        self.emit_mov32(1, TABLE_ALLOC_SZ);
        self.emit_addr(0, 1);
        self.emit_stm32(RT_TMP3, 0); // RT_TMP3 = probe_limit

        // R3 = iteration counter (FP already saved on stack)
        self.emit_mov(3, TABLE_CAP as u16);

        // loop: linear probe
        let gt_loop  = self.fresh_label("gt_loop");
        let gt_found = self.fresh_label("gt_found");
        let gt_miss  = self.fresh_label("gt_miss");
        let gt_wrap  = self.fresh_label("gt_wrap");
        self.emit_label(&gt_loop);

        // guard against infinite loop if table full
        self.emit_jz(3, &gt_miss);
        self.emit_mov(0, 1);
        self.emit_subr(3, 0); // R3 -= 1

        // slot_key = mem32[probe]
        self.emit_ldm32(0, RT_TMP2);
        self.emit_ldm32i(1, 0); // R1 = slot_key

        // if slot_key == SENTINEL → miss
        self.emit_mov32(2, TABLE_SENTINEL);
        self.code.push(OP_EQ); self.code.push(0); self.code.push(1); self.code.push(2);
        self.emit_jnz(0, &gt_miss);

        // if slot_key == key → found
        self.emit_ldm32(2, RT_TMP1); // R2 = search key
        self.code.push(OP_EQ); self.code.push(0); self.code.push(1); self.code.push(2);
        self.emit_jnz(0, &gt_found);

        // probe += 8; if probe >= limit → wrap to ptr+HDR_SZ
        self.emit_ldm32(0, RT_TMP2);
        self.emit_mov(1, TABLE_ENTRY_SZ as u16);
        self.emit_addr(0, 1);
        self.emit_stm32(RT_TMP2, 0); // probe += 8

        self.emit_ldm32(1, RT_TMP3); // R1 = limit
        // if probe >= limit: SLTS R2, probe, limit → R2 = (probe < limit)
        self.code.push(OP_SLTS); self.code.push(2); self.code.push(0); self.code.push(1);
        self.emit_jz(2, &gt_wrap); // probe >= limit → wrap
        self.emit_jmp(&gt_loop);

        self.emit_label(&gt_wrap);
        self.emit_ldm32(0, RT_TMP0);
        self.emit_mov(1, TABLE_HDR_SZ as u16);
        self.emit_addr(0, 1);
        self.emit_stm32(RT_TMP2, 0);
        self.emit_jmp(&gt_loop);

        self.emit_label(&gt_found);
        // R0 = mem32[probe+4]
        self.emit_ldm32(0, RT_TMP2);
        self.emit_mov(1, 4);
        self.emit_addr(0, 1);
        self.emit_ldm32i(0, 0);
        self.emit_pop(3);
        self.code.push(OP_RET);

        self.emit_label(&gt_miss);
        self.emit_mov_r0_imm(0);
        self.emit_pop(3);
        self.code.push(OP_RET);

        // ── __rt_settab ──────────────────────────────────────────────
        // In: R0=ptr, R1=key, R2=val  Out: (nothing)  R0=ptr (RT_TMP0)
        self.emit_label("__rt_settab");
        self.emit_push(3);
        self.emit_stm32(RT_TMP0, 0); // ptr
        self.emit_stm32(RT_TMP1, 1); // key
        self.emit_stm32(RT_TMP2, 2); // val

        // hash = key & (TABLE_CAP-1)
        self.emit_mov32(2, TABLE_CAP - 1);
        self.emit_and_reg(1, 2);

        // probe = ptr + HDR_SZ + hash*8
        self.emit_mov32(2, TABLE_ENTRY_SZ);
        self.emit_mul_reg(1, 2);
        self.emit_ldm32(0, RT_TMP0);
        self.emit_mov(2, TABLE_HDR_SZ as u16);
        self.emit_addr(0, 2);
        self.emit_addr(0, 1);
        self.emit_stm32(RT_TMP3, 0); // probe

        // probe_limit
        self.emit_ldm32(0, RT_TMP0);
        self.emit_mov32(1, TABLE_ALLOC_SZ);
        self.emit_addr(0, 1);
        // save probe_limit in R3 temporarily (FP already saved on stack)
        self.emit_movr(3, 0); // R3 = probe_limit

        // init iteration counter
        self.emit_mov(0, TABLE_CAP as u16);
        self.emit_stm32(RT_TMP4, 0);

        let st_loop      = self.fresh_label("st_loop");
        let st_write     = self.fresh_label("st_write");
        let st_overwrite = self.fresh_label("st_overwrite");
        let st_wrap      = self.fresh_label("st_wrap");
        let st_done      = self.fresh_label("st_done");
        self.emit_label(&st_loop);

        // guard against infinite loop if table full
        self.emit_ldm32(0, RT_TMP4);
        self.emit_jz(0, &st_done);
        self.emit_mov(1, 1);
        self.emit_subr(0, 1); // counter -= 1
        self.emit_stm32(RT_TMP4, 0);

        self.emit_ldm32(0, RT_TMP3); // R0 = probe
        self.emit_ldm32i(1, 0);      // R1 = slot_key

        // if sentinel or key matches → write
        self.emit_mov32(2, TABLE_SENTINEL);
        self.code.push(OP_EQ); self.code.push(2); self.code.push(1); self.code.push(2);
        self.emit_jnz(2, &st_write);

        self.emit_ldm32(2, RT_TMP1);
        self.code.push(OP_EQ); self.code.push(2); self.code.push(1); self.code.push(2);
        self.emit_jnz(2, &st_write);

        // probe += 8; wrap if >= limit
        self.emit_ldm32(0, RT_TMP3);
        self.emit_mov(1, TABLE_ENTRY_SZ as u16);
        self.emit_addr(0, 1);
        self.emit_stm32(RT_TMP3, 0);

        // if probe >= limit (R3) → wrap
        self.code.push(OP_SLTS); self.code.push(1); self.code.push(0); self.code.push(3);
        self.emit_jz(1, &st_wrap);
        self.emit_jmp(&st_loop);

        self.emit_label(&st_wrap);
        self.emit_ldm32(0, RT_TMP0);
        self.emit_mov(1, TABLE_HDR_SZ as u16);
        self.emit_addr(0, 1);
        self.emit_stm32(RT_TMP3, 0);
        self.emit_jmp(&st_loop);

        self.emit_label(&st_write);
        // Increment count only when inserting into a sentinel (new key)
        // R1 = slot_key at this point
        self.emit_mov32(2, TABLE_SENTINEL);
        self.code.push(OP_EQ); self.code.push(2); self.code.push(1); self.code.push(2);
        self.emit_jz(2, &st_overwrite); // R2=0 → existing key, skip increment
        self.emit_ldm32(0, RT_TMP0);    // R0 = ptr
        self.emit_mov(1, 4);
        self.emit_addr(0, 1);           // R0 = ptr+4 (count field)
        self.emit_ldm32i(1, 0);         // R1 = count
        self.emit_mov(2, 1);
        self.emit_addr(1, 2);           // R1 = count+1
        self.emit_stm32i(0, 1);         // mem[ptr+4] = count+1

        self.emit_label(&st_overwrite);
        // write key at probe
        self.emit_ldm32(0, RT_TMP3);
        self.emit_ldm32(1, RT_TMP1);
        self.emit_stm32i(0, 1);
        // write val at probe+4
        self.emit_mov(1, 4);
        self.emit_addr(0, 1);
        self.emit_ldm32(1, RT_TMP2);
        self.emit_stm32i(0, 1);

        // st_done: table full, silently drop (no write)
        self.emit_label(&st_done);
        // restore R3 from stack (was pushed at entry)
        self.emit_pop(3);
        self.code.push(OP_RET);
    }

    pub fn compile(&mut self, file: &SourceFile) -> Result<()> {
        // Register consts
        for c in &file.consts {
            self.consts.insert(c.name.clone(), c.value);
        }

        // Allocate global slots (don't emit init yet — done in start block)
        for g in &file.globals {
            self.alloc_global(&g.name);
        }

        // Emit: JMP __start_
        self.emit_jmp("__start_");

        // Emit RT helpers (newtable / gettab / settab)
        self.emit_rt_helpers();

        // Emit function bodies
        for func in &file.functions {
            self.compile_fn(func)?;
        }

        // __start_ label
        self.emit_label("__start_");

        // CPY string pool from ROM to RAM (src and len patched in finish())
        self.code.push(OP_CPY);
        self.emit_addr16(STRING_POOL_BASE);
        self.cpy_src_patch = self.code.len();
        self.code.push(0); self.code.push(0); // src — patched
        self.cpy_len_patch = self.code.len();
        self.code.push(0); self.code.push(0); // len — patched

        // Initialize heap top
        self.emit_mov32(0, HEAP_BASE);
        self.emit_stm32(HEAP_TOP_ADDR, 0);

        // Initialize globals
        for g in &file.globals {
            let addr = *self.globals.get(&g.name).unwrap();
            self.lower_expr_r0(&g.init)?;
            self.emit_stm32(addr, 0);
        }

        // init block
        if let Some(block) = &file.init_block {
            let block = block.clone();
            self.compile_block(&block)?;
        }

        // __loop_ label
        self.emit_label("__loop_");

        if let Some(block) = &file.loop_block {
            let block = block.clone();
            self.compile_block(&block)?;
        }

        self.emit_jmp("__loop_");

        Ok(())
    }

    fn compile_fn(&mut self, func: &FnDecl) -> Result<()> {
        self.emit_label(&func.name);

        // Function entry: save FP, set FP = SP
        self.emit_push(3); // push old FP
        self.emit_getsp(3); // FP = SP (points to the word after old FP was pushed)

        // Stack at entry (after PUSH R3; GETSP R3):
        //   mem[FP]   = old FP
        //   mem[FP+4] = return addr (pushed by JSR)
        //   mem[FP+8 + i*4] = arg_i  (caller pushes args in reverse: argN-1 first, arg0 last)

        self.fn_ctx = Some(FnCtx::new(func.params.clone()));

        let body = func.body.clone();
        self.compile_block(&body)?;

        // Function exit: SETSP R3; POP R3; RET
        self.emit_setsp(3);
        self.emit_pop(3);
        self.code.push(OP_RET);

        self.fn_ctx = None;
        Ok(())
    }

    fn compile_block(&mut self, block: &[Stmt]) -> Result<()> {
        if let Some(ctx) = &mut self.fn_ctx {
            ctx.push_scope();
        }
        for stmt in block {
            self.compile_stmt(stmt)?;
        }
        if let Some(ctx) = &mut self.fn_ctx {
            let freed = ctx.pop_scope();
            if freed > 0 {
                // SP += freed * 4  (reclaim locals)
                self.emit_getsp(1);
                self.emit_mov(2, (freed * 4) as u16);
                self.emit_addr(1, 2);
                self.emit_setsp(1);
            }
        }
        Ok(())
    }

    fn stmt_line(stmt: &Stmt) -> usize {
        match stmt {
            Stmt::Local { line, .. } | Stmt::Assign { line, .. } | Stmt::Do { line, .. }
            | Stmt::While { line, .. } | Stmt::Repeat { line, .. } | Stmt::If { line, .. }
            | Stmt::NumericFor { line, .. } | Stmt::Return { line, .. } | Stmt::Break { line }
            | Stmt::ExprStmt { line, .. } | Stmt::SetField { line, .. }
            | Stmt::SetIndex { line, .. } => *line,
        }
    }

    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<()> {
        let line = Self::stmt_line(stmt);
        self.source_map.set_src_line(self.code.len(), line);
        match stmt {
            Stmt::ExprStmt { expr, .. } => {
                self.lower_expr_r0(expr)?;
            }
            Stmt::Assign { target, value, line } => {
                self.lower_expr_r0(value)?;
                match self.lookup_var(target).ok_or_else(|| LangError::UndefinedVariable {
                    line: *line,
                    name: target.clone(),
                })? {
                    VarLoc::Const(_) => {
                        return Err(LangError::UndefinedVariable { line: *line, name: target.clone() });
                    }
                    VarLoc::Global(addr) => {
                        self.emit_stm32(addr, 0);
                    }
                    VarLoc::Local(slot) => {
                        self.emit_store_local(slot);
                    }
                    VarLoc::Param(idx) => {
                        self.emit_store_param(idx);
                    }
                }
            }
            Stmt::Local { names, inits, line } => {
                for (i, name) in names.iter().enumerate() {
                    if let Some(init) = inits.get(i) {
                        self.lower_expr_r0(init)?;
                    } else {
                        self.emit_mov_r0_imm(0); // nil → 0
                    }
                    if let Some(ctx) = &mut self.fn_ctx {
                        ctx.alloc_local(name.clone());
                        // PUSH R0 puts value on stack at slot
                        self.emit_push(0);
                    } else {
                        // top-level local = global
                        let addr = if let Some(&a) = self.globals.get(name) {
                            a
                        } else {
                            self.alloc_global(name)
                        };
                        self.emit_stm32(addr, 0);
                        let _ = line;
                    }
                }
            }
            Stmt::If { cond, then_block, elseif_clauses, else_block, .. } => {
                let end_label = self.fresh_label("if_end");
                let mut next_label = self.fresh_label("if_else");

                self.lower_expr_r0(cond)?;
                self.emit_jz(0, &next_label.clone());

                let then_block = then_block.clone();
                self.compile_block(&then_block)?;
                self.emit_jmp(&end_label);

                for (ei, (ec, eb)) in elseif_clauses.iter().enumerate() {
                    self.emit_label(&next_label);
                    next_label = if ei + 1 < elseif_clauses.len() || else_block.is_some() {
                        self.fresh_label("if_elseif")
                    } else {
                        end_label.clone()
                    };
                    let ec = ec.clone();
                    self.lower_expr_r0(&ec)?;
                    self.emit_jz(0, &next_label.clone());
                    let eb = eb.clone();
                    self.compile_block(&eb)?;
                    self.emit_jmp(&end_label);
                }

                self.emit_label(&next_label);
                if let Some(eb) = else_block {
                    let eb = eb.clone();
                    self.compile_block(&eb)?;
                }

                self.emit_label(&end_label);
            }
            Stmt::While { cond, body, .. } => {
                let loop_label = self.fresh_label("while_loop");
                let end_label = self.fresh_label("while_end");

                let slots_at_entry = self.fn_ctx.as_ref().map(|c| c.next_slot).unwrap_or(0);
                if let Some(ctx) = &mut self.fn_ctx {
                    ctx.break_targets.push(BreakTarget { end_label: end_label.clone(), slots_at_entry });
                } else {
                    self.top_break_targets.push(BreakTarget { end_label: end_label.clone(), slots_at_entry: 0 });
                }

                self.emit_label(&loop_label);
                let cond = cond.clone();
                self.lower_expr_r0(&cond)?;
                self.emit_jz(0, &end_label);

                let body = body.clone();
                self.compile_block(&body)?;
                self.emit_jmp(&loop_label);
                self.emit_label(&end_label);

                if let Some(ctx) = &mut self.fn_ctx {
                    ctx.break_targets.pop();
                } else {
                    self.top_break_targets.pop();
                }
            }
            Stmt::Repeat { body, cond, .. } => {
                let loop_label = self.fresh_label("repeat_loop");
                let end_label = self.fresh_label("repeat_end");

                let slots_at_entry = self.fn_ctx.as_ref().map(|c| c.next_slot).unwrap_or(0);
                if let Some(ctx) = &mut self.fn_ctx {
                    ctx.break_targets.push(BreakTarget { end_label: end_label.clone(), slots_at_entry });
                } else {
                    self.top_break_targets.push(BreakTarget { end_label: end_label.clone(), slots_at_entry: 0 });
                }

                self.emit_label(&loop_label);
                let body = body.clone();
                self.compile_block(&body)?;
                let cond = cond.clone();
                self.lower_expr_r0(&cond)?;
                self.emit_jz(0, &loop_label); // repeat until cond is TRUE
                self.emit_label(&end_label);

                if let Some(ctx) = &mut self.fn_ctx {
                    ctx.break_targets.pop();
                } else {
                    self.top_break_targets.pop();
                }
            }
            Stmt::NumericFor { var, start, stop, step, body, line } => {
                // for var = start, stop [, step] do body end
                // Allocate 3 locals: __for_var, __for_stop, __for_step
                let loop_label = self.fresh_label("for_loop");
                let end_label = self.fresh_label("for_end");

                let slots_at_entry = self.fn_ctx.as_ref().map(|c| c.next_slot).unwrap_or(0);
                if let Some(ctx) = &mut self.fn_ctx {
                    ctx.break_targets.push(BreakTarget { end_label: end_label.clone(), slots_at_entry });
                }

                // We compile numeric for by emitting explicit local management.
                // Push scope, allocate var/__stop/__step, loop, pop scope.
                if let Some(ctx) = &mut self.fn_ctx {
                    ctx.push_scope();
                }

                // init var = start
                let start = start.clone();
                self.lower_expr_r0(&start)?;
                let var_slot = if let Some(ctx) = &mut self.fn_ctx {
                    let slot = ctx.alloc_local(var.clone());
                    self.emit_push(0);
                    slot
                } else {
                    return Err(LangError::NotImplemented { line: *line, feature: "for at top-level".to_string() });
                };

                // stop
                let stop = stop.clone();
                self.lower_expr_r0(&stop)?;
                let stop_slot = if let Some(ctx) = &mut self.fn_ctx {
                    let slot = ctx.alloc_local("__for_stop".to_string());
                    self.emit_push(0);
                    slot
                } else { unreachable!() };

                // step (default 1)
                let step_val = step.clone().unwrap_or_else(|| Expr::Number(1, *line));
                self.lower_expr_r0(&step_val)?;
                let step_slot = if let Some(ctx) = &mut self.fn_ctx {
                    let slot = ctx.alloc_local("__for_step".to_string());
                    self.emit_push(0);
                    slot
                } else { unreachable!() };

                self.emit_label(&loop_label);

                // Condition: var <= stop (assuming positive step)
                // R0 = var, R1 = stop
                self.emit_load_local(var_slot);
                self.emit_push(0);
                self.emit_load_local(stop_slot);
                self.emit_movr(1, 0);
                self.emit_pop(0);
                // SLTS R2, R1, R0 → R2 = (stop < var) i.e. var > stop → exit
                self.code.push(OP_SLTS);
                self.code.push(2);
                self.code.push(1);
                self.code.push(0);
                self.emit_jnz(2, &end_label);

                // body
                let body = body.clone();
                self.compile_block(&body)?;

                // var += step
                self.emit_load_local(var_slot);
                self.emit_push(0);
                self.emit_load_local(step_slot);
                self.emit_movr(1, 0);
                self.emit_pop(0);
                self.emit_addr(0, 1);
                self.emit_store_local(var_slot);

                self.emit_jmp(&loop_label);
                self.emit_label(&end_label);

                // Pop scope (frees var + stop + step)
                let freed = self.fn_ctx.as_mut().map(|ctx| ctx.pop_scope()).unwrap_or(0);
                if freed > 0 {
                    self.emit_getsp(1);
                    self.emit_mov(2, (freed * 4) as u16);
                    self.emit_addr(1, 2);
                    self.emit_setsp(1);
                }
                if let Some(ctx) = &mut self.fn_ctx {
                    ctx.break_targets.pop();
                }
            }
            Stmt::Do { body, .. } => {
                let body = body.clone();
                self.compile_block(&body)?;
            }
            Stmt::Return { values, line } => {
                if self.fn_ctx.is_none() {
                    return Err(LangError::ReturnOutsideFunction { line: *line });
                }
                // Single return value in R0
                if let Some(val) = values.first() {
                    let val = val.clone();
                    self.lower_expr_r0(&val)?;
                }
                // Function exit
                self.emit_setsp(3);
                self.emit_pop(3);
                self.code.push(OP_RET);
            }
            Stmt::Break { line } => {
                let (end_label, slots_at_entry) = {
                    let target = if let Some(ctx) = &self.fn_ctx {
                        ctx.break_targets.last()
                    } else {
                        self.top_break_targets.last()
                    };
                    match target {
                        None => return Err(LangError::BreakOutsideLoop { line: *line }),
                        Some(t) => (t.end_label.clone(), t.slots_at_entry),
                    }
                };
                // Restore SP to state at loop entry
                let current_slots = self.fn_ctx.as_ref().map(|c| c.next_slot).unwrap_or(0);
                if current_slots > slots_at_entry {
                    let diff = current_slots - slots_at_entry;
                    self.emit_getsp(1);
                    self.emit_mov(2, (diff * 4) as u16);
                    self.emit_addr(1, 2);
                    self.emit_setsp(1);
                }
                self.emit_jmp(&end_label);
            }
            Stmt::SetField { table, name, value, .. } => {
                let table = table.clone();
                let value = value.clone();
                let key_ptr = self.intern_string(name);
                self.lower_expr_r0(&table)?;
                self.emit_push(0);             // push ptr (value eval may clobber scratch)
                self.lower_expr_r0(&value)?;
                self.emit_stm32(SCRATCH_BASE, 0); // save val
                self.emit_pop(0);              // R0 = ptr
                self.emit_mov(1, key_ptr);
                self.emit_ldm32(2, SCRATCH_BASE);
                self.emit_jsr("__rt_settab");
            }
            Stmt::SetIndex { table, key, value, .. } => {
                let table = table.clone();
                let key = key.clone();
                let value = value.clone();
                self.lower_expr_r0(&table)?;
                self.emit_push(0);  // push ptr
                self.lower_expr_r0(&key)?;
                self.emit_push(0);  // push key
                self.lower_expr_r0(&value)?;
                // R0=val, stack=[ptr, key(top)]
                self.emit_movr(2, 0); // R2 = val
                self.emit_pop(1);     // R1 = key
                self.emit_pop(0);     // R0 = ptr
                self.emit_jsr("__rt_settab");
            }
        }
        Ok(())
    }

    fn lower_expr_r0(&mut self, expr: &Expr) -> Result<()> {
        match expr {
            Expr::Nil(_) => {
                self.emit_mov_r0_imm(0);
            }
            Expr::True(_) => {
                self.emit_mov_r0_imm(1);
            }
            Expr::False(_) => {
                self.emit_mov_r0_imm(0);
            }
            Expr::Number(n, _) => {
                self.emit_mov_r0_imm(*n);
            }
            Expr::Str(s, _) => {
                let ptr = self.intern_string(s);
                self.emit_mov(0, ptr);
            }
            Expr::Var(name, line) => {
                match self.lookup_var(name) {
                    None => return Err(LangError::UndefinedVariable { line: *line, name: name.clone() }),
                    Some(VarLoc::Const(v)) => { self.emit_mov_r0_imm(v); }
                    Some(VarLoc::Global(addr)) => { self.emit_ldm32(0, addr); }
                    Some(VarLoc::Local(slot)) => { self.emit_load_local(slot); }
                    Some(VarLoc::Param(idx)) => { self.emit_load_param(idx); }
                }
            }
            Expr::UnOp { op, expr, line } => {
                let inner = expr.as_ref().clone();
                match op {
                    UnOp::Len => {
                        // Only string literals supported at compile time
                        match &inner {
                            Expr::Str(s, _) => { self.emit_mov_r0_imm(s.len() as u32); }
                            _ => return Err(LangError::NotImplemented { line: *line, feature: "#".to_string() }),
                        }
                    }
                    _ => {
                        self.lower_expr_r0(&inner)?;
                        match op {
                            UnOp::Neg => { self.code.push(OP_NEG); self.code.push(0); }
                            UnOp::Not => {
                                self.emit_mov(1, 0);
                                self.code.push(OP_EQ);
                                self.code.push(0); self.code.push(0); self.code.push(1);
                            }
                            UnOp::Len => unreachable!(),
                        }
                    }
                }
            }
            Expr::BinOp { op, left, right, line } => {
                let left = left.as_ref().clone();
                let right = right.as_ref().clone();
                match op {
                    BinOp::And => {
                        // short-circuit: if left is falsy, result = left (0); else result = right
                        let false_label = self.fresh_label("and_false");
                        let end_label = self.fresh_label("and_end");
                        self.lower_expr_r0(&left)?;
                        self.emit_jz(0, &false_label);
                        self.lower_expr_r0(&right)?;
                        self.emit_jmp(&end_label);
                        self.emit_label(&false_label);
                        self.emit_mov_r0_imm(0);
                        self.emit_label(&end_label);
                    }
                    BinOp::Or => {
                        // short-circuit: if left is truthy, result = left; else result = right
                        let true_label = self.fresh_label("or_true");
                        let end_label = self.fresh_label("or_end");
                        self.lower_expr_r0(&left)?;
                        self.emit_jnz(0, &true_label);
                        self.lower_expr_r0(&right)?;
                        self.emit_jmp(&end_label);
                        self.emit_label(&true_label);
                        // R0 still holds left value (truthy); jump skips right-eval
                        self.emit_label(&end_label);
                    }
                    BinOp::Concat => {
                        match (&left, &right) {
                            (Expr::Str(ls, _), Expr::Str(rs, _)) => {
                                let combined = format!("{}{}", ls, rs);
                                let ptr = self.intern_string(&combined);
                                self.emit_mov(0, ptr);
                            }
                            _ => return Err(LangError::NotImplemented { line: *line, feature: "dynamic ..".to_string() }),
                        }
                    }
                    BinOp::Pow => {
                        return Err(LangError::NotImplemented { line: *line, feature: "^".to_string() });
                    }
                    _ => {
                        // General: eval left → push; eval right → R1; pop R0; op R0, R1
                        self.lower_expr_r0(&left)?;
                        self.emit_push(0);
                        self.lower_expr_r0(&right)?;
                        self.emit_movr(1, 0);
                        self.emit_pop(0);
                        match op {
                            BinOp::Add => self.emit_addr(0, 1),
                            BinOp::Sub => self.emit_subr(0, 1),
                            BinOp::Mul => { self.code.push(OP_MUL); self.code.push(0); self.code.push(1); }
                            BinOp::Div => { self.code.push(OP_DIV); self.code.push(0); self.code.push(1); }
                            BinOp::Mod => { self.code.push(OP_MOD); self.code.push(0); self.code.push(1); }
                            BinOp::Eq => {
                                self.code.push(OP_EQ); self.code.push(0); self.code.push(0); self.code.push(1);
                            }
                            BinOp::NotEq => {
                                self.code.push(OP_EQ); self.code.push(0); self.code.push(0); self.code.push(1);
                                // invert: R1 = 0; EQ R0, R0, R1
                                self.emit_mov(1, 0);
                                self.code.push(OP_EQ); self.code.push(0); self.code.push(0); self.code.push(1);
                            }
                            BinOp::Lt => {
                                // SLTS R0, R0, R1
                                self.code.push(OP_SLTS); self.code.push(0); self.code.push(0); self.code.push(1);
                            }
                            BinOp::Gt => {
                                // R0 > R1 ↔ R1 < R0 → SLTS R0, R1, R0
                                self.code.push(OP_SLTS); self.code.push(0); self.code.push(1); self.code.push(0);
                            }
                            BinOp::LtEq => {
                                // R0 <= R1 ↔ !(R0 > R1) ↔ !(R1 < R0)
                                self.code.push(OP_SLTS); self.code.push(0); self.code.push(1); self.code.push(0);
                                self.emit_mov(1, 0);
                                self.code.push(OP_EQ); self.code.push(0); self.code.push(0); self.code.push(1);
                            }
                            BinOp::GtEq => {
                                // R0 >= R1 ↔ !(R0 < R1)
                                self.code.push(OP_SLTS); self.code.push(0); self.code.push(0); self.code.push(1);
                                self.emit_mov(1, 0);
                                self.code.push(OP_EQ); self.code.push(0); self.code.push(0); self.code.push(1);
                            }
                            _ => unreachable!(),
                        }
                    }
                }
            }
            Expr::Call { func, args, line } => {
                self.lower_call(func, args, *line)?;
            }
            Expr::Table { fields, line: _ } => {
                // Call __rt_newtable → R0 = ptr
                self.emit_jsr("__rt_newtable");
                let mut array_idx: u32 = 1;
                for field in fields {
                    let field = field.clone();
                    // ptr in R0 at top of each iteration; push to survive value eval
                    match field {
                        TableField::NameField { name, value } => {
                            let key_ptr = self.intern_string(&name);
                            self.emit_push(0);              // push ptr
                            self.lower_expr_r0(&value)?;
                            self.emit_stm32(SCRATCH_BASE, 0); // save val (no nested eval after)
                            self.emit_pop(0);               // R0 = ptr
                            self.emit_mov(1, key_ptr);
                            self.emit_ldm32(2, SCRATCH_BASE);
                            self.emit_jsr("__rt_settab");
                            self.emit_ldm32(0, RT_TMP0);    // recover ptr for next iteration
                        }
                        TableField::IndexField { key, value } => {
                            self.emit_push(0);              // push ptr
                            self.lower_expr_r0(&key)?;
                            self.emit_push(0);              // push key
                            self.lower_expr_r0(&value)?;
                            self.emit_movr(2, 0);           // R2 = val
                            self.emit_pop(1);               // R1 = key
                            self.emit_pop(0);               // R0 = ptr
                            self.emit_jsr("__rt_settab");
                            self.emit_ldm32(0, RT_TMP0);    // recover ptr
                        }
                        TableField::ValueField { value } => {
                            let key = array_idx;
                            array_idx += 1;
                            self.emit_push(0);              // push ptr
                            self.lower_expr_r0(&value)?;
                            self.emit_stm32(SCRATCH_BASE, 0);
                            self.emit_pop(0);               // R0 = ptr
                            self.emit_mov32(1, key);
                            self.emit_ldm32(2, SCRATCH_BASE);
                            self.emit_jsr("__rt_settab");
                            self.emit_ldm32(0, RT_TMP0);    // recover ptr
                        }
                    }
                }
                // R0 = table ptr (already set by last settab / newtable if no fields)
            }
            Expr::Func { line, .. } => {
                return Err(LangError::NotImplemented { line: *line, feature: "function expression".to_string() });
            }
            Expr::Index { table, key, line: _ } => {
                let key = key.as_ref().clone();
                let table = table.as_ref().clone();
                self.lower_expr_r0(&table)?;
                self.emit_push(0);              // push ptr (key eval may clobber scratch)
                self.lower_expr_r0(&key)?;
                self.emit_movr(1, 0);           // R1 = key
                self.emit_pop(0);               // R0 = ptr
                self.emit_jsr("__rt_gettab");
            }
            Expr::Field { table, name, line: _ } => {
                let table = table.as_ref().clone();
                let key_ptr = self.intern_string(name);
                self.lower_expr_r0(&table)?;
                // R0 = ptr; no nested eval, so no scratch collision risk
                self.emit_mov(1, key_ptr);
                self.emit_jsr("__rt_gettab");
            }
        }
        Ok(())
    }

    fn lower_call(&mut self, func: &Expr, args: &[Expr], line: usize) -> Result<()> {
        // Dynamic calls (Field/Index): eval func to R0 and treat as user-defined function addr
        // Only handle Var (builtin/user) and Field (method dispatch → lookup + call)
        if !matches!(func, Expr::Var(..)) {
            // Generic dynamic dispatch: evaluate func → R0 (table field lookup),
            // then push args and JSR — not supported in this VM (needs indirect JSR).
            // For now emit an error unless it's a field (desugar already done in parser).
            return Err(LangError::NotImplemented { line, feature: "dynamic call".to_string() });
        }
        let name = match func {
            Expr::Var(n, _) => n.clone(),
            _ => unreachable!(),
        };

        match name.as_str() {
            "cls" => {
                if !args.is_empty() {
                    return Err(LangError::ArgCount { line, name, expected: 0, got: args.len() });
                }
                self.code.push(OP_CLS);
            }
            "wait" => {
                if !args.is_empty() {
                    return Err(LangError::ArgCount { line, name, expected: 0, got: args.len() });
                }
                self.code.push(OP_WAIT);
            }
            "key" | "btn" => {
                if args.len() != 1 {
                    return Err(LangError::ArgCount { line, name, expected: 1, got: args.len() });
                }
                let key = self.require_literal_u8(&args[0], line, &name)?;
                self.code.push(OP_IN);
                self.code.push(0);
                self.code.push(key);
            }
            "spr" => {
                // spr(x, y, addr) → DPXR R0, R1, sprite_w=8, sprite_h=8, palette=0
                // Actually we need SPT (sprite tile) or DPXR.
                // spr(x, y, tile_addr) — use SPT R_x, R_y, R_addr
                if args.len() != 3 {
                    return Err(LangError::ArgCount { line, name, expected: 3, got: args.len() });
                }
                // Load x→scratch0, y→scratch1, addr→scratch2
                self.lower_expr_r0(&args[0])?;
                self.emit_stm32(SCRATCH_BASE, 0);
                self.lower_expr_r0(&args[1])?;
                self.emit_stm32(SCRATCH_BASE + SCRATCH_STEP, 0);
                self.lower_expr_r0(&args[2])?;
                self.emit_stm32(SCRATCH_BASE + SCRATCH_STEP * 2, 0);

                self.emit_ldm32(0, SCRATCH_BASE);
                self.emit_ldm32(1, SCRATCH_BASE + SCRATCH_STEP);
                self.emit_ldm32(2, SCRATCH_BASE + SCRATCH_STEP * 2);
                // SPT R0, R1, R2
                self.code.push(OP_SPT);
                self.code.push(0);
                self.code.push(1);
                self.code.push(2);
            }
            "pal" => {
                // pal(idx, r, g, b) → PAL idx r g b
                if args.len() != 4 {
                    return Err(LangError::ArgCount { line, name, expected: 4, got: args.len() });
                }
                let idx = self.require_literal_u8(&args[0], line, &name)?;
                let r   = self.require_literal_u8(&args[1], line, &name)?;
                let g   = self.require_literal_u8(&args[2], line, &name)?;
                let b   = self.require_literal_u8(&args[3], line, &name)?;
                self.code.push(OP_PAL);
                self.code.push(idx);
                self.code.push(r);
                self.code.push(g);
                self.code.push(b);
            }
            "cls_col" | "fill" => {
                if args.len() != 1 {
                    return Err(LangError::ArgCount { line, name, expected: 1, got: args.len() });
                }
                let col = self.require_literal_u8(&args[0], line, &name)?;
                self.code.push(OP_FILL);
                self.code.push(col);
            }
            "sfx" => {
                if args.len() != 1 {
                    return Err(LangError::ArgCount { line, name, expected: 1, got: args.len() });
                }
                let id = self.require_literal_u8(&args[0], line, &name)?;
                self.code.push(OP_SFX);
                self.code.push(id);
            }
            "music" => {
                if args.len() != 1 {
                    return Err(LangError::ArgCount { line, name, expected: 1, got: args.len() });
                }
                let id = self.require_literal_u8(&args[0], line, &name)?;
                self.code.push(OP_MUS);
                self.code.push(id);
            }
            "nomusic" => {
                if !args.is_empty() {
                    return Err(LangError::ArgCount { line, name, expected: 0, got: args.len() });
                }
                self.code.push(OP_NOMUS);
            }
            "pset" | "dpx" => {
                // pset(x, y, color_idx, palette) or dpx(x, y, r, g, b)
                if args.len() == 5 {
                    let x   = self.require_literal_u8(&args[0], line, &name)?;
                    let y   = self.require_literal_u8(&args[1], line, &name)?;
                    let r   = self.require_literal_u8(&args[2], line, &name)?;
                    let g   = self.require_literal_u8(&args[3], line, &name)?;
                    let b   = self.require_literal_u8(&args[4], line, &name)?;
                    self.code.push(OP_DPX);
                    self.code.push(x);
                    self.code.push(y);
                    self.code.push(r);
                    self.code.push(g);
                    self.code.push(b);
                } else {
                    return Err(LangError::ArgCount { line, name, expected: 5, got: args.len() });
                }
            }
            "txt" => {
                // txt(x, y, str, color) — str must be a string literal (len known at compile time)
                // TXT opcode: R_x, R_y, R_color, R_base, len(byte)
                if args.len() != 4 {
                    return Err(LangError::ArgCount { line, name, expected: 4, got: args.len() });
                }
                let literal = Self::literal_str(&args[2]).ok_or_else(|| LangError::NotImplemented {
                    line,
                    feature: "txt with non-literal string (use a string literal directly)".to_string(),
                })?;
                let str_len = literal.len();
                self.save_fp_if_needed();
                // x → scratch[0], y → scratch[1], str_ptr → scratch[2], color → scratch[3]
                self.lower_expr_r0(&args[0])?;
                self.emit_stm32(SCRATCH_BASE, 0);
                self.lower_expr_r0(&args[1])?;
                self.emit_stm32(SCRATCH_BASE + SCRATCH_STEP, 0);
                self.lower_expr_r0(&args[2])?; // Expr::Str → R0 = ptr
                self.emit_stm32(SCRATCH_BASE + SCRATCH_STEP * 2, 0);
                self.lower_expr_r0(&args[3])?;
                self.emit_stm32(SCRATCH_BASE + SCRATCH_STEP * 3, 0);
                // Load: R0=x, R1=y, R2=color, R3=str_ptr
                self.emit_ldm32(0, SCRATCH_BASE);
                self.emit_ldm32(1, SCRATCH_BASE + SCRATCH_STEP);
                self.emit_ldm32(2, SCRATCH_BASE + SCRATCH_STEP * 3);
                self.emit_ldm32(3, SCRATCH_BASE + SCRATCH_STEP * 2);
                self.code.push(OP_TXT);
                self.code.push(0); // Rx
                self.code.push(1); // Ry
                self.code.push(2); // Rcolor
                self.code.push(3); // Rbase
                self.code.push(str_len as u8);
                self.restore_fp_if_needed();
            }
            "num" => {
                if args.len() != 4 {
                    return Err(LangError::ArgCount { line, name, expected: 4, got: args.len() });
                }
                self.save_fp_if_needed();
                self.lower_expr_r0(&args[0])?;
                self.emit_stm32(SCRATCH_BASE, 0);
                self.lower_expr_r0(&args[1])?;
                self.emit_stm32(SCRATCH_BASE + SCRATCH_STEP, 0);
                self.lower_expr_r0(&args[2])?;
                self.emit_stm32(SCRATCH_BASE + SCRATCH_STEP * 2, 0);
                self.lower_expr_r0(&args[3])?;
                self.emit_stm32(SCRATCH_BASE + SCRATCH_STEP * 3, 0);
                self.emit_ldm32(0, SCRATCH_BASE);
                self.emit_ldm32(1, SCRATCH_BASE + SCRATCH_STEP);
                self.emit_ldm32(2, SCRATCH_BASE + SCRATCH_STEP * 2);
                self.emit_ldm32(3, SCRATCH_BASE + SCRATCH_STEP * 3);
                self.code.push(OP_NUM);
                self.code.push(0);
                self.code.push(1);
                self.code.push(2);
                self.code.push(3);
                self.restore_fp_if_needed();
            }
            "til" => {
                // til(R0, R1, R2, R3, flags, scale)
                if args.len() < 4 {
                    return Err(LangError::ArgCount { line, name, expected: 6, got: args.len() });
                }
                self.save_fp_if_needed();
                self.lower_expr_r0(&args[0])?;
                self.emit_stm32(SCRATCH_BASE, 0);
                self.lower_expr_r0(&args[1])?;
                self.emit_stm32(SCRATCH_BASE + SCRATCH_STEP, 0);
                self.lower_expr_r0(&args[2])?;
                self.emit_stm32(SCRATCH_BASE + SCRATCH_STEP * 2, 0);
                self.lower_expr_r0(&args[3])?;
                self.emit_stm32(SCRATCH_BASE + SCRATCH_STEP * 3, 0);
                let flags = if args.len() > 4 { self.require_literal_u8(&args[4], line, &name)? } else { 0 };
                let scale = if args.len() > 5 { self.require_literal_u8(&args[5], line, &name)? } else { 1 };
                self.emit_ldm32(0, SCRATCH_BASE);
                self.emit_ldm32(1, SCRATCH_BASE + SCRATCH_STEP);
                self.emit_ldm32(2, SCRATCH_BASE + SCRATCH_STEP * 2);
                self.emit_ldm32(3, SCRATCH_BASE + SCRATCH_STEP * 3);
                self.code.push(OP_TIL);
                self.code.push(0);
                self.code.push(1);
                self.code.push(2);
                self.code.push(3);
                self.code.push(flags);
                self.code.push(scale);
                self.restore_fp_if_needed();
            }
            "rnd" => {
                if args.len() != 1 {
                    return Err(LangError::ArgCount { line, name, expected: 1, got: args.len() });
                }
                let max = self.require_literal_u16(&args[0], line, &name)?;
                self.code.push(OP_RND);
                self.code.push(0);
                self.emit_addr16(max);
            }
            _ => {
                // User-defined function call
                if !self.labels.contains_key(&name) && !self.patches.iter().any(|(_, l)| l == &name) {
                    // Check if it's a declared function (we'll validate at link time)
                    // For now, just emit JSR and hope label resolves
                }
                // Push args in reverse order
                let args_clone: Vec<Expr> = args.to_vec();
                for arg in args_clone.iter().rev() {
                    self.lower_expr_r0(arg)?;
                    self.emit_push(0);
                }
                self.emit_jsr(&name);
                // Clean up args: SP += nargs * 4
                let n = args.len();
                if n > 0 {
                    self.emit_getsp(1);
                    self.emit_mov(2, (n * 4) as u16);
                    self.emit_addr(1, 2);
                    self.emit_setsp(1);
                }
                // Result in R0
            }
        }
        Ok(())
    }

    fn literal_str(expr: &Expr) -> Option<String> {
        match expr {
            Expr::Str(s, _) => Some(s.clone()),
            Expr::BinOp { op: BinOp::Concat, left, right, .. } => {
                let l = Self::literal_str(left)?;
                let r = Self::literal_str(right)?;
                Some(format!("{}{}", l, r))
            }
            _ => None,
        }
    }

    fn require_literal_u8(&self, expr: &Expr, line: usize, name: &str) -> Result<u8> {
        let v = self.require_literal_u32(expr, line, name)?;
        if v > 255 {
            Err(LangError::RequiresLiteral { line, name: name.to_string() })
        } else {
            Ok(v as u8)
        }
    }

    fn require_literal_u16(&self, expr: &Expr, line: usize, name: &str) -> Result<u16> {
        let v = self.require_literal_u32(expr, line, name)?;
        if v > 0xFFFF {
            Err(LangError::RequiresLiteral { line, name: name.to_string() })
        } else {
            Ok(v as u16)
        }
    }

    fn require_literal_u32(&self, expr: &Expr, line: usize, name: &str) -> Result<u32> {
        match expr {
            Expr::Number(n, _) => Ok(*n),
            Expr::Var(vname, _) => {
                if let Some(&v) = self.consts.get(vname) {
                    Ok(v)
                } else {
                    Err(LangError::RequiresLiteral { line, name: name.to_string() })
                }
            }
            _ => Err(LangError::RequiresLiteral { line, name: name.to_string() }),
        }
    }
}
