pub mod framebuffer;
pub mod lcd;
pub mod renderer;

pub use framebuffer::{
    FramebufferArray, COLOR_SHADE_0, COLOR_SHADE_1, COLOR_SHADE_2, COLOR_SHADE_3, SCREEN_HEIGHT,
    SCREEN_WIDTH,
};
pub use lcd::{LcdRegs, Lcdc, Stat};
pub use renderer::ScanlineRenderer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PpuMode {
    HBlank = 0,
    VBlank = 1,
    OamSearch = 2,
    Transfer = 3,
}

pub struct Ppu {
    pub framebuffer: [u32; 160 * 144],
    pub vblank_interrupt: bool,
    pub stat_interrupt: bool,

    pub mode: PpuMode,
    pub scanline_cycles: u32,
    pub window_line: u16,
    pub line_rendered: bool,

    pub regs: LcdRegs,
    pub prev_stat_line: bool,
    pub frame_ready: bool,
}

impl Ppu {
    pub fn new() -> Self {
        let regs = LcdRegs::new();
        let mut ppu = Self {
            framebuffer: [COLOR_SHADE_0; 160 * 144],
            vblank_interrupt: false,
            stat_interrupt: false,
            mode: PpuMode::OamSearch,
            scanline_cycles: 0,
            window_line: 0,
            line_rendered: false,
            regs,
            prev_stat_line: false,
            frame_ready: false,
        };
        ppu.update_lyc_compare();
        ppu
    }

    /// Step PPU by specified T-cycles.
    pub fn step(&mut self, cycles: u32, vram: &[u8; 0x2000], oam: &[u8; 0xA0]) {
        if !Lcdc::new(self.regs.lcdc).lcd_enable() {
            return;
        }

        for _ in 0..cycles {
            self.scanline_cycles += 1;

            if self.regs.ly < 144 {
                if self.scanline_cycles >= 80 && !self.line_rendered {
                    let rendered_win = ScanlineRenderer::render_scanline(
                        self.regs.ly,
                        self.regs.lcdc,
                        self.regs.scy,
                        self.regs.scx,
                        self.regs.wy,
                        self.regs.wx,
                        self.regs.bgp,
                        self.regs.obp0,
                        self.regs.obp1,
                        self.window_line,
                        vram,
                        oam,
                        &mut self.framebuffer,
                    );
                    if rendered_win {
                        self.window_line += 1;
                    }
                    self.line_rendered = true;
                }

                let target_mode = if self.scanline_cycles < 80 {
                    PpuMode::OamSearch
                } else if self.scanline_cycles < 252 {
                    PpuMode::Transfer
                } else {
                    PpuMode::HBlank
                };

                if self.mode != target_mode {
                    self.set_mode(target_mode);
                }
            }

            if self.scanline_cycles >= 456 {
                self.scanline_cycles = 0;
                self.regs.ly = (self.regs.ly + 1) % 154;
                self.line_rendered = false;

                self.update_lyc_compare();

                if self.regs.ly == 144 {
                    self.set_mode(PpuMode::VBlank);
                    self.vblank_interrupt = true;
                    self.frame_ready = true;
                    self.window_line = 0;
                } else if self.regs.ly == 0 {
                    self.set_mode(PpuMode::OamSearch);
                    self.window_line = 0;
                } else if self.regs.ly < 144 {
                    self.set_mode(PpuMode::OamSearch);
                }
            }
        }
    }

    #[inline]
    fn set_mode(&mut self, new_mode: PpuMode) {
        self.mode = new_mode;
        self.regs.stat = (self.regs.stat & 0xFC) | (new_mode as u8);
        self.check_stat_interrupt();
    }

    #[inline]
    fn update_lyc_compare(&mut self) {
        if self.regs.ly == self.regs.lyc {
            self.regs.stat |= 0x04;
        } else {
            self.regs.stat &= !0x04;
        }
        self.check_stat_interrupt();
    }

    pub fn check_stat_interrupt(&mut self) {
        if !Lcdc::new(self.regs.lcdc).lcd_enable() {
            self.prev_stat_line = false;
            return;
        }

        let stat = Stat::new(self.regs.stat);
        let stat_line = (stat.lyc_interrupt_enable() && stat.lyc_ly_flag())
            || (stat.oam_interrupt_enable() && self.mode == PpuMode::OamSearch)
            || (stat.vblank_interrupt_enable() && self.mode == PpuMode::VBlank)
            || (stat.hblank_interrupt_enable() && self.mode == PpuMode::HBlank);

        if !self.prev_stat_line && stat_line {
            self.stat_interrupt = true;
        }
        self.prev_stat_line = stat_line;
    }

    pub fn read_reg(&self, addr: u16) -> u8 {
        self.regs.read(addr)
    }

