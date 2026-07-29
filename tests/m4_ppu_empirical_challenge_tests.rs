//! Empirical Challenge Test Suite for Milestone M4 (PPU 2D Graphics Engine & OAM DMA)

use gb_emulator::mmu::Bus;
use gb_emulator::mmu::Mmu;
use gb_emulator::ppu::framebuffer::{
    resolve_palette_color, COLOR_SHADE_0, COLOR_SHADE_1, COLOR_SHADE_2, COLOR_SHADE_3,
};
use gb_emulator::ppu::renderer::ScanlineRenderer;
use gb_emulator::ppu::{Ppu, PpuMode};

// ============================================================================
// 1. Background Layer Rendering Empirical Tests
// ============================================================================

#[test]
fn challenge_bg_scrolling_scx_scy_wrap() {
    let mut vram = [0u8; 0x2000];
    let oam = [0u8; 0xA0];
    let mut fb = [0u32; 160 * 144];

    // Set tile at 0x8000 to solid color index 2 (Dark Gray: byte0 = 0x00, byte1 = 0xFF across all 8 rows)
    for r in 0..8 {
        vram[r * 2] = 0x00;
        vram[r * 2 + 1] = 0xFF;
    }

    // Tile map at 0x9800: tile at (31, 31) [offset 31*32 + 31 = 1023 (0x3FF)] set to tile index 0
    vram[0x1800 + 1023] = 0;

    let lcdc = 0x91; // LCD on, BG on, Tile map 0x9800, Tile data 0x8000 unsigned
    let bgp = 0xE4; // Standard palette (0->0, 1->1, 2->2, 3->3)

    // With SCY = 250, SCX = 250, scanline LY = 0 maps to BG Y = 250.
    // Screen X = 0..5 maps to BG X = 250..255 (which is tile col 31, sub_x 2..7).
    // Tile col 31, row 31 is selected.
    ScanlineRenderer::render_scanline(
        0, lcdc, 250, 250, 0, 0, bgp, 0xFF, 0xFF, 0, &vram, &oam, &mut fb,
    );

    // Screen X = 0..5 should render tile (31, 31) color 2 (Dark Gray)
    for x in 0..6 {
        assert_eq!(
            fb[x], COLOR_SHADE_2,
            "Pixel at x={} should be Dark Gray from tile (31,31)",
            x
        );
    }
}

#[test]
fn challenge_bg_tile_map_selection_0x9800_vs_0x9c00() {
    let mut vram = [0u8; 0x2000];
    let oam = [0u8; 0xA0];
    let mut fb = [0u32; 160 * 144];

    // Tile 1: solid color 1 (Light Gray: byte0 = 0xFF, byte1 = 0x00)
    vram[16] = 0xFF;
    vram[17] = 0x00;
    // Tile 2: solid color 3 (Black: byte0 = 0xFF, byte1 = 0xFF)
    vram[32] = 0xFF;
    vram[33] = 0xFF;

    // Map 0x9800 (offset 0x1800) tile (0,0) -> tile index 1
    vram[0x1800] = 1;
    // Map 0x9C00 (offset 0x1C00) tile (0,0) -> tile index 2
    vram[0x1C00] = 2;

    let bgp = 0xE4;

    // Test map 0x9800 (LCDC bit 3 = 0)
    let lcdc_9800 = 0x91; // 1001 0001
    ScanlineRenderer::render_scanline(
        0, lcdc_9800, 0, 0, 0, 0, bgp, 0xFF, 0xFF, 0, &vram, &oam, &mut fb,
    );
    assert_eq!(
        fb[0], COLOR_SHADE_1,
        "Map 0x9800 should render tile 1 (Light Gray)"
    );

    // Test map 0x9C00 (LCDC bit 3 = 1)
    let lcdc_9c00 = 0x99; // 1001 1001
    ScanlineRenderer::render_scanline(
        0, lcdc_9c00, 0, 0, 0, 0, bgp, 0xFF, 0xFF, 0, &vram, &oam, &mut fb,
    );
    assert_eq!(
        fb[0], COLOR_SHADE_3,
        "Map 0x9C00 should render tile 2 (Black)"
    );
}

