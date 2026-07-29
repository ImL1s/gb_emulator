use crate::cpu::alu::Alu;
use crate::cpu::Cpu;
use crate::mmu::bus::Bus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Condition {
    NZ, // Not Zero (!Z)
    Z,  // Zero (Z)
    NC, // No Carry (!C)
    C,  // Carry (C)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum R8 {
    B = 0,
    C = 1,
    D = 2,
    E = 3,
    H = 4,
    L = 5,
    HL = 6, // (HL) memory reference
    A = 7,
}

impl Cpu {
    /// Read R8 operand value (if R8::HL, read byte from bus at address HL).
    #[inline]
    pub fn read_r8(&self, bus: &impl Bus, reg: R8) -> u8 {
        match reg {
            R8::B => self.registers.b,
            R8::C => self.registers.c,
            R8::D => self.registers.d,
            R8::E => self.registers.e,
            R8::H => self.registers.h,
            R8::L => self.registers.l,
            R8::HL => bus.read_byte(self.registers.hl()),
            R8::A => self.registers.a,
        }
    }

    /// Write R8 operand value (if R8::HL, write byte to bus at address HL).
    #[inline]
    pub fn write_r8(&mut self, bus: &mut impl Bus, reg: R8, val: u8) {
        match reg {
            R8::B => self.registers.b = val,
            R8::C => self.registers.c = val,
            R8::D => self.registers.d = val,
            R8::E => self.registers.e = val,
            R8::H => self.registers.h = val,
            R8::L => self.registers.l = val,
            R8::HL => bus.write_byte(self.registers.hl(), val),
            R8::A => self.registers.a = val,
        }
    }

    /// Check if condition is met for conditional jumps, calls, and returns.
    #[inline]
    pub fn check_condition(&self, cond: Condition) -> bool {
        match cond {
            Condition::NZ => !self.registers.flag_z(),
            Condition::Z => self.registers.flag_z(),
            Condition::NC => !self.registers.flag_c(),
            Condition::C => self.registers.flag_c(),
        }
    }

