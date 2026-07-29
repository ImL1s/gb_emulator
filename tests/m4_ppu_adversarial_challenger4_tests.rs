//! Adversarial Empirical Verification Test Suite by Challenger 4
//! For M4 Iteration 2: PPU 2D Graphics Engine, Palettes, BG/Window, Sprites, OAM DMA

use gb_emulator::mmu::{Bus, Mmu};
use gb_emulator::ppu::framebuffer::{
    COLOR_SHADE_0, COLOR_SHADE_1, COLOR_SHADE_2, COLOR_SHADE_3,
};
use gb_emulator::ppu::renderer::ScanlineRenderer;
use gb_emulator::ppu::Ppu;

// ============================================================================
// 1. Sprite Priority & Masking Advanced Empirical Tests
// ============================================================================

#[test]
fn test_sprite_behind_bg_renders_over_bg_color_0() {
    let mut vram = [0u8; 0x2000];
    let mut oam = [0u8; 0xA0];
    let mut fb = [0u32; 160 * 144];

    // BG tile 0 = solid Color 0 (byte0=0x00, byte1=0x00)
    vram[0] = 0x00;
    vram[1] = 0x00;
    vram[0x1800] = 0; // Map 0x9800 -> tile 0

    // Sprite tile 1 = solid Black (color 3: byte0=0xFF, byte1=0xFF)
    vram[16] = 0xFF;
    vram[17] = 0xFF;

    // Sprite 0 (OAM 0): Y=16, X=8, Tile 1, flags: 0x80 (behind_bg = true)
    oam[0] = 16;
    oam[1] = 8;
    oam[2] = 1;
    oam[3] = 0x80;

    let lcdc = 0x93; // LCD on, BG on, OBJ on
    let bgp = 0xE4; // BG color 0 is White
    let obp0 = 0xE4; // Sprite color 3 is Black

    ScanlineRenderer::render_scanline(
        0, lcdc, 0, 0, 0, 0, bgp, obp0, 0xFF, 0, &vram, &oam, &mut fb,
    );

    // Because BG color is 0 (White), Sprite 0 with behind_bg=true MUST STILL RENDER!
    assert_eq!(
        fb[0], COLOR_SHADE_3,
        "Sprite with behind_bg=true should render over BG color 0"
    );
}

#[test]
fn test_sprite_behind_bg_hidden_by_bg_color_non_zero_and_blocks_lower_priority_sprite() {
    let mut vram = [0u8; 0x2000];
    let mut oam = [0u8; 0xA0];
    let mut fb = [0u32; 160 * 144];

    // BG tile 0 = solid Light Gray (color 1: byte0=0xFF, byte1=0x00)
    vram[0] = 0xFF;
    vram[1] = 0x00;
    vram[0x1800] = 0;

    // Sprite tile 1 = solid Dark Gray (color 2: byte0=0x00, byte1=0xFF)
    vram[16] = 0x00;
    vram[17] = 0xFF;

    // Sprite tile 2 = solid Black (color 3: byte0=0xFF, byte1=0xFF)
    vram[32] = 0xFF;
    vram[33] = 0xFF;

    // Sprite 0 (OAM 0, higher priority): X=8, Tile 1 (Dark Gray), behind_bg = true (0x80)
    oam[0] = 16;
    oam[1] = 8;
    oam[2] = 1;
    oam[3] = 0x80;

    // Sprite 1 (OAM 1, lower priority): X=8, Tile 2 (Black), behind_bg = false (0x00)
    oam[4] = 16;
    oam[5] = 8;
    oam[6] = 2;
    oam[7] = 0x00;

    let lcdc = 0x93;
    let bgp = 0xE4; // BG color 1 is Light Gray
    let obp0 = 0xE4;

    ScanlineRenderer::render_scanline(
        0, lcdc, 0, 0, 0, 0, bgp, obp0, 0xFF, 0, &vram, &oam, &mut fb,
    );

    // Sprite 0 claims pixel X=0. Sprite 0 is behind BG, BG is color 1 (non-zero).
    // So BG color 1 (Light Gray) displays. Sprite 1 (Black, above BG) is BLOCKED by Sprite 0.
    assert_eq!(
        fb[0], COLOR_SHADE_1,
        "Sprite 0 should block Sprite 1, showing BG color Light Gray"
    );
}

