// fn_ctx is Some while lowering a function body (see module comment in mod.rs).
#![allow(clippy::unwrap_used)]

use super::*;

impl Compiler {
    pub(super) fn compile_block(&mut self, block: &[Stmt]) -> Result<()> {
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

    pub(super) fn stmt_line(stmt: &Stmt) -> usize {
        match stmt {
            Stmt::Local { line, .. }
            | Stmt::Assign { line, .. }
            | Stmt::Do { line, .. }
            | Stmt::While { line, .. }
            | Stmt::Repeat { line, .. }
            | Stmt::If { line, .. }
            | Stmt::NumericFor { line, .. }
            | Stmt::Return { line, .. }
            | Stmt::Break { line }
            | Stmt::ExprStmt { line, .. }
            | Stmt::SetField { line, .. }
            | Stmt::SetIndex { line, .. }
            | Stmt::GenericFor { line, .. } => *line,
        }
    }

    pub(super) fn compile_stmt(&mut self, stmt: &Stmt) -> Result<()> {
        let line = Self::stmt_line(stmt);
        self.source_map.set_src_line(self.code.len(), line);
        match stmt {
            Stmt::ExprStmt { expr, .. } => {
                self.lower_expr_r0(expr)?;
            }
            Stmt::Assign {
                target,
                value,
                line,
            } => {
                self.lower_expr_r0(value)?;
                match self
                    .lookup_var(target)
                    .ok_or_else(|| LangError::UndefinedVariable {
                        line: *line,
                        name: target.clone(),
                    })? {
                    VarLoc::Const(_) => {
                        return Err(LangError::UndefinedVariable {
                            line: *line,
                            name: target.clone(),
                        });
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
                    VarLoc::Upvalue(i) => {
                        self.emit_store_upval(i);
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
            Stmt::If {
                cond,
                then_block,
                elseif_clauses,
                else_block,
                ..
            } => {
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
                    ctx.break_targets.push(BreakTarget {
                        end_label: end_label.clone(),
                        slots_at_entry,
                    });
                } else {
                    self.top_break_targets.push(BreakTarget {
                        end_label: end_label.clone(),
                        slots_at_entry: 0,
                    });
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
                    ctx.break_targets.push(BreakTarget {
                        end_label: end_label.clone(),
                        slots_at_entry,
                    });
                } else {
                    self.top_break_targets.push(BreakTarget {
                        end_label: end_label.clone(),
                        slots_at_entry: 0,
                    });
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
            Stmt::NumericFor {
                var,
                start,
                stop,
                step,
                body,
                line,
            } => {
                // for var = start, stop [, step] do body end
                let loop_label = self.fresh_label("nfor_loop");
                let end_label = self.fresh_label("nfor_end");
                let step_val = step.clone().unwrap_or(Expr::Number(1, *line));

                // Detect step sign at compile time so we emit the correct condition.
                // Positive (default): exit when var > stop  (SLTS R2, stop, var)
                // Negative:           exit when var < stop  (SLTS R2, var,  stop)
                // Number is u32 so negative literals parse as UnOp::Neg(Number(n)).
                let neg_step = matches!(&step_val, Expr::UnOp { op: UnOp::Neg, .. });

                if self.fn_ctx.is_some() {
                    // ── inside a function: use stack locals ──────────────────
                    let slots_at_entry = self.fn_ctx.as_ref().unwrap().next_slot;
                    self.fn_ctx
                        .as_mut()
                        .unwrap()
                        .break_targets
                        .push(BreakTarget {
                            end_label: end_label.clone(),
                            slots_at_entry,
                        });
                    self.fn_ctx.as_mut().unwrap().push_scope();

                    let start = start.clone();
                    self.lower_expr_r0(&start)?;
                    let var_slot = {
                        let s = self.fn_ctx.as_mut().unwrap().alloc_local(var.clone());
                        self.emit_push(0);
                        s
                    };

                    let stop = stop.clone();
                    self.lower_expr_r0(&stop)?;
                    let stop_slot = {
                        let s = self
                            .fn_ctx
                            .as_mut()
                            .unwrap()
                            .alloc_local("__nfor_stop".to_string());
                        self.emit_push(0);
                        s
                    };

                    self.lower_expr_r0(&step_val)?;
                    let step_slot = {
                        let s = self
                            .fn_ctx
                            .as_mut()
                            .unwrap()
                            .alloc_local("__nfor_step".to_string());
                        self.emit_push(0);
                        s
                    };

                    self.emit_label(&loop_label);

                    // Load var → R0, stop → R1
                    self.emit_load_local(var_slot);
                    self.emit_push(0);
                    self.emit_load_local(stop_slot);
                    self.emit_movr(1, 0);
                    self.emit_pop(0);
                    // Condition: positive step → exit when stop < var (R1 < R0)
                    //            negative step → exit when var  < stop (R0 < R1)
                    self.code.push(OP_SLTS);
                    self.code.push(2);
                    if neg_step {
                        self.code.push(0);
                        self.code.push(1);
                    } else {
                        self.code.push(1);
                        self.code.push(0);
                    }
                    self.emit_jnz(2, &end_label);

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

                    let freed = self.fn_ctx.as_mut().unwrap().pop_scope();
                    if freed > 0 {
                        self.emit_getsp(1);
                        self.emit_mov(2, (freed * 4) as u16);
                        self.emit_addr(1, 2);
                        self.emit_setsp(1);
                    }
                    self.fn_ctx.as_mut().unwrap().break_targets.pop();
                } else {
                    // ── top-level: use global memory slots ──────────────────
                    let uid = self.fresh_label("nfor");
                    let var_name = format!("__nfor_v_{}", uid);
                    let stop_name = format!("__nfor_s_{}", uid);
                    let step_name = format!("__nfor_t_{}", uid);

                    let var_addr = self.alloc_global(&var_name);
                    let stop_addr = self.alloc_global(&stop_name);
                    let step_addr = self.alloc_global(&step_name);
                    // Make loop var visible by name so body can reference it
                    self.globals.insert(var.clone(), var_addr);

                    let start = start.clone();
                    self.lower_expr_r0(&start)?;
                    self.emit_stm32(var_addr, 0);

                    let stop = stop.clone();
                    self.lower_expr_r0(&stop)?;
                    self.emit_stm32(stop_addr, 0);

                    self.lower_expr_r0(&step_val)?;
                    self.emit_stm32(step_addr, 0);

                    self.emit_label(&loop_label);

                    // Load var → R0, stop → R1
                    self.emit_ldm32(0, var_addr);
                    self.emit_ldm32(1, stop_addr);
                    // Condition: same sign logic as function path
                    self.code.push(OP_SLTS);
                    self.code.push(2);
                    if neg_step {
                        self.code.push(0);
                        self.code.push(1);
                    } else {
                        self.code.push(1);
                        self.code.push(0);
                    }
                    self.emit_jnz(2, &end_label);

                    let body = body.clone();
                    self.compile_block(&body)?;

                    // var += step
                    self.emit_ldm32(0, var_addr);
                    self.emit_ldm32(1, step_addr);
                    self.emit_addr(0, 1);
                    self.emit_stm32(var_addr, 0);

                    self.emit_jmp(&loop_label);
                    self.emit_label(&end_label);
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
            Stmt::SetField {
                table, name, value, ..
            } => {
                let table = table.clone();
                let value = value.clone();
                let key_ptr = self.intern_string(name);
                self.lower_expr_r0(&table)?;
                self.emit_push(0); // push ptr (value eval may clobber scratch)
                self.lower_expr_r0(&value)?;
                self.emit_stm32(SCRATCH_BASE, 0); // save val
                self.emit_pop(0); // R0 = ptr
                self.emit_mov(1, key_ptr);
                self.emit_ldm32(2, SCRATCH_BASE);
                self.emit_jsr("__rt_settab");
            }
            Stmt::SetIndex {
                table, key, value, ..
            } => {
                let table = table.clone();
                let key = key.clone();
                let value = value.clone();
                self.lower_expr_r0(&table)?;
                self.emit_push(0); // push ptr
                self.lower_expr_r0(&key)?;
                self.emit_push(0); // push key
                self.lower_expr_r0(&value)?;
                // R0=val, stack=[ptr, key(top)]
                self.emit_movr(2, 0); // R2 = val
                self.emit_pop(1); // R1 = key
                self.emit_pop(0); // R0 = ptr
                self.emit_jsr("__rt_settab");
            }
            Stmt::GenericFor {
                key_var,
                val_var,
                table,
                body,
                line: _,
            } => {
                // for key_var [, val_var] in table do body end
                // Iterates TABLE_CAP slots linearly, skipping sentinel (0xFFFF_FFFF) keys.
                // Table layout: ptr+0=cap, ptr+4=count, ptr+8+i*8=key, ptr+12+i*8=val
                let key_var = key_var.clone();
                let val_var = val_var.clone();
                let table = table.clone();
                let body = body.clone();
                let loop_label = self.fresh_label("gfor_loop");
                let end_label = self.fresh_label("gfor_end");
                let skip_label = self.fresh_label("gfor_skip");

                if self.fn_ctx.is_some() {
                    // ── inside function: use stack locals ────────────────────────

                    let slots_at_entry = self.fn_ctx.as_ref().unwrap().next_slot;
                    self.fn_ctx
                        .as_mut()
                        .unwrap()
                        .break_targets
                        .push(BreakTarget {
                            end_label: end_label.clone(),
                            slots_at_entry,
                        });

                    self.lower_expr_r0(&table)?;
                    self.fn_ctx.as_mut().unwrap().push_scope();

                    let ptr_slot = {
                        let s = self
                            .fn_ctx
                            .as_mut()
                            .unwrap()
                            .alloc_local("__iter_ptr".to_string());
                        self.emit_push(0);
                        s
                    };
                    self.emit_mov(0, 0);
                    let idx_slot = {
                        let s = self
                            .fn_ctx
                            .as_mut()
                            .unwrap()
                            .alloc_local("__iter_idx".to_string());
                        self.emit_push(0);
                        s
                    };

                    self.emit_label(&loop_label);

                    // if idx >= TABLE_CAP → end
                    self.emit_load_local(idx_slot);
                    self.emit_mov32(1, TABLE_CAP);
                    self.code.push(OP_SLTS);
                    self.code.push(2);
                    self.code.push(0);
                    self.code.push(1);
                    self.emit_jz(2, &end_label);

                    // R0 = entry_addr = iter_ptr + HDR_SZ + idx * ENTRY_SZ
                    self.emit_load_local(idx_slot);
                    self.emit_mov32(1, TABLE_ENTRY_SZ);
                    self.emit_mul_reg(0, 1); // R0 = idx * 8
                    self.emit_mov32(1, TABLE_HDR_SZ);
                    self.emit_addr(0, 1);
                    self.emit_movr(2, 0); // R2 = offset
                    self.emit_load_local(ptr_slot); // R0 = iter_ptr
                    self.emit_addr(0, 2); // R0 = entry_addr

                    // key = mem32[entry_addr]; save entry_addr in R2
                    self.emit_movr(2, 0);
                    self.emit_ldm32i(0, 0); // R0 = key

                    // if sentinel → skip
                    self.emit_mov32(1, TABLE_SENTINEL);
                    self.code.push(OP_EQ);
                    self.code.push(1);
                    self.code.push(0);
                    self.code.push(1);
                    self.emit_jnz(1, &skip_label);

                    // val = mem32[entry_addr + 4]; R2 = entry_addr
                    self.emit_push(0); // save key
                    self.emit_mov32(1, 4);
                    self.emit_addr(2, 1); // R2 = entry_addr + 4
                    self.emit_ldm32i(1, 2); // R1 = val
                    self.emit_pop(0); // R0 = key

                    // bind key_var and val_var in inner scope
                    self.fn_ctx.as_mut().unwrap().push_scope();
                    let _ = {
                        let s = self.fn_ctx.as_mut().unwrap().alloc_local(key_var.clone());
                        self.emit_push(0);
                        s
                    };
                    self.emit_movr(0, 1);
                    let _ = {
                        let s = self.fn_ctx.as_mut().unwrap().alloc_local(val_var.clone());
                        self.emit_push(0);
                        s
                    };

                    self.compile_block(&body)?;

                    let freed = self.fn_ctx.as_mut().unwrap().pop_scope();
                    if freed > 0 {
                        self.emit_getsp(1);
                        self.emit_mov(2, (freed * 4) as u16);
                        self.emit_addr(1, 2);
                        self.emit_setsp(1);
                    }

                    self.emit_label(&skip_label);
                    self.emit_load_local(idx_slot);
                    self.emit_mov(1, 1);
                    self.emit_addr(0, 1);
                    self.emit_store_local(idx_slot);
                    self.emit_jmp(&loop_label);
                    self.emit_label(&end_label);

                    let freed = self.fn_ctx.as_mut().unwrap().pop_scope();
                    if freed > 0 {
                        self.emit_getsp(1);
                        self.emit_mov(2, (freed * 4) as u16);
                        self.emit_addr(1, 2);
                        self.emit_setsp(1);
                    }
                    self.fn_ctx.as_mut().unwrap().break_targets.pop();
                } else {
                    // ── top-level (init/loop block): use globals ─────────────────

                    self.top_break_targets.push(BreakTarget {
                        end_label: end_label.clone(),
                        slots_at_entry: 0,
                    });

                    // Alloc anonymous globals for iter state
                    let lc = self.label_counter;
                    let ptr_name = format!("__gfor_ptr_{}", lc);
                    let idx_name = format!("__gfor_idx_{}", lc);
                    let ptr_addr = self.alloc_global(&ptr_name);
                    let idx_addr = self.alloc_global(&idx_name);

                    // Alloc globals for key_var and val_var (so body lookups find them)
                    let key_addr = if let Some(&a) = self.globals.get(&key_var) {
                        a
                    } else {
                        self.alloc_global(&key_var)
                    };
                    let val_addr = if let Some(&a) = self.globals.get(&val_var) {
                        a
                    } else {
                        self.alloc_global(&val_var)
                    };

                    self.lower_expr_r0(&table)?;
                    self.emit_stm32(ptr_addr, 0); // iter_ptr = table
                    self.emit_mov(0, 0);
                    self.emit_stm32(idx_addr, 0); // iter_idx = 0

                    self.emit_label(&loop_label);

                    // if idx >= TABLE_CAP → end
                    self.emit_ldm32(0, idx_addr);
                    self.emit_mov32(1, TABLE_CAP);
                    self.code.push(OP_SLTS);
                    self.code.push(2);
                    self.code.push(0);
                    self.code.push(1);
                    self.emit_jz(2, &end_label);

                    // R0 = entry_addr = iter_ptr + HDR_SZ + idx * ENTRY_SZ
                    self.emit_ldm32(0, idx_addr);
                    self.emit_mov32(1, TABLE_ENTRY_SZ);
                    self.emit_mul_reg(0, 1); // R0 = idx * 8
                    self.emit_mov32(1, TABLE_HDR_SZ);
                    self.emit_addr(0, 1);
                    self.emit_movr(2, 0); // R2 = offset
                    self.emit_ldm32(0, ptr_addr); // R0 = iter_ptr
                    self.emit_addr(0, 2); // R0 = entry_addr

                    // key = mem32[entry_addr]
                    self.emit_movr(2, 0); // R2 = entry_addr
                    self.emit_ldm32i(0, 0); // R0 = key

                    // if sentinel → skip
                    self.emit_mov32(1, TABLE_SENTINEL);
                    self.code.push(OP_EQ);
                    self.code.push(1);
                    self.code.push(0);
                    self.code.push(1);
                    self.emit_jnz(1, &skip_label);

                    // store key, load val
                    self.emit_stm32(key_addr, 0); // key_var = key
                    self.emit_mov32(1, 4);
                    self.emit_addr(2, 1); // R2 = entry_addr + 4
                    self.emit_ldm32i(0, 2); // R0 = val
                    self.emit_stm32(val_addr, 0); // val_var = val

                    self.compile_block(&body)?;

                    self.emit_label(&skip_label);
                    self.emit_ldm32(0, idx_addr);
                    self.emit_mov(1, 1);
                    self.emit_addr(0, 1);
                    self.emit_stm32(idx_addr, 0);
                    self.emit_jmp(&loop_label);
                    self.emit_label(&end_label);
                    self.top_break_targets.pop();
                }
            }
        }
        Ok(())
    }
}
