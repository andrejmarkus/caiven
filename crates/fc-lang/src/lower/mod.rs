// Internal invariants (fn_ctx present while lowering a function body); full
// rewrite of this module is deliberately deferred.
#![allow(clippy::unwrap_used)]

use crate::ast::*;
use crate::error::{LangError, Result};
use fc_asm::SourceMap;
use fc_asm::opcodes::*;
use std::collections::HashMap;

mod emit;
mod expr;
mod free_vars;
mod runtime;
mod stmt;

const GLOBALS_BASE: u16 = 0x0000;
const SCRATCH_BASE: u16 = 0x3FF0;
const SCRATCH_STEP: u16 = 4;
const FP_SAVE_ADDR: u16 = 0x3FEC;
const STRING_POOL_BASE: u16 = 0x3800;

// Heap allocator (bump pointer)
const HEAP_BASE: u32 = 0x6000;
const HEAP_TOP_ADDR: u16 = 0x5000; // u32 at 0x5000: current heap top
// Runtime scratch (non-reentrant; table ops don't nest)
const RT_TMP0: u16 = 0x5004;
const RT_TMP1: u16 = 0x5008;
const RT_TMP2: u16 = 0x500C;
const RT_TMP3: u16 = 0x5010;
const RT_TMP4: u16 = 0x5014; // iteration counter for __rt_settab probe loop
// String runtime scratch (non-reentrant; string ops don't nest)
const RT_STR_TMP0: u16 = 0x5018;
const RT_STR_TMP1: u16 = 0x501C;
const RT_STR_TMP2: u16 = 0x5020;
const RT_STR_TMP3: u16 = 0x5024;
const RT_STR_TMP4: u16 = 0x5028;
const RT_STR_TMP5: u16 = 0x502C;
// Table layout constants
const TABLE_CAP: u32 = 8; // fixed capacity (power-of-2 → bitmask works)
const TABLE_ENTRY_SZ: u32 = 8; // key(u32) + val(u32)
const TABLE_HDR_SZ: u32 = 8; // cap(u32) + count(u32)
const TABLE_ALLOC_SZ: u32 = TABLE_HDR_SZ + TABLE_CAP * TABLE_ENTRY_SZ; // 72
const TABLE_SENTINEL: u32 = 0xFFFFFFFF; // marks empty slot key

#[derive(Clone, Debug)]
enum VarLoc {
    Const(u32),
    Global(u16),    // absolute RAM address (4-byte cell)
    Local(usize),   // slot index (FP - slot*4)
    Param(usize),   // actual param index including hidden env_ptr for closures
    Upvalue(usize), // upval index; loaded from env_ptr (param[0]) + i*4
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
    upvals: Vec<String>,
    is_closure: bool,
}

impl FnCtx {
    fn new(params: Vec<String>) -> Self {
        FnCtx {
            params,
            scopes: vec![HashMap::new()],
            next_slot: 0,
            break_targets: Vec::new(),
            upvals: Vec::new(),
            is_closure: false,
        }
    }

    fn new_closure(params: Vec<String>, upvals: Vec<String>) -> Self {
        FnCtx {
            params,
            scopes: vec![HashMap::new()],
            next_slot: 0,
            break_targets: Vec::new(),
            upvals,
            is_closure: true,
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
        // Check params — for closures, param[0] is hidden env_ptr so user params start at 1
        for (i, p) in self.params.iter().enumerate() {
            if p == name {
                let actual_idx = if self.is_closure { i + 1 } else { i };
                return Some(VarLoc::Param(actual_idx));
            }
        }
        // Check upvals
        for (i, u) in self.upvals.iter().enumerate() {
            if u == name {
                return Some(VarLoc::Upvalue(i));
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
    fn_names: std::collections::HashSet<String>,
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
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
            fn_names: std::collections::HashSet::new(),
        }
    }

    pub fn finish(mut self) -> Result<(Vec<u8>, SourceMap)> {
        self.apply_patches()?;
        // Patch CPY src/len and append string pool to ROM
        let pool_src = self.code.len();
        let pool_len = self.string_pool.len();
        self.code[self.cpy_src_patch] = (pool_src & 0xFF) as u8;
        self.code[self.cpy_src_patch + 1] = ((pool_src >> 8) & 0xFF) as u8;
        self.code[self.cpy_len_patch] = (pool_len & 0xFF) as u8;
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
                return Err(crate::error::LangError::UnresolvedLabel {
                    label: label.clone(),
                });
            }
        }
        Ok(())
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

        // Emit RT helpers (newtable / gettab / settab / strlen / strcat / tostr)
        self.emit_rt_helpers();
        self.emit_str_helpers();

        // Populate fn_names for static-vs-dynamic call dispatch
        for func in &file.functions {
            self.fn_names.insert(func.name.clone());
        }

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
        self.code.push(0);
        self.code.push(0); // src — patched
        self.cpy_len_patch = self.code.len();
        self.code.push(0);
        self.code.push(0); // len — patched

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

    // Emit a closure body at the current code position.
    // Calling convention: param[0] = env_ptr (hidden), param[1..] = user args.
    // env_ptr points to upval array: [upval[0](u32), upval[1](u32), ...]
    fn compile_closure_fn(
        &mut self,
        params: &[String],
        body: &[Stmt],
        upvals: Vec<String>,
    ) -> Result<()> {
        self.emit_push(3);
        self.emit_getsp(3);

        let saved_ctx = self.fn_ctx.take();
        self.fn_ctx = Some(FnCtx::new_closure(params.to_vec(), upvals));

        let body = body.to_vec();
        self.compile_block(&body)?;

        self.emit_setsp(3);
        self.emit_pop(3);
        self.code.push(OP_RET);

        self.fn_ctx = saved_ctx;
        Ok(())
    }
}
