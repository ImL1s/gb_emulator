#[cfg(test)]
mod tests {
    use crate::cpu::registers::Flag;
    use crate::cpu::{Cpu, ImeState};
    use crate::mmu::bus::{Bus, MockBus};

    /// Determine whether a conditional instruction is taken given the current flag register value.
    fn is_condition_met(opcode: u8, flags: u8) -> bool {
        let z = (flags & (Flag::Z as u8)) != 0;
        let c = (flags & (Flag::C as u8)) != 0;

        match opcode {
            // NZ: Not Zero
            0x20 | 0xC0 | 0xC2 | 0xC4 => !z,
            // Z: Zero
            0x28 | 0xC8 | 0xCA | 0xCC => z,
            // NC: No Carry
            0x30 | 0xD0 | 0xD2 | 0xD4 => !c,
            // C: Carry
            0x38 | 0xD8 | 0xDA | 0xDC => c,
            _ => true,
        }
    }

    /// Expected T-cycles for CB-prefixed instructions (including 0xCB prefix fetch).
    fn expected_cb_cycles(cb_op: u8) -> u32 {
        let is_hl = (cb_op & 0x07) == 6;
        let group = cb_op >> 6;
        if group == 1 {
            // BIT b, r8
            if is_hl {
                12
            } else {
                8
            }
        } else {
            // Rotates/Shifts/Swap (group 0), RES (group 2), SET (group 3)
            if is_hl {
                16
            } else {
                8
            }
        }
    }

    /// Expected T-cycles for unprefixed opcodes when executed via Cpu::step.
    fn expected_unprefixed_cycles(opcode: u8, flags: u8, sub_cb_op: u8) -> u32 {
        if opcode == 0xCB {
            return expected_cb_cycles(sub_cb_op);
        }

        let taken = is_condition_met(opcode, flags);

        match opcode {
            // NOP
            0x00 => 4,

            // 16-bit Loads & Indirect
            0x01 | 0x11 | 0x21 | 0x31 => 12,
            0x02 | 0x12 | 0x0A | 0x1A | 0x22 | 0x32 | 0x2A | 0x3A => 8,
            0x08 => 20,

            // 16-bit INC / DEC
            0x03 | 0x13 | 0x23 | 0x33 | 0x0B | 0x1B | 0x2B | 0x3B => 8,

            // 8-bit INC / DEC r8 vs (HL)
            0x04 | 0x0C | 0x14 | 0x1C | 0x24 | 0x2C | 0x3C => 4,
            0x34 => 12,
            0x05 | 0x0D | 0x15 | 0x1D | 0x25 | 0x2D | 0x3D => 4,
            0x35 => 12,

            // 8-bit Immediate Loads
            0x06 | 0x0E | 0x16 | 0x1E | 0x26 | 0x2E | 0x3E => 8,
            0x36 => 12,

            // Rotates A
            0x07 | 0x0F | 0x17 | 0x1F => 4,

            // ADD HL, r16
            0x09 | 0x19 | 0x29 | 0x39 => 8,

            // JR e8
            0x18 => 12,
            0x20 | 0x28 | 0x30 | 0x38 => {
                if taken {
                    12
                } else {
                    8
                }
            }

            // Special Math & Flags
            0x27 | 0x2F | 0x37 | 0x3F => 4,

            // Block 0x40..=0x7F: LD r8, r8' & HALT
            0x40..=0x7F => {
                if opcode == 0x76 {
                    4 // HALT
                } else if (opcode & 0x07 == 6) || ((opcode >> 3) & 0x07 == 6) {
                    8 // Memory access (HL)
                } else {
                    4
                }
            }

            // Block 0x80..=0xBF: ALU r8 / (HL)
            0x80..=0xBF => {
                if (opcode & 0x07) == 6 {
                    8
                } else {
                    4
                }
            }

            // Returns
            0xC0 | 0xC8 | 0xD0 | 0xD8 => {
                if taken {
                    20
                } else {
                    8
                }
            }
            0xC9 | 0xD9 => 16,

            // POP & PUSH
            0xC1 | 0xD1 | 0xE1 | 0xF1 => 12,
            0xC5 | 0xD5 | 0xE5 | 0xF5 => 16,

            // Jumps
            0xC2 | 0xCA | 0xD2 | 0xDA => {
                if taken {
                    16
                } else {
                    12
                }
            }
            0xC3 => 16,
            0xE9 => 4,

            // Calls
            0xC4 | 0xCC | 0xD4 | 0xDC => {
                if taken {
                    24
                } else {
                    12
                }
            }
            0xCD => 24,

            // Immediate ALU
            0xC6 | 0xCE | 0xD6 | 0xDE | 0xE6 | 0xEE | 0xF6 | 0xFE => 8,

            // RST vectors
            0xC7 | 0xCF | 0xD7 | 0xDF | 0xE7 | 0xEF | 0xF7 | 0xFF => 16,

            // I/O & High memory
            0xE0 | 0xF0 => 12,
            0xE2 | 0xF2 => 8,
            0xEA | 0xFA => 16,

            // SP Math
            0xE8 => 16,
            0xF8 => 12,
            0xF9 => 8,

            // Control
            0xF3 | 0xFB | 0x10 => 4,

            // CB Prefix & Unused / Illegal LR35902 opcodes
            0xCB | 0xD3 | 0xDB | 0xDD | 0xE3 | 0xE4 | 0xEB | 0xEC | 0xED | 0xF4 | 0xFC | 0xFD => 4,
        }
    }