    /// Execute single unprefixed opcode. Returns total T-cycles (4.194304 MHz).
    pub fn execute_unprefixed(&mut self, bus: &mut impl Bus, opcode: u8) -> u32 {
        match opcode {
            // NOP
            0x00 => 4,

            // 16-bit Immediate Loads (LD r16, n16)
            0x01 => {
                let val = self.fetch_word(bus);
                self.registers.set_bc(val);
                12
            }
            0x11 => {
                let val = self.fetch_word(bus);
                self.registers.set_de(val);
                12
            }
            0x21 => {
                let val = self.fetch_word(bus);
                self.registers.set_hl(val);
                12
            }
            0x31 => {
                let val = self.fetch_word(bus);
                self.registers.sp = val;
                12
            }

            // 16-bit Indirect Loads / Stores
            0x02 => {
                bus.write_byte(self.registers.bc(), self.registers.a);
                8
            }
            0x12 => {
                bus.write_byte(self.registers.de(), self.registers.a);
                8
            }
            0x0A => {
                self.registers.a = bus.read_byte(self.registers.bc());
                8
            }
            0x1A => {
                self.registers.a = bus.read_byte(self.registers.de());
                8
            }

            0x22 => {
                let hl = self.registers.hl();
                bus.write_byte(hl, self.registers.a);
                self.registers.set_hl(hl.wrapping_add(1));
                8
            }
            0x32 => {
                let hl = self.registers.hl();
                bus.write_byte(hl, self.registers.a);
                self.registers.set_hl(hl.wrapping_sub(1));
                8
            }
            0x2A => {
                let hl = self.registers.hl();
                self.registers.a = bus.read_byte(hl);
                self.registers.set_hl(hl.wrapping_add(1));
                8
            }
            0x3A => {
                let hl = self.registers.hl();
                self.registers.a = bus.read_byte(hl);
                self.registers.set_hl(hl.wrapping_sub(1));
                8
            }

            // LD (n16), SP
            0x08 => {
                let addr = self.fetch_word(bus);
                bus.write_word(addr, self.registers.sp);
                20
            }

            // 16-bit INC & DEC (Flags unaffected)
            0x03 => {
                self.registers.set_bc(self.registers.bc().wrapping_add(1));
                8
            }
            0x13 => {
                self.registers.set_de(self.registers.de().wrapping_add(1));
                8
            }
            0x23 => {
                self.registers.set_hl(self.registers.hl().wrapping_add(1));
                8
            }
            0x33 => {
                self.registers.sp = self.registers.sp.wrapping_add(1);
                8
            }
            0x0B => {
                self.registers.set_bc(self.registers.bc().wrapping_sub(1));
                8
            }
            0x1B => {
                self.registers.set_de(self.registers.de().wrapping_sub(1));
                8
            }
            0x2B => {
                self.registers.set_hl(self.registers.hl().wrapping_sub(1));
                8
            }
            0x3B => {
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                8
            }

            // 8-bit INC r8 / (HL)
            0x04 => {
                self.op_inc_r8(bus, R8::B);
                4
            }
            0x0C => {
                self.op_inc_r8(bus, R8::C);
                4
            }
            0x14 => {
                self.op_inc_r8(bus, R8::D);
                4
            }
            0x1C => {
                self.op_inc_r8(bus, R8::E);
                4
            }
            0x24 => {
                self.op_inc_r8(bus, R8::H);
                4
            }
            0x2C => {
                self.op_inc_r8(bus, R8::L);
                4
            }
            0x34 => {
                self.op_inc_r8(bus, R8::HL);
                12
            }
            0x3C => {
                self.op_inc_r8(bus, R8::A);
                4
            }

            // 8-bit DEC r8 / (HL)
            0x05 => {
                self.op_dec_r8(bus, R8::B);
                4
            }
            0x0D => {
                self.op_dec_r8(bus, R8::C);
                4
            }
            0x15 => {
                self.op_dec_r8(bus, R8::D);
                4
            }
            0x1D => {
                self.op_dec_r8(bus, R8::E);
                4
            }
            0x25 => {
                self.op_dec_r8(bus, R8::H);
                4
            }
            0x2D => {
                self.op_dec_r8(bus, R8::L);
                4
            }
            0x35 => {
                self.op_dec_r8(bus, R8::HL);
                12
            }
            0x3D => {
                self.op_dec_r8(bus, R8::A);
                4
            }

            // 8-bit LD r8, n8 / (HL), n8
            0x06 => {
                let val = self.fetch_byte(bus);
                self.registers.b = val;
                8
            }
            0x0E => {
                let val = self.fetch_byte(bus);
                self.registers.c = val;
                8
            }
            0x16 => {
                let val = self.fetch_byte(bus);
                self.registers.d = val;
                8
            }
            0x1E => {
                let val = self.fetch_byte(bus);
                self.registers.e = val;
                8
            }
            0x26 => {
                let val = self.fetch_byte(bus);
                self.registers.h = val;
                8
            }
            0x2E => {
                let val = self.fetch_byte(bus);
                self.registers.l = val;
                8
            }
            0x36 => {
                let val = self.fetch_byte(bus);
                bus.write_byte(self.registers.hl(), val);
                12
            }
            0x3E => {
                let val = self.fetch_byte(bus);
                self.registers.a = val;
                8
            }

            // Rotates A (Unprefixed)
            0x07 => {
                let res = Alu::rlc(self.registers.a, false);
                self.registers.a = res.value;
                self.registers.set_f(res.flags);
                4
            }
            0x0F => {
                let res = Alu::rrc(self.registers.a, false);
                self.registers.a = res.value;
                self.registers.set_f(res.flags);
                4
            }
            0x17 => {
                let res = Alu::rl(self.registers.a, self.registers.flag_c(), false);
                self.registers.a = res.value;
                self.registers.set_f(res.flags);
                4
            }
            0x1F => {
                let res = Alu::rr(self.registers.a, self.registers.flag_c(), false);
                self.registers.a = res.value;
                self.registers.set_f(res.flags);
                4
            }

            // ADD HL, r16
            0x09 => {
                self.op_add_hl(self.registers.bc());
                8
            }
            0x19 => {
                self.op_add_hl(self.registers.de());
                8
            }
            0x29 => {
                self.op_add_hl(self.registers.hl());
                8
            }
            0x39 => {
                self.op_add_hl(self.registers.sp);
                8
            }

            // JR e8 & JR cc, e8
            0x18 => {
                self.op_jr_unconditional(bus);
                12
            }
            0x20 => self.op_jr_conditional(bus, Condition::NZ),
            0x28 => self.op_jr_conditional(bus, Condition::Z),
            0x30 => self.op_jr_conditional(bus, Condition::NC),
            0x38 => self.op_jr_conditional(bus, Condition::C),

            // Special Math / Flags
            0x27 => {
                let res = Alu::daa(self.registers.a, self.registers.f);
                self.registers.a = res.value;
                self.registers.set_f(res.flags);
                4
            }
            0x2F => {
                let res = Alu::cpl(self.registers.a, self.registers.f);
                self.registers.a = res.value;
                self.registers.set_f(res.flags);
                4
            }
            0x37 => {
                let flags = Alu::scf(self.registers.f);
                self.registers.set_f(flags);
                4
            }
            0x3F => {
                let flags = Alu::ccf(self.registers.f);
                self.registers.set_f(flags);
                4
            }

            // 8-bit Register Block LD r8, r8' (0x40..0x7F)
            0x40..=0x7F => {
                if opcode == 0x76 {
                    self.halted = true;
                    4
                } else {
                    let src_idx = opcode & 0x07;
                    let dst_idx = (opcode >> 3) & 0x07;
                    let src_reg = self.index_to_r8(src_idx);
                    let dst_reg = self.index_to_r8(dst_idx);

                    let val = self.read_r8(bus, src_reg);
                    self.write_r8(bus, dst_reg, val);

                    if src_reg == R8::HL || dst_reg == R8::HL {
                        8
                    } else {
                        4
                    }
                }
            }

            // 8-bit Arithmetic / Logic Block (0x80..0xBF)
            0x80..=0xBF => {
                let reg = self.index_to_r8(opcode & 0x07);
                let val = self.read_r8(bus, reg);
                let operation = (opcode >> 3) & 0x07;

                match operation {
                    0 => {
                        // ADD A, r8
                        let res = Alu::add_8(self.registers.a, val);
                        self.registers.a = res.value;
                        self.registers.set_f(res.flags);
                    }
                    1 => {
                        // ADC A, r8
                        let res = Alu::adc_8(self.registers.a, val, self.registers.flag_c());
                        self.registers.a = res.value;
                        self.registers.set_f(res.flags);
                    }
                    2 => {
                        // SUB A, r8
                        let res = Alu::sub_8(self.registers.a, val);
                        self.registers.a = res.value;
                        self.registers.set_f(res.flags);
                    }
                    3 => {
                        // SBC A, r8
                        let res = Alu::sbc_8(self.registers.a, val, self.registers.flag_c());
                        self.registers.a = res.value;
                        self.registers.set_f(res.flags);
                    }
                    4 => {
                        // AND A, r8
                        let res = Alu::and_8(self.registers.a, val);
                        self.registers.a = res.value;
                        self.registers.set_f(res.flags);
                    }
                    5 => {
                        // XOR A, r8
                        let res = Alu::xor_8(self.registers.a, val);
                        self.registers.a = res.value;
                        self.registers.set_f(res.flags);
                    }
                    6 => {
                        // OR A, r8
                        let res = Alu::or_8(self.registers.a, val);
                        self.registers.a = res.value;
                        self.registers.set_f(res.flags);
                    }
                    _ => {
                        // CP A, r8
                        let flags = Alu::cp_8(self.registers.a, val);
                        self.registers.set_f(flags);
                    }
                }

                if reg == R8::HL {
                    8
                } else {
                    4
                }
            }

            // Returns & Conditional Returns
            0xC0 => self.op_ret_conditional(bus, Condition::NZ),
            0xC8 => self.op_ret_conditional(bus, Condition::Z),
            0xD0 => self.op_ret_conditional(bus, Condition::NC),
            0xD8 => self.op_ret_conditional(bus, Condition::C),
            0xC9 => {
                self.registers.pc = self.pop_stack_16(bus);
                16
            }
            0xD9 => {
                self.registers.pc = self.pop_stack_16(bus);
                self.ime_state = crate::cpu::ImeState::Enabled;
                16
            }

            // POP & PUSH r16
            0xC1 => {
                let val = self.pop_stack_16(bus);
                self.registers.set_bc(val);
                12
            }
            0xD1 => {
                let val = self.pop_stack_16(bus);
                self.registers.set_de(val);
                12
            }
            0xE1 => {
                let val = self.pop_stack_16(bus);
                self.registers.set_hl(val);
                12
            }
            0xF1 => {
                let val = self.pop_stack_16(bus);
                self.registers.set_af(val);
                12
            } // Masked to 0xF0

            0xC5 => {
                self.push_stack_16(bus, self.registers.bc());
                16
            }
            0xD5 => {
                self.push_stack_16(bus, self.registers.de());
                16
            }
            0xE5 => {
                self.push_stack_16(bus, self.registers.hl());
                16
            }
            0xF5 => {
                self.push_stack_16(bus, self.registers.af());
                16
            }

            // Jumps & Conditional Jumps
            0xC2 => self.op_jp_conditional(bus, Condition::NZ),
            0xCA => self.op_jp_conditional(bus, Condition::Z),
            0xD2 => self.op_jp_conditional(bus, Condition::NC),
            0xDA => self.op_jp_conditional(bus, Condition::C),
            0xC3 => {
                self.registers.pc = self.fetch_word(bus);
                16
            }
            0xE9 => {
                self.registers.pc = self.registers.hl();
                4
            }

            // Calls & Conditional Calls
            0xC4 => self.op_call_conditional(bus, Condition::NZ),
            0xCC => self.op_call_conditional(bus, Condition::Z),
            0xD4 => self.op_call_conditional(bus, Condition::NC),
            0xDC => self.op_call_conditional(bus, Condition::C),
            0xCD => {
                let target = self.fetch_word(bus);
                self.push_stack_16(bus, self.registers.pc);
                self.registers.pc = target;
                24
            }

            // Immediate ALU (ADD, ADC, SUB, SBC, AND, XOR, OR, CP A, n8)
            0xC6 => {
                let val = self.fetch_byte(bus);
                let res = Alu::add_8(self.registers.a, val);
                self.registers.a = res.value;
                self.registers.set_f(res.flags);
                8
            }
            0xCE => {
                let val = self.fetch_byte(bus);
                let res = Alu::adc_8(self.registers.a, val, self.registers.flag_c());
                self.registers.a = res.value;
                self.registers.set_f(res.flags);
                8
            }
            0xD6 => {
                let val = self.fetch_byte(bus);
                let res = Alu::sub_8(self.registers.a, val);
                self.registers.a = res.value;
                self.registers.set_f(res.flags);
                8
            }
            0xDE => {
                let val = self.fetch_byte(bus);
                let res = Alu::sbc_8(self.registers.a, val, self.registers.flag_c());
                self.registers.a = res.value;
                self.registers.set_f(res.flags);
                8
            }
            0xE6 => {
                let val = self.fetch_byte(bus);
                let res = Alu::and_8(self.registers.a, val);
                self.registers.a = res.value;
                self.registers.set_f(res.flags);
                8
            }
            0xEE => {
                let val = self.fetch_byte(bus);
                let res = Alu::xor_8(self.registers.a, val);
                self.registers.a = res.value;
                self.registers.set_f(res.flags);
                8
            }
            0xF6 => {
                let val = self.fetch_byte(bus);
                let res = Alu::or_8(self.registers.a, val);
                self.registers.a = res.value;
                self.registers.set_f(res.flags);
                8
            }
            0xFE => {
                let val = self.fetch_byte(bus);
                let flags = Alu::cp_8(self.registers.a, val);
                self.registers.set_f(flags);
                8
            }

            // RST vectors
            0xC7 => {
                self.op_rst(bus, 0x0000);
                16
            }
            0xCF => {
                self.op_rst(bus, 0x0008);
                16
            }
            0xD7 => {
                self.op_rst(bus, 0x0010);
                16
            }
            0xDF => {
                self.op_rst(bus, 0x0018);
                16
            }
            0xE7 => {
                self.op_rst(bus, 0x0020);
                16
            }
            0xEF => {
                self.op_rst(bus, 0x0028);
                16
            }
            0xF7 => {
                self.op_rst(bus, 0x0030);
                16
            }
            0xFF => {
                self.op_rst(bus, 0x0038);
                16
            }

            // I/O & High Memory Shortcuts
            0xE0 => {
                let offset = self.fetch_byte(bus) as u16;
                bus.write_byte(0xFF00 + offset, self.registers.a);
                12
            }
            0xF0 => {
                let offset = self.fetch_byte(bus) as u16;
                self.registers.a = bus.read_byte(0xFF00 + offset);
                12
            }
            0xE2 => {
                bus.write_byte(0xFF00 + (self.registers.c as u16), self.registers.a);
                8
            }
            0xF2 => {
                self.registers.a = bus.read_byte(0xFF00 + (self.registers.c as u16));
                8
            }
            0xEA => {
                let addr = self.fetch_word(bus);
                bus.write_byte(addr, self.registers.a);
                16
            }
            0xFA => {
                let addr = self.fetch_word(bus);
                self.registers.a = bus.read_byte(addr);
                16
            }

            // SP Math & SP Load
            0xE8 => {
                let offset = self.fetch_byte(bus) as i8;
                let res = Alu::add_sp_i8(self.registers.sp, offset);
                self.registers.sp = res.value;
                self.registers.set_f(res.flags);
                16
            }
            0xF8 => {
                let offset = self.fetch_byte(bus) as i8;
                let res = Alu::add_sp_i8(self.registers.sp, offset);
                self.registers.set_hl(res.value);
                self.registers.set_f(res.flags);
                12
            }
            0xF9 => {
                self.registers.sp = self.registers.hl();
                8
            }

            // Interrupt Control & STOP
            0xF3 => {
                self.ime_state = crate::cpu::ImeState::Disabled;
                4
            }
            0xFB => {
                self.ime_state = crate::cpu::ImeState::PendingEnable;
                4
            }
            0x10 => {
                self.fetch_byte(bus);
                self.stopped = true;
                4
            } // STOP 0

            // CB Prefix (If called directly)
            0xCB => 4,

            // Unused / Illegal Opcodes on LR35902
            0xD3 | 0xDB | 0xDD | 0xE3 | 0xE4 | 0xEB | 0xEC | 0xED | 0xF4 | 0xFC | 0xFD => 4,
        }
    }

