#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmFault {
    InvalidOpcode(u8),
    InvalidRegister(usize),
    MemoryOutOfBounds(usize),
    StackOverflow,
    StepLimitExceeded,
    DivisionByZero,
}