#[test]
fn challenge_bg_tile_data_unsigned_0x8000_vs_signed_0x8800() {
    let mut vram = [0u8; 0x2000];
    let oam = [0u8; 0xA0];
    let mut fb = [0u32; 160 * 144];

    // In unsigned mode (0x8000 base, LCDC bit 4 = 1): tile index 0 is at 0x8000 (VRAM 0x0000).
    // Tile 0 at 0x8000 = color 1 (Light Gray)
    vram[0] = 0xFF;
    vram[1] = 0x00;

    // In signed mode (0x8800/0x9000 base, LCDC bit 4 = 0): tile index 0 is at 0x9000 (VRAM 0x1000).
    // Tile 0 at 0x9000 = color 3 (Black)
    vram[0x1000] = 0xFF;
    vram[0x1001] = 0xFF;

    // Signed tile index -128 (0x80) is at 0x8800 (VRAM 0x0800) -> color 2 (Dark Gray)
    vram[0x0800] = 0x00;
    vram[0x0801] = 0xFF;

    // Set map 0x9800 tile (0,0) to tile index 0
    vram[0x1800] = 0;
    // Set map 0x9800 tile (1,0) to tile index 128 (0x80)
    vram[0x1801] = 0x80;

    let bgp = 0xE4;

    // 1. Unsigned mode (bit 4 = 1, LCDC = 0x91)
    ScanlineRenderer::render_scanline(
        0, 0x91, 0, 0, 0, 0, bgp, 0xFF, 0xFF, 0, &vram, &oam, &mut fb,
    );
    assert_eq!(fb[0], COLOR_SHADE_1, "Unsigned tile 0 should be Light Gray");

    // 2. Signed mode (bit 4 = 0, LCDC = 0x81)
    ScanlineRenderer::render_scanline(
        0, 0x81, 0, 0, 0, 0, bgp, 0xFF, 0xFF, 0, &vram, &oam, &mut fb,
    );
    assert_eq!(
        fb[0], COLOR_SHADE_3,
        "Signed tile 0 (at 0x9000) should be Black"
    );
    assert_eq!(
        fb[8], COLOR_SHADE_2,
        "Signed tile 128 (at 0x8800) should be Dark Gray"
    );
}

#[test]
fn challenge_bg_disabled_dmg_blank_color() {
    let mut vram = [0u8; 0x2000];
    let oam = [0u8; 0xA0];
    let mut fb = [0u32; 160 * 144];

    // Fill VRAM tile 0 with solid Black (color 3)
    vram[0] = 0xFF;
    vram[1] = 0xFF;
    vram[0x1800] = 0;

    let bgp = 0xE4; // Color 0 is White, Color 3 is Black

    // LCDC bit 0 = 0 (BG disabled)
    let lcdc_bg_off = 0x90; // 1001 0000
    ScanlineRenderer::render_scanline(
        0,
        lcdc_bg_off,
        0,
        0,
        0,
        0,
        bgp,
        0xFF,
        0xFF,
        0,
        &vram,
        &oam,
        &mut fb,
    );

    // With BG disabled on DMG, screen should be blank color 0 (White)
    for x in 0..160 {
        assert_eq!(
            fb[x], COLOR_SHADE_0,
            "Pixel at x={} should be blank White when BG is disabled",
            x
        );
    }
}

// ============================================================================
// 2. Window Layer Rendering Empirical Tests
// ============================================================================

#[test]
fn challenge_window_positioning_wx_wy() {
    let mut vram = [0u8; 0x2000];
    let oam = [0u8; 0xA0];
    let mut fb = [0u32; 160 * 144];

    // BG tile (tile 0) = solid White (color 0: byte0=0, byte1=0)
    // Window tile (tile 1) = solid Black (color 3: byte0=0xFF, byte1=0xFF)
    vram[16] = 0xFF;
    vram[17] = 0xFF;

    // Window map 0x9C00 (LCDC bit 6 = 1) -> tile (0,0) set to tile index 1
    vram[0x1C00] = 1;

    let lcdc = 0xF1; // LCD on, Win Map 0x9C00, Win enable, Tile 0x8000, BG Map 0x9800, BG enable
    let bgp = 0xE4;

    // Window set to WX = 15 (screen X = 15 - 7 = 8), WY = 10.
    // Scanline LY = 5 (LY < WY, so window NOT rendered)
    let rendered_win_ly5 = ScanlineRenderer::render_scanline(
        5, lcdc, 0, 0, 10, 15, bgp, 0xFF, 0xFF, 0, &vram, &oam, &mut fb,
    );
    assert!(!rendered_win_ly5, "Window should not render on LY < WY");
    assert_eq!(
        fb[5 * 160 + 8],
        COLOR_SHADE_0,
        "LY=5 pixel 8 should be BG White"
    );

    // Scanline LY = 10 (LY == WY, window rendered starting at X=8)
    let rendered_win_ly10 = ScanlineRenderer::render_scanline(
        10, lcdc, 0, 0, 10, 15, bgp, 0xFF, 0xFF, 0, &vram, &oam, &mut fb,
    );
    assert!(rendered_win_ly10, "Window should render on LY >= WY");
    // Pixels 0..7 should be BG White
    for x in 0..8 {
        assert_eq!(
            fb[10 * 160 + x],
            COLOR_SHADE_0,
            "Pixel x={} should be BG White",
            x
        );
    }
    // Pixels 8..15 should be Window Black
    for x in 8..16 {
        assert_eq!(
            fb[10 * 160 + x],
            COLOR_SHADE_3,
            "Pixel x={} should be Window Black",
            x
        );
    }
}

#[test]
fn challenge_window_wx_less_than_7_offset() {
    let mut vram = [0u8; 0x2000];
    let oam = [0u8; 0xA0];
    let mut fb = [0u32; 160 * 144];

    // Window tile map 0x9800: tile (0,0) = tile 0 (White), tile (1,0) = tile 1 (Black)
    vram[16] = 0xFF;
    vram[17] = 0xFF;
    vram[0x1800] = 0;
    vram[0x1801] = 1;

    let lcdc = 0xB1; // LCD on, Win map 0x9800, Win on, BG on
    let bgp = 0xE4;

    // WX = 0 means window is scrolled 7 pixels off-screen left.
    // Screen X = 0 displays window pixel X = 7 (which is bit 0 of tile 0).
    // Screen X = 1 displays window pixel X = 8 (which is bit 7 of tile 1, color 3 Black).
    ScanlineRenderer::render_scanline(
        0, lcdc, 0, 0, 0, 0, bgp, 0xFF, 0xFF, 0, &vram, &oam, &mut fb,
    );

    assert_eq!(
        fb[0], COLOR_SHADE_0,
        "Screen pixel 0 (win x=7) should be tile 0 (White)"
    );
    assert_eq!(
        fb[1], COLOR_SHADE_3,
        "Screen pixel 1 (win x=8) should be tile 1 (Black)"
    );
}