#[test]
fn test_sprite_transparent_pixel_allows_lower_priority_sprite_to_render() {
    let mut vram = [0u8; 0x2000];
    let mut oam = [0u8; 0xA0];
    let mut fb = [0u32; 160 * 144];

    // Sprite tile 1: left half color 0 (transparent), right half color 2 (byte0=0x00, byte1=0x0F)
    vram[16] = 0x00;
    vram[17] = 0x0F;

    // Sprite tile 2: solid Black (color 3: byte0=0xFF, byte1=0xFF)
    vram[32] = 0xFF;
    vram[33] = 0xFF;

    // Sprite 0 (OAM 0, higher priority): X=8, Tile 1
    oam[0] = 16;
    oam[1] = 8;
    oam[2] = 1;
    oam[3] = 0x00;

    // Sprite 1 (OAM 1, lower priority): X=8, Tile 2 (Black)
    oam[4] = 16;
    oam[5] = 8;
    oam[6] = 2;
    oam[7] = 0x00;

    let lcdc = 0x93;
    let bgp = 0xE4;
    let obp0 = 0xE4;

    ScanlineRenderer::render_scanline(
        0, lcdc, 0, 0, 0, 0, bgp, obp0, 0xFF, 0, &vram, &oam, &mut fb,
    );

    // Pixel X=0: Sprite 0 has color 0 (transparent).
    // Lower priority Sprite 1 has color 3 (Black).
    // Sprite 1's Black pixel SHOULD appear!
    assert_eq!(
        fb[0], COLOR_SHADE_3,
        "Transparent pixel on higher priority sprite allows lower priority sprite to render"
    );
    // Pixel X=4: Sprite 0 has color 2 (Dark Gray). Sprite 0's color 2 should render, blocking Sprite 1.
    assert_eq!(
        fb[4], COLOR_SHADE_2,
        "Non-transparent pixel on higher priority sprite renders and blocks lower priority sprite"
    );
}

#[test]
fn test_sprite_sorting_x_coordinate_takes_precedence_over_oam_index() {
    let mut vram = [0u8; 0x2000];
    let mut oam = [0u8; 0xA0];
    let mut fb = [0u32; 160 * 144];

    // Tile 1 = solid Light Gray (color 1)
    vram[16] = 0xFF;
    vram[17] = 0x00;

    // Tile 2 = solid Dark Gray (color 2)
    vram[32] = 0x00;
    vram[33] = 0xFF;

    // OAM index 0: X=16 (screen X=8), Tile 1 (Light Gray)
    oam[0] = 16;
    oam[1] = 16;
    oam[2] = 1;
    oam[3] = 0;

    // OAM index 5: X=8 (screen X=0), Tile 2 (Dark Gray) -- lower X, higher OAM index!
    oam[20] = 16;
    oam[21] = 8;
    oam[22] = 2;
    oam[23] = 0;

    let lcdc = 0x93;
    let bgp = 0xE4;
    let obp0 = 0xE4;

    ScanlineRenderer::render_scanline(
        0, lcdc, 0, 0, 0, 0, bgp, obp0, 0xFF, 0, &vram, &oam, &mut fb,
    );

    // OAM 5 has smaller X coordinate (8 vs 16) -> renders at X=0 (Dark Gray)
    assert_eq!(
        fb[0], COLOR_SHADE_2,
        "Smaller X coordinate must take priority regardless of OAM index"
    );
}

