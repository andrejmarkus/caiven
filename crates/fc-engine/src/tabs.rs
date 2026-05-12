use crate::app::AppMode;
use fc_core::{Color, Vec2};
use fc_vm::rendering::{font::Font, screen::ScreenLayer, text::draw_text};

pub const TAB_BAR_Y: u32 = 120;
const TAB_H: u32 = 8;
const TAB_W: u32 = 18;

// 7 tabs × 18px = 126px (128px screen, 2px unused at right edge)
const TABS: &[(&str, AppMode)] = &[
    ("RUN", AppMode::Run),
    ("SPR", AppMode::Sprite),
    ("MAP", AppMode::Map),
    ("SFX", AppMode::Sfx),
    ("MUS", AppMode::Music),
    ("PAL", AppMode::Palette),
    ("MET", AppMode::Meta),
];

pub fn draw_tab_bar(layer: &mut ScreenLayer, font: &Font, active: AppMode) {
    let bg_inactive = Color::new_rgb(30, 30, 30);
    let bg_active = Color::new_rgb(180, 180, 180);
    let text_inactive = Color::new_rgb(110, 110, 110);
    let text_active = Color::new_rgb(0, 0, 0);

    for (i, (label, mode)) in TABS.iter().enumerate() {
        let x = i as u32 * TAB_W;
        let is_active = *mode == active;
        let bg = if is_active { bg_active } else { bg_inactive };
        let fg = if is_active { text_active } else { text_inactive };

        for dy in 0..TAB_H {
            for dx in 0..TAB_W {
                layer.set_pixel(Vec2::new(x + dx, TAB_BAR_Y + dy), bg);
            }
        }
        // 3 chars × 4px each = 12px; centered in 18px tab → x+3
        draw_text(font, layer, label, Vec2::new(x + 3, TAB_BAR_Y + 2), fg);
    }
}

/// Returns the AppMode for the tab clicked at screen coordinates (x, y), if any.
pub fn hit_test(x: u32, y: u32) -> Option<AppMode> {
    if y < TAB_BAR_Y || y >= TAB_BAR_Y + TAB_H {
        return None;
    }
    let tab = (x / TAB_W) as usize;
    TABS.get(tab).map(|(_, mode)| *mode)
}
