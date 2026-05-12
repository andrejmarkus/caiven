pub const NAME: &str = "fantasy console";

pub const MEMORY_PAGE_SIZE: usize = 6;
pub const MEMORY_ROW_BYTES: usize = 8;
pub const MEMORY_BYTES_PER_PAGE: usize = MEMORY_PAGE_SIZE * MEMORY_ROW_BYTES;
pub const MEMORY_PAGE_COUNT: usize = (32 * 1024_usize).div_ceil(MEMORY_BYTES_PER_PAGE);
