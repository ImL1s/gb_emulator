#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Flag {
    Z = 0b1000_0000, // Bit 7: Zero
    N = 0b0100_0000, // Bit 6: Subtraction
    H = 0b0010_0000, // Bit 5: Half Carry
    C = 0b0001_0000, // Bit 4: Carry
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Registers {
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub f: u8,
    pub sp: u16,
    pub pc: u16,
}

impl Registers {
    /// Creates a default register state matching DMG hardware post-boot state.
    pub fn new() -> Self {
        Self {
            a: 0x01,
            f: 0xB0, // Z=1, N=0, H=1, C=1 (lower 4 bits masked to 0)
            b: 0x00,
            c: 0x13,
            d: 0x00,
            e: 0xD8,
            h: 0x01,
            l: 0x4D,
            sp: 0xFFFE,
            pc: 0x0100,
        }
    }

    /// Set F register value while strictly enforcing lower 4 bits zero invariant (`f & 0xF0`).
    #[inline(always)]
    pub fn set_f(&mut self, val: u8) {
        self.f = val & 0xF0;
    }

    /// Read flag boolean value.
    #[inline(always)]
    pub fn get_flag(&self, flag: Flag) -> bool {
        (self.f & (flag as u8)) != 0
    }

    /// Set or clear flag bit while preserving other flags and enforcing `f & 0xF0`.
    #[inline(always)]
    pub fn set_flag(&mut self, flag: Flag, value: bool) {
        if value {
            self.f |= flag as u8;
        } else {
            self.f &= !(flag as u8);
        }
        self.f &= 0xF0; // Hard guarantee lower 4 bits are 0
    }

    // Direct convenience getters
    #[inline(always)] pub fn flag_z(&self) -> bool { self.get_flag(Flag::Z) }
    #[inline(always)] pub fn flag_n(&self) -> bool { self.get_flag(Flag::N) }
    #[inline(always)] pub fn flag_h(&self) -> bool { self.get_flag(Flag::H) }
    #[inline(always)] pub fn flag_c(&self) -> bool { self.get_flag(Flag::C) }

    // --- 16-bit Register Pair Accessors ---
    #[inline(always)]
    pub fn af(&self) -> u16 {
        ((self.a as u16) << 8) | (self.f as u16)
    }

    #[inline(always)]
    pub fn set_af(&mut self, val: u16) {
        self.a = (val >> 8) as u8;
        self.f = (val as u8) & 0xF0; // Enforce zeroing of lower 4 bits
    }

    #[inline(always)]
    pub fn bc(&self) -> u16 {
        ((self.b as u16) << 8) | (self.c as u16)
    }

    #[inline(always)]
    pub fn set_bc(&mut self, val: u16) {
        self.b = (val >> 8) as u8;
        self.c = val as u8;
    }

    #[inline(always)]
    pub fn de(&self) -> u16 {
        ((self.d as u16) << 8) | (self.e as u16)
    }

    #[inline(always)]
    pub fn set_de(&mut self, val: u16) {
        self.d = (val >> 8) as u8;
        self.e = val as u8;
    }

    #[inline(always)]
    pub fn hl(&self) -> u16 {
        ((self.h as u16) << 8) | (self.l as u16)
    }

    #[inline(always)]
    pub fn set_hl(&mut self, val: u16) {
        self.h = (val >> 8) as u8;
        self.l = val as u8;
    }
}

impl Default for Registers {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_registers() {
        let regs = Registers::new();
        assert_eq!(regs.a, 0x01);
        assert_eq!(regs.f, 0xB0); // Z=1, N=0, H=1, C=1, lower 4 bits 0
        assert_eq!(regs.b, 0x00);
        assert_eq!(regs.c, 0x13);
        assert_eq!(regs.d, 0x00);
        assert_eq!(regs.e, 0xD8);
        assert_eq!(regs.h, 0x01);
        assert_eq!(regs.l, 0x4D);
        assert_eq!(regs.sp, 0xFFFE);
        assert_eq!(regs.pc, 0x0100);
    }

    #[test]
    fn test_lower_4_bits_zero_invariant() {
        let mut regs = Registers::new();

        // 1. set_f with 0xFF
        regs.set_f(0xFF);
        assert_eq!(regs.f, 0xF0);

        // 2. set_af with lower byte containing set bits
        regs.set_af(0x12FF);
        assert_eq!(regs.a, 0x12);
        assert_eq!(regs.f, 0xF0);
        assert_eq!(regs.af(), 0x12F0);

        // 3. set_flag operations
        regs.set_f(0x00);
        regs.set_flag(Flag::Z, true);
        regs.set_flag(Flag::N, true);
        regs.set_flag(Flag::H, true);
        regs.set_flag(Flag::C, true);
        assert_eq!(regs.f, 0xF0);

        regs.set_flag(Flag::Z, false);
        assert_eq!(regs.f, 0x70);
        assert_eq!(regs.f & 0x0F, 0);
    }

    #[test]
    fn test_16bit_accessors() {
        let mut regs = Registers::new();

        regs.set_bc(0x1234);
        assert_eq!(regs.b, 0x12);
        assert_eq!(regs.c, 0x34);
        assert_eq!(regs.bc(), 0x1234);

        regs.set_de(0x5678);
        assert_eq!(regs.d, 0x56);
        assert_eq!(regs.e, 0x78);
        assert_eq!(regs.de(), 0x5678);

        regs.set_hl(0x9ABC);
        assert_eq!(regs.h, 0x9A);
        assert_eq!(regs.l, 0xBC);
        assert_eq!(regs.hl(), 0x9ABC);
    }
}