    #[test]
    fn test_exhaustive_all_256_unprefixed_opcodes_cycles_taken() {
        let mut cpu = Cpu::new();
        let mut bus = MockBus::new();

        // Setup conditions: Z=1, C=1
        cpu.registers.set_flag(Flag::Z, true);
        cpu.registers.set_flag(Flag::C, true);
        let flags = cpu.registers.f;

        for op in 0..=255 {
            let opcode = op as u8;
            cpu.registers.f = flags; // Restore flags for consistent condition testing
            cpu.registers.pc = 0xC000;
            cpu.registers.sp = 0xFFFE;
            cpu.halted = false;
            cpu.stopped = false;

            bus.write_byte(0xC000, opcode);
            bus.write_byte(0xC001, 0x05); // Dummy immediate operand byte 1
            bus.write_byte(0xC002, 0x00); // Dummy immediate operand byte 2

            let expected = expected_unprefixed_cycles(opcode, flags, 0x05);
            let actual = cpu.step(&mut bus);

            assert_eq!(
                actual, expected,
                "Unprefixed Opcode {:#04X} cycle mismatch! Expected: {}, Got: {}",
                opcode, expected, actual
            );
        }
    }

    #[test]
    fn test_exhaustive_all_256_unprefixed_opcodes_cycles_not_taken() {
        let mut cpu = Cpu::new();
        let mut bus = MockBus::new();

        // Setup conditions: Z=0, C=0
        cpu.registers.set_flag(Flag::Z, false);
        cpu.registers.set_flag(Flag::C, false);
        let flags = cpu.registers.f;

        for op in 0..=255 {
            let opcode = op as u8;
            cpu.registers.f = flags; // Restore flags
            cpu.registers.pc = 0xC000;
            cpu.registers.sp = 0xFFFE;
            cpu.halted = false;
            cpu.stopped = false;

            bus.write_byte(0xC000, opcode);
            bus.write_byte(0xC001, 0x05); // Dummy immediate operand byte 1
            bus.write_byte(0xC002, 0x00); // Dummy immediate operand byte 2

            let expected = expected_unprefixed_cycles(opcode, flags, 0x05);
            let actual = cpu.step(&mut bus);

            assert_eq!(
                actual, expected,
                "Unprefixed Opcode {:#04X} cycle mismatch! Expected: {}, Got: {}",
                opcode, expected, actual
            );
        }
    }

