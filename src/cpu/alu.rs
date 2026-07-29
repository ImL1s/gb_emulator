use crate::cpu::registers::Flag;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AluResult<T> {
    pub value: T,
    pub flags: u8,
}

impl<T> AluResult<T> {
    pub fn new(value: T, flags: u8) -> Self {
        Self {
            value,
            flags: flags & 0xF0,
        }
    }
}

pub struct Alu;

impl Alu {
    pub fn add_8(a: u8, b: u8) -> AluResult<u8> {
        let res = a.wrapping_add(b);
        let z = res == 0;
        let n = false;
        let h = (a & 0x0F) + (b & 0x0F) > 0x0F;
        let c = (a as u16) + (b as u16) > 0xFF;

        let mut flags = 0u8;
        if z {
            flags |= Flag::Z as u8;
        }
        if n {
            flags |= Flag::N as u8;
        }
        if h {
            flags |= Flag::H as u8;
        }
        if c {
            flags |= Flag::C as u8;
        }

        AluResult::new(res, flags)
    }

    pub fn adc_8(a: u8, b: u8, carry_in: bool) -> AluResult<u8> {
        let cin = if carry_in { 1u16 } else { 0u16 };
        let sum = (a as u16) + (b as u16) + cin;
        let res = sum as u8;

        let z = res == 0;
        let n = false;
        let h = (a & 0x0F) + (b & 0x0F) + (cin as u8) > 0x0F;
        let c = sum > 0xFF;

        let mut flags = 0u8;
        if z {
            flags |= Flag::Z as u8;
        }
        if n {
            flags |= Flag::N as u8;
        }
        if h {
            flags |= Flag::H as u8;
        }
        if c {
            flags |= Flag::C as u8;
        }

        AluResult::new(res, flags)
    }

    pub fn sub_8(a: u8, b: u8) -> AluResult<u8> {
        let res = a.wrapping_sub(b);
        let z = res == 0;
        let n = true;
        let h = (a & 0x0F) < (b & 0x0F);
        let c = a < b;

        let mut flags = 0u8;
        if z {
            flags |= Flag::Z as u8;
        }
        if n {
            flags |= Flag::N as u8;
        }
        if h {
            flags |= Flag::H as u8;
        }
        if c {
            flags |= Flag::C as u8;
        }

        AluResult::new(res, flags)
    }

    pub fn sbc_8(a: u8, b: u8, carry_in: bool) -> AluResult<u8> {
        let cin = if carry_in { 1u16 } else { 0u16 };
        let full_diff = (a as i32) - (b as i32) - (cin as i32);
        let res = full_diff as u8;

        let z = res == 0;
        let n = true;
        let h = (a & 0x0F) < (b & 0x0F) + (cin as u8);
        let c = (a as u16) < (b as u16) + cin;

        let mut flags = 0u8;
        if z {
            flags |= Flag::Z as u8;
        }
        if n {
            flags |= Flag::N as u8;
        }
        if h {
            flags |= Flag::H as u8;
        }
        if c {
            flags |= Flag::C as u8;
        }

        AluResult::new(res, flags)
    }

    pub fn and_8(a: u8, b: u8) -> AluResult<u8> {
        let res = a & b;
        let z = res == 0;

        let mut flags = Flag::H as u8; // AND always sets H=1
        if z {
            flags |= Flag::Z as u8;
        }

        AluResult::new(res, flags)
    }

    pub fn or_8(a: u8, b: u8) -> AluResult<u8> {
        let res = a | b;
        let z = res == 0;

        let mut flags = 0u8;
        if z {
            flags |= Flag::Z as u8;
        }

        AluResult::new(res, flags)
    }

    pub fn xor_8(a: u8, b: u8) -> AluResult<u8> {
        let res = a ^ b;
        let z = res == 0;

        let mut flags = 0u8;
        if z {
            flags |= Flag::Z as u8;
        }

        AluResult::new(res, flags)
    }

    pub fn cp_8(a: u8, b: u8) -> u8 {
        Self::sub_8(a, b).flags
    }

    pub fn inc_8(val: u8, current_flags: u8) -> AluResult<u8> {
        let res = val.wrapping_add(1);
        let z = res == 0;
        let n = false;
        let h = (val & 0x0F) == 0x0F;
        let c = (current_flags & Flag::C as u8) != 0;

        let mut flags = 0u8;
        if z {
            flags |= Flag::Z as u8;
        }
        if n {
            flags |= Flag::N as u8;
        }
        if h {
            flags |= Flag::H as u8;
        }
        if c {
            flags |= Flag::C as u8;
        }

        AluResult::new(res, flags)
    }

