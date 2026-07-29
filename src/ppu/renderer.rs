//! Scanline rendering implementation for Background, Window, and Sprites (OBJ)

use super::framebuffer::{resolve_palette_color, FramebufferArray, SCREEN_WIDTH};
use super::lcd::Lcdc;

/// Structure representing a Sprite entry in OAM
#[derive(Debug, Clone, Copy)]
pub struct Sprite {
    pub oam_index: usize,
    pub y_pos: i16,
    pub x_pos: i16,
    pub tile_index: u8,
    pub flags: u8,
}

impl Sprite {
    pub fn priority_behind_bg(&self) -> bool {
        (self.flags & 0x80) != 0
    }

    pub fn y_flip(&self) -> bool {
        (self.flags & 0x40) != 0
    }

    pub fn x_flip(&self) -> bool {
        (self.flags & 0x20) != 0
    }

    pub fn palette_obp1(&self) -> bool {
        (self.flags & 0x10) != 0
    }
}

pub struct ScanlineRenderer;

impl ScanlineRenderer {
    /// Render current scanline `ly` into `framebuffer` and return true if window was rendered
    #[allow(clippy::too_many_arguments)]
    pub fn render_scanline(
        ly: u8,
        lcdc_raw: u8,
        scy: u8,
        scx: u8,
        wy: u8,
        wx: u8,
        bgp: u8,
        obp0: u8,
        obp1: u8,
        window_line: u16,
        vram: &[u8; 0x2000],
        oam: &[u8; 0xA0],
        framebuffer: &mut FramebufferArray,
    ) -> bool {
        let lcdc = Lcdc::new(lcdc_raw);
        if !lcdc.lcd_enable() {
            return false;
        }

        let mut bg_color_indices = [0u8; SCREEN_WIDTH];

        // 1. Background Layer
        Self::render_background(
            ly,
            lcdc,
            scy,
            scx,
            bgp,
            vram,
            framebuffer,
            &mut bg_color_indices,
        );

        // 2. Window Layer
        let window_rendered = Self::render_window(
            ly,
            lcdc,
            wy,
            wx,
            window_line,
            bgp,
            vram,
            framebuffer,
            &mut bg_color_indices,
        );

        // 3. Sprite (OBJ) Layer
        if lcdc.sprite_enable() {
            Self::render_sprites(
                ly,
                lcdc,
                obp0,
                obp1,
                vram,
                oam,
                framebuffer,
                &bg_color_indices,
            );
        }

        window_rendered
    }