#[test]
fn challenge_window_internal_line_counter() {
    let mut ppu = Ppu::new();
    let vram = [0u8; 0x2000];
    let oam = [0u8; 0xA0];

    // Enable Window: WX = 7 (screen X=0), WY = 10, LCDC = 0xF1
    ppu.write_reg(0xFF40, 0xF1);
    ppu.write_reg(0xFF4A, 10); // WY = 10
    ppu.write_reg(0xFF4B, 7); // WX = 7

    assert_eq!(ppu.window_line, 0);

    // Step scanlines 0..9 (10 lines * 456 cycles)
    ppu.step(10 * 456, &vram, &oam);
    assert_eq!(ppu.regs.ly, 10);
    // On scanlines 0..9, window was NOT rendered (LY < WY), so window_line should remain 0
    assert_eq!(ppu.window_line, 0, "window_line should be 0 before WY=10");

    // Step scanlines 10..14 (5 lines * 456 cycles)
    ppu.step(5 * 456, &vram, &oam);
    assert_eq!(ppu.regs.ly, 15);
    // Window rendered on scanlines 10, 11, 12, 13, 14 -> window_line should be 5
    assert_eq!(
        ppu.window_line, 5,
        "window_line should be 5 after 5 window lines rendered"
    );

    // Step to VBlank (LY=144)
    ppu.step((144 - 15) * 456, &vram, &oam);
    assert_eq!(ppu.regs.ly, 144);
    assert_eq!(
        ppu.window_line, 0,
        "window_line should reset to 0 on VBlank"
    );
}

// ============================================================================
// 3. Sprite (OBJ) Layer Rendering Empirical Tests
// ============================================================================

#[test]
fn challenge_sprite_8x8_and_8x16_modes() {
    let mut vram = [0u8; 0x2000];
    let mut oam = [0u8; 0xA0];
    let mut fb = [0u32; 160 * 144];

    // Tile 2 (0x8020): solid Light Gray (color 1: byte0=0xFF, byte1=0x00)
    vram[32] = 0xFF;
    vram[33] = 0x00;
    // Tile 3 (0x8030): solid Dark Gray (color 2: byte0=0x00, byte1=0xFF)
    vram[48] = 0x00;
    vram[49] = 0xFF;

    // Sprite 0: Y=16 (screen Y=0), X=8 (screen X=0), tile=3 (odd index for 8x16 test)
    oam[0] = 16;
    oam[1] = 8;
    oam[2] = 3;
    oam[3] = 0;

    let bgp = 0xE4;
    let obp0 = 0xE4;

    // 1. 8x8 Mode (LCDC bit 2 = 0 -> LCDC = 0x93)
    ScanlineRenderer::render_scanline(
        0, 0x93, 0, 0, 0, 0, bgp, obp0, 0xFF, 0, &vram, &oam, &mut fb,
    );
    assert_eq!(
        fb[0], COLOR_SHADE_2,
        "8x8 mode should use tile 3 (Dark Gray)"
    );

    // 2. 8x16 Mode (LCDC bit 2 = 1 -> LCDC = 0x97)
    // In 8x16 mode, tile 3 has bit 0 masked to 0 for top 8x8 half -> uses Tile 2 (Light Gray)!
    ScanlineRenderer::render_scanline(
        0, 0x97, 0, 0, 0, 0, bgp, obp0, 0xFF, 0, &vram, &oam, &mut fb,
    );
    assert_eq!(
        fb[0], COLOR_SHADE_1,
        "8x16 mode top half should use tile 2 (Light Gray)"
    );

    // On scanline LY = 8 (bottom 8x8 half of 8x16 sprite), uses Tile 3 (Dark Gray)!
    ScanlineRenderer::render_scanline(
        8, 0x97, 0, 0, 0, 0, bgp, obp0, 0xFF, 0, &vram, &oam, &mut fb,
    );
    assert_eq!(
        fb[8 * 160],
        COLOR_SHADE_2,
        "8x16 mode bottom half should use tile 3 (Dark Gray)"
    );
}