    /// Execute single CB-prefixed opcode. Returns total T-cycles for CB instruction execution.
    pub fn execute_cb(&mut self, bus: &mut impl Bus, cb_opcode: u8) -> u32 {
        let reg = self.index_to_r8(cb_opcode & 0x07);
        let bit = (cb_opcode >> 3) & 0x07;
        let group = cb_opcode >> 6;

        let val = self.read_r8(bus, reg);

        match group {
            0 => {
                // Rotate / Shift / SWAP
                let res = match (cb_opcode >> 3) & 0x07 {
                    0 => Alu::rlc(val, true),
                    1 => Alu::rrc(val, true),
                    2 => Alu::rl(val, self.registers.flag_c(), true),
                    3 => Alu::rr(val, self.registers.flag_c(), true),
                    4 => Alu::sla(val),
                    5 => Alu::sra(val),
                    6 => Alu::swap(val),
                    _ => Alu::srl(val),
                };
                self.write_r8(bus, reg, res.value);
                self.registers.set_f(res.flags);
                if reg == R8::HL {
                    16
                } else {
                    8
                }
            }
            1 => {
                // BIT b, r8
                let flags = Alu::bit(val, bit, self.registers.f);
                self.registers.set_f(flags);
                if reg == R8::HL {
                    12
                } else {
                    8
                }
            }
            2 => {
                // RES b, r8
                let res = Alu::res(val, bit);
                self.write_r8(bus, reg, res);
                if reg == R8::HL {
                    16
                } else {
                    8
                }
            }
            _ => {
                // SET b, r8
                let res = Alu::set(val, bit);
                self.write_r8(bus, reg, res);
                if reg == R8::HL {
                    16
                } else {
                    8
                }
            }
        }
    }

