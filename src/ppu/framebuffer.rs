//! Framebuffer structures and 4-shade grayscale palette resolution

pub const SCREEN_WIDTH: usize = 160;
pub const SCREEN_HEIGHT: usize = 144;
pub const FRAMEBUFFER_SIZE: usize = SCREEN_WIDTH * SCREEN_HEIGHT;

/// 4-shade Game Boy grayscale colors in RGBA8888 format
pub const COLOR_SHADE_0: u32 = 0xFFFFFFFF; // White
pub const COLOR_SHADE_1: u32 = 0xAAAAAAFF; // Light Gray
pub const COLOR_SHADE_2: u32 = 0x555555FF; // Dark Gray
pub const COLOR_SHADE_3: u32 = 0x000000FF; // Black

pub const PALETTE_LUT: [u32; 4] = [
    COLOR_SHADE_0,
    COLOR_SHADE_1,
    COLOR_SHADE_2,
    COLOR_SHADE_3,
];

pub type FramebufferArray = [u32; FRAMEBUFFER_SIZE];

/// Translate raw 2-bit color index (0..3) using palette register value (BGP/OBP0/OBP1) to RGBA color
pub fn resolve_palette_color(palette_reg: u8, color_index: u8) -> u32 {
    let shade = (palette_reg >> ((color_index & 0x03) * 2)) & 0x03;
    PALETTE_LUT[shade as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_palette_color_resolution() {
        // Standard palette 0xE4 = 11 10 01 00 (3, 2, 1, 0)
        let bgp = 0xE4;
        assert_eq!(resolve_palette_color(bgp, 0), COLOR_SHADE_0);
        assert_eq!(resolve_palette_color(bgp, 1), COLOR_SHADE_1);
        assert_eq!(resolve_palette_color(bgp, 2), COLOR_SHADE_2);
        assert_eq!(resolve_palette_color(bgp, 3), COLOR_SHADE_3);

        // Inverted palette 0x1B = 00 01 10 11 (0, 1, 2, 3)
        let bgp_inv = 0x1B;
        assert_eq!(resolve_palette_color(bgp_inv, 0), COLOR_SHADE_3);
        assert_eq!(resolve_palette_color(bgp_inv, 1), COLOR_SHADE_2);
        assert_eq!(resolve_palette_color(bgp_inv, 2), COLOR_SHADE_1);
        assert_eq!(resolve_palette_color(bgp_inv, 3), COLOR_SHADE_0);
    }
}