#[test]
fn challenge_sprite_x_flip_and_y_flip() {
    let mut vram = [0u8; 0x2000];
    let mut oam = [0u8; 0xA0];
    let mut fb = [0u32; 160 * 144];

    // Tile 1 (0x8010): row 0 has left half color 1 (byte0=0xF0, byte1=0x00), row 7 has solid color 3
    vram[16] = 0xF0;
    vram[17] = 0x00;
    vram[16 + 14] = 0xFF;
    vram[16 + 15] = 0xFF;

    // OAM entry 0: Y=16, X=8, tile=1, flags: Y-flip (0x40) | X-flip (0x20)
    oam[0] = 16;
    oam[1] = 8;
    oam[2] = 1;
    oam[3] = 0x60;

    let lcdc = 0x93; // LCD on, BG on, OBJ on
    let bgp = 0xE4;
    let obp0 = 0xE4;

    // Render scanline LY = 0 (top of sprite).
    // Because Y-flip is set, LY=0 renders row 7 of tile (solid Black color 3)!
    ScanlineRenderer::render_scanline(
        0, lcdc, 0, 0, 0, 0, bgp, obp0, 0xFF, 0, &vram, &oam, &mut fb,
    );
    assert_eq!(
        fb[0], COLOR_SHADE_3,
        "Y-flipped row 0 should be tile row 7 (Black)"
    );

    // Render scanline LY = 7 (bottom of sprite).
    // Because Y-flip is set, LY=7 renders row 0 of tile (left half color 1).
    // Because X-flip is set, left half color 1 is flipped to right half (pixels 4..7)!
    ScanlineRenderer::render_scanline(
        7, lcdc, 0, 0, 0, 0, bgp, obp0, 0xFF, 0, &vram, &oam, &mut fb,
    );
    // Pixels 0..3 should be transparent/BG White (since tile row 0 right half is color 0)
    assert_eq!(
        fb[7 * 160 + 0],
        COLOR_SHADE_0,
        "X-flipped left side should be transparent/BG"
    );
    // Pixels 4..7 should be color 1 (Light Gray)
    assert_eq!(
        fb[7 * 160 + 4],
        COLOR_SHADE_1,
        "X-flipped right side should be Light Gray"
    );
}

#[test]
fn challenge_sprite_10_per_line_limit() {
    let mut vram = [0u8; 0x2000];
    let mut oam = [0u8; 0xA0];
    let mut fb = [0u32; 160 * 144];

    // Tile 1 = solid Black (color 3)
    vram[16] = 0xFF;
    vram[17] = 0xFF;

    // Place 15 sprites on scanline LY = 0 (Y=16)
    for i in 0..15 {
        let addr = i * 4;
        oam[addr] = 16; // Y = 0 on screen
        oam[addr + 1] = (i as u8) * 8 + 8; // X = i * 8
        oam[addr + 2] = 1; // Tile 1
        oam[addr + 3] = 0;
    }

    let lcdc = 0x93;
    let bgp = 0xE4;
    let obp0 = 0xE4;

    ScanlineRenderer::render_scanline(
        0, lcdc, 0, 0, 0, 0, bgp, obp0, 0xFF, 0, &vram, &oam, &mut fb,
    );

    // First 10 sprites (x = 0..10*8 = 0..80) should be rendered (Black color 3)
    for i in 0..10 {
        assert_eq!(
            fb[i * 8],
            COLOR_SHADE_3,
            "Sprite {} (x={}) should be rendered",
            i,
            i * 8
        );
    }

    // Sprites 10..14 (x = 80..120) should NOT be rendered (remain BG White color 0)
    for i in 10..15 {
        assert_eq!(
            fb[i * 8],
            COLOR_SHADE_0,
            "Sprite {} (x={}) should be ignored due to 10 sprite/line limit",
            i,
            i * 8
        );
    }
}

#[test]
fn challenge_sprite_priority_dmg_x_and_oam_index() {
    let mut vram = [0u8; 0x2000];
    let mut oam = [0u8; 0xA0];
    let mut fb = [0u32; 160 * 144];

    // Tile 1 = solid Light Gray (color 1)
    vram[16] = 0xFF;
    vram[17] = 0x00;

    // Tile 2 = solid Dark Gray (color 2)
    vram[32] = 0x00;
    vram[33] = 0xFF;

    // Sprite 0 (OAM 0): X=16 (screen X=8), Tile 1 (Light Gray)
    oam[0] = 16;
    oam[1] = 16;
    oam[2] = 1;
    oam[3] = 0;

    // Sprite 1 (OAM 1): X=8 (screen X=0), Tile 2 (Dark Gray) -- smaller X!
    oam[4] = 16;
    oam[5] = 8;
    oam[6] = 2;
    oam[7] = 0;

    let lcdc = 0x93;
    let bgp = 0xE4;
    let obp0 = 0xE4;

    ScanlineRenderer::render_scanline(
        0, lcdc, 0, 0, 0, 0, bgp, obp0, 0xFF, 0, &vram, &oam, &mut fb,
    );

    // Sprite 1 at X=0 has smaller X -> rendered at X=0 (Dark Gray)
    assert_eq!(
        fb[0], COLOR_SHADE_2,
        "Sprite 1 (smaller X) should render at X=0"
    );
    // Sprite 0 at X=8 rendered at X=8 (Light Gray)
    assert_eq!(fb[8], COLOR_SHADE_1, "Sprite 0 should render at X=8");

    // Now test tie-break by OAM index when X positions are equal:
    // Sprite 0: X=8, Tile 1 (Light Gray)
    oam[1] = 8;
    // Sprite 1: X=8, Tile 2 (Dark Gray)
    oam[5] = 8;

    ScanlineRenderer::render_scanline(
        0, lcdc, 0, 0, 0, 0, bgp, obp0, 0xFF, 0, &vram, &oam, &mut fb,
    );

    // On DMG, when X positions are equal, OAM 0 has higher priority than OAM 1.
    // So Sprite 0 (Light Gray) should appear on top!
    assert_eq!(
        fb[0], COLOR_SHADE_1,
        "When X coords are equal, lower OAM index 0 (Light Gray) should win over OAM 1"
    );
}