    pub fn write_reg(&mut self, addr: u16, val: u8) {
        match addr {
            0xFF40 => {
                let old_enable = Lcdc::new(self.regs.lcdc).lcd_enable();
                let new_enable = Lcdc::new(val).lcd_enable();
                self.regs.write(0xFF40, val);

                if old_enable && !new_enable {
                    self.regs.ly = 0;
                    self.scanline_cycles = 0;
                    self.window_line = 0;
                    self.mode = PpuMode::HBlank;
                    self.regs.stat &= 0xFC;
                    self.prev_stat_line = false;
                } else if !old_enable && new_enable {
                    self.regs.ly = 0;
                    self.scanline_cycles = 0;
                    self.window_line = 0;
                    self.set_mode(PpuMode::OamSearch);
                    self.update_lyc_compare();
                }
            }
            0xFF41 => {
                self.regs.write(0xFF41, val);
                self.check_stat_interrupt();
            }
            0xFF44 => {
                self.regs.write(0xFF44, val);
                if Lcdc::new(self.regs.lcdc).lcd_enable() {
                    self.update_lyc_compare();
                }
            }
            0xFF45 => {
                self.regs.write(0xFF45, val);
                if Lcdc::new(self.regs.lcdc).lcd_enable() {
                    self.update_lyc_compare();
                }
            }
            _ => self.regs.write(addr, val),
        }
    }
}

impl Default for Ppu {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ppu_mode_transitions_and_cycle_counts() {
        let mut ppu = Ppu::new();
        let vram = [0u8; 0x2000];
        let oam = [0u8; 0xA0];

        assert_eq!(ppu.mode, PpuMode::OamSearch);
        assert_eq!(ppu.regs.ly, 0);

        // Step 80 cycles -> enter Transfer
        ppu.step(80, &vram, &oam);
        assert_eq!(ppu.mode, PpuMode::Transfer);

        // Step 172 cycles (total 252) -> enter HBlank
        ppu.step(172, &vram, &oam);
        assert_eq!(ppu.mode, PpuMode::HBlank);

        // Step remaining 204 cycles (total 456) -> LY=1, mode=OamSearch
        ppu.step(204, &vram, &oam);
        assert_eq!(ppu.regs.ly, 1);
        assert_eq!(ppu.mode, PpuMode::OamSearch);
    }

    #[test]
    fn test_vblank_interrupt_on_scanline_144() {
        let mut ppu = Ppu::new();
        let vram = [0u8; 0x2000];
        let oam = [0u8; 0xA0];

        // Step 144 lines (144 * 456 cycles)
        ppu.step(144 * 456, &vram, &oam);

        assert_eq!(ppu.regs.ly, 144);
        assert_eq!(ppu.mode, PpuMode::VBlank);
        assert!(ppu.vblank_interrupt);
        assert!(ppu.frame_ready);
    }

    #[test]
    fn test_full_frame_timing_70224_cycles() {
        let mut ppu = Ppu::new();
        let vram = [0u8; 0x2000];
        let oam = [0u8; 0xA0];

        // Step full frame 70,224 cycles (154 * 456)
        ppu.step(70224, &vram, &oam);

        assert_eq!(ppu.regs.ly, 0);
        assert_eq!(ppu.mode, PpuMode::OamSearch);
        assert!(ppu.frame_ready);
    }

    #[test]
    fn test_lyc_coincidence_flag_and_stat_interrupt() {
        let mut ppu = Ppu::new();
        let vram = [0u8; 0x2000];
        let oam = [0u8; 0xA0];

        // Set LYC = 5, enable STAT LYC interrupt (bit 6)
        ppu.write_reg(0xFF45, 5);
        ppu.write_reg(0xFF41, 0x40);

        // Step 5 scanlines
        ppu.step(5 * 456, &vram, &oam);

        assert_eq!(ppu.regs.ly, 5);
        assert_ne!(ppu.read_reg(0xFF41) & 0x04, 0); // STAT bit 2 (LYC==LY) set
        assert!(ppu.stat_interrupt);
    }

    #[test]
    fn test_lcd_disable_resets_ppu_state() {
        let mut ppu = Ppu::new();
        let vram = [0u8; 0x2000];
        let oam = [0u8; 0xA0];

        ppu.step(10 * 456, &vram, &oam);
        assert_eq!(ppu.regs.ly, 10);

        // Turn off LCD
        ppu.write_reg(0xFF40, 0x01); // Bit 7 off
        assert_eq!(ppu.regs.ly, 0);
        assert_eq!(ppu.mode, PpuMode::HBlank);

        ppu.step(1000, &vram, &oam);
        assert_eq!(ppu.regs.ly, 0); // Does not advance while off
    }
}
