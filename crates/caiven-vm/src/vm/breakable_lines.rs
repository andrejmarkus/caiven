//! Which source lines a breakpoint can stop on, read from the compiled
//! chunk's line info, so Studio can move a breakpoint off a blank line,
//! comment, `end` or function header onto the code that runs.

use mlua::{Lua, LuaOptions, StdLib};
use std::collections::{BTreeMap, BTreeSet};

/// Lines of one Lua source that run code, and where each function body starts.
#[derive(Debug, Default)]
pub struct BreakableLines {
    lines: BTreeSet<usize>,
    /// Function header line → first line of that function's body.
    bodies: BTreeMap<usize, usize>,
}

impl BreakableLines {
    /// Empty when `source` doesn't compile; [`Self::resolve`] then keeps lines.
    pub fn of(source: &str) -> Self {
        let Ok(lua) = Lua::new_with(StdLib::NONE, LuaOptions::default()) else {
            return Self::default();
        };
        let Ok(function) = lua.load(source).into_function() else {
            return Self::default();
        };
        let mut found = Self::default();
        match parse(&function.dump(false), &mut found) {
            Some(()) => found,
            None => Self::default(),
        }
    }

    /// Where a breakpoint set on `line` stops: a function header at the
    /// body's first line, a line without code at the next line with code.
    pub fn resolve(&self, line: usize) -> usize {
        if let Some(&body) = self.bodies.get(&line) {
            return body;
        }
        self.lines.range(line..).next().copied().unwrap_or(line)
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
}

impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Option<&'a [u8]> {
        let (head, rest) = self.bytes.split_at_checked(count)?;
        self.bytes = rest;
        Some(head)
    }

    fn byte(&mut self) -> Option<u8> {
        self.take(1)?.first().copied()
    }

    /// `ldump.c`'s `dumpSize`: 7-bit groups, high bit set on the last one.
    fn size(&mut self) -> Option<usize> {
        let mut value = 0usize;
        loop {
            let byte = self.byte()?;
            value = value.checked_mul(128)? | usize::from(byte & 0x7f);
            if byte & 0x80 != 0 {
                return Some(value);
            }
        }
    }

    fn string(&mut self) -> Option<()> {
        let size = self.size()?;
        self.take(size.saturating_sub(1))?;
        Some(())
    }
}

/// Walks a Lua 5.4 `string.dump` image; `None` on anything unexpected.
fn parse(bytes: &[u8], found: &mut BreakableLines) -> Option<()> {
    let mut reader = Reader { bytes };
    // Signature, version 5.4, official format.
    if reader.take(6)? != b"\x1bLua\x54\x00" {
        return None;
    }
    reader.take(6)?;
    // Sizes of an instruction, a lua_Integer and a lua_Number.
    if reader.take(3)? != [4, 8, 8] {
        return None;
    }
    reader.take(16)?;
    reader.byte()?;
    function(&mut reader, found, true)
}

fn function(reader: &mut Reader, found: &mut BreakableLines, main: bool) -> Option<()> {
    reader.string()?;
    let line_defined = reader.size()?;
    reader.size()?;
    reader.byte()?;
    let vararg = reader.byte()? != 0;
    reader.byte()?;
    let code = reader.size()?;
    reader.take(code.checked_mul(4)?)?;
    for _ in 0..reader.size()? {
        match reader.byte()? {
            // Integer and float constants.
            0x03 | 0x13 => {
                reader.take(8)?;
            }
            // Short and long strings.
            0x04 | 0x14 => reader.string()?,
            _ => {}
        }
    }
    let upvalues = reader.size()?;
    reader.take(upvalues.checked_mul(3)?)?;
    for _ in 0..reader.size()? {
        function(reader, found, false)?;
    }
    let count = reader.size()?;
    let deltas = reader.take(count)?;
    let mut absolute = Vec::new();
    for _ in 0..reader.size()? {
        absolute.push((reader.size()?, reader.size()?));
    }
    for _ in 0..reader.size()? {
        reader.string()?;
        reader.size()?;
        reader.size()?;
    }
    for _ in 0..reader.size()? {
        reader.string()?;
    }

    let mut line = i64::try_from(line_defined).ok()?;
    let mut first: Option<usize> = None;
    for (pc, &delta) in deltas.iter().enumerate() {
        line = if delta == 0x80 {
            let (_, absolute_line) = absolute.iter().find(|(at, _)| *at == pc)?;
            i64::try_from(*absolute_line).ok()?
        } else {
            line + i64::from(delta.cast_signed())
        };
        // A vararg function's first instruction never raises a line event.
        if vararg && pc == 0 {
            continue;
        }
        let Ok(line) = usize::try_from(line) else {
            continue;
        };
        if line > 0 {
            found.lines.insert(line);
            first = Some(first.map_or(line, |first| first.min(line)));
        }
    }
    if !main && let Some(first) = first {
        found.bodies.entry(line_defined).or_insert(first);
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::BreakableLines;

    #[test]
    fn moves_breakpoints_onto_running_code() {
        let lines = BreakableLines::of(
            "local speed = 2\n\n-- move\nlocal function step(x)\n  if x then\n    x = x + speed\n  end\n  return x\nend\n\nfunction _update()\n  step(1)\nend\n",
        );
        assert_eq!(lines.resolve(1), 1, "top-level code");
        assert_eq!(lines.resolve(2), 5, "blank line");
        assert_eq!(lines.resolve(3), 5, "comment");
        assert_eq!(lines.resolve(4), 5, "local function header");
        assert_eq!(lines.resolve(7), 8, "end of an if block");
        assert_eq!(lines.resolve(11), 12, "global function header");
        assert_eq!(lines.resolve(99), 99, "past the end");
        assert_eq!(BreakableLines::of("x = = 1").resolve(3), 3, "syntax error");
    }
}