#[test]
fn challenge_sprite_transparency_color_0() {
    let mut vram = [0u8; 0x2000];
    let mut oam = [0u8; 0xA0];
    let mut fb = [0u32; 160 * 144];

    // Tile 1: left half color 0 (byte0=0x00, byte1=0x00), right half color 3 (byte0=0x0F, byte1=0x0F)
    vram[16] = 0x0F;
    vram[17] = 0x0F;

    // BG tile 0 = solid Dark Gray (color 2: byte0=0x00, byte1=0xFF)
    vram[0] = 0x00;
    vram[1] = 0xFF;
    vram[0x1800] = 0;

    // Sprite 0 at X=0 (OAM 0)
    oam[0] = 16;
    oam[1] = 8;
    oam[2] = 1;
    oam[3] = 0;

    let lcdc = 0x93;
    let bgp = 0xE4; // Color 2 is Dark Gray
    let obp0 = 0xE4;

    ScanlineRenderer::render_scanline(
        0, lcdc, 0, 0, 0, 0, bgp, obp0, 0xFF, 0, &vram, &oam, &mut fb,
    );

    // Pixels 0..3 of sprite are color index 0 -> transparent, so BG color 2 (Dark Gray) shows through!
    assert_eq!(
        fb[0], COLOR_SHADE_2,
        "Sprite color 0 should be transparent and show BG"
    );
    // Pixels 4..7 of sprite are color index 3 -> opaque Black (color 3)
    assert_eq!(
        fb[4], COLOR_SHADE_3,
        "Sprite color 3 should be opaque Black"
    );
}

// ============================================================================
// 4. Palette Color Mapping Empirical Tests
// ============================================================================

#[test]
fn challenge_palette_color_mapping_exhaustive() {
    // Test custom palette: BGP = 0x27 (00 10 01 11)
    // Color 0 -> shade 3 (Black)
    // Color 1 -> shade 1 (Light Gray)
    // Color 2 -> shade 2 (Dark Gray)
    // Color 3 -> shade 0 (White)
    let palette = 0x27;
    assert_eq!(resolve_palette_color(palette, 0), COLOR_SHADE_3);
    assert_eq!(resolve_palette_color(palette, 1), COLOR_SHADE_1);
    assert_eq!(resolve_palette_color(palette, 2), COLOR_SHADE_2);
    assert_eq!(resolve_palette_color(palette, 3), COLOR_SHADE_0);
}

// ============================================================================
// 5. OAM DMA Transfer Empirical Tests
// ============================================================================

#[test]
fn challenge_oam_dma_transfer_all_regions() {
    let mut mmu = Mmu::new();

    // 1. Fill WRAM 0xC100..0xC19F with pattern 0x10..0xAF
    for i in 0..160 {
        mmu.write_byte(0xC100 + i as u16, (i + 0x10) as u8);
    }
    mmu.write_byte(0xFF46, 0xC1); // DMA from 0xC100
    assert_eq!(mmu.read_byte(0xFF46), 0xC1);

    for i in 0..160 {
        assert_eq!(mmu.read_byte(0xFE00 + i as u16), (i + 0x10) as u8);
    }

    // 2. Fill HRAM 0xFF80..0xFFDF with pattern
    for i in 0..127 {
        mmu.write_byte(0xFF80 + i as u16, (i + 0x80) as u8);
    }
    mmu.write_byte(0xFF46, 0xFF); // DMA from 0xFF00
    assert_eq!(mmu.read_byte(0xFE80), 0x80);
}

// ============================================================================
// 6. PPU Mode Timing State Machine & LCD STAT Empirical Tests
// ============================================================================

#[test]
fn challenge_ppu_mode_timing_cycle_counts() {
    let mut ppu = Ppu::new();
    let vram = [0u8; 0x2000];
    let oam = [0u8; 0xA0];

    assert_eq!(ppu.mode, PpuMode::OamSearch);
    assert_eq!(ppu.regs.ly, 0);

    // OAM Search duration is 80 cycles (0..79)
    ppu.step(79, &vram, &oam);
    assert_eq!(ppu.mode, PpuMode::OamSearch);

    ppu.step(1, &vram, &oam);
    assert_eq!(ppu.mode, PpuMode::Transfer); // at 80 cycles

    // Transfer duration is ~172 cycles (80..251)
    ppu.step(171, &vram, &oam);
    assert_eq!(ppu.mode, PpuMode::Transfer);

    ppu.step(1, &vram, &oam);
    assert_eq!(ppu.mode, PpuMode::HBlank); // at 252 cycles

    // HBlank duration is ~204 cycles (252..455)
    ppu.step(203, &vram, &oam);
    assert_eq!(ppu.mode, PpuMode::HBlank);

    ppu.step(1, &vram, &oam);
    assert_eq!(ppu.regs.ly, 1);
    assert_eq!(ppu.mode, PpuMode::OamSearch); // wrapped scanline at 456 cycles
}

