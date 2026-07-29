use gb_emulator::cpu::alu::Alu;
use gb_emulator::cpu::registers::Flag;
use gb_emulator::cpu::{Cpu, ImeState};
use gb_emulator::mmu::bus::{Bus, MockBus};

// ============================================================================
// 1. DAA Calculation Across BCD Edge Cases
// ============================================================================

/// Reference Game Boy LR35902 DAA truth table oracle function.
fn reference_gb_daa(a: u8, flags: u8) -> (u8, u8) {
    let n = (flags & (Flag::N as u8)) != 0;
    let h = (flags & (Flag::H as u8)) != 0;
    let mut c = (flags & (Flag::C as u8)) != 0;
    let mut correction = 0u8;

    if !n {
        if h || (a & 0x0F) > 0x09 {
            correction |= 0x06;
        }
        if c || a > 0x99 {
            correction |= 0x60;
            c = true;
        }
    } else {
        if h {
            correction |= 0x06;
        }
        if c {
            correction |= 0x60;
        }
    }

    let res = if !n {
        a.wrapping_add(correction)
    } else {
        a.wrapping_sub(correction)
    };

    let mut res_flags = 0u8;
    if res == 0 {
        res_flags |= Flag::Z as u8;
    }
    if n {
        res_flags |= Flag::N as u8;
    }
    // H flag is always cleared on Game Boy DAA
    if c {
        res_flags |= Flag::C as u8;
    }

    (res, res_flags)
}

#[test]
fn stress_test_daa_exhaustive_oracle_4096_states() {
    // Test all 256 accumulator values x 16 flag combinations (Z, N, H, C)
    let flag_combinations = [
        0,
        Flag::Z as u8,
        Flag::N as u8,
        Flag::H as u8,
        Flag::C as u8,
        (Flag::Z as u8) | (Flag::N as u8),
        (Flag::Z as u8) | (Flag::H as u8),
        (Flag::Z as u8) | (Flag::C as u8),
        (Flag::N as u8) | (Flag::H as u8),
        (Flag::N as u8) | (Flag::C as u8),
        (Flag::H as u8) | (Flag::C as u8),
        (Flag::Z as u8) | (Flag::N as u8) | (Flag::H as u8),
        (Flag::Z as u8) | (Flag::N as u8) | (Flag::C as u8),
        (Flag::Z as u8) | (Flag::H as u8) | (Flag::C as u8),
        (Flag::N as u8) | (Flag::H as u8) | (Flag::C as u8),
        (Flag::Z as u8) | (Flag::N as u8) | (Flag::H as u8) | (Flag::C as u8),
    ];

    for a in 0..=255u8 {
        for &in_flags in &flag_combinations {
            let res = Alu::daa(a, in_flags);
            let (expected_val, expected_flags) = reference_gb_daa(a, in_flags);

            assert_eq!(
                res.value, expected_val,
                "DAA value mismatch for A={:#04X}, flags={:#04X}. Expected {:#04X}, got {:#04X}",
                a, in_flags, expected_val, res.value
            );
            assert_eq!(
                res.flags, expected_flags,
                "DAA flags mismatch for A={:#04X}, flags={:#04X}. Expected {:#04X}, got {:#04X}",
                a, in_flags, expected_flags, res.flags
            );
        }
    }
}

