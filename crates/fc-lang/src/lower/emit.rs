use super::*;

impl Compiler {
    pub(super) fn fresh_label(&mut self, prefix: &str) -> String {
        let n = self.label_counter;
        self.label_counter += 1;
        format!("__{}__{}", prefix, n)
    }

    pub(super) fn emit_label(&mut self, name: &str) {
        self.labels.insert(name.to_string(), self.code.len());
    }

    pub(super) fn emit_jmp(&mut self, label: &str) {
        self.code.push(OP_JMP);
        self.patches.push((self.code.len(), label.to_string()));
        self.code.push(0);
        self.code.push(0);
    }

    pub(super) fn emit_jz(&mut self, reg: u8, label: &str) {
        self.code.push(OP_JZ);
        self.code.push(reg);
        self.patches.push((self.code.len(), label.to_string()));
        self.code.push(0);
        self.code.push(0);
    }

    pub(super) fn emit_jnz(&mut self, reg: u8, label: &str) {
        self.code.push(OP_JNZ);
        self.code.push(reg);
        self.patches.push((self.code.len(), label.to_string()));
        self.code.push(0);
        self.code.push(0);
    }

    pub(super) fn emit_jsr(&mut self, label: &str) {
        self.code.push(OP_JSR);
        self.patches.push((self.code.len(), label.to_string()));
        self.code.push(0);
        self.code.push(0);
    }

    pub(super) fn emit_addr16(&mut self, addr: u16) {
        self.code.push((addr & 0xFF) as u8);
        self.code.push(((addr >> 8) & 0xFF) as u8);
    }

    // MOV Rd, imm16 (address-sized immediate)
    pub(super) fn emit_mov(&mut self, rd: u8, imm16: u16) {
        self.code.push(OP_MOV);
        self.code.push(rd);
        self.emit_addr16(imm16);
    }

    // MOV32 Rd, imm32
    pub(super) fn emit_mov32(&mut self, rd: u8, imm32: u32) {
        self.code.push(OP_MOV32);
        self.code.push(rd);
        self.code.push((imm32 & 0xFF) as u8);
        self.code.push(((imm32 >> 8) & 0xFF) as u8);
        self.code.push(((imm32 >> 16) & 0xFF) as u8);
        self.code.push(((imm32 >> 24) & 0xFF) as u8);
    }

    // Emit load of u32 value into register, choosing MOV vs MOV32
    pub(super) fn emit_mov_r0_imm(&mut self, val: u32) {
        if val <= 0xFFFF {
            self.emit_mov(0, val as u16);
        } else {
            self.emit_mov32(0, val);
        }
    }

    pub(super) fn emit_movr(&mut self, rd: u8, rs: u8) {
        self.code.push(OP_MOVR);
        self.code.push(rd);
        self.code.push(rs);
    }

    pub(super) fn emit_addr(&mut self, rd: u8, rs: u8) {
        self.code.push(OP_ADDR);
        self.code.push(rd);
        self.code.push(rs);
    }

    pub(super) fn emit_subr(&mut self, rd: u8, rs: u8) {
        self.code.push(OP_SUBR);
        self.code.push(rd);
        self.code.push(rs);
    }

    pub(super) fn emit_push(&mut self, reg: u8) {
        self.code.push(OP_PUSH);
        self.code.push(reg);
    }

    pub(super) fn emit_pop(&mut self, reg: u8) {
        self.code.push(OP_POP);
        self.code.push(reg);
    }

    pub(super) fn emit_ldm32(&mut self, rd: u8, addr: u16) {
        self.code.push(OP_LDM32);
        self.code.push(rd);
        self.emit_addr16(addr);
    }

    pub(super) fn emit_stm32(&mut self, addr: u16, rs: u8) {
        self.code.push(OP_STM32);
        self.emit_addr16(addr);
        self.code.push(rs);
    }

    pub(super) fn emit_ldm32i(&mut self, rd: u8, raddr: u8) {
        self.code.push(OP_LDM32I);
        self.code.push(rd);
        self.code.push(raddr);
    }

    pub(super) fn emit_stm32i(&mut self, raddr: u8, rs: u8) {
        self.code.push(OP_STM32I);
        self.code.push(raddr);
        self.code.push(rs);
    }

    // Byte indirect: Rd = mem[Raddr] (zero-extended)
    pub(super) fn emit_ldmi(&mut self, rd: u8, raddr: u8) {
        self.code.push(OP_LDMI);
        self.code.push(rd);
        self.code.push(raddr);
    }

    // Byte indirect: mem[Raddr] = Rs & 0xFF
    pub(super) fn emit_stmi(&mut self, raddr: u8, rs: u8) {
        self.code.push(OP_STMI);
        self.code.push(raddr);
        self.code.push(rs);
    }

    pub(super) fn emit_div_reg(&mut self, rd: u8, rs: u8) {
        self.code.push(OP_DIV);
        self.code.push(rd);
        self.code.push(rs);
    }

