use super::*;

impl Compiler {
    /// Emit __rt_newtable, __rt_gettab, __rt_settab subroutines.
    /// Must be called between JMP __start_ and fn bodies.
    pub(super) fn emit_rt_helpers(&mut self) {
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
        let fill_end = self.fresh_label("nt_fill_end");
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
        self.emit_mul_reg(1, 2); // R1 = hash * 8
        self.emit_ldm32(0, RT_TMP0);
        self.emit_mov(2, TABLE_HDR_SZ as u16);
        self.emit_addr(0, 2); // R0 = ptr + HDR_SZ
        self.emit_addr(0, 1); // R0 = ptr + HDR_SZ + hash*8
        self.emit_stm32(RT_TMP2, 0); // RT_TMP2 = probe

        // probe_limit = ptr + TABLE_ALLOC_SZ
        self.emit_ldm32(0, RT_TMP0);
        self.emit_mov32(1, TABLE_ALLOC_SZ);
        self.emit_addr(0, 1);
        self.emit_stm32(RT_TMP3, 0); // RT_TMP3 = probe_limit

        // R3 = iteration counter (FP already saved on stack)
        self.emit_mov(3, TABLE_CAP as u16);

        // loop: linear probe
        let gt_loop = self.fresh_label("gt_loop");
        let gt_found = self.fresh_label("gt_found");
        let gt_miss = self.fresh_label("gt_miss");
        let gt_wrap = self.fresh_label("gt_wrap");
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
        self.code.push(OP_EQ);
        self.code.push(0);
        self.code.push(1);
        self.code.push(2);
        self.emit_jnz(0, &gt_miss);

        // if slot_key == key → found
        self.emit_ldm32(2, RT_TMP1); // R2 = search key
        self.code.push(OP_EQ);
        self.code.push(0);
        self.code.push(1);
        self.code.push(2);
        self.emit_jnz(0, &gt_found);

        // probe += 8; if probe >= limit → wrap to ptr+HDR_SZ
        self.emit_ldm32(0, RT_TMP2);
        self.emit_mov(1, TABLE_ENTRY_SZ as u16);
        self.emit_addr(0, 1);
        self.emit_stm32(RT_TMP2, 0); // probe += 8

        self.emit_ldm32(1, RT_TMP3); // R1 = limit
        // if probe >= limit: SLTS R2, probe, limit → R2 = (probe < limit)
        self.code.push(OP_SLTS);
        self.code.push(2);
        self.code.push(0);
        self.code.push(1);
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

        let st_loop = self.fresh_label("st_loop");
        let st_write = self.fresh_label("st_write");
        let st_overwrite = self.fresh_label("st_overwrite");
        let st_wrap = self.fresh_label("st_wrap");
        let st_done = self.fresh_label("st_done");
        self.emit_label(&st_loop);

        // guard against infinite loop if table full
        self.emit_ldm32(0, RT_TMP4);
        self.emit_jz(0, &st_done);
        self.emit_mov(1, 1);
        self.emit_subr(0, 1); // counter -= 1
        self.emit_stm32(RT_TMP4, 0);

        self.emit_ldm32(0, RT_TMP3); // R0 = probe
        self.emit_ldm32i(1, 0); // R1 = slot_key

        // if sentinel or key matches → write
        self.emit_mov32(2, TABLE_SENTINEL);
        self.code.push(OP_EQ);
        self.code.push(2);
        self.code.push(1);
        self.code.push(2);
        self.emit_jnz(2, &st_write);

        self.emit_ldm32(2, RT_TMP1);
        self.code.push(OP_EQ);
        self.code.push(2);
        self.code.push(1);
        self.code.push(2);
        self.emit_jnz(2, &st_write);

        // probe += 8; wrap if >= limit
        self.emit_ldm32(0, RT_TMP3);
        self.emit_mov(1, TABLE_ENTRY_SZ as u16);
        self.emit_addr(0, 1);
        self.emit_stm32(RT_TMP3, 0);

        // if probe >= limit (R3) → wrap
        self.code.push(OP_SLTS);
        self.code.push(1);
        self.code.push(0);
        self.code.push(3);
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
        self.code.push(OP_EQ);
        self.code.push(2);
        self.code.push(1);
        self.code.push(2);
        self.emit_jz(2, &st_overwrite); // R2=0 → existing key, skip increment
        self.emit_ldm32(0, RT_TMP0); // R0 = ptr
        self.emit_mov(1, 4);
        self.emit_addr(0, 1); // R0 = ptr+4 (count field)
        self.emit_ldm32i(1, 0); // R1 = count
        self.emit_mov(2, 1);
        self.emit_addr(1, 2); // R1 = count+1
        self.emit_stm32i(0, 1); // mem[ptr+4] = count+1

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

    pub(super) fn emit_str_helpers(&mut self) {
        // ── __rt_strlen ──────────────────────────────────────────────
        // In: R0=ptr (null-terminated string)  Out: R0=length
        // Clobbers: R1, R2 (no stack push — callers save their own state)
        self.emit_label("__rt_strlen");
        self.emit_movr(2, 0); // R2 = start ptr

        let sl_loop = self.fresh_label("sl_loop");
        let sl_done = self.fresh_label("sl_done");
        self.emit_label(&sl_loop);
        self.emit_ldmi(1, 0); // R1 = mem[R0] (byte)
        self.emit_jz(1, &sl_done); // null → stop
        self.emit_mov(1, 1);
        self.emit_addr(0, 1); // R0++
        self.emit_jmp(&sl_loop);
        self.emit_label(&sl_done);
        self.emit_subr(0, 2); // R0 = end - start = length
        self.code.push(OP_RET);

        // ── __rt_strcat ──────────────────────────────────────────────
        // In: R0=ptr_a, R1=ptr_b  Out: R0=new heap ptr to concat string
        // Clobbers: R0–R3
        self.emit_label("__rt_strcat");
        self.emit_push(3);
        self.emit_stm32(RT_STR_TMP0, 0); // save ptr_a
        self.emit_stm32(RT_STR_TMP1, 1); // save ptr_b

        // len_a
        self.emit_jsr("__rt_strlen");
        self.emit_stm32(RT_STR_TMP2, 0); // save len_a

        // len_b
        self.emit_ldm32(0, RT_STR_TMP1); // R0 = ptr_b
        self.emit_jsr("__rt_strlen");
        self.emit_stm32(RT_STR_TMP3, 0); // save len_b

        // alloc = len_a + len_b + 4
        self.emit_ldm32(1, RT_STR_TMP2); // R1 = len_a
        self.emit_addr(0, 1); // R0 = len_b + len_a
        self.emit_mov(1, 4);
        self.emit_addr(0, 1); // R0 = total + 4

        // new_ptr = heap_top; heap_top += alloc
        self.emit_ldm32(1, HEAP_TOP_ADDR);
        self.emit_stm32(RT_STR_TMP4, 1); // save new_ptr
        self.emit_addr(0, 1);
        self.emit_stm32(HEAP_TOP_ADDR, 0);

        // copy ptr_a → new_ptr
        self.emit_ldm32(0, RT_STR_TMP0); // R0 = src (ptr_a)
        self.emit_ldm32(1, RT_STR_TMP4); // R1 = dst (new_ptr)
        self.emit_ldm32(2, RT_STR_TMP2); // R2 = count (len_a)
        let sca_loop = self.fresh_label("sca_loop");
        let sca_done = self.fresh_label("sca_done");
        self.emit_label(&sca_loop);
        self.emit_jz(2, &sca_done);
        self.emit_ldmi(3, 0); // R3 = *src
        self.emit_stmi(1, 3); // *dst = R3
        self.emit_mov(3, 1);
        self.emit_addr(0, 3); // src++
        self.emit_addr(1, 3); // dst++
        self.emit_mov(3, 1);
        self.emit_subr(2, 3); // count--
        self.emit_jmp(&sca_loop);
        self.emit_label(&sca_done);
        self.emit_stm32(RT_STR_TMP5, 1); // save dst cursor

        // copy ptr_b → dst cursor
        self.emit_ldm32(0, RT_STR_TMP1); // R0 = src (ptr_b)
        self.emit_ldm32(1, RT_STR_TMP5); // R1 = dst cursor
        self.emit_ldm32(2, RT_STR_TMP3); // R2 = count (len_b)
        let scb_loop = self.fresh_label("scb_loop");
        let scb_done = self.fresh_label("scb_done");
        self.emit_label(&scb_loop);
        self.emit_jz(2, &scb_done);
        self.emit_ldmi(3, 0);
        self.emit_stmi(1, 3);
        self.emit_mov(3, 1);
        self.emit_addr(0, 3);
        self.emit_addr(1, 3);
        self.emit_mov(3, 1);
        self.emit_subr(2, 3);
        self.emit_jmp(&scb_loop);
        self.emit_label(&scb_done);

        // null terminator at R1 (current dst)
        self.emit_mov(2, 0);
        self.emit_stmi(1, 2);

        self.emit_ldm32(0, RT_STR_TMP4);
        self.emit_pop(3);
        self.code.push(OP_RET);

        // ── __rt_tostr ───────────────────────────────────────────────
        // In: R0=signed integer  Out: R0=heap ptr to null-terminated decimal string
        // Clobbers: R0–R3
        self.emit_label("__rt_tostr");
        self.emit_push(3);

        // sign check: R1 = (R0 < 0) ? 1 : 0
        self.emit_mov(2, 0);
        self.code.push(OP_SLTS);
        self.code.push(1);
        self.code.push(0);
        self.code.push(2);
        self.emit_stm32(RT_STR_TMP0, 1); // save sign flag
        let ts_pos = self.fresh_label("ts_pos");
        self.emit_jz(1, &ts_pos);
        self.code.push(OP_NEG);
        self.code.push(0); // R0 = -R0
        self.emit_label(&ts_pos);

        // alloc 16 bytes on heap
        self.emit_ldm32(1, HEAP_TOP_ADDR);
        self.emit_stm32(RT_STR_TMP2, 1); // save new_ptr
        self.emit_mov(2, 16);
        self.emit_addr(1, 2);
        self.emit_stm32(HEAP_TOP_ADDR, 1);

        // write null at new_ptr + 12
        self.emit_ldm32(1, RT_STR_TMP2);
        self.emit_mov(2, 12);
        self.emit_addr(1, 2); // R1 = new_ptr + 12
        self.emit_mov(2, 0);
        self.emit_stmi(1, 2); // mem[new_ptr+12] = 0

        // write_pos = new_ptr + 11
        self.emit_ldm32(1, RT_STR_TMP2);
        self.emit_mov(2, 11);
        self.emit_addr(1, 2); // R1 = new_ptr + 11
        self.emit_stm32(RT_STR_TMP3, 1);

        // handle R0 == 0 specially
        let ts_dig_loop = self.fresh_label("ts_dig_loop");
        let ts_sign = self.fresh_label("ts_sign");
        let ts_ret = self.fresh_label("ts_ret");
        let ts_done_dig = self.fresh_label("ts_done_dig");
        self.emit_jnz(0, &ts_dig_loop);
        // write '0'
        self.emit_ldm32(1, RT_STR_TMP3);
        self.emit_mov32(2, 0x30); // '0'
        self.emit_stmi(1, 2);
        self.emit_mov(2, 1);
        self.emit_subr(1, 2);
        self.emit_stm32(RT_STR_TMP3, 1);
        self.emit_jmp(&ts_sign);

        // digit extraction loop
        self.emit_label(&ts_dig_loop);
        self.emit_jz(0, &ts_done_dig);
        self.emit_movr(3, 0); // R3 = save value
        self.emit_mov32(1, 10);
        self.emit_mod_reg(0, 1); // R0 = value % 10
        self.emit_mov32(1, 0x30);
        self.emit_addr(0, 1); // R0 = '0' + digit
        self.emit_ldm32(1, RT_STR_TMP3);
        self.emit_stmi(1, 0); // mem[write_pos] = char
        self.emit_mov(0, 1);
        self.emit_subr(1, 0); // write_pos--
        self.emit_stm32(RT_STR_TMP3, 1);
        self.emit_movr(0, 3); // R0 = saved value
        self.emit_mov32(1, 10);
        self.emit_div_reg(0, 1); // R0 = value / 10
        self.emit_jmp(&ts_dig_loop);
        self.emit_label(&ts_done_dig);

        self.emit_label(&ts_sign);
        self.emit_ldm32(1, RT_STR_TMP0); // R1 = sign flag
        self.emit_jz(1, &ts_ret);
        self.emit_ldm32(1, RT_STR_TMP3);
        self.emit_mov32(2, 0x2D); // '-'
        self.emit_stmi(1, 2);
        self.emit_mov(2, 1);
        self.emit_subr(1, 2);
        self.emit_stm32(RT_STR_TMP3, 1);

        self.emit_label(&ts_ret);
        self.emit_ldm32(0, RT_STR_TMP3);
        self.emit_mov(1, 1);
        self.emit_addr(0, 1); // return write_pos + 1
        self.emit_pop(3);
        self.code.push(OP_RET);
    }
}
