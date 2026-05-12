#[derive(Debug, Clone, Copy)]
pub struct VmConfig {
    pub width: u32,
    pub height: u32,
    pub sprite_size: u32,
    pub memory_size: usize,
    pub register_count: usize,
    pub palette_size: usize,
}

impl Default for VmConfig {
    fn default() -> Self {
        Self {
            width: 128,
            height: 128,
            sprite_size: 8,
            memory_size: 32 * 1024,
            register_count: 4,
            palette_size: 16,
        }
    }
}
