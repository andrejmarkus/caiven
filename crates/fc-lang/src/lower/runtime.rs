use super::*;

impl Compiler {
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

        // ── __rt_substr ──────────────────────────────────────────────
        // In: R0=ptr, R1=i, R2=j (1-based, inclusive)
        // Out: R0=heap ptr to the null-terminated substring
        // Clobbers: R0–R2 (R3 saved/restored)
        self.emit_label("__rt_substr");
        self.emit_push(3);
        self.emit_stm32(RT_STR_TMP0, 0); // save ptr
        self.emit_stm32(RT_STR_TMP1, 1); // save i
        self.emit_stm32(RT_STR_TMP2, 2); // save j

        // len = strlen(ptr)
        self.emit_jsr("__rt_strlen");
        self.emit_stm32(RT_STR_TMP3, 0); // save len

        // i = max(i, 1)
        self.emit_ldm32(0, RT_STR_TMP1);
        self.emit_mov(1, 1);
        self.code.push(OP_MAX);
        self.code.push(0);
        self.code.push(1);
        self.emit_stm32(RT_STR_TMP1, 0);

        // j = min(j, len)
        self.emit_ldm32(0, RT_STR_TMP2);
        self.emit_ldm32(1, RT_STR_TMP3);
        self.code.push(OP_MIN);
        self.code.push(0);
        self.code.push(1);
        self.emit_stm32(RT_STR_TMP2, 0);

        // n = j - i + 1; clamp negative to 0
        self.emit_ldm32(1, RT_STR_TMP1);
        self.emit_subr(0, 1); // R0 = j - i
        self.emit_mov(1, 1);
        self.emit_addr(0, 1); // R0 = n
        self.emit_mov(1, 0);
        self.code.push(OP_MAX); // signed max(n, 0)
        self.code.push(0);
        self.code.push(1);
        self.emit_stm32(RT_STR_TMP4, 0); // save n

        // new_ptr = heap_top; heap_top += n + 1
        self.emit_mov(1, 1);
        self.emit_addr(0, 1); // R0 = n + 1
        self.emit_ldm32(1, HEAP_TOP_ADDR);
        self.emit_stm32(RT_STR_TMP5, 1); // save new_ptr
        self.emit_addr(0, 1);
        self.emit_stm32(HEAP_TOP_ADDR, 0);

        // src = ptr + i - 1, dst = new_ptr, count = n
        self.emit_ldm32(0, RT_STR_TMP0);
        self.emit_ldm32(1, RT_STR_TMP1);
        self.emit_addr(0, 1);
        self.emit_mov(1, 1);
        self.emit_subr(0, 1); // R0 = src
        self.emit_ldm32(1, RT_STR_TMP5); // R1 = dst
        self.emit_ldm32(2, RT_STR_TMP4); // R2 = count

        let ss_loop = self.fresh_label("ss_loop");
        let ss_done = self.fresh_label("ss_done");
        self.emit_label(&ss_loop);
        self.emit_jz(2, &ss_done);
        self.emit_ldmi(3, 0);
        self.emit_stmi(1, 3);
        self.emit_mov(3, 1);
        self.emit_addr(0, 3); // src++
        self.emit_addr(1, 3); // dst++
        self.emit_mov(3, 1);
        self.emit_subr(2, 3); // count--
        self.emit_jmp(&ss_loop);
        self.emit_label(&ss_done);

        // null terminator; return new_ptr
        self.emit_mov(2, 0);
        self.emit_stmi(1, 2);
        self.emit_ldm32(0, RT_STR_TMP5);
        self.emit_pop(3);
        self.code.push(OP_RET);
    }
}
