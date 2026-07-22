// fn_ctx is Some while lowering a function body (see module comment in mod.rs).
#![allow(clippy::unwrap_used)]

use super::free_vars::collect_free_vars;
use super::*;

impl Compiler {
    // Emit an indirect call through a closure pointer in R0.
    // Stack layout before JREG: [...args (reversed)... env_ptr(top)]
    // env_ptr = closure_ptr + 8; code_ptr = mem32[closure_ptr]
    pub(super) fn emit_dynamic_call(
        &mut self,
        func: &Expr,
        args: &[Expr],
        _line: usize,
    ) -> Result<()> {
        // Push args in reverse order (argN-1 first, arg0 last = top of stack)
        let args_clone: Vec<Expr> = args.to_vec();
        for arg in args_clone.iter().rev() {
            self.lower_expr_r0(arg)?;
            self.emit_push(0);
        }
        // Eval func → R0 = closure_ptr
        self.lower_expr_r0(func)?;
        // R0 = closure_ptr; R1 = code_ptr = mem32[closure_ptr]
        self.emit_ldm32i(1, 0);
        // R0 = env_ptr = closure_ptr + 8
        self.emit_mov(2, 8);
        self.emit_addr(0, 2);
        self.emit_push(0); // push env_ptr (becomes param[0] of closure)
        self.emit_jreg(1); // jump to code_ptr; pushes 2-byte return addr
        // Cleanup: pop env_ptr + all args
        let total = (args.len() + 1) * 4;
        if total > 0 {
            self.emit_getsp(1);
            self.emit_mov(2, total as u16);
            self.emit_addr(1, 2);
            self.emit_setsp(1);
        }
        Ok(())
    }