    pub fn dec_8(val: u8, current_flags: u8) -> AluResult<u8> {
        let res = val.wrapping_sub(1);
        let z = res == 0;
        let n = true;
        let h = (val & 0x0F) == 0x00;
        let c = (current_flags & Flag::C as u8) != 0;

        let mut flags = 0u8;
        if z {
            flags |= Flag::Z as u8;
        }
        if n {
            flags |= Flag::N as u8;
        }
        if h {
            flags |= Flag::H as u8;
        }
        if c {
            flags |= Flag::C as u8;
        }

        AluResult::new(res, flags)
    }

    pub fn add_hl(hl: u16, val: u16, current_flags: u8) -> AluResult<u16> {
        let res = hl.wrapping_add(val);
        let z = (current_flags & Flag::Z as u8) != 0;
        let n = false;
        let h = (hl & 0x0FFF) + (val & 0x0FFF) > 0x0FFF;
        let c = (hl as u32) + (val as u32) > 0xFFFF;

        let mut flags = 0u8;
        if z {
            flags |= Flag::Z as u8;
        }
        if n {
            flags |= Flag::N as u8;
        }
        if h {
            flags |= Flag::H as u8;
        }
        if c {
            flags |= Flag::C as u8;
        }

        AluResult::new(res, flags)
    }

    pub fn add_sp_i8(sp: u16, offset: i8) -> AluResult<u16> {
        let raw = offset as u8 as u16;
        let res = sp.wrapping_add(offset as i16 as u16);

        let h = (sp & 0x0F) + (raw & 0x0F) > 0x0F;
        let c = (sp & 0xFF) + (raw & 0xFF) > 0xFF;

        let mut flags = 0u8;
        if h {
            flags |= Flag::H as u8;
        }
        if c {
            flags |= Flag::C as u8;
        }

        AluResult::new(res, flags)
    }

    pub fn rlc(val: u8, is_cb: bool) -> AluResult<u8> {
        let c_out = (val & 0x80) >> 7;
        let res = (val << 1) | c_out;

        let z = if is_cb { res == 0 } else { false };
        let c = c_out != 0;

        let mut flags = 0u8;
        if z {
            flags |= Flag::Z as u8;
        }
        if c {
            flags |= Flag::C as u8;
        }

        AluResult::new(res, flags)
    }

    pub fn rrc(val: u8, is_cb: bool) -> AluResult<u8> {
        let c_out = val & 0x01;
        let res = (val >> 1) | (c_out << 7);

        let z = if is_cb { res == 0 } else { false };
        let c = c_out != 0;

        let mut flags = 0u8;
        if z {
            flags |= Flag::Z as u8;
        }
        if c {
            flags |= Flag::C as u8;
        }

        AluResult::new(res, flags)
    }

    pub fn rl(val: u8, c_in: bool, is_cb: bool) -> AluResult<u8> {
        let c_out = (val & 0x80) >> 7;
        let cin = if c_in { 1 } else { 0 };
        let res = (val << 1) | cin;

        let z = if is_cb { res == 0 } else { false };
        let c = c_out != 0;

        let mut flags = 0u8;
        if z {
            flags |= Flag::Z as u8;
        }
        if c {
            flags |= Flag::C as u8;
        }

        AluResult::new(res, flags)
    }

    pub fn rr(val: u8, c_in: bool, is_cb: bool) -> AluResult<u8> {
        let c_out = val & 0x01;
        let cin = if c_in { 0x80 } else { 0 };
        let res = (val >> 1) | cin;

        let z = if is_cb { res == 0 } else { false };
        let c = c_out != 0;

        let mut flags = 0u8;
        if z {
            flags |= Flag::Z as u8;
        }
        if c {
            flags |= Flag::C as u8;
        }

        AluResult::new(res, flags)
    }

    pub fn sla(val: u8) -> AluResult<u8> {
        let c_out = (val & 0x80) >> 7;
        let res = val << 1;

        let z = res == 0;
        let c = c_out != 0;

        let mut flags = 0u8;
        if z {
            flags |= Flag::Z as u8;
        }
        if c {
            flags |= Flag::C as u8;
        }

        AluResult::new(res, flags)
    }

    pub fn sra(val: u8) -> AluResult<u8> {
        let c_out = val & 0x01;
        let res = (val >> 1) | (val & 0x80);

        let z = res == 0;
        let c = c_out != 0;

        let mut flags = 0u8;
        if z {
            flags |= Flag::Z as u8;
        }
        if c {
            flags |= Flag::C as u8;
        }

        AluResult::new(res, flags)
    }

