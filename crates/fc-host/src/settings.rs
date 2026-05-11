pub const NAME: &str = "fantasy console";
pub const WIDTH: u32 = 128;
pub const HEIGHT: u32 = 128;
pub const SCREEN_WIDTH: u32 = WIDTH * 4;
pub const SCREEN_HEIGHT: u32 = HEIGHT * 4;
pub const SPRITE_SIZE: u32 = 8;

pub const MEMORY_SIZE: usize = 32 * 1024;
pub const MEMORY_PAGE_SIZE: usize = 6;
pub const MEMORY_ROW_BYTES: usize = 8;
pub const MEMORY_BYTES_PER_PAGE: usize = MEMORY_PAGE_SIZE * MEMORY_ROW_BYTES;
pub const MEMORY_PAGE_COUNT: usize =
    (MEMORY_SIZE + MEMORY_BYTES_PER_PAGE - 1) / MEMORY_BYTES_PER_PAGE;

pub const REGISTER_COUNT: usize = 4;

pub const PALETTE_SIZE: usize = 16;