    pub(super) fn emit_mod_reg(&mut self, rd: u8, rs: u8) {
        self.code.push(OP_MOD);
        self.code.push(rd);
        self.code.push(rs);
    }

    pub(super) fn emit_getsp(&mut self, rd: u8) {
        self.code.push(OP_GETSP);
        self.code.push(rd);
    }

    // Intern a string literal into the pool; returns its RAM address after CPY.
    pub(super) fn intern_string(&mut self, s: &str) -> u16 {
        if let Some(&off) = self.string_offsets.get(s) {
            return STRING_POOL_BASE + off;
        }
        let off = self.string_pool.len() as u16;
        self.string_offsets.insert(s.to_string(), off);
        self.string_pool.extend_from_slice(s.as_bytes());
        self.string_pool.push(0); // null terminator
        STRING_POOL_BASE + off
    }

    pub(super) fn emit_setsp(&mut self, rs: u8) {
        self.code.push(OP_SETSP);
        self.code.push(rs);
    }

    pub(super) fn emit_jreg(&mut self, reg: u8) {
        self.code.push(OP_JREG);
        self.code.push(reg);
    }

    // MOV Rd, label_addr — patched at apply_patches time
    pub(super) fn emit_mov_label(&mut self, rd: u8, label: &str) {
        self.code.push(OP_MOV);
        self.code.push(rd);
        self.patches.push((self.code.len(), label.to_string()));
        self.code.push(0);
        self.code.push(0);
    }

    // Load upval[i] → R0: env_ptr = param[0]; R0 = mem32[env_ptr + i*4]
    pub(super) fn emit_load_upval(&mut self, i: usize) {
        self.emit_load_param(0); // R0 = env_ptr
        self.emit_mov(1, (i * 4) as u16); // R1 = i*4
        self.emit_addr(0, 1); // R0 = env_ptr + i*4
        self.emit_ldm32i(0, 0); // R0 = mem32[R0]
    }

    // Store R0 into upval[i]: mem32[env_ptr + i*4] = R0
    pub(super) fn emit_store_upval(&mut self, i: usize) {
        self.emit_push(0); // save value
        self.emit_load_param(0); // R0 = env_ptr
        self.emit_mov(1, (i * 4) as u16); // R1 = i*4
        self.emit_addr(0, 1); // R0 = env_ptr + i*4 (address)
        self.emit_movr(1, 0); // R1 = address
        self.emit_pop(0); // R0 = value
        self.emit_stm32i(1, 0); // mem32[address] = value
    }

    pub(super) fn emit_mul_reg(&mut self, rd: u8, rs: u8) {
        self.code.push(OP_MUL);
        self.code.push(rd);
        self.code.push(rs);
    }

    pub(super) fn emit_and_reg(&mut self, rd: u8, rs: u8) {
        self.code.push(OP_AND);
        self.code.push(rd);
        self.code.push(rs);
    }

    // Load local slot i into R0: R0 = mem[FP - (slot+1)*4]
    // slot 0 → FP-4, slot 1 → FP-8, ...
    pub(super) fn emit_load_local(&mut self, slot: usize) {
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
    pub(super) fn emit_store_local(&mut self, slot: usize) {
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
    pub(super) fn emit_load_param(&mut self, param_idx: usize) {
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
    pub(super) fn emit_store_param(&mut self, param_idx: usize) {
        // R1 = FP
        self.emit_movr(1, 3);
        // R2 = 6 + param_idx*4
        self.emit_mov(2, (6 + param_idx * 4) as u16);
        // R1 = R1 + R2
        self.emit_addr(1, 2);
        // mem[R1] = R0
        self.emit_stm32i(1, 0);
    }

    pub(super) fn lookup_var(&self, name: &str) -> Option<VarLoc> {
        if let Some(ctx) = &self.fn_ctx
            && let Some(loc) = ctx.lookup(name)
        {
            return Some(loc);
        }
        if let Some(&val) = self.consts.get(name) {
            return Some(VarLoc::Const(val));
        }
        if let Some(&addr) = self.globals.get(name) {
            return Some(VarLoc::Global(addr));
        }
        None
    }

    pub(super) fn alloc_global(&mut self, name: &str) -> u16 {
        let addr = self.next_global;
        self.globals.insert(name.to_string(), addr);
        self.next_global += 4;
        addr
    }

    // Save FP (R3) to FP_SAVE_ADDR if we are inside a function
    pub(super) fn save_fp_if_needed(&mut self) {
        if self.fn_ctx.is_some() {
            self.emit_stm32(FP_SAVE_ADDR, 3);
        }
    }

    // Restore FP (R3) from FP_SAVE_ADDR if we are inside a function
    pub(super) fn restore_fp_if_needed(&mut self) {
        if self.fn_ctx.is_some() {
            self.emit_ldm32(3, FP_SAVE_ADDR);
        }
    }
}