#[test]
fn stress_test_daa_bcd_addition_subtraction_sequences() {
    // Test realistic BCD addition sequences (e.g. 15 + 27 = 42)
    let add_cases = [
        (0x15u8, 0x27u8, 0x42u8, false), // 15 + 27 = 42
        (0x99, 0x01, 0x00, true),        // 99 + 01 = 00 (Carry = 1)
        (0x58, 0x46, 0x04, true),        // 58 + 46 = 104 -> 04 (Carry = 1)
        (0x00, 0x00, 0x00, false),       // 0 + 0 = 0
        (0x45, 0x55, 0x00, true),        // 45 + 55 = 100 -> 00 (Carry = 1)
        (0x09, 0x09, 0x18, false),       // 9 + 9 = 18
    ];

    for (x, y, expected_bcd, expected_c) in add_cases {
        let add_res = Alu::add_8(x, y);
        let daa_res = Alu::daa(add_res.value, add_res.flags);

        assert_eq!(
            daa_res.value, expected_bcd,
            "BCD Add failed for {:#04X} + {:#04X}: expected {:#04X}, got {:#04X}",
            x, y, expected_bcd, daa_res.value
        );
        assert_eq!(
            (daa_res.flags & (Flag::C as u8)) != 0,
            expected_c,
            "BCD Add Carry mismatch for {:#04X} + {:#04X}",
            x,
            y
        );
        assert_eq!(daa_res.flags & (Flag::H as u8), 0, "DAA must clear H flag");
    }

    // Test realistic BCD subtraction sequences (e.g. 42 - 27 = 15)
    let sub_cases = [
        (0x42u8, 0x27u8, 0x15u8, false), // 42 - 27 = 15
        (0x00, 0x01, 0x99, true),        // 00 - 01 = 99 (Borrow/Carry = 1)
        (0x50, 0x01, 0x49, false),       // 50 - 01 = 49
        (0x99, 0x99, 0x00, false),       // 99 - 99 = 00
    ];

    for (x, y, expected_bcd, expected_c) in sub_cases {
        let sub_res = Alu::sub_8(x, y);
        let daa_res = Alu::daa(sub_res.value, sub_res.flags);

        assert_eq!(
            daa_res.value, expected_bcd,
            "BCD Sub failed for {:#04X} - {:#04X}: expected {:#04X}, got {:#04X}",
            x, y, expected_bcd, daa_res.value
        );
        assert_eq!(
            (daa_res.flags & (Flag::C as u8)) != 0,
            expected_c,
            "BCD Sub Carry mismatch for {:#04X} - {:#04X}",
            x,
            y
        );
        assert_eq!(daa_res.flags & (Flag::H as u8), 0, "DAA must clear H flag");
    }
}

// ============================================================================
// 2. F Register Lower 4 Bits Zeroing Invariant
// ============================================================================

#[test]
fn stress_test_f_register_lower_4_bits_zero_across_all_opcodes() {
    let mut cpu = Cpu::new();
    let mut bus = MockBus::new();

    // 1. Verify set_f, set_af, set_flag invariants for all 256 byte values
    for val in 0..=255u8 {
        cpu.registers.set_f(val);
        assert_eq!(
            cpu.registers.f & 0x0F,
            0,
            "set_f({:#04X}) failed lower 4 bits zero invariant",
            val
        );
    }

    for word in 0..=65535u16 {
        cpu.registers.set_af(word);
        assert_eq!(
            cpu.registers.f & 0x0F,
            0,
            "set_af({:#06X}) failed lower 4 bits zero invariant",
            word
        );
    }

    // 2. Unprefixed opcodes: Execute all 256 opcodes with set_f(0xFF) before execution
    for opcode in 0..=255u8 {
        cpu.registers.pc = 0x0100;
        cpu.registers.sp = 0xFFFE;
        cpu.registers.set_f(0xFF); // Sets F to 0xF0 via setter

        bus.memory.fill(0x00);
        bus.write_byte(0x0100, opcode);

        cpu.step(&mut bus);

        assert_eq!(
            cpu.registers.f & 0x0F,
            0,
            "Unprefixed Opcode {:#04X} violated lower 4 bits zero invariant! F={:#04X}",
            opcode,
            cpu.registers.f
        );
    }

    // 3. CB-prefixed opcodes: Execute all 256 CB opcodes with set_f(0xFF) before execution
    for cb_op in 0..=255u8 {
        cpu.registers.pc = 0x0100;
        cpu.registers.sp = 0xFFFE;
        cpu.registers.set_f(0xFF);

        bus.memory.fill(0x00);
        bus.write_byte(0x0100, 0xCB);
        bus.write_byte(0x0101, cb_op);

        cpu.step(&mut bus);

        assert_eq!(
            cpu.registers.f & 0x0F,
            0,
            "CB Opcode {:#04X} violated lower 4 bits zero invariant! F={:#04X}",
            cb_op,
            cpu.registers.f
        );
    }
}

#[test]
fn stress_test_pop_af_lower_4_bits_masking() {
    let mut cpu = Cpu::new();
    let mut bus = MockBus::new();

    let dirty_stack_values = [
        0x12FFu16, 0x567F, 0xABCD, 0x000F, 0xFFFF, 0x1234, 0x5678, 0x9ABC, 0xDEF0,
    ];

    for &stack_val in &dirty_stack_values {
        cpu.registers.sp = 0xFFFC;
        bus.write_word(0xFFFC, stack_val);
        bus.write_byte(0x0100, 0xF1); // POP AF
        cpu.registers.pc = 0x0100;

        cpu.step(&mut bus);

        let expected_a = (stack_val >> 8) as u8;
        let expected_f = (stack_val as u8) & 0xF0;

        assert_eq!(
            cpu.registers.a, expected_a,
            "POP AF A register mismatch for stack value {:#06X}",
            stack_val
        );
        assert_eq!(
            cpu.registers.f, expected_f,
            "POP AF F register failed to zero lower 4 bits for stack value {:#06X}. Expected {:#04X}, got {:#04X}",
            stack_val, expected_f, cpu.registers.f
        );
        assert_eq!(cpu.registers.f & 0x0F, 0, "POP AF lower 4 bits non-zero");
    }
}

