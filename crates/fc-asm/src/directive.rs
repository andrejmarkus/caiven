use crate::expr::{EvalError, eval_expr};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DirectiveError {
    #[error(transparent)]
    Eval(#[from] EvalError),

    #[error("{directive} value {value} out of range")]
    ValueOutOfRange { directive: &'static str, value: u16 },
}

pub type DirectiveSizeFn = fn(args: &[&str], pc: u16, symbols: &HashMap<String, u16>) -> usize;
pub type DirectiveEmitFn =
    fn(args: &[&str], symbols: &HashMap<String, u16>, pc: u16) -> Result<Vec<u8>, DirectiveError>;

pub struct Directive {
    pub name: &'static str,
    pub size: DirectiveSizeFn,
    pub emit: DirectiveEmitFn,
}

pub struct DirectiveSet {
    directives: Vec<Directive>,
}

impl Default for DirectiveSet {
    fn default() -> Self {
        Self::new()
    }
}

impl DirectiveSet {
    pub fn new() -> Self {
        Self {
            directives: Vec::new(),
        }
    }

    pub fn register(&mut self, directive: Directive) {
        self.directives.push(directive);
    }

    pub fn get_by_name(&self, name: &str) -> Option<&Directive> {
        self.directives
            .iter()
            .find(|d| d.name.eq_ignore_ascii_case(name))
    }
}

pub fn default_directives() -> DirectiveSet {
    let mut set = DirectiveSet::new();

    set.register(Directive {
        name: ".DB",
        size: |args, _, _| {
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
        emit: |args, symbols, _| {
            let mut bytes = Vec::new();
            for arg in args {
                if arg.starts_with('"') && arg.ends_with('"') {
                    bytes.extend_from_slice(&arg.as_bytes()[1..arg.len() - 1]);
                } else {
                    let val = eval_expr(arg, symbols)?;
                    if val > 255 {
                        return Err(DirectiveError::ValueOutOfRange {
                            directive: ".DB",
                            value: val,
                        });
                    }
                    bytes.push(val as u8);
                }
            }
            Ok(bytes)
        },
    });

    set.register(Directive {
        name: ".DW",
        size: |args, _, _| args.len() * 2,
        emit: |args, symbols, _| {
            let mut bytes = Vec::new();
            for arg in args {
                let val = eval_expr(arg, symbols)?;
                bytes.extend_from_slice(&val.to_le_bytes());
            }
            Ok(bytes)
        },
    });

    set.register(Directive {
        name: ".DS",
        size: |args, _, symbols| {
            if args.is_empty() {
                return 0;
            }
            eval_expr(args[0], symbols).unwrap_or(0) as usize
        },
        emit: |args, symbols, _| {
            if args.is_empty() {
                return Ok(vec![]);
            }
            let size = eval_expr(args[0], symbols)?;
            Ok(vec![0; size as usize])
        },
    });

    set.register(Directive {
        name: ".ORG",
        size: |args, pc, symbols| {
            if args.is_empty() {
                return 0;
            }
            let target = eval_expr(args[0], symbols).unwrap_or(pc);
            if target > pc {
                (target - pc) as usize
            } else {
                0
            }
        },
        emit: |args, symbols, pc| {
            if args.is_empty() {
                return Ok(vec![]);
            }
            let target = eval_expr(args[0], symbols)?;
            if target > pc {
                Ok(vec![0; (target - pc) as usize])
            } else {
                Ok(vec![])
            }
        },
    });

    set.register(Directive {
        name: ".FILL",
        size: |args, _, symbols| {
            if args.len() < 2 {
                return 0;
            }
            eval_expr(args[0], symbols).unwrap_or(0) as usize
        },
        emit: |args, symbols, _| {
            if args.len() < 2 {
                return Ok(vec![]);
            }
            let count = eval_expr(args[0], symbols)?;
            let value = eval_expr(args[1], symbols)?;
            if value > 255 {
                return Err(DirectiveError::ValueOutOfRange {
                    directive: ".FILL",
                    value,
                });
            }
            Ok(vec![value as u8; count as usize])
        },
    });

    set
}
