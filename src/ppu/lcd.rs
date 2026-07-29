//! LCD Control, Status, and Position Registers

/// LCD Control Register (0xFF40 - LCDC)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lcdc {
    pub raw: u8,
}

impl Lcdc {
    pub fn new(raw: u8) -> Self {
        Self { raw }
    }

    /// Bit 7: LCD & PPU enable (0 = Off, 1 = On)
    pub fn lcd_enable(&self) -> bool {
        (self.raw & 0x80) != 0
    }

    /// Bit 6: Window Tile Map Display Select (0 = 0x9800-0x9BFF, 1 = 0x9C00-0x9FFF)
    pub fn window_tile_map_base(&self) -> u16 {
        if (self.raw & 0x40) != 0 {
            0x9C00
        } else {
            0x9800
        }
    }

    /// Bit 5: Window Display Enable (0 = Off, 1 = On)
    pub fn window_enable(&self) -> bool {
        (self.raw & 0x20) != 0
    }

    /// Bit 4: BG & Window Tile Data Select (0 = 0x8800-0x97FF [Signed], 1 = 0x8000-0x8FFF [Unsigned])
    /// Returns (base_address, is_signed)
    pub fn bg_window_tile_data_base(&self) -> (u16, bool) {
        if (self.raw & 0x10) != 0 {
            (0x8000, false)
        } else {
            (0x9000, true)
        }
    }

    /// Bit 3: BG Tile Map Display Select (0 = 0x9800-0x9BFF, 1 = 0x9C00-0x9FFF)
    pub fn bg_tile_map_base(&self) -> u16 {
        if (self.raw & 0x08) != 0 {
            0x9C00
        } else {
            0x9800
        }
    }

    /// Bit 2: OBJ (Sprite) Size (0 = 8x8, 1 = 8x16)
    pub fn sprite_height(&self) -> u8 {
        if (self.raw & 0x04) != 0 {
            16
        } else {
            8
        }
    }

    /// Bit 1: OBJ (Sprite) Display Enable (0 = Off, 1 = On)
    pub fn sprite_enable(&self) -> bool {
        (self.raw & 0x02) != 0
    }

    /// Bit 0: BG & Window Display Enable / Priority (0 = Off, 1 = On)
    pub fn bg_window_enable(&self) -> bool {
        (self.raw & 0x01) != 0
    }
}

/// LCD Status Register (0xFF41 - STAT)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stat {
    pub raw: u8,
}

impl Stat {
    pub fn new(raw: u8) -> Self {
        Self { raw: raw | 0x80 }
    }

    /// Bit 6: LYC=LY STAT Interrupt Enable
    pub fn lyc_interrupt_enable(&self) -> bool {
        (self.raw & 0x40) != 0
    }

    /// Bit 5: Mode 2 OAM Search STAT Interrupt Enable
    pub fn oam_interrupt_enable(&self) -> bool {
        (self.raw & 0x20) != 0
    }

    /// Bit 4: Mode 1 VBlank STAT Interrupt Enable
    pub fn vblank_interrupt_enable(&self) -> bool {
        (self.raw & 0x10) != 0
    }

    /// Bit 3: Mode 0 HBlank STAT Interrupt Enable
    pub fn hblank_interrupt_enable(&self) -> bool {
        (self.raw & 0x08) != 0
    }

    /// Bit 2: LYC==LY Coincidence Flag
    pub fn lyc_ly_flag(&self) -> bool {
        (self.raw & 0x04) != 0
    }

    /// Bits 1..0: PPU Mode (00: HBlank, 01: VBlank, 10: OAM, 11: Transfer)
    pub fn mode(&self) -> u8 {
        self.raw & 0x03
    }
}

/// Collection of LCD Hardware I/O Registers (0xFF40..0xFF4B)
#[derive(Debug, Clone)]
pub struct LcdRegs {
    pub lcdc: u8,
    pub stat: u8,
    pub scy: u8,
    pub scx: u8,
    pub ly: u8,
    pub lyc: u8,
    pub dma: u8,
    pub bgp: u8,
    pub obp0: u8,
    pub obp1: u8,
    pub wy: u8,
    pub wx: u8,
}

impl LcdRegs {
    pub fn new() -> Self {
        Self {
            lcdc: 0x91,
            stat: 0x80,
            scy: 0,
            scx: 0,
            ly: 0,
            lyc: 0,
            dma: 0,
            bgp: 0xFC,
            obp0: 0xFF,
            obp1: 0xFF,
            wy: 0,
            wx: 0,
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0xFF40 => self.lcdc,
            0xFF41 => self.stat | 0x80,
            0xFF42 => self.scy,
            0xFF43 => self.scx,
            0xFF44 => self.ly,
            0xFF45 => self.lyc,
            0xFF46 => self.dma,
            0xFF47 => self.bgp,
            0xFF48 => self.obp0,
            0xFF49 => self.obp1,
            0xFF4A => self.wy,
            0xFF4B => self.wx,
            _ => 0xFF,
        }
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        match addr {
            0xFF40 => self.lcdc = val,
            0xFF41 => self.stat = (val & 0x78) | (self.stat & 0x87) | 0x80,
            0xFF42 => self.scy = val,
            0xFF43 => self.scx = val,
            0xFF44 => self.ly = 0,
            0xFF45 => self.lyc = val,
            0xFF46 => self.dma = val,
            0xFF47 => self.bgp = val,
            0xFF48 => self.obp0 = val,
            0xFF49 => self.obp1 = val,
            0xFF4A => self.wy = val,
            0xFF4B => self.wx = val,
            _ => {}
        }
    }
}

impl Default for LcdRegs {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lcdc_bitfield_helpers() {
        let lcdc = Lcdc::new(0x91); // 1001 0001
        assert!(lcdc.lcd_enable());
        assert!(!lcdc.window_enable());
        assert_eq!(lcdc.window_tile_map_base(), 0x9800);
        assert_eq!(lcdc.bg_window_tile_data_base(), (0x8000, false));
        assert_eq!(lcdc.bg_tile_map_base(), 0x9800);
        assert_eq!(lcdc.sprite_height(), 8);
        assert!(!lcdc.sprite_enable());
        assert!(lcdc.bg_window_enable());

        let lcdc_signed = Lcdc::new(0x44); // 0100 0100 -> window 0x9C00, tile data 0x9000 signed, 8x16 sprites
        assert!(!lcdc_signed.lcd_enable());
        assert_eq!(lcdc_signed.window_tile_map_base(), 0x9C00);
        assert_eq!(lcdc_signed.bg_window_tile_data_base(), (0x9000, true));
        assert_eq!(lcdc_signed.sprite_height(), 16);
    }

    #[test]
    fn test_stat_register_masks() {
        let mut regs = LcdRegs::new();
        assert_eq!(regs.read(0xFF41) & 0x80, 0x80);

        regs.write(0xFF41, 0xFF);
        assert_eq!(regs.read(0xFF41) & 0x78, 0x78);
    }
}