// ============================================================================
// 3. EI 1-Instruction Delay Edge Cases
// ============================================================================

#[test]
fn stress_test_ei_delay_followed_by_di() {
    let mut cpu = Cpu::new();
    let mut bus = MockBus::new();

    // Sequence: EI (0xFB), DI (0xF3), NOP (0x00)
    bus.write_byte(0x0100, 0xFB); // EI
    bus.write_byte(0x0101, 0xF3); // DI
    bus.write_byte(0x0102, 0x00); // NOP
    cpu.registers.pc = 0x0100;
    cpu.ime_state = ImeState::Disabled;

    // Set pending interrupt
    bus.write_byte(0xFFFF, 0x01);
    bus.write_byte(0xFF0F, 0x01);

    // Step 1: EI (0xFB)
    cpu.step(&mut bus);
    assert_eq!(cpu.ime_state, ImeState::PendingEnable);

    // Step 2: DI (0xF3) directly after EI
    // Interrupt must NOT be serviced here because IME is PendingEnable, not Enabled
    let cycles = cpu.step(&mut bus);
    assert_eq!(cycles, 4); // Executed DI, not interrupt handler (20 cycles)
    assert_eq!(cpu.ime_state, ImeState::Disabled);
    assert_eq!(cpu.registers.pc, 0x0102);

    // Step 3: NOP (0x0102). IME stays Disabled.
    let cycles3 = cpu.step(&mut bus);
    assert_eq!(cycles3, 4);
    assert_eq!(cpu.ime_state, ImeState::Disabled);
    assert_eq!(cpu.registers.pc, 0x0103);
}

#[test]
fn stress_test_ei_delay_followed_by_halt() {
    // Case 1: Pending interrupt present when EI -> HALT is executed
    let mut cpu = Cpu::new();
    let mut bus = MockBus::new();

    bus.write_byte(0x0100, 0xFB); // EI
    bus.write_byte(0x0101, 0x76); // HALT
    bus.write_byte(0x0102, 0x00); // NOP
    cpu.registers.pc = 0x0100;
    cpu.ime_state = ImeState::Disabled;

    // Pending VBlank interrupt
    bus.write_byte(0xFFFF, 0x01);
    bus.write_byte(0xFF0F, 0x01);

    // Step 1: EI
    cpu.step(&mut bus);
    assert_eq!(cpu.ime_state, ImeState::PendingEnable);

    // Step 2: HALT. During this step, ime_state advances to Enabling -> Enabled.
    cpu.step(&mut bus);

    // Step 3: Next step sees IME Enabled and pending interrupt -> dispatches interrupt to 0x0040!
    let cycles = cpu.step(&mut bus);
    assert_eq!(cycles, 20);
    assert_eq!(cpu.registers.pc, 0x0040);
    assert_eq!(cpu.ime_state, ImeState::Disabled);
}

#[test]
fn stress_test_ei_followed_by_ei_sequence() {
    let mut cpu = Cpu::new();
    let mut bus = MockBus::new();

    bus.write_byte(0x0100, 0xFB); // EI
    bus.write_byte(0x0101, 0xFB); // EI
    bus.write_byte(0x0102, 0x00); // NOP
    bus.write_byte(0x0103, 0x00); // NOP
    cpu.registers.pc = 0x0100;
    cpu.ime_state = ImeState::Disabled;

    // Step 1: EI -> PendingEnable
    cpu.step(&mut bus);
    assert_eq!(cpu.ime_state, ImeState::PendingEnable);

    // Step 2: EI -> PendingEnable (state advances to Enabling, but EI sets PendingEnable)
    cpu.step(&mut bus);
    assert_eq!(cpu.ime_state, ImeState::PendingEnable);

    // Step 3: NOP -> Enabling -> Enabled
    cpu.step(&mut bus);
    assert_eq!(cpu.ime_state, ImeState::Enabled);
}

// ============================================================================
// 4. HALT Bug Behavior
// ============================================================================