    // --- Private Helper Instructions ---
    #[inline]
    fn index_to_r8(&self, idx: u8) -> R8 {
        match idx & 0x07 {
            0 => R8::B,
            1 => R8::C,
            2 => R8::D,
            3 => R8::E,
            4 => R8::H,
            5 => R8::L,
            6 => R8::HL,
            _ => R8::A,
        }
    }

    fn op_inc_r8(&mut self, bus: &mut impl Bus, reg: R8) {
        let val = self.read_r8(bus, reg);
        let res = Alu::inc_8(val, self.registers.f);
        self.write_r8(bus, reg, res.value);
        self.registers.set_f(res.flags);
    }

    fn op_dec_r8(&mut self, bus: &mut impl Bus, reg: R8) {
        let val = self.read_r8(bus, reg);
        let res = Alu::dec_8(val, self.registers.f);
        self.write_r8(bus, reg, res.value);
        self.registers.set_f(res.flags);
    }

    fn op_add_hl(&mut self, val: u16) {
        let res = Alu::add_hl(self.registers.hl(), val, self.registers.f);
        self.registers.set_hl(res.value);
        self.registers.set_f(res.flags);
    }

    fn op_jr_unconditional(&mut self, bus: &impl Bus) {
        let offset = self.fetch_byte(bus) as i8;
        self.registers.pc = self.registers.pc.wrapping_add_signed(offset as i16);
    }

