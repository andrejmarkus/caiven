use std::collections::HashMap;

pub type DirectiveHandler =
    fn(args: &[&str], labels: &HashMap<String, u16>, pc: u16) -> Result<Vec<u8>, String>;
pub type DirectiveSizeHandler = fn(args: &[&str], pc: u16) -> usize;

pub struct Directive {
    pub name: &'static str,
    pub size: DirectiveSizeHandler,
    pub emit: DirectiveHandler,
}