#[test]
fn challenge_stat_interrupt_sources() {
    let mut ppu = Ppu::new();
    let vram = [0u8; 0x2000];
    let oam = [0u8; 0xA0];

    // Enable STAT Mode 0 HBlank interrupt (bit 3)
    ppu.write_reg(0xFF41, 0x08);

    // Step to HBlank (252 cycles)
    ppu.step(252, &vram, &oam);
    assert_eq!(ppu.mode, PpuMode::HBlank);
    assert!(ppu.stat_interrupt, "HBlank STAT interrupt should trigger");
}

#[test]
fn challenge_lcd_disable_reset_behavior() {
    let mut ppu = Ppu::new();
    let vram = [0u8; 0x2000];
    let oam = [0u8; 0xA0];

    // Advance 5 scanlines
    ppu.step(5 * 456, &vram, &oam);
    assert_eq!(ppu.regs.ly, 5);

    // Turn off LCD (LCDC bit 7 = 0)
    ppu.write_reg(0xFF40, 0x00);
    assert_eq!(ppu.regs.ly, 0);
    assert_eq!(ppu.mode, PpuMode::HBlank);
    assert_eq!(ppu.scanline_cycles, 0);
    assert_eq!(ppu.window_line, 0);

    // Stepping while disabled should not change LY
    ppu.step(1000, &vram, &oam);
    assert_eq!(ppu.regs.ly, 0);

    // Turn LCD back on (LCDC bit 7 = 1)
    ppu.write_reg(0xFF40, 0x80);
    assert_eq!(ppu.regs.ly, 0);
    assert_eq!(ppu.mode, PpuMode::OamSearch);
}

#[test]
fn challenge_ppu_step_multi_cycle_rendering_skipped() {
    let mut ppu = Ppu::new();
    let mut vram = [0u8; 0x2000];
    let oam = [0u8; 0xA0];

    // Set tile 0 at 0x8000 to solid Black (color 3)
    for i in 0..16 {
        vram[i] = 0xFF;
    }
    vram[0x1800] = 0; // Map 0x9800 tile (0,0) -> tile 0

    // Step 456 cycles all at once for scanline 0
    ppu.step(456, &vram, &oam);

    // Scanline 0 pixel 0 should have been rendered as Black (COLOR_SHADE_3).
    assert_eq!(
        ppu.framebuffer[0], COLOR_SHADE_3,
        "Scanline 0 pixel 0 should be Black when stepping 456 cycles"
    );
}

#[test]
fn challenge_sprite_priority_behind_bg_hides_lower_priority_sprite() {
    let mut vram = [0u8; 0x2000];
    let mut oam = [0u8; 0xA0];
    let mut fb = [0u32; 160 * 144];

    // BG tile 0 = solid Light Gray (color 1: byte0=0xFF, byte1=0x00)
    for r in 0..8 {
        vram[r * 2] = 0xFF;
        vram[r * 2 + 1] = 0x00;
    }
    vram[0x1800] = 0; // Map 0x9800 -> tile 0

    // Sprite tile 1 = solid Dark Gray (color 2: byte0=0x00, byte1=0xFF)
    for r in 0..8 {
        vram[16 + r * 2] = 0x00;
        vram[16 + r * 2 + 1] = 0xFF;
    }

    // Sprite 0 (OAM 0, higher priority): Y=16, X=8, Tile 1, flags: 0x80 (behind BG = true)
    oam[0] = 16;
    oam[1] = 8;
    oam[2] = 1;
    oam[3] = 0x80;

    // Sprite 1 (OAM 1, lower priority): Y=16, X=8, Tile 1, flags: 0x00 (behind BG = false)
    oam[4] = 16;
    oam[5] = 8;
    oam[6] = 1;
    oam[7] = 0x00;

    let lcdc = 0x93; // LCD on, BG on, OBJ on
    let bgp = 0xE4; // BG color 1 is Light Gray
    let obp0 = 0xE4; // Sprite color 2 is Dark Gray

    ScanlineRenderer::render_scanline(
        0, lcdc, 0, 0, 0, 0, bgp, obp0, 0xFF, 0, &vram, &oam, &mut fb,
    );

    // According to Game Boy DMG specs:
    // High-priority Sprite 0 is chosen first.
    // Because Sprite 0 has behind_bg = true AND BG color is non-zero (Light Gray),
    // BG color (Light Gray) is displayed. Low-priority Sprite 1 (Dark Gray) must NOT display.
    assert_eq!(
        fb[0], COLOR_SHADE_1,
        "High priority Sprite 0 with behind_bg=true should hide lower priority Sprite 1, showing BG Light Gray"
    );
}

// ============================================================================
// 7. Multi-Cycle Stepping Empirical Challenge Tests
// ============================================================================