#[test]
fn stress_test_halt_bug_single_byte_opcode_duplication() {
    let mut cpu = Cpu::new();
    let mut bus = MockBus::new();

    // Condition: IME = Disabled, pending interrupt present
    cpu.registers.pc = 0x0100;
    cpu.ime_state = ImeState::Disabled;
    bus.write_byte(0xFFFF, 0x01);
    bus.write_byte(0xFF0F, 0x01);

    // 0x0100: HALT (0x76)
    // 0x0101: INC B (0x04)
    bus.write_byte(0x0100, 0x76);
    bus.write_byte(0x0101, 0x04);

    // Step 1: Execute HALT. Triggers HALT bug (halted=false, halt_bug=true, PC=0x0101)
    let c1 = cpu.step(&mut bus);
    assert_eq!(c1, 4);
    assert_eq!(cpu.halted, false);
    assert_eq!(cpu.halt_bug, true);
    assert_eq!(cpu.registers.pc, 0x0101);

    // Step 2: Execute INC B at 0x0101. PC is NOT incremented due to halt_bug. PC stays 0x0101.
    let c2 = cpu.step(&mut bus);
    assert_eq!(c2, 4);
    assert_eq!(cpu.registers.b, 1);
    assert_eq!(cpu.halt_bug, false);
    assert_eq!(cpu.registers.pc, 0x0101); // PC duplicated!

    // Step 3: Execute INC B at 0x0101 AGAIN. PC increments normally to 0x0102.
    let c3 = cpu.step(&mut bus);
    assert_eq!(c3, 4);
    assert_eq!(cpu.registers.b, 2);
    assert_eq!(cpu.registers.pc, 0x0102);
}

#[test]
fn stress_test_halt_bug_multibyte_immediate_opcode() {
    let mut cpu = Cpu::new();
    let mut bus = MockBus::new();

    // Condition: IME = Disabled, pending interrupt present
    cpu.registers.pc = 0x0100;
    cpu.ime_state = ImeState::Disabled;
    bus.write_byte(0xFFFF, 0x01);
    bus.write_byte(0xFF0F, 0x01);

    // 0x0100: HALT (0x76)
    // 0x0101: LD B, 0x42 (0x06 0x42)
    // 0x0103: NOP (0x00)
    bus.write_byte(0x0100, 0x76);
    bus.write_byte(0x0101, 0x06); // LD B, n8
    bus.write_byte(0x0102, 0x42);
    bus.write_byte(0x0103, 0x00);

    // Step 1: Execute HALT -> halt_bug=true, PC=0x0101
    cpu.step(&mut bus);
    assert_eq!(cpu.halt_bug, true);
    assert_eq!(cpu.registers.pc, 0x0101);

    // Step 2: Execute LD B, n8 (0x06).
    // Opcode fetch reads 0x06 from 0x0101, halt_bug clears, PC stays 0x0101.
    // LD B, n8 fetches operand via fetch_byte from PC=0x0101, reading 0x06 as operand!
    // B becomes 0x06, PC becomes 0x0102.
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.b, 0x06);
    assert_eq!(cpu.registers.pc, 0x0102);

    // Step 3: Next step fetches opcode at 0x0102 (0x42 -> LD B, D)
    cpu.registers.d = 0x99;
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.b, 0x99); // Executed 0x42 (LD B, D)
    assert_eq!(cpu.registers.pc, 0x0103);
}

#[test]
fn stress_test_halt_bug_cb_prefixed_opcode() {
    let mut cpu = Cpu::new();
    let mut bus = MockBus::new();

    // Condition: IME = Disabled, pending interrupt present
    cpu.registers.pc = 0x0100;
    cpu.ime_state = ImeState::Disabled;
    bus.write_byte(0xFFFF, 0x01);
    bus.write_byte(0xFF0F, 0x01);

    // 0x0100: HALT (0x76)
    // 0x0101: CB 0x30 (SWAP B: 0xCB 0x30)
    bus.write_byte(0x0100, 0x76);
    bus.write_byte(0x0101, 0xCB);
    bus.write_byte(0x0102, 0x30);

    // Step 1: HALT
    cpu.step(&mut bus);
    assert_eq!(cpu.halt_bug, true);
    assert_eq!(cpu.registers.pc, 0x0101);

    // Step 2: CB instruction fetch
    // Reads 0xCB at 0x0101, halt_bug clears, PC stays 0x0101.
    // CB handler fetches CB sub-opcode via fetch_byte from PC=0x0101, reading 0xCB!
    // 0xCB in CB sub-opcodes is SET 1, E (bit 1 of E set to 1).
    // PC becomes 0x0102.
    cpu.registers.e = 0x00;
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.e, 0x02); // Executed SET 1, E (0xCB sub-opcode)
    assert_eq!(cpu.registers.pc, 0x0102);
}