    #[test]
    fn test_exhaustive_all_256_cb_opcodes_cycles_via_step() {
        let mut cpu = Cpu::new();
        let mut bus = MockBus::new();

        for op in 0..=255 {
            let cb_op = op as u8;
            cpu.registers.pc = 0xC000;
            cpu.registers.set_hl(0xD000);

            // Write 0xCB followed by sub-opcode
            bus.write_byte(0xC000, 0xCB);
            bus.write_byte(0xC001, cb_op);

            let expected = expected_cb_cycles(cb_op);
            let actual = cpu.step(&mut bus);

            assert_eq!(
                actual, expected,
                "CB Opcode {:#04X} (sub-op {:#04X}) cycle mismatch! Expected: {}, Got: {}",
                0xCB, cb_op, expected, actual
            );

            assert_eq!(
                cpu.registers.pc, 0xC002,
                "CB Opcode {:#04X} must advance PC by 2",
                cb_op
            );
        }
    }

    #[test]
    fn test_pc_wrapping_at_0xffff_boundary() {
        let mut cpu = Cpu::new();
        let mut bus = MockBus::new();

        // 1. Single byte instruction NOP at 0xFFFF -> PC wraps to 0x0000
        cpu.registers.pc = 0xFFFF;
        bus.write_byte(0xFFFF, 0x00); // NOP
        let cycles = cpu.step(&mut bus);
        assert_eq!(cycles, 4);
        assert_eq!(cpu.registers.pc, 0x0000);

        // 2. 3-byte instruction LD BC, n16 at 0xFFFF (0x01 at 0xFFFF, 0x34 at 0x0000, 0x12 at 0x0001)
        cpu.registers.pc = 0xFFFF;
        bus.write_byte(0xFFFF, 0x01); // LD BC, n16
        bus.write_byte(0x0000, 0x34); // Low byte
        bus.write_byte(0x0001, 0x12); // High byte

        let cycles = cpu.step(&mut bus);
        assert_eq!(cycles, 12);
        assert_eq!(cpu.registers.bc(), 0x1234);
        assert_eq!(cpu.registers.pc, 0x0002);

        // 3. JR offset wrapping backwards (PC=0x0000, JR -5 -> wraps to 0xFFFD)
        cpu.registers.pc = 0x0000;
        bus.write_byte(0x0000, 0x18); // JR e8
        bus.write_byte(0x0001, (-5i8) as u8); // offset = -5

        let cycles = cpu.step(&mut bus);
        assert_eq!(cycles, 12);
        assert_eq!(cpu.registers.pc, 0xFFFD);
    }

    #[test]
    fn test_stack_operations_underflow_overflow_wrapping() {
        let mut cpu = Cpu::new();
        let mut bus = MockBus::new();

        // 1. PUSH when SP = 0x0000
        cpu.registers.sp = 0x0000;
        cpu.push_stack_16(&mut bus, 0xABCD);
        assert_eq!(cpu.registers.sp, 0xFFFE);
        assert_eq!(bus.read_byte(0xFFFF), 0xAB); // High byte
        assert_eq!(bus.read_byte(0xFFFE), 0xCD); // Low byte

        // 2. POP when SP = 0xFFFE
        let val = cpu.pop_stack_16(&mut bus);
        assert_eq!(val, 0xABCD);
        assert_eq!(cpu.registers.sp, 0x0000);

        // 3. POP when SP = 0xFFFF -> Wraps low byte at 0xFFFF, high byte at 0x0000
        cpu.registers.sp = 0xFFFF;
        bus.write_byte(0xFFFF, 0x78);
        bus.write_byte(0x0000, 0x56);
        let val = cpu.pop_stack_16(&mut bus);
        assert_eq!(val, 0x5678);
        assert_eq!(cpu.registers.sp, 0x0001);
    }