#[test]
fn challenge_multi_cycle_step_equivalence_across_step_sizes() {
    let mut vram = [0u8; 0x2000];
    let oam = [0u8; 0xA0];

    // Set up BG pattern: tile 0 at 0x8000 solid Black (color 3)
    for i in 0..16 {
        vram[i] = 0xFF;
    }
    vram[0x1800] = 0; // BG map tile (0,0) -> tile 0

    // Set up Window pattern: tile 1 at 0x8010 solid Dark Gray (color 2)
    for r in 0..8 {
        vram[16 + r * 2] = 0x00;
        vram[16 + r * 2 + 1] = 0xFF;
    }
    vram[0x1C00] = 1; // Win map tile (0,0) -> tile 1

    let setup_ppu = |lcdc: u8, wy: u8, wx: u8| {
        let mut ppu = Ppu::new();
        ppu.write_reg(0xFF47, 0xE4);
        ppu.write_reg(0xFF40, lcdc);
        ppu.write_reg(0xFF4A, wy);
        ppu.write_reg(0xFF4B, wx);
        ppu
    };

    let lcdc = 0xF1; // LCD on, Win map 0x9C00, Win on, Tile 0x8000, BG map 0x9800, BG on
    let wy = 5;
    let wx = 7;

    // Instance 1: Step 1 cycle at a time
    let mut ppu_1 = setup_ppu(lcdc, wy, wx);
    for _ in 0..(456 * 20) {
        ppu_1.step(1, &vram, &oam);
    }

    // Instance 2: Step 4 cycles at a time (1 M-cycle)
    let mut ppu_4 = setup_ppu(lcdc, wy, wx);
    for _ in 0..(114 * 20) {
        ppu_4.step(4, &vram, &oam);
    }

    // Instance 3: Step 12 cycles at a time (3 M-cycles)
    let mut ppu_12 = setup_ppu(lcdc, wy, wx);
    for _ in 0..(38 * 20) {
        ppu_12.step(12, &vram, &oam);
    }

    // Instance 4: Step 80, 172, 204 cycles per scanline (mode boundaries)
    let mut ppu_modes = setup_ppu(lcdc, wy, wx);
    for _ in 0..20 {
        ppu_modes.step(80, &vram, &oam);
        ppu_modes.step(172, &vram, &oam);
        ppu_modes.step(204, &vram, &oam);
    }

    // Instance 5: Step 456 cycles at a time (1 scanline)
    let mut ppu_456 = setup_ppu(lcdc, wy, wx);
    for _ in 0..20 {
        ppu_456.step(456, &vram, &oam);
    }

    // Instance 6: Step 456 * 20 cycles all at once (20 scanlines)
    let mut ppu_multi = setup_ppu(lcdc, wy, wx);
    ppu_multi.step(456 * 20, &vram, &oam);

    // Verify all instances reach identical PPU state
    assert_eq!(ppu_1.regs.ly, 20);
    assert_eq!(ppu_4.regs.ly, 20);
    assert_eq!(ppu_12.regs.ly, 20);
    assert_eq!(ppu_modes.regs.ly, 20);
    assert_eq!(ppu_456.regs.ly, 20);
    assert_eq!(ppu_multi.regs.ly, 20);

    assert_eq!(ppu_1.window_line, 15);
    assert_eq!(ppu_4.window_line, 15);
    assert_eq!(ppu_12.window_line, 15);
    assert_eq!(ppu_modes.window_line, 15);
    assert_eq!(ppu_456.window_line, 15);
    assert_eq!(ppu_multi.window_line, 15);

    assert_eq!(ppu_1.framebuffer, ppu_4.framebuffer);
    assert_eq!(ppu_1.framebuffer, ppu_12.framebuffer);
    assert_eq!(ppu_1.framebuffer, ppu_modes.framebuffer);
    assert_eq!(ppu_1.framebuffer, ppu_456.framebuffer);
    assert_eq!(ppu_1.framebuffer, ppu_multi.framebuffer);
}

#[test]
fn challenge_multi_cycle_step_full_frame_70224_cycles_and_multiple_frames() {
    let mut vram = [0u8; 0x2000];
    let oam = [0u8; 0xA0];

    // Set tile 0 at 0x8000 to solid Light Gray (color 1)
    for r in 0..8 {
        vram[r * 2] = 0xFF;
        vram[r * 2 + 1] = 0x00;
    }
    vram[0x1800] = 0; // BG map tile (0,0) -> tile 0

    // 1. Single step of 70,224 cycles (1 full frame)
    let mut ppu_single = Ppu::new();
    ppu_single.write_reg(0xFF47, 0xE4);
    ppu_single.write_reg(0xFF40, 0x91); // LCD on, BG on
    ppu_single.step(70224, &vram, &oam);

    assert_eq!(ppu_single.regs.ly, 0);
    assert_eq!(ppu_single.mode, PpuMode::OamSearch);
    assert!(ppu_single.frame_ready);
    assert!(ppu_single.vblank_interrupt);

    // Framebuffer for all 160x144 pixels should be Light Gray (COLOR_SHADE_1)
    for y in 0..144 {
        for x in 0..160 {
            assert_eq!(
                ppu_single.framebuffer[y * 160 + x],
                COLOR_SHADE_1,
                "Pixel ({}, {}) should be Light Gray after full frame single step",
                x,
                y
            );
        }
    }

    // 2. Single step of 70,224 * 3 cycles (3 full frames)
    let mut ppu_3frames = Ppu::new();
    ppu_3frames.write_reg(0xFF47, 0xE4);
    ppu_3frames.write_reg(0xFF40, 0x91);
    ppu_3frames.step(70224 * 3, &vram, &oam);

    assert_eq!(ppu_3frames.regs.ly, 0);
    assert_eq!(ppu_3frames.mode, PpuMode::OamSearch);
    assert!(ppu_3frames.frame_ready);
    assert_eq!(ppu_single.framebuffer, ppu_3frames.framebuffer);
}