    #[allow(clippy::too_many_arguments)]
    fn render_background(
        ly: u8,
        lcdc: Lcdc,
        scy: u8,
        scx: u8,
        bgp: u8,
        vram: &[u8; 0x2000],
        framebuffer: &mut FramebufferArray,
        bg_color_indices: &mut [u8; SCREEN_WIDTH],
    ) {
        if !lcdc.bg_window_enable() {
            // DMG: If Bit 0 is 0, BG is blank (Color 0 / White)
            let color = resolve_palette_color(bgp, 0);
            for x in 0..SCREEN_WIDTH {
                framebuffer[ly as usize * SCREEN_WIDTH + x] = color;
                bg_color_indices[x] = 0;
            }
            return;
        }

        let map_base = lcdc.bg_tile_map_base();
        let (tile_data_base, is_signed) = lcdc.bg_window_tile_data_base();
        let bg_y = (ly as u16 + scy as u16) & 255;

        for x in 0..SCREEN_WIDTH {
            let bg_x = (x as u16 + scx as u16) & 255;

            let tile_row = bg_y / 8;
            let tile_col = bg_x / 8;
            let tile_map_addr = map_base + tile_row * 32 + tile_col;
            let tile_index = vram[(tile_map_addr - 0x8000) as usize];

            let tile_data_addr = if is_signed {
                let signed_idx = tile_index as i8 as i16;
                (tile_data_base as i32 + signed_idx as i32 * 16) as u16
            } else {
                tile_data_base + tile_index as u16 * 16
            };

            let sub_y = bg_y % 8;
            let byte0_addr = (tile_data_addr - 0x8000) as usize + (sub_y * 2) as usize;
            let byte0 = vram[byte0_addr];
            let byte1 = vram[byte0_addr + 1];

            let sub_x = bg_x % 8;
            let bit = 7 - sub_x;
            let color_index = (((byte1 >> bit) & 1) << 1) | ((byte0 >> bit) & 1);

            bg_color_indices[x] = color_index;
            let color = resolve_palette_color(bgp, color_index);
            framebuffer[ly as usize * SCREEN_WIDTH + x] = color;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_window(
        ly: u8,
        lcdc: Lcdc,
        wy: u8,
        wx: u8,
        window_line: u16,
        bgp: u8,
        vram: &[u8; 0x2000],
        framebuffer: &mut FramebufferArray,
        bg_color_indices: &mut [u8; SCREEN_WIDTH],
    ) -> bool {
        if !lcdc.window_enable() || !lcdc.bg_window_enable() || ly < wy || wy > 143 || wx > 166 {
            return false;
        }

        let map_base = lcdc.window_tile_map_base();
        let (tile_data_base, is_signed) = lcdc.bg_window_tile_data_base();
        let win_x_start = if wx >= 7 { wx as i16 - 7 } else { 0 };

        let win_y = window_line;

        for x in win_x_start..SCREEN_WIDTH as i16 {
            let win_x = (x - (wx as i16 - 7)) as u16;

            let tile_row = win_y / 8;
            let tile_col = win_x / 8;
            let tile_map_addr = map_base + tile_row * 32 + tile_col;
            let tile_index = vram[(tile_map_addr - 0x8000) as usize];

            let tile_data_addr = if is_signed {
                let signed_idx = tile_index as i8 as i16;
                (tile_data_base as i32 + signed_idx as i32 * 16) as u16
            } else {
                tile_data_base + tile_index as u16 * 16
            };

            let sub_y = win_y % 8;
            let byte0_addr = (tile_data_addr - 0x8000) as usize + (sub_y * 2) as usize;
            let byte0 = vram[byte0_addr];
            let byte1 = vram[byte0_addr + 1];

            let sub_x = win_x % 8;
            let bit = 7 - sub_x;
            let color_index = (((byte1 >> bit) & 1) << 1) | ((byte0 >> bit) & 1);

            let ux = x as usize;
            bg_color_indices[ux] = color_index;
            let color = resolve_palette_color(bgp, color_index);
            framebuffer[ly as usize * SCREEN_WIDTH + ux] = color;
        }

        true
    }

    #[allow(clippy::too_many_arguments)]
    fn render_sprites(
        ly: u8,
        lcdc: Lcdc,
        obp0: u8,
        obp1: u8,
        vram: &[u8; 0x2000],
        oam: &[u8; 0xA0],
        framebuffer: &mut FramebufferArray,
        bg_color_indices: &[u8; SCREEN_WIDTH],
    ) {
        let sprite_height = lcdc.sprite_height() as i16;

        struct DecodedSprite {
            sprite: Sprite,
            byte0: u8,
            byte1: u8,
        }

        let mut visible_sprites: Vec<DecodedSprite> = Vec::with_capacity(10);

        // Scan OAM for matching sprites (Max 10 per scanline)
        for i in 0..40 {
            let oam_addr = i * 4;
            let y_pos = oam[oam_addr] as i16 - 16;
            let x_pos = oam[oam_addr + 1] as i16 - 8;
            let tile_index = oam[oam_addr + 2];
            let flags = oam[oam_addr + 3];

            let scanline = ly as i16;
            if scanline >= y_pos && scanline < y_pos + sprite_height {
                let sprite = Sprite {
                    oam_index: i,
                    y_pos,
                    x_pos,
                    tile_index,
                    flags,
                };

                let mut line = (ly as i16 - sprite.y_pos) as u16;
                if sprite.y_flip() {
                    line = (sprite_height as u16 - 1) - line;
                }

                let effective_tile_index = if sprite_height == 16 {
                    if line < 8 {
                        sprite.tile_index & 0xFE
                    } else {
                        sprite.tile_index | 0x01
                    }
                } else {
                    sprite.tile_index
                };

                let tile_line = line % 8;
                let tile_data_addr = 0x8000 + effective_tile_index as u16 * 16 + tile_line * 2;
                let byte0 = vram[(tile_data_addr - 0x8000) as usize];
                let byte1 = vram[(tile_data_addr - 0x8000 + 1) as usize];

                visible_sprites.push(DecodedSprite {
                    sprite,
                    byte0,
                    byte1,
                });
                if visible_sprites.len() == 10 {
                    break;
                }
            }
        }

        // DMG priority sorting: smaller X position first, tie-break by lower OAM index
        visible_sprites.sort_by(|a, b| {
            a.sprite
                .x_pos
                .cmp(&b.sprite.x_pos)
                .then_with(|| a.sprite.oam_index.cmp(&b.sprite.oam_index))
        });

        // For each screen column x, evaluate highest priority non-transparent sprite
        for x in 0..SCREEN_WIDTH {
            let sx = x as i16;
            for item in &visible_sprites {
                let s = &item.sprite;
                if sx >= s.x_pos && sx < s.x_pos + 8 {
                    let sub_x = (sx - s.x_pos) as usize;
                    let bit = if s.x_flip() { sub_x } else { 7 - sub_x };
                    let color_index = (((item.byte1 >> bit) & 1) << 1) | ((item.byte0 >> bit) & 1);

                    if color_index != 0 {
                        if !(s.priority_behind_bg() && bg_color_indices[x] != 0) {
                            let palette_reg = if s.palette_obp1() { obp1 } else { obp0 };
                            let color = resolve_palette_color(palette_reg, color_index);
                            framebuffer[ly as usize * SCREEN_WIDTH + x] = color;
                        }
                        break; // Higher priority sprite handles pixel x; ignore lower priority sprites
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ppu::framebuffer::{COLOR_SHADE_0, COLOR_SHADE_3};

    #[test]
    fn test_background_rendering_unsigned() {
        let mut vram = [0u8; 0x2000];
        let oam = [0u8; 0xA0];
        let mut fb = [0u32; 160 * 144];

        // Set tile 0 at 0x8000 to solid color index 3 (black: byte0 = 0xFF, byte1 = 0xFF)
        vram[0] = 0xFF;
        vram[1] = 0xFF;

        // Tile map at 0x9800 points tile (0,0) to tile index 0 and (1,0) to tile index 1
        vram[0x1800] = 0;
        vram[0x1801] = 1;

        let lcdc = 0x91; // LCD on, BG on, Tile map 0x9800, Tile data 0x8000 unsigned
        let bgp = 0xE4; // Standard palette

        ScanlineRenderer::render_scanline(
            0, lcdc, 0, 0, 0, 0, bgp, 0xFF, 0xFF, 0, &vram, &oam, &mut fb,
        );

        // First 8 pixels should be shade 3 (black)
        for x in 0..8 {
            assert_eq!(fb[x], COLOR_SHADE_3);
        }
        // Pixel 8 is tile (1,0) which points to tile index 1 (white)
        assert_eq!(fb[8], COLOR_SHADE_0);
    }

    #[test]
    fn test_background_rendering_signed() {
        let mut vram = [0u8; 0x2000];
        let oam = [0u8; 0xA0];
        let mut fb = [0u32; 160 * 144];

        // Signed mode 0x8800/0x9000: tile index 0 is at 0x9000 (VRAM offset 0x1000)
        vram[0x1000] = 0xFF;
        vram[0x1001] = 0xFF;

        vram[0x1800] = 0; // Map points to tile index 0

        let lcdc = 0x81; // LCD on, BG on, Tile map 0x9800, Tile data 0x9000 signed (bit 4 = 0)
        let bgp = 0xE4;

        ScanlineRenderer::render_scanline(
            0, lcdc, 0, 0, 0, 0, bgp, 0xFF, 0xFF, 0, &vram, &oam, &mut fb,
        );

        for x in 0..8 {
            assert_eq!(fb[x], COLOR_SHADE_3);
        }
    }

    #[test]
    fn test_sprite_rendering_with_transparency_and_priority() {
        let mut vram = [0u8; 0x2000];
        let mut oam = [0u8; 0xA0];
        let mut fb = [0u32; 160 * 144];

        // Sprite tile 1 at 0x8010: left half color 1 (byte0=0xF0, byte1=0x00), right half color 2 (byte0=0x00, byte1=0x0F)
        vram[16] = 0xF0;
        vram[17] = 0x00;

        // OAM entry 0: Y=16 (screen Y=0), X=8 (screen X=0), tile=1, flags=0
        oam[0] = 16;
        oam[1] = 8;
        oam[2] = 1;
        oam[3] = 0;

        let lcdc = 0x93; // LCD on, BG on, Sprites on
        let bgp = 0xE4;
        let obp0 = 0xE4;

        ScanlineRenderer::render_scanline(
            0, lcdc, 0, 0, 0, 0, bgp, obp0, 0xFF, 0, &vram, &oam, &mut fb,
        );

        // Pixels 0..4 should be color index 1 (Light Gray)
        assert_eq!(fb[0], super::super::framebuffer::COLOR_SHADE_1);
    }
}
