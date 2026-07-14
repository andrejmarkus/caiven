use super::*;

pub(super) fn collect_free_vars<F>(params: &[String], body: &[Stmt], mut lookup: F) -> Vec<String>
where
    F: FnMut(&str) -> Option<VarLoc>,
{
    let mut refs: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut collect = |s: String| {
        refs.insert(s);
    };
    for stmt in body {
        refs_stmt_owned(stmt, &mut collect);
    }
    // Remove closure's own params
    for p in params {
        refs.remove(p.as_str());
    }
    // Keep only names that resolve to a runtime location in outer scope
    let mut upvals: Vec<String> = refs
        .into_iter()
        .filter(|name| {
            matches!(
                lookup(name),
                Some(VarLoc::Local(_)) | Some(VarLoc::Param(_)) | Some(VarLoc::Upvalue(_))
            )
        })
        .collect();
    upvals.sort(); // deterministic order
    upvals
}

pub(super) fn refs_stmt_owned(stmt: &Stmt, out: &mut dyn FnMut(String)) {
    match stmt {
        Stmt::Assign { target, value, .. } => {
            out(target.clone());
            refs_expr_owned(value, out);
        }
        Stmt::Local { inits, .. } => {
            for e in inits {
                refs_expr_owned(e, out);
            }
        }
        Stmt::ExprStmt { expr, .. } => {
            refs_expr_owned(expr, out);
        }
        Stmt::If {
            cond,
            then_block,
            elseif_clauses,
            else_block,
            ..
        } => {
            refs_expr_owned(cond, out);
            for s in then_block {
                refs_stmt_owned(s, out);
            }
            for (e, b) in elseif_clauses {
                refs_expr_owned(e, out);
                for s in b {
                    refs_stmt_owned(s, out);
                }
            }
            if let Some(b) = else_block {
                for s in b {
                    refs_stmt_owned(s, out);
                }
            }
        }
        Stmt::While { cond, body, .. } => {
            refs_expr_owned(cond, out);
            for s in body {
                refs_stmt_owned(s, out);
            }
        }
        Stmt::Repeat { body, cond, .. } => {
            for s in body {
                refs_stmt_owned(s, out);
            }
            refs_expr_owned(cond, out);
        }
        Stmt::NumericFor {
            start,
            stop,
            step,
            body,
            ..
        } => {
            refs_expr_owned(start, out);
            refs_expr_owned(stop, out);
            if let Some(e) = step {
                refs_expr_owned(e, out);
            }
            for s in body {
                refs_stmt_owned(s, out);
            }
        }
        Stmt::Return { values, .. } => {
            for e in values {
                refs_expr_owned(e, out);
            }
        }
        Stmt::Do { body, .. } => {
            for s in body {
                refs_stmt_owned(s, out);
            }
        }
        Stmt::SetField { table, value, .. } => {
            refs_expr_owned(table, out);
            refs_expr_owned(value, out);
        }
        Stmt::SetIndex {
            table, key, value, ..
        } => {
            refs_expr_owned(table, out);
            refs_expr_owned(key, out);
            refs_expr_owned(value, out);
        }
        Stmt::GenericFor { table, body, .. } => {
            refs_expr_owned(table, out);
            for s in body {
                refs_stmt_owned(s, out);
            }
        }
        Stmt::Break { .. } => {}
    }
}

pub(super) fn refs_expr_owned(expr: &Expr, out: &mut dyn FnMut(String)) {
    match expr {
        Expr::Var(name, _) => out(name.clone()),
        Expr::UnOp { expr, .. } => refs_expr_owned(expr, out),
        Expr::BinOp { left, right, .. } => {
            refs_expr_owned(left, out);
            refs_expr_owned(right, out);
        }
        Expr::Call { func, args, .. } => {
            refs_expr_owned(func, out);
            for a in args {
                refs_expr_owned(a, out);
            }
        }
        Expr::Index { table, key, .. } => {
            refs_expr_owned(table, out);
            refs_expr_owned(key, out);
        }
        Expr::Field { table, .. } => refs_expr_owned(table, out),
        Expr::Table { fields, .. } => {
            for f in fields {
                match f {
                    TableField::NameField { value, .. } => refs_expr_owned(value, out),
                    TableField::IndexField { key, value } => {
                        refs_expr_owned(key, out);
                        refs_expr_owned(value, out);
                    }
                    TableField::ValueField { value } => refs_expr_owned(value, out),
                }
            }
        }
        Expr::Func { params, body, .. } => {
            // Body refs minus inner params (they shadow outer)
            let mut inner_refs: std::collections::HashSet<String> =
                std::collections::HashSet::new();
            let mut collect = |s: String| {
                inner_refs.insert(s);
            };
            for s in body {
                refs_stmt_owned(s, &mut collect);
            }
            for p in params {
                inner_refs.remove(p);
            }
            for r in inner_refs {
                out(r);
            }
        }
        _ => {}
    }
}