    pub(super) fn lower_expr_r0(&mut self, expr: &Expr) -> Result<()> {
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
            Expr::Var(name, line) => match self.lookup_var(name) {
                None => {
                    return Err(LangError::UndefinedVariable {
                        line: *line,
                        name: name.clone(),
                    });
                }
                Some(VarLoc::Const(v)) => {
                    self.emit_mov_r0_imm(v);
                }
                Some(VarLoc::Global(addr)) => {
                    self.emit_ldm32(0, addr);
                }
                Some(VarLoc::Local(slot)) => {
                    self.emit_load_local(slot);
                }
                Some(VarLoc::Param(idx)) => {
                    self.emit_load_param(idx);
                }
                Some(VarLoc::Upvalue(i)) => {
                    self.emit_load_upval(i);
                }
            },
            Expr::UnOp { op, expr, line: _ } => {
                let inner = expr.as_ref().clone();
                match op {
                    UnOp::Len => {
                        match &inner {
                            Expr::Str(s, _) => {
                                self.emit_mov_r0_imm(s.len() as u32);
                            }
                            _ => {
                                // Table id in R0 → entry count
                                self.lower_expr_r0(&inner)?;
                                self.emit_tlen(0, 0);
                            }
                        }
                    }
                    _ => {
                        self.lower_expr_r0(&inner)?;
                        match op {
                            UnOp::Neg => {
                                self.code.push(OP_NEG);
                                self.code.push(0);
                            }
                            UnOp::Not => {
                                self.emit_mov(1, 0);
                                self.code.push(OP_EQ);
                                self.code.push(0);
                                self.code.push(0);
                                self.code.push(1);
                            }
                            UnOp::Len => unreachable!(),
                        }
                    }
                }
            }
            Expr::BinOp {
                op,
                left,
                right,
                line,
            } => {
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
                                // compile-time concat
                                let combined = format!("{}{}", ls, rs);
                                let ptr = self.intern_string(&combined);
                                self.emit_mov(0, ptr);
                            }
                            _ => {
                                // dynamic: eval both, call __rt_strcat
                                self.lower_expr_r0(&left)?;
                                self.emit_push(0);
                                self.lower_expr_r0(&right)?;
                                self.emit_movr(1, 0);
                                self.emit_pop(0);
                                self.emit_jsr("__rt_strcat");
                            }
                        }
                    }
                    BinOp::Pow => {
                        return Err(LangError::NotImplemented {
                            line: *line,
                            feature: "^".to_string(),
                        });
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
                            BinOp::Mul => {
                                self.code.push(OP_MUL);
                                self.code.push(0);
                                self.code.push(1);
                            }
                            BinOp::Div => {
                                self.code.push(OP_DIV);
                                self.code.push(0);
                                self.code.push(1);
                            }
                            BinOp::Mod => {
                                self.code.push(OP_MOD);
                                self.code.push(0);
                                self.code.push(1);
                            }
                            BinOp::Eq => {
                                self.code.push(OP_EQ);
                                self.code.push(0);
                                self.code.push(0);
                                self.code.push(1);
                            }
                            BinOp::NotEq => {
                                self.code.push(OP_EQ);
                                self.code.push(0);
                                self.code.push(0);
                                self.code.push(1);
                                // invert: R1 = 0; EQ R0, R0, R1
                                self.emit_mov(1, 0);
                                self.code.push(OP_EQ);
                                self.code.push(0);
                                self.code.push(0);
                                self.code.push(1);
                            }
                            BinOp::Lt => {
                                // SLTS R0, R0, R1
                                self.code.push(OP_SLTS);
                                self.code.push(0);
                                self.code.push(0);
                                self.code.push(1);
                            }
                            BinOp::Gt => {
                                // R0 > R1 ↔ R1 < R0 → SLTS R0, R1, R0
                                self.code.push(OP_SLTS);
                                self.code.push(0);
                                self.code.push(1);
                                self.code.push(0);
                            }
                            BinOp::LtEq => {
                                // R0 <= R1 ↔ !(R0 > R1) ↔ !(R1 < R0)
                                self.code.push(OP_SLTS);
                                self.code.push(0);
                                self.code.push(1);
                                self.code.push(0);
                                self.emit_mov(1, 0);
                                self.code.push(OP_EQ);
                                self.code.push(0);
                                self.code.push(0);
                                self.code.push(1);
                            }
                            BinOp::GtEq => {
                                // R0 >= R1 ↔ !(R0 < R1)
                                self.code.push(OP_SLTS);
                                self.code.push(0);
                                self.code.push(0);
                                self.code.push(1);
                                self.emit_mov(1, 0);
                                self.code.push(OP_EQ);
                                self.code.push(0);
                                self.code.push(0);
                                self.code.push(1);
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
                // TNEW → R0 = table id; id survives value evals on the stack.
                self.code.push(OP_TNEW);
                self.code.push(0);
                let mut array_idx: u32 = 1;
                for field in fields {
                    let field = field.clone();
                    match field {
                        TableField::NameField { name, value } => {
                            let key_ptr = self.intern_string(&name);
                            self.emit_push(0); // save table id
                            self.lower_expr_r0(&value)?;
                            self.emit_movr(2, 0); // R2 = val
                            self.emit_mov(1, key_ptr); // R1 = key
                            self.emit_pop(0); // R0 = table id
                            self.emit_tset(0, 1, 2);
                        }
                        TableField::IndexField { key, value } => {
                            self.emit_push(0); // save table id
                            self.lower_expr_r0(&key)?;
                            self.emit_push(0); // save key
                            self.lower_expr_r0(&value)?;
                            self.emit_movr(2, 0); // R2 = val
                            self.emit_pop(1); // R1 = key
                            self.emit_pop(0); // R0 = table id
                            self.emit_tset(0, 1, 2);
                        }
                        TableField::ValueField { value } => {
                            let key = array_idx;
                            array_idx += 1;
                            self.emit_push(0); // save table id
                            self.lower_expr_r0(&value)?;
                            self.emit_movr(2, 0); // R2 = val
                            self.emit_mov32(1, key); // R1 = key
                            self.emit_pop(0); // R0 = table id
                            self.emit_tset(0, 1, 2);
                        }
                    }
                }
                // R0 = table id
            }
            Expr::Func { params, body, line } => {
                let params = params.clone();
                let body = body.clone();
                let line = *line;
                // Free-variable analysis
                let upvals = collect_free_vars(&params, &body, |name| self.lookup_var(name));
                // Emit closure struct: [code_ptr(u32) | n_upvals(u32) | upval[0]..upval[n-1]]
                // Allocate heap: size = 8 + n*4
                let n = upvals.len();
                let alloc_size = (8 + n * 4) as u32;
                // R0 = heap_top (closure_ptr); advance heap_top
                self.emit_ldm32(0, HEAP_TOP_ADDR); // R0 = closure_ptr
                self.emit_push(0); // save closure_ptr
                self.emit_mov32(1, alloc_size);
                self.emit_addr(0, 1);
                self.emit_stm32(HEAP_TOP_ADDR, 0); // heap_top += alloc_size
                self.emit_pop(0); // R0 = closure_ptr

                // Store code_ptr (patched label) at [closure_ptr]
                let fn_label = format!("__closure_{}_{}", self.code.len(), line);
                self.emit_push(0); // save closure_ptr
                self.emit_mov_label(1, &fn_label); // R1 = code_ptr (patched)
                self.emit_pop(0); // R0 = closure_ptr
                self.emit_stm32i(0, 1); // mem32[closure_ptr] = code_ptr
                // Store n_upvals at [closure_ptr+4]
                // Store n_upvals at [closure_ptr+4]: R0 = closure_ptr
                self.emit_push(0); // save closure_ptr
                self.emit_mov32(1, n as u32); // R1 = n
                self.emit_mov(2, 4); // R2 = 4
                self.emit_addr(0, 2); // R0 = closure_ptr+4
                self.emit_stm32i(0, 1); // mem32[closure_ptr+4] = n
                self.emit_pop(0); // R0 = closure_ptr (restore)

                // Store each upval: mem32[closure_ptr + 8 + i*4] = value
                let upvals_clone = upvals.clone();
                for (i, uname) in upvals_clone.iter().enumerate() {
                    // R1 = closure_ptr (saved in R1 above) — but R1 may be clobbered
                    // Use stm32 with known offset from base is not possible without indirect
                    // Use stm32i: need address in a reg
                    // Capture upval value into R0
                    let loc = self.lookup_var(uname).unwrap();
                    match loc {
                        VarLoc::Local(slot) => self.emit_load_local(slot),
                        VarLoc::Param(idx) => self.emit_load_param(idx),
                        VarLoc::Upvalue(ui) => self.emit_load_upval(ui),
                        VarLoc::Global(addr) => self.emit_ldm32(0, addr),
                        VarLoc::Const(v) => self.emit_mov32(0, v),
                    }
                    self.emit_push(0); // save upval value
                    // R0 = closure_ptr (need to reload)
                    self.emit_ldm32(0, HEAP_TOP_ADDR); // current heap_top
                    self.emit_mov32(1, alloc_size);
                    self.emit_subr(0, 1); // R0 = closure_ptr = heap_top - alloc_size
                    self.emit_mov32(1, (8 + i * 4) as u32);
                    self.emit_addr(0, 1); // R0 = &upval[i]
                    self.emit_movr(1, 0); // R1 = address
                    self.emit_pop(0); // R0 = upval value
                    self.emit_stm32i(1, 0); // mem32[&upval[i]] = value
                }

                // Result = closure_ptr: reload
                self.emit_ldm32(0, HEAP_TOP_ADDR);
                self.emit_mov32(1, alloc_size);
                self.emit_subr(0, 1); // R0 = closure_ptr

                // Emit closure body after a JMP to skip it
                let after_label = format!("__closure_after_{}_{}", self.code.len(), line);
                self.emit_jmp(&after_label);
                self.emit_label(&fn_label);
                self.compile_closure_fn(&params, &body, upvals)?;
                self.emit_label(&after_label);
                let _ = line;
            }
            Expr::Index {
                table,
                key,
                line: _,
            } => {
                let key = key.as_ref().clone();
                let table = table.as_ref().clone();
                self.lower_expr_r0(&table)?;
                self.emit_push(0); // save table id (key eval may clobber regs)
                self.lower_expr_r0(&key)?;
                self.emit_movr(1, 0); // R1 = key
                self.emit_pop(0); // R0 = table id
                self.emit_tget(0, 0, 1);
            }
            Expr::Field {
                table,
                name,
                line: _,
            } => {
                let table = table.as_ref().clone();
                let key_ptr = self.intern_string(name);
                self.lower_expr_r0(&table)?;
                self.emit_mov(1, key_ptr);
                self.emit_tget(0, 0, 1);
            }
            Expr::Varargs(line) => {
                return Err(LangError::NotImplemented {
                    line: *line,
                    feature: "... is only supported as the sole init of a local statement"
                        .to_string(),
                });
            }
        }
        Ok(())
    }

    pub(super) fn lower_call(&mut self, func: &Expr, args: &[Expr], line: usize) -> Result<()> {
        // Dynamic calls (Field/Index): eval func to R0 and treat as user-defined function addr
        // Only handle Var (builtin/user) and Field (method dispatch → lookup + call)
        if !matches!(func, Expr::Var(..)) {
            return self.emit_dynamic_call(func, args, line);
        }
        let name = match func {
            Expr::Var(n, _) => n.clone(),
            _ => unreachable!(),
        };

        // User-defined functions and callable variables shadow builtins
        // (Lua-style): a game may freely define its own `add` or `map`.
        if self.fn_names.contains(&name) || self.lookup_var(&name).is_some() {
            return self.emit_named_call(&name, args, line);
        }

        match name.as_str() {
            "cls" => {
                if !args.is_empty() {
                    return Err(LangError::ArgCount {
                        line,
                        name,
                        expected: 0,
                        got: args.len(),
                    });
                }
                self.code.push(OP_CLS);
            }
            "wait" => {
                if !args.is_empty() {
                    return Err(LangError::ArgCount {
                        line,
                        name,
                        expected: 0,
                        got: args.len(),
                    });
                }
                self.code.push(OP_WAIT);
            }
            // ── Input ────────────────────────────────────────────────
            "key" | "btn" => {
                self.check_args(&name, line, args, 1)?;
                self.emit_builtin_op(OP_INR, true, args)?;
            }
            "keyp" | "btnp" => {
                self.check_args(&name, line, args, 1)?;
                self.emit_builtin_op(OP_INPR, true, args)?;
            }
            // ── Sprites / map ────────────────────────────────────────
            "spr" => {
                // spr(id, x, y [, flip]) — sprite by id, optional flip
                // flags (bit 0 = horizontal, bit 1 = vertical).
                if args.len() != 3 && args.len() != 4 {
                    return Err(LangError::ArgCount {
                        line,
                        name,
                        expected: 3,
                        got: args.len(),
                    });
                }
                self.save_fp_if_needed();
                self.stage_args_at(args, 0)?;
                if args.len() == 3 {
                    self.emit_mov(3, 0); // no flip
                }
                self.code.push(OP_SPR);
                self.code.push(0);
                self.code.push(1);
                self.code.push(2);
                self.code.push(3);
                self.restore_fp_if_needed();
            }
            "map" => {
                // map(cel_x, cel_y, sx, sy, w, h)
                self.check_args(&name, line, args, 6)?;
                self.emit_builtin_op(OP_MAPD, false, args)?;
            }
            "mget" => {
                self.check_args(&name, line, args, 2)?;
                self.emit_builtin_op(OP_MGET, true, args)?;
            }
            "mset" => {
                self.check_args(&name, line, args, 3)?;
                self.emit_builtin_op(OP_MSET, false, args)?;
            }
            "fget" => {
                self.check_args(&name, line, args, 1)?;
                self.emit_builtin_op(OP_FGET, true, args)?;
            }
            "fset" => {
                self.check_args(&name, line, args, 2)?;
                self.emit_builtin_op(OP_FSET, false, args)?;
            }
            // ── Camera ───────────────────────────────────────────────
            "camera" => {
                // camera(x, y) or camera() to reset.
                if args.is_empty() {
                    self.emit_mov(0, 0);
                    self.emit_mov(1, 0);
                    self.code.push(OP_POSC);
                    self.code.push(0);
                    self.code.push(1);
                } else {
                    self.check_args(&name, line, args, 2)?;
                    self.emit_builtin_op(OP_POSC, false, args)?;
                }
            }
            // ── Drawing ──────────────────────────────────────────────
            "pal" => {
                self.check_args(&name, line, args, 4)?;
                self.emit_builtin_op(OP_PALR, false, args)?;
            }
            "cls_col" | "fill" => {
                self.check_args(&name, line, args, 1)?;
                self.emit_builtin_op(OP_FILLR, false, args)?;
            }
            "pset" => {
                // pset(x, y, color) — palette color pixel.
                self.check_args(&name, line, args, 3)?;
                self.emit_builtin_op(OP_PSET, false, args)?;
            }
            "line" => {
                self.check_args(&name, line, args, 5)?;
                self.emit_builtin_op(OP_LINE, false, args)?;
            }
            "rect" => {
                self.check_args(&name, line, args, 5)?;
                self.emit_builtin_op(OP_RECT, false, args)?;
            }
            "rectfill" => {
                self.check_args(&name, line, args, 5)?;
                self.emit_builtin_op(OP_RECTF, false, args)?;
            }
            "circ" => {
                self.check_args(&name, line, args, 4)?;
                self.emit_builtin_op(OP_CIRC, false, args)?;
            }
            "circfill" => {
                self.check_args(&name, line, args, 4)?;
                self.emit_builtin_op(OP_CIRCF, false, args)?;
            }
            // ── Audio ────────────────────────────────────────────────
            "sfx" => {
                self.check_args(&name, line, args, 1)?;
                self.emit_builtin_op(OP_SFXR, false, args)?;
            }
            "music" | "mus" => {
                self.check_args(&name, line, args, 1)?;
                self.emit_builtin_op(OP_MUSR, false, args)?;
            }
            "nomusic" | "nomus" => {
                self.check_args(&name, line, args, 0)?;
                self.code.push(OP_NOMUS);
            }
            // ── Text ─────────────────────────────────────────────────
            "print" | "txt" => {
                // print(str, x, y, color)
                // Literal string: TXT opcode with compile-time length.
                // Dynamic expression: TXTZ opcode (null-terminated).
                self.check_args(&name, line, args, 4)?;
                let str_len = Self::literal_str(&args[0]).map(|s| s.len());
                self.save_fp_if_needed();
                self.stage_args_at(args, 0)?; // R0=str, R1=x, R2=y, R3=color
                if let Some(len) = str_len {
                    self.code.push(OP_TXT);
                    self.code.push(1); // Rx
                    self.code.push(2); // Ry
                    self.code.push(3); // Rcolor
                    self.code.push(0); // Rbase
                    self.code.push(len as u8);
                } else {
                    self.code.push(OP_TXTZ);
                    self.code.push(1); // Rx
                    self.code.push(2); // Ry
                    self.code.push(3); // Rcolor
                    self.code.push(0); // Rbase
                }
                self.restore_fp_if_needed();
            }
            "num" => {
                // num(value, x, y, color)
                self.check_args(&name, line, args, 4)?;
                self.save_fp_if_needed();
                self.stage_args_at(args, 0)?; // R0=value, R1=x, R2=y, R3=color
                self.code.push(OP_NUM);
                self.code.push(1); // Rx
                self.code.push(2); // Ry
                self.code.push(3); // Rcolor
                self.code.push(0); // Rvalue
                self.restore_fp_if_needed();
            }
            "sin" | "cos" | "abs" | "flr" | "sqrt" => {
                if args.len() != 1 {
                    return Err(LangError::ArgCount {
                        line,
                        name: name.clone(),
                        expected: 1,
                        got: args.len(),
                    });
                }
                let kind: u8 = match name.as_str() {
                    "sin" => 0,
                    "cos" => 1,
                    "abs" => 2,
                    "flr" => 3,
                    "sqrt" => 4,
                    _ => unreachable!(),
                };
                self.lower_expr_r0(&args[0])?;
                self.emit_movr(1, 0);
                self.code.push(OP_MATH1);
                self.code.push(0); // dest R0
                self.code.push(1); // src R1
                self.code.push(kind);
            }
            "max" | "min" => {
                if args.len() != 2 {
                    return Err(LangError::ArgCount {
                        line,
                        name: name.clone(),
                        expected: 2,
                        got: args.len(),
                    });
                }
                self.lower_expr_r0(&args[0])?;
                self.emit_push(0); // save arg0 — R1 may be clobbered by arg1 eval
                self.lower_expr_r0(&args[1])?; // arg1 → R0
                self.emit_pop(1); // arg0 → R1
                let op = if name == "max" { OP_MAX } else { OP_MIN };
                self.code.push(op);
                self.code.push(0);
                self.code.push(1);
            }
            "rnd" => {
                self.check_args(&name, line, args, 1)?;
                self.emit_builtin_op(OP_RNDR, true, args)?;
            }
            // ── Tables ───────────────────────────────────────────────
            "len" => {
                self.check_args(&name, line, args, 1)?;
                self.emit_builtin_op(OP_TLEN, true, args)?;
            }
            "add" => {
                // add(t, v) — append v at integer key len(t)+1.
                self.check_args(&name, line, args, 2)?;
                self.stage_args_at(args, 0)?; // R0 = table, R1 = value
                self.emit_tlen(2, 0); // R2 = len
                self.emit_mov(4, 1); // R4 = 1 (R3 is FP — leave it alone)
                self.emit_addr(2, 4); // R2 = len + 1
                self.emit_tset(0, 2, 1);
            }
            // ── Strings ──────────────────────────────────────────────
            "sub" => {
                // sub(s, i, j) — 1-based inclusive substring.
                self.check_args(&name, line, args, 3)?;
                self.stage_args_at(args, 0)?; // R0 = ptr, R1 = i, R2 = j
                self.emit_jsr("__rt_substr");
            }
            "strlen" => {
                if args.len() != 1 {
                    return Err(LangError::ArgCount {
                        line,
                        name,
                        expected: 1,
                        got: args.len(),
                    });
                }
                self.lower_expr_r0(&args[0])?;
                self.emit_jsr("__rt_strlen");
            }
            "tostring" => {
                if args.len() != 1 {
                    return Err(LangError::ArgCount {
                        line,
                        name,
                        expected: 1,
                        got: args.len(),
                    });
                }
                self.lower_expr_r0(&args[0])?;
                self.emit_jsr("__rt_tostr");
            }
            _ => {
                self.emit_named_call(&name, args, line)?;
            }
        }
        Ok(())
    }

    // Call a user-defined function or callable variable by name. Runtime
    // values (local/param/upvalue/global holding a closure ptr) dispatch
    // dynamically via JREG; named top-level functions use direct JSR.
    fn emit_named_call(&mut self, name: &str, args: &[Expr], line: usize) -> Result<()> {
        let is_static_fn = self.fn_names.contains(name)
            || matches!(self.lookup_var(name), None | Some(VarLoc::Const(_)));
        if !is_static_fn && let Some(loc) = self.lookup_var(name) {
            match loc {
                VarLoc::Local(_) | VarLoc::Param(_) | VarLoc::Upvalue(_) | VarLoc::Global(_) => {
                    let func_expr = Expr::Var(name.to_string(), line);
                    return self.emit_dynamic_call(&func_expr, args, line);
                }
                VarLoc::Const(_) => {}
            }
        }
        // Static call to top-level named function
        let args_clone: Vec<Expr> = args.to_vec();
        for arg in args_clone.iter().rev() {
            self.lower_expr_r0(arg)?;
            self.emit_push(0);
        }
        if self.variadic_fns.contains(name) {
            // R1 = actual arg count, read by the callee's variadic prologue
            self.emit_mov(1, args.len() as u16);
        }
        self.emit_jsr(name);
        // Clean up args: SP += nargs * 4
        let n = args.len();
        if n > 0 {
            self.emit_getsp(1);
            self.emit_mov(2, (n * 4) as u16);
            self.emit_addr(1, 2);
            self.emit_setsp(1);
        }
        // Result in R0
        Ok(())
    }

    fn check_args(&self, name: &str, line: usize, args: &[Expr], expected: usize) -> Result<()> {
        if args.len() != expected {
            return Err(LangError::ArgCount {
                line,
                name: name.to_string(),
                expected,
                got: args.len(),
            });
        }
        Ok(())
    }

    pub(super) fn literal_str(expr: &Expr) -> Option<String> {
        match expr {
            Expr::Str(s, _) => Some(s.clone()),
            Expr::BinOp {
                op: BinOp::Concat,
                left,
                right,
                ..
            } => {
                let l = Self::literal_str(left)?;
                let r = Self::literal_str(right)?;
                Some(format!("{}{}", l, r))
            }
            _ => None,
        }
    }
}
