#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmFault {
    MemoryOutOfBounds(usize),
    LuaError,
}
