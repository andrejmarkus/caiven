#[derive(Debug, Clone, Copy)]
pub struct VmConfig {
    pub width: u32,
    pub height: u32,
    pub sprite_size: u32,
    pub memory_size: usize,
    pub palette_size: usize,
}

impl Default for VmConfig {
    fn default() -> Self {
        Self {
            width: caiven_core::memory::SCREEN_WIDTH,
            height: caiven_core::memory::SCREEN_HEIGHT,
            sprite_size: caiven_core::memory::SPRITE_SIZE,
            memory_size: caiven_core::memory::RAM_SIZE,
            palette_size: caiven_core::memory::PALETTE_SIZE,
        }
    }
}
