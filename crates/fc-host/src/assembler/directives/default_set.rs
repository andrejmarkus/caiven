use crate::assembler::{Directive, DirectiveSet};

pub fn default_directive_set() -> DirectiveSet {
    let mut set = DirectiveSet::new();

    set.register(Directive {
        name: ".DB",
        size: |args, _| {
            let mut size = 0;
            for arg in args {
                if arg.starts_with('"') && arg.ends_with('"') {
                    size += arg.len() - 2;
                } else {
                    size += 1;
                }
            }
            size
        },
        emit: |args, _, _| {
            let mut bytes = Vec::new();
            for arg in args {
                if arg.starts_with('"') && arg.ends_with('"') {
                    let s = &arg[1..arg.len() - 1];
                    bytes.extend_from_slice(s.as_bytes());
                } else {
                    let val = parse_u8(arg)?;
                    bytes.push(val);
                }
            }
            Ok(bytes)
        },
    });

    set.register(Directive {
        name: ".DW",
        size: |args, _| args.len() * 2,
        emit: |args, labels, _| {
            let mut bytes = Vec::new();
            for arg in args {
                let val = match parse_u16(arg) {
                    Ok(v) => v,
                    Err(_) => *labels
                        .get(*arg)
                        .ok_or_else(|| format!("Unknown label or invalid value: {}", arg))?,
                };
                bytes.extend_from_slice(&val.to_le_bytes());
            }
            Ok(bytes)
        },
    });

    set.register(Directive {
        name: ".DS",
        size: |args, _| {
            if args.is_empty() {
                return 0;
            }
            parse_u16(args[0]).unwrap_or(0) as usize
        },
        emit: |args, _, _| {
            if args.is_empty() {
                return Ok(vec![]);
            }
            let size = parse_u16(args[0])?;
            Ok(vec![0; size as usize])
        },
    });

    set.register(Directive {
        name: ".ORG",
        size: |args, pc| {
            if args.is_empty() {
                return 0;
            }
            let target = parse_u16(args[0]).unwrap_or(pc);
            if target > pc {
                (target - pc) as usize
            } else {
                0
            }
        },
        emit: |args, _, pc| {
            if args.is_empty() {
                return Ok(vec![]);
            }
            let target = parse_u16(args[0])?;
            if target > pc {
                Ok(vec![0; (target - pc) as usize])
            } else {
                Ok(vec![])
            }
        },
    });

    set.register(Directive {
        name: ".FILL",
        size: |args, _| {
            if args.len() < 2 {
                return 0;
            }
            parse_u16(args[0]).unwrap_or(0) as usize
        },
        emit: |args, _, _| {
            if args.len() < 2 {
                return Ok(vec![]);
            }
            let count = parse_u16(args[0])?;
            let value = parse_u8(args[1])?;
            Ok(vec![value; count as usize])
        },
    });

    set
}

fn parse_u8(s: &str) -> Result<u8, String> {
    if s.starts_with('\'') && s.ends_with('\'') && s.len() == 3 {
        return Ok(s.as_bytes()[1]);
    }
    if s.starts_with("0x") {
        u8::from_str_radix(&s[2..], 16).map_err(|e| e.to_string())
    } else if s.starts_with("0b") {
        u8::from_str_radix(&s[2..], 2).map_err(|e| e.to_string())
    } else {
        s.parse::<u8>().map_err(|e| e.to_string())
    }
}

fn parse_u16(s: &str) -> Result<u16, String> {
    if s.starts_with('\'') && s.ends_with('\'') && s.len() == 3 {
        return Ok(s.as_bytes()[1] as u16);
    }
    if s.starts_with("0x") {
        u16::from_str_radix(&s[2..], 16).map_err(|e| e.to_string())
    } else if s.starts_with("0b") {
        u16::from_str_radix(&s[2..], 2).map_err(|e| e.to_string())
    } else {
        s.parse::<u16>().map_err(|e| e.to_string())
    }
}