    pub fn srl(val: u8) -> AluResult<u8> {
        let c_out = val & 0x01;
        let res = val >> 1;

        let z = res == 0;
        let c = c_out != 0;

        let mut flags = 0u8;
        if z {
            flags |= Flag::Z as u8;
        }
        if c {
            flags |= Flag::C as u8;
        }

        AluResult::new(res, flags)
    }

    pub fn swap(val: u8) -> AluResult<u8> {
        let res = val.rotate_left(4);
        let z = res == 0;

        let mut flags = 0u8;
        if z {
            flags |= Flag::Z as u8;
        }

        AluResult::new(res, flags)
    }

    pub fn bit(val: u8, bit: u8, current_flags: u8) -> u8 {
        let is_zero = (val & (1 << bit)) == 0;
        let c = (current_flags & Flag::C as u8) != 0;

        let mut flags = Flag::H as u8; // BIT always sets H=1
        if is_zero {
            flags |= Flag::Z as u8;
        }
        if c {
            flags |= Flag::C as u8;
        }

        flags & 0xF0
    }

    pub fn set(val: u8, bit: u8) -> u8 {
        val | (1 << bit)
    }

    pub fn res(val: u8, bit: u8) -> u8 {
        val & !(1 << bit)
    }

    pub fn daa(a: u8, current_flags: u8) -> AluResult<u8> {
        let n_set = (current_flags & (Flag::N as u8)) != 0;
        let h_set = (current_flags & (Flag::H as u8)) != 0;
        let mut c_set = (current_flags & (Flag::C as u8)) != 0;

        let mut correction = 0u8;

        if !n_set {
            if h_set || (a & 0x0F) > 0x09 {
                correction |= 0x06;
            }
            if c_set || a > 0x99 {
                correction |= 0x60;
                c_set = true;
            }
        } else {
            if h_set {
                correction |= 0x06;
            }
            if c_set {
                correction |= 0x60;
            }
        }

        let res = if !n_set {
            a.wrapping_add(correction)
        } else {
            a.wrapping_sub(correction)
        };

        let mut flags = 0u8;
        if res == 0 {
            flags |= Flag::Z as u8;
        }
        if n_set {
            flags |= Flag::N as u8;
        }
        if c_set {
            flags |= Flag::C as u8;
        }

        AluResult::new(res, flags)
    }

    pub fn cpl(a: u8, current_flags: u8) -> AluResult<u8> {
        let res = !a;
        let z = (current_flags & Flag::Z as u8) != 0;
        let c = (current_flags & Flag::C as u8) != 0;

        let mut flags = (Flag::N as u8) | (Flag::H as u8);
        if z {
            flags |= Flag::Z as u8;
        }
        if c {
            flags |= Flag::C as u8;
        }

        AluResult::new(res, flags)
    }

    pub fn scf(current_flags: u8) -> u8 {
        let z = (current_flags & Flag::Z as u8) != 0;

        let mut flags = Flag::C as u8;
        if z {
            flags |= Flag::Z as u8;
        }

        flags & 0xF0
    }