// ============================================================================
// 5. Interrupt Priority and 20 T-Cycle Vector Jumping
// ============================================================================

#[test]
fn stress_test_interrupt_priority_order_all_5_pending() {
    let mut cpu = Cpu::new();
    let mut bus = MockBus::new();

    // Enable all 5 interrupts in IE and request all 5 in IF
    bus.write_byte(0xFFFF, 0x1F); // IE
    bus.write_byte(0xFF0F, 0x1F); // IF

    let expected_sequence = [
        (0, 0x0040u16, 0x1Eu8), // 1. VBlank (bit 0) -> vector 0x0040, remaining IF 0x1E
        (1, 0x0048u16, 0x1Cu8), // 2. STAT   (bit 1) -> vector 0x0048, remaining IF 0x1C
        (2, 0x0050u16, 0x18u8), // 3. Timer  (bit 2) -> vector 0x0050, remaining IF 0x18
        (3, 0x0058u16, 0x10u8), // 4. Serial (bit 3) -> vector 0x0058, remaining IF 0x10
        (4, 0x0060u16, 0x00u8), // 5. Joypad (bit 4) -> vector 0x0060, remaining IF 0x00
    ];

    for (step_idx, (_bit, vector, remaining_if)) in expected_sequence.iter().enumerate() {
        cpu.ime_state = ImeState::Enabled;
        cpu.registers.pc = 0x0200 + (step_idx as u16) * 0x10;
        cpu.registers.sp = 0xFFFE;

        let cycles = cpu.step(&mut bus);

        assert_eq!(
            cycles, 20,
            "Step {}: Interrupt dispatch must take exactly 20 T-cycles",
            step_idx
        );
        assert_eq!(
            cpu.registers.pc, *vector,
            "Step {}: Expected jump to vector {:#06X}, got {:#06X}",
            step_idx, vector, cpu.registers.pc
        );
        assert_eq!(
            bus.read_byte(0xFF0F),
            *remaining_if,
            "Step {}: Expected remaining IF {:#04X}, got {:#04X}",
            step_idx,
            remaining_if,
            bus.read_byte(0xFF0F)
        );
        assert_eq!(
            cpu.ime_state,
            ImeState::Disabled,
            "Step {}: IME must be disabled after dispatch",
            step_idx
        );

        // Verify stack return address
        let return_pc = bus.read_word(cpu.registers.sp);
        assert_eq!(
            return_pc,
            0x0200 + (step_idx as u16) * 0x10,
            "Step {}: Stack return PC mismatch",
            step_idx
        );
    }
}

#[test]
fn stress_test_interrupt_masked_by_ie_register() {
    let mut cpu = Cpu::new();
    let mut bus = MockBus::new();

    // All interrupts requested in IF (0x1F), but ONLY Timer (bit 2) enabled in IE (0x04)
    bus.write_byte(0xFF0F, 0x1F); // IF = all 5 requested
    bus.write_byte(0xFFFF, 0x04); // IE = only Timer (bit 2) enabled

    cpu.ime_state = ImeState::Enabled;
    cpu.registers.pc = 0x0300;
    cpu.registers.sp = 0xFFFE;

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 20);
    assert_eq!(cpu.registers.pc, 0x0050, "Must jump to Timer vector 0x0050");
    assert_eq!(
        bus.read_byte(0xFF0F),
        0x1B,
        "Timer bit 2 must be cleared, leaving IF = 0x1B"
    );
}

#[test]
fn stress_test_interrupt_stack_push_layout() {
    let mut cpu = Cpu::new();
    let mut bus = MockBus::new();

    cpu.registers.pc = 0x1234;
    cpu.registers.sp = 0xC000;
    cpu.ime_state = ImeState::Enabled;

    bus.write_byte(0xFFFF, 0x01); // VBlank enabled
    bus.write_byte(0xFF0F, 0x01); // VBlank requested

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 20);
    assert_eq!(cpu.registers.sp, 0xBFFE);
    // Stack layout: SP-1 (0xBFFF) = High byte (0x12), SP-2 (0xBFFE) = Low byte (0x34)
    assert_eq!(bus.read_byte(0xBFFF), 0x12);
    assert_eq!(bus.read_byte(0xBFFE), 0x34);
    assert_eq!(bus.read_word(0xBFFE), 0x1234);
}