    fn op_jr_conditional(&mut self, bus: &impl Bus, cond: Condition) -> u32 {
        let offset = self.fetch_byte(bus) as i8;
        if self.check_condition(cond) {
            self.registers.pc = self.registers.pc.wrapping_add_signed(offset as i16);
            12
        } else {
            8
        }
    }

    fn op_jp_conditional(&mut self, bus: &impl Bus, cond: Condition) -> u32 {
        let target = self.fetch_word(bus);
        if self.check_condition(cond) {
            self.registers.pc = target;
            16
        } else {
            12
        }
    }

    fn op_call_conditional(&mut self, bus: &mut impl Bus, cond: Condition) -> u32 {
        let target = self.fetch_word(bus);
        if self.check_condition(cond) {
            self.push_stack_16(bus, self.registers.pc);
            self.registers.pc = target;
            24
        } else {
            12
        }
    }

    fn op_ret_conditional(&mut self, bus: &mut impl Bus, cond: Condition) -> u32 {
        if self.check_condition(cond) {
            self.registers.pc = self.pop_stack_16(bus);
            20
        } else {
            8
        }
    }

    fn op_rst(&mut self, bus: &mut impl Bus, vec: u16) {
        self.push_stack_16(bus, self.registers.pc);
        self.registers.pc = vec;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::registers::Flag;
    use crate::mmu::bus::MockBus;

    #[test]
    fn test_all_256_unprefixed_decoding() {
        let mut cpu = Cpu::new();
        let mut bus = MockBus::new();

        for opcode in 0..=255 {
            cpu.registers.pc = 0x0100;
            cpu.registers.sp = 0xFFFE;
            let cycles = cpu.execute_unprefixed(&mut bus, opcode as u8);
            assert!(
                cycles >= 4 && cycles <= 24,
                "Opcode {:#04X} returned invalid cycle count: {}",
                opcode,
                cycles
            );
        }
    }

    #[test]
    fn test_all_256_cb_decoding() {
        let mut cpu = Cpu::new();
        let mut bus = MockBus::new();

        for cb_op in 0..=255 {
            cpu.registers.pc = 0x0100;
            let cycles = cpu.execute_cb(&mut bus, cb_op as u8);
            assert!(
                cycles == 8 || cycles == 12 || cycles == 16,
                "CB Opcode {:#04X} returned invalid cycle count: {}",
                cb_op,
                cycles
            );
        }
    }

    #[test]
    fn test_conditional_jump_call_ret_cycles() {
        let mut cpu = Cpu::new();
        let mut bus = MockBus::new();

        // 1. JR NZ: Taken (12 T) vs Not Taken (8 T)
        cpu.registers.set_flag(Flag::Z, false);
        cpu.registers.pc = 0x0100;
        bus.write_byte(0x0100, 0x05); // offset = +5
        let taken_cycles = cpu.op_jr_conditional(&bus, Condition::NZ);
        assert_eq!(taken_cycles, 12);
        assert_eq!(cpu.registers.pc, 0x0106);

        cpu.registers.set_flag(Flag::Z, true);
        cpu.registers.pc = 0x0100;
        let not_taken_cycles = cpu.op_jr_conditional(&bus, Condition::NZ);
        assert_eq!(not_taken_cycles, 8);
        assert_eq!(cpu.registers.pc, 0x0101);

        // 2. JP NZ: Taken (16 T) vs Not Taken (12 T)
        cpu.registers.set_flag(Flag::Z, false);
        cpu.registers.pc = 0x0100;
        bus.write_word(0x0100, 0x1234);
        let jp_taken = cpu.op_jp_conditional(&bus, Condition::NZ);
        assert_eq!(jp_taken, 16);
        assert_eq!(cpu.registers.pc, 0x1234);

        cpu.registers.set_flag(Flag::Z, true);
        cpu.registers.pc = 0x0100;
        let jp_not_taken = cpu.op_jp_conditional(&bus, Condition::NZ);
        assert_eq!(jp_not_taken, 12);

        // 3. CALL NZ: Taken (24 T) vs Not Taken (12 T)
        cpu.registers.set_flag(Flag::Z, false);
        cpu.registers.sp = 0xFFFE;
        cpu.registers.pc = 0x0100;
        bus.write_word(0x0100, 0x2000);
        let call_taken = cpu.op_call_conditional(&mut bus, Condition::NZ);
        assert_eq!(call_taken, 24);
        assert_eq!(cpu.registers.pc, 0x2000);
        assert_eq!(cpu.registers.sp, 0xFFFC);

        cpu.registers.set_flag(Flag::Z, true);
        cpu.registers.pc = 0x0100;
        let call_not_taken = cpu.op_call_conditional(&mut bus, Condition::NZ);
        assert_eq!(call_not_taken, 12);

        // 4. RET NZ: Taken (20 T) vs Not Taken (8 T)
        cpu.registers.set_flag(Flag::Z, false);
        cpu.push_stack_16(&mut bus, 0x0500);
        let ret_taken = cpu.op_ret_conditional(&mut bus, Condition::NZ);
        assert_eq!(ret_taken, 20);
        assert_eq!(cpu.registers.pc, 0x0500);

        cpu.registers.set_flag(Flag::Z, true);
        let ret_not_taken = cpu.op_ret_conditional(&mut bus, Condition::NZ);
        assert_eq!(ret_not_taken, 8);
    }

    #[test]
    fn test_cb_hl_timing() {
        let mut cpu = Cpu::new();
        let mut bus = MockBus::new();

        cpu.registers.set_hl(0xC000);

        // BIT 0, (HL) -> 0x46 -> 12 T-cycles
        let bit_hl_cycles = cpu.execute_cb(&mut bus, 0x46);
        assert_eq!(bit_hl_cycles, 12);

        // SET 0, (HL) -> 0xC6 -> 16 T-cycles
        let set_hl_cycles = cpu.execute_cb(&mut bus, 0xC6);
        assert_eq!(set_hl_cycles, 16);

        // RLC (HL) -> 0x06 -> 16 T-cycles
        let rlc_hl_cycles = cpu.execute_cb(&mut bus, 0x06);
        assert_eq!(rlc_hl_cycles, 16);
    }

    #[test]
    fn test_pop_af_invariant() {
        let mut cpu = Cpu::new();
        let mut bus = MockBus::new();

        cpu.registers.sp = 0xFFFC;
        bus.write_word(0xFFFC, 0x12FF); // A = 0x12, F = 0xFF

        let cycles = cpu.execute_unprefixed(&mut bus, 0xF1); // POP AF
        assert_eq!(cycles, 12);
        assert_eq!(cpu.registers.a, 0x12);
        assert_eq!(cpu.registers.f, 0xF0); // Lower 4 bits zeroed!
    }
}