    pub fn ccf(current_flags: u8) -> u8 {
        let z = (current_flags & Flag::Z as u8) != 0;
        let c = (current_flags & Flag::C as u8) == 0; // Invert Carry

        let mut flags = 0u8;
        if z {
            flags |= Flag::Z as u8;
        }
        if c {
            flags |= Flag::C as u8;
        }

        flags & 0xF0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_8() {
        let res = Alu::add_8(0x0F, 0x01);
        assert_eq!(res.value, 0x10);
        assert_eq!(res.flags & (Flag::H as u8), Flag::H as u8);
        assert_eq!(res.flags & (Flag::Z as u8), 0);

        let res = Alu::add_8(0xFF, 0x01);
        assert_eq!(res.value, 0x00);
        assert_eq!(res.flags & (Flag::Z as u8), Flag::Z as u8);
        assert_eq!(res.flags & (Flag::C as u8), Flag::C as u8);
    }

    #[test]
    fn test_adc_8() {
        let res = Alu::adc_8(0x00, 0x00, true);
        assert_eq!(res.value, 0x01);
        assert_eq!(res.flags & (Flag::Z as u8), 0);

        let res = Alu::adc_8(0x0F, 0x00, true);
        assert_eq!(res.value, 0x10);
        assert_eq!(res.flags & (Flag::H as u8), Flag::H as u8);

        let res = Alu::adc_8(0xFF, 0x00, true);
        assert_eq!(res.value, 0x00);
        assert_eq!(res.flags & (Flag::Z as u8), Flag::Z as u8);
        assert_eq!(res.flags & (Flag::C as u8), Flag::C as u8);
    }

    #[test]
    fn test_sub_8() {
        let res = Alu::sub_8(0x10, 0x01);
        assert_eq!(res.value, 0x0F);
        assert_eq!(res.flags & (Flag::N as u8), Flag::N as u8);
        assert_eq!(res.flags & (Flag::H as u8), Flag::H as u8);

        let res = Alu::sub_8(0x00, 0x01);
        assert_eq!(res.value, 0xFF);
        assert_eq!(res.flags & (Flag::C as u8), Flag::C as u8);

        let res = Alu::sub_8(0x05, 0x05);
        assert_eq!(res.value, 0x00);
        assert_eq!(res.flags & (Flag::Z as u8), Flag::Z as u8);
    }

    #[test]
    fn test_sbc_8() {
        let res = Alu::sbc_8(0x10, 0x00, true);
        assert_eq!(res.value, 0x0F);
        assert_eq!(res.flags & (Flag::H as u8), Flag::H as u8);
    }

    #[test]
    fn test_and_or_xor() {
        let res = Alu::and_8(0xFF, 0x0F);
        assert_eq!(res.value, 0x0F);
        assert_eq!(res.flags & (Flag::H as u8), Flag::H as u8);

        let res = Alu::or_8(0x00, 0x00);
        assert_eq!(res.value, 0x00);
        assert_eq!(res.flags & (Flag::Z as u8), Flag::Z as u8);

        let res = Alu::xor_8(0xAA, 0xAA);
        assert_eq!(res.value, 0x00);
        assert_eq!(res.flags & (Flag::Z as u8), Flag::Z as u8);
    }

    #[test]
    fn test_inc_dec_8() {
        let res = Alu::inc_8(0x0F, Flag::C as u8);
        assert_eq!(res.value, 0x10);
        assert_eq!(res.flags & (Flag::H as u8), Flag::H as u8);
        assert_eq!(res.flags & (Flag::C as u8), Flag::C as u8);

        let res = Alu::dec_8(0x10, Flag::C as u8);
        assert_eq!(res.value, 0x0F);
        assert_eq!(res.flags & (Flag::H as u8), Flag::H as u8);
        assert_eq!(res.flags & (Flag::C as u8), Flag::C as u8);
    }

    #[test]
    fn test_add_hl() {
        let res = Alu::add_hl(0x0FFF, 0x0001, Flag::Z as u8);
        assert_eq!(res.value, 0x1000);
        assert_eq!(res.flags & (Flag::H as u8), Flag::H as u8);
        assert_eq!(res.flags & (Flag::Z as u8), Flag::Z as u8); // Z preserved

        let res = Alu::add_hl(0xFFFF, 0x0001, 0);
        assert_eq!(res.value, 0x0000);
        assert_eq!(res.flags & (Flag::C as u8), Flag::C as u8);
    }

    #[test]
    fn test_daa_scenarios() {
        // 1. ADD 0x15 + 0x27 = 0x3C (with H=1 after 5+7=12) -> DAA = 0x42
        let add_flags = Flag::H as u8;
        let daa1 = Alu::daa(0x3C, add_flags);
        assert_eq!(daa1.value, 0x42);

        // 2. SUB 0x42 - 0x27 = 0x1B (with N=1, H=1) -> DAA = 0x15
        let sub_flags = (Flag::N as u8) | (Flag::H as u8);
        let daa2 = Alu::daa(0x1B, sub_flags);
        assert_eq!(daa2.value, 0x15);

        // 3. Overflow 0x99 + 0x01 = 0x9A -> DAA = 0x00, C=1, Z=1
        let daa3 = Alu::daa(0x9A, 0);
        assert_eq!(daa3.value, 0x00);
        assert_eq!(daa3.flags & (Flag::Z as u8), Flag::Z as u8);
        assert_eq!(daa3.flags & (Flag::C as u8), Flag::C as u8);
    }

    #[test]
    fn test_shifts_rotates_and_bit() {
        let res = Alu::rlc(0x80, true);
        assert_eq!(res.value, 0x01);
        assert_eq!(res.flags & (Flag::C as u8), Flag::C as u8);

        let res = Alu::rrc(0x01, true);
        assert_eq!(res.value, 0x80);
        assert_eq!(res.flags & (Flag::C as u8), Flag::C as u8);

        let res = Alu::swap(0xF0);
        assert_eq!(res.value, 0x0F);

        let bit_flags = Alu::bit(0x08, 3, 0);
        assert_eq!(bit_flags & (Flag::Z as u8), 0);
        assert_eq!(bit_flags & (Flag::H as u8), Flag::H as u8);

        let bit_flags_zero = Alu::bit(0x00, 3, 0);
        assert_eq!(bit_flags_zero & (Flag::Z as u8), Flag::Z as u8);
    }
}