#[test]
fn challenge_multi_cycle_step_arbitrary_odd_step_sizes() {
    let mut vram = [0u8; 0x2000];
    let oam = [0u8; 0xA0];

    // Set tile 0 at 0x8000 to solid Dark Gray (color 2)
    for r in 0..8 {
        vram[r * 2] = 0x00;
        vram[r * 2 + 1] = 0xFF;
    }
    vram[0x1800] = 0;

    let mut ppu = Ppu::new();
    ppu.write_reg(0xFF47, 0xE4);
    ppu.write_reg(0xFF40, 0x91);

    // Sequence of irregular step sizes that sum to 70,224
    let step_sizes = [
        3, 7, 11, 13, 23, 53, 97, 101, 199, 443, 457, 1000, 7777, 15000, 20000, 25040,
    ];
    let total_cycles: u32 = step_sizes.iter().sum();
    assert_eq!(total_cycles, 70224, "Step sizes must total 70224 cycles");

    for &step_sz in &step_sizes {
        ppu.step(step_sz, &vram, &oam);
    }

    assert_eq!(ppu.regs.ly, 0);
    assert_eq!(ppu.mode, PpuMode::OamSearch);
    assert!(ppu.frame_ready);

    // Verify all 160x144 pixels were rendered as Dark Gray
    for y in 0..144 {
        for x in 0..160 {
            assert_eq!(
                ppu.framebuffer[y * 160 + x],
                COLOR_SHADE_2,
                "Pixel ({}, {}) should be Dark Gray",
                x,
                y
            );
        }
    }
}

#[test]
fn challenge_multi_cycle_step_lyc_stat_interrupt_triggering() {
    let mut ppu = Ppu::new();
    let vram = [0u8; 0x2000];
    let oam = [0u8; 0xA0];

    // Set LYC = 45 and enable STAT LYC interrupt (bit 6)
    ppu.write_reg(0xFF45, 45);
    ppu.write_reg(0xFF41, 0x40);

    // Step up to scanline 44 (44 * 456 cycles)
    ppu.step(44 * 456, &vram, &oam);
    assert_eq!(ppu.regs.ly, 44);
    assert_eq!(ppu.read_reg(0xFF41) & 0x04, 0); // STAT bit 2 (LYC==LY) false
    assert!(
        !ppu.stat_interrupt,
        "STAT interrupt should not trigger before LYC"
    );

    // Clear any previous STAT interrupt flag
    ppu.stat_interrupt = false;

    // Step 456 cycles in ONE call to advance from LY=44 to LY=45
    ppu.step(456, &vram, &oam);
    assert_eq!(ppu.regs.ly, 45);
    assert_ne!(
        ppu.read_reg(0xFF41) & 0x04,
        0,
        "STAT bit 2 (LYC==LY) should be set at LY=45"
    );
    assert!(
        ppu.stat_interrupt,
        "STAT interrupt should fire when stepping through LYC=45"
    );
}

#[test]
fn challenge_multi_cycle_step_window_rendering_and_line_counter() {
    let mut ppu = Ppu::new();
    let mut vram = [0u8; 0x2000];
    let oam = [0u8; 0xA0];

    // Tile 1 at 0x8010 = solid Black (color 3)
    for r in 0..8 {
        vram[16 + r * 2] = 0xFF;
        vram[16 + r * 2 + 1] = 0xFF;
    }
    // Fill window tile map 0x9C00 (offset 0x1C00..0x2000) with tile 1
    for i in 0x1C00..0x2000 {
        vram[i] = 1;
    }

    // Enable LCD, BG, Window (Win map 0x9C00, Tile data 0x8000)
    ppu.write_reg(0xFF47, 0xE4);
    ppu.write_reg(0xFF40, 0xF1);
    ppu.write_reg(0xFF4A, 10); // WY = 10
    ppu.write_reg(0xFF4B, 7); // WX = 7 (Screen X = 0)

    // Step 456 * 10 cycles (reaches LY=10)
    ppu.step(456 * 10, &vram, &oam);
    assert_eq!(ppu.regs.ly, 10);
    assert_eq!(ppu.window_line, 0);

    // Step 456 * 50 cycles (reaches LY=60, 50 window lines rendered)
    ppu.step(456 * 50, &vram, &oam);
    assert_eq!(ppu.regs.ly, 60);
    assert_eq!(ppu.window_line, 50);

    // Verify framebuffer scanlines 10..59 contain Window Black pixels
    for y in 10..60 {
        assert_eq!(
            ppu.framebuffer[y * 160],
            COLOR_SHADE_3,
            "Scanline {} pixel 0 should be Window Black",
            y
        );
    }

    // Step remaining scanlines to reach LY=144 (VBlank)
    ppu.step(456 * (144 - 60), &vram, &oam);
    assert_eq!(ppu.regs.ly, 144);
    assert_eq!(
        ppu.window_line, 0,
        "window_line should reset to 0 at VBlank"
    );
}
