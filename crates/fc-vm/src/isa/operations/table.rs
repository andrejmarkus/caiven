//! Native table instructions: TNEW, TGET, TSET, TLEN, TIDX.
//! Tables live outside guest RAM so they can grow without a fixed capacity.
//! A table value is an id; 0 is "no table". Reads through an invalid id
//! yield 0 and writes are dropped.

use crate::vm::{ExecutionContext, VmFault};

/// Key reported by TIDX past the end of a table; the compiler's generic-for
/// loops terminate on it.
pub const TABLE_ITER_END: u32 = 0xFFFF_FFFF;

fn table_index(id: u32) -> Option<usize> {
    (id != 0).then(|| id as usize - 1)
}

pub fn table_new(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let rdest = ctx.read_register_index()?;
    ctx.tables.push(Vec::new());
    ctx.cpu.set_register(rdest, ctx.tables.len() as u32);
    Ok(())
}

pub fn table_get(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let rdest = ctx.read_register_index()?;
    let id = ctx.read_register_value()?;
    let key = ctx.read_register_value()?;

    let value = table_index(id)
        .and_then(|i| ctx.tables.get(i))
        .and_then(|t| t.iter().find(|(k, _)| *k == key))
        .map(|(_, v)| *v)
        .unwrap_or(0);
    ctx.cpu.set_register(rdest, value);
    Ok(())
}

pub fn table_set(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let id = ctx.read_register_value()?;
    let key = ctx.read_register_value()?;
    let val = ctx.read_register_value()?;

    if let Some(table) = table_index(id).and_then(|i| ctx.tables.get_mut(i)) {
        if let Some(entry) = table.iter_mut().find(|(k, _)| *k == key) {
            entry.1 = val;
        } else {
            table.push((key, val));
        }
    }
    Ok(())
}

pub fn table_len(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let rdest = ctx.read_register_index()?;
    let id = ctx.read_register_value()?;
    let len = table_index(id)
        .and_then(|i| ctx.tables.get(i))
        .map(|t| t.len() as u32)
        .unwrap_or(0);
    ctx.cpu.set_register(rdest, len);
    Ok(())
}

pub fn table_entry_at(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let rkey = ctx.read_register_index()?;
    let rval = ctx.read_register_index()?;
    let id = ctx.read_register_value()?;
    let idx = ctx.read_register_value()? as usize;

    let entry = table_index(id)
        .and_then(|i| ctx.tables.get(i))
        .and_then(|t| t.get(idx))
        .copied();
    let (key, val) = entry.unwrap_or((TABLE_ITER_END, 0));
    ctx.cpu.set_register(rkey, key);
    ctx.cpu.set_register(rval, val);
    Ok(())
}