    #[test]
    fn test_push_pop_af_flag_masking() {
        let mut cpu = Cpu::new();
        let mut bus = MockBus::new();

        // Explicitly write non-zero to lower 4 bits in stack memory
        cpu.registers.sp = 0xFFFE;
        bus.write_word(0xFFFE, 0x34FF); // A = 0x34, F = 0xFF

        cpu.registers.pc = 0xC000;
        bus.write_byte(0xC000, 0xF1); // POP AF
        cpu.step(&mut bus);

        assert_eq!(cpu.registers.a, 0x34);
        assert_eq!(cpu.registers.f, 0xF0, "F register lower 4 bits MUST be 0");
    }

    #[test]
    fn test_add_sp_i8_flags_and_math() {
        let mut cpu = Cpu::new();
        let mut bus = MockBus::new();

        // ADD SP, +1 (0xE8)
        cpu.registers.sp = 0x000F;
        cpu.registers.pc = 0xC000;
        bus.write_byte(0xC000, 0xE8);
        bus.write_byte(0xC001, 1);

        let cycles = cpu.step(&mut bus);
        assert_eq!(cycles, 16);
        assert_eq!(cpu.registers.sp, 0x0010);
        assert_eq!(cpu.registers.flag_z(), false); // Z must be 0
        assert_eq!(cpu.registers.flag_n(), false); // N must be 0
        assert_eq!(cpu.registers.flag_h(), true); // Half-carry from 0xF + 1
        assert_eq!(cpu.registers.flag_c(), false); // No carry from 0x0F + 1

        // LD HL, SP-1 (0xF8)
        cpu.registers.sp = 0x0100;
        cpu.registers.pc = 0xC000;
        bus.write_byte(0xC000, 0xF8);
        bus.write_byte(0xC001, (-1i8) as u8);

        let cycles = cpu.step(&mut bus);
        assert_eq!(cycles, 12);
        assert_eq!(cpu.registers.hl(), 0x00FF);
        assert_eq!(cpu.registers.sp, 0x0100); // SP unchanged by LD HL, SP+r8
    }

    #[test]
    fn test_unprefixed_rotates_always_clear_z_flag() {
        let mut cpu = Cpu::new();
        let mut bus = MockBus::new();

        // RLCA (0x07) with A = 0x00
        cpu.registers.a = 0x00;
        cpu.registers.set_flag(Flag::Z, true); // Set Z initially
        cpu.registers.pc = 0xC000;
        bus.write_byte(0xC000, 0x07);

        cpu.step(&mut bus);
        assert_eq!(cpu.registers.a, 0x00);
        assert_eq!(
            cpu.registers.flag_z(),
            false,
            "RLCA MUST clear Z flag even if A == 0"
        );

        // RLC A (CB 0x07) with A = 0x00
        cpu.registers.a = 0x00;
        cpu.registers.set_flag(Flag::Z, false);
        cpu.registers.pc = 0xC000;
        bus.write_byte(0xC000, 0xCB);
        bus.write_byte(0xC001, 0x07);

        cpu.step(&mut bus);
        assert_eq!(cpu.registers.a, 0x00);
        assert_eq!(
            cpu.registers.flag_z(),
            true,
            "CB RLC A MUST set Z flag if A == 0"
        );
    }

    #[test]
    fn test_reti_enables_ime_immediately() {
        let mut cpu = Cpu::new();
        let mut bus = MockBus::new();

        cpu.registers.pc = 0xC000;
        cpu.registers.sp = 0xFFFC;
        bus.write_word(0xFFFC, 0x1234); // Stack return PC
        bus.write_byte(0xC000, 0xD9); // RETI
        cpu.ime_state = ImeState::Disabled;

        let cycles = cpu.step(&mut bus);
        assert_eq!(cycles, 16);
        assert_eq!(cpu.registers.pc, 0x1234);
        assert_eq!(
            cpu.ime_state,
            ImeState::Enabled,
            "RETI must set IME to Enabled immediately"
        );
    }
}