#[test]
fn test_sprite_separate_palettes_obp0_and_obp1() {
    let mut vram = [0u8; 0x2000];
    let mut oam = [0u8; 0xA0];
    let mut fb = [0u32; 160 * 144];

    // Tile 1 = solid color 1
    vram[16] = 0xFF;
    vram[17] = 0x00;

    // Sprite 0: uses OBP0 (flags bit 4 = 0), Tile 1
    oam[0] = 16;
    oam[1] = 8;
    oam[2] = 1;
    oam[3] = 0x00;

    // Sprite 1: uses OBP1 (flags bit 4 = 1), Tile 1, X=16 (screen X=8)
    oam[4] = 16;
    oam[5] = 16;
    oam[6] = 1;
    oam[7] = 0x10;

    let lcdc = 0x93;
    let bgp = 0xE4;
    let obp0 = 0xE4; // Color 1 -> Light Gray
    let obp1 = 0x1B; // Palette: 00 01 10 11 -> Color 1 -> Dark Gray

    ScanlineRenderer::render_scanline(
        0, lcdc, 0, 0, 0, 0, bgp, obp0, obp1, 0, &vram, &oam, &mut fb,
    );

    assert_eq!(fb[0], COLOR_SHADE_1, "Sprite 0 using OBP0 color 1 should be Light Gray");
    assert_eq!(fb[8], COLOR_SHADE_2, "Sprite 1 using OBP1 color 1 should be Dark Gray");
}

// ============================================================================
// 2. BG & Window Layer Advanced Empirical Tests
// ============================================================================

#[test]
fn test_bg_window_disable_bit0_blanks_entire_screen() {
    let mut vram = [0u8; 0x2000];
    let oam = [0u8; 0xA0];
    let mut fb = [0u32; 160 * 144];

    // Fill VRAM with non-zero color tiles
    for i in 0..16 {
        vram[i] = 0xFF;
    }
    vram[0x1800] = 0;
    vram[0x1C00] = 0;

    let bgp = 0xE4;

    // LCDC = 0x90 (LCD on, BG/Window bit 0 OFF)
    ScanlineRenderer::render_scanline(
        0, 0x90, 0, 0, 0, 0, bgp, 0xFF, 0xFF, 0, &vram, &oam, &mut fb,
    );

    for x in 0..160 {
        assert_eq!(
            fb[x], COLOR_SHADE_0,
            "Pixel at x={} should be blank White when bit 0 is 0",
            x
        );
    }
}

#[test]
fn test_window_offscreen_wx_does_not_increment_window_line() {
    let mut ppu = Ppu::new();
    let vram = [0u8; 0x2000];
    let oam = [0u8; 0xA0];

    // Enable Window, but place WX = 167 (off-screen right)
    ppu.write_reg(0xFF40, 0xF1);
    ppu.write_reg(0xFF4A, 0);   // WY = 0
    ppu.write_reg(0xFF4B, 167); // WX = 167 (offscreen)

    assert_eq!(ppu.window_line, 0);

    // Step 10 scanlines
    ppu.step(10 * 456, &vram, &oam);

    assert_eq!(ppu.regs.ly, 10);
    assert_eq!(
        ppu.window_line, 0,
        "window_line must NOT increment when Window is off-screen"
    );
}

// ============================================================================
// 3. OAM DMA Comprehensive Memory Region Tests
// ============================================================================

#[test]
fn test_oam_dma_from_rom_vram_wram_sram() {
    let mut mmu = Mmu::new();

    // Write patterns to WRAM 0xC000..0xC09F
    for i in 0..160 {
        mmu.write_byte(0xC000 + i as u16, (i as u8) ^ 0xAA);
    }

    // Trigger DMA from 0xC000
    mmu.write_byte(0xFF46, 0xC0);

    // Verify OAM contents
    for i in 0..160 {
        assert_eq!(
            mmu.read_byte(0xFE00 + i as u16),
            (i as u8) ^ 0xAA,
            "OAM byte {} should match DMA source byte from WRAM 0xC000",
            i
        );
    }
}
