use gb_emulator::cpu::{Cpu, ImeState};
use gb_emulator::joypad::JoypadKey;
use gb_emulator::mmu::bus::Bus;
use gb_emulator::mmu::Mmu;

// ============================================================================
// 1. SERIAL PORT INTERCEPTOR EMPIRICAL STRESS TESTS
// ============================================================================

#[test]
fn challenge_serial_blargg_output_accumulation() {
    let mut mmu = Mmu::new();

    let test_string = "cpu_instrs\n\n01:ok  02:ok  03:ok\n\nPassed\n";

    for (idx, ch) in test_string.bytes().enumerate() {
        mmu.write_byte(0xFF01, ch);
        assert_eq!(mmu.read_byte(0xFF01), ch);

        // Writing 0x81 triggers transfer start (internal clock bit 0, start bit 7)
        mmu.write_byte(0xFF02, 0x81);

        // Bit 7 of SC should be reset immediately on completion
        assert_eq!(
            mmu.read_byte(0xFF02) & 0x80,
            0,
            "SC bit 7 must reset after transfer completion at index {}",
            idx
        );

        // Unused bits 1..6 of SC must read as 1 (0x7E mask)
        assert_eq!(
            mmu.read_byte(0xFF02) & 0x7E,
            0x7E,
            "SC unused bits 1..6 must read as 1"
        );

        // Verify accumulated output buffer step by step
        let current_expected = &test_string[..=idx];
        assert_eq!(mmu.get_serial_output(), current_expected);

        // Verify Serial Interrupt flag (IF bit 3) is set
        assert_ne!(
            mmu.read_byte(0xFF0F) & 0x08,
            0,
            "Serial interrupt (IF bit 3) must be set on transfer start"
        );

        // Clear IF bit 3 for next iteration
        let current_if = mmu.read_byte(0xFF0F);
        mmu.write_byte(0xFF0F, current_if & !0x08);
    }

    assert_eq!(mmu.get_serial_output(), test_string);
}

#[test]
fn challenge_serial_sc_bit7_reset_and_interrupt_triggering() {
    let mut mmu = Mmu::new();

    // 1. Write SB without setting bit 7 of SC
    mmu.write_byte(0xFF01, b'X');
    mmu.write_byte(0xFF02, 0x01); // Bit 7 is 0

    assert_eq!(mmu.get_serial_output(), "");
    assert_eq!(
        mmu.read_byte(0xFF0F) & 0x08,
        0,
        "Serial interrupt must NOT be requested if SC bit 7 is 0"
    );

    // 2. Overwrite SB with 'Y' and trigger transfer with 0x80 (external clock bit 0 is 0, start bit 7 is 1)
    mmu.write_byte(0xFF01, b'Y');
    mmu.write_byte(0xFF02, 0x80);

    assert_eq!(mmu.get_serial_output(), "Y");
    assert_eq!(mmu.read_byte(0xFF02) & 0x80, 0);
    assert_ne!(
        mmu.read_byte(0xFF0F) & 0x08,
        0,
        "Serial interrupt must be requested when SC bit 7 is written as 1"
    );

    // Clear serial output via MMU inner serial struct
    mmu.serial.clear_output();
    assert_eq!(mmu.get_serial_output(), "");

    // 3. Test `take_output`
    mmu.write_byte(0xFF01, b'Z');
    mmu.write_byte(0xFF02, 0x81);
    let taken = mmu.serial.take_output();
    assert_eq!(taken, "Z");
    assert_eq!(mmu.get_serial_output(), "");
}

// ============================================================================
// 2. JOYPAD MATRIX ACTIVE-LOW POLLING STRESS TESTS
// ============================================================================

#[test]
fn challenge_joypad_matrix_directional_selection() {
    let mut mmu = Mmu::new();

    // Select Directional keys: P15 (bit 5) = 1 (deselected), P14 (bit 4) = 0 (selected) -> 0x20
    mmu.write_byte(0xFF00, 0x20);

    // Verify initial state: all buttons unpressed -> lower 4 bits are 1111 (0x0F)
    assert_eq!(mmu.read_byte(0xFF00), 0xEF); // 0xC0 | 0x20 | 0x0F

    // Test Right (bit 0)
    mmu.press_key(JoypadKey::Right);
    assert_eq!(mmu.read_byte(0xFF00) & 0x0F, 0x0E); // Bit 0 = 0
    mmu.release_key(JoypadKey::Right);
    assert_eq!(mmu.read_byte(0xFF00) & 0x0F, 0x0F);

    // Test Left (bit 1)
    mmu.press_key(JoypadKey::Left);
    assert_eq!(mmu.read_byte(0xFF00) & 0x0F, 0x0D); // Bit 1 = 0
    mmu.release_key(JoypadKey::Left);

    // Test Up (bit 2)
    mmu.press_key(JoypadKey::Up);
    assert_eq!(mmu.read_byte(0xFF00) & 0x0F, 0x0B); // Bit 2 = 0
    mmu.release_key(JoypadKey::Up);

    // Test Down (bit 3)
    mmu.press_key(JoypadKey::Down);
    assert_eq!(mmu.read_byte(0xFF00) & 0x0F, 0x07); // Bit 3 = 0
    mmu.release_key(JoypadKey::Down);

    // Verify Action buttons have NO effect when Directional line selected
    mmu.press_key(JoypadKey::A);
    mmu.press_key(JoypadKey::B);
    mmu.press_key(JoypadKey::Select);
    mmu.press_key(JoypadKey::Start);
    assert_eq!(mmu.read_byte(0xFF00) & 0x0F, 0x0F);
}

#[test]
fn challenge_joypad_matrix_action_selection() {
    let mut mmu = Mmu::new();

    // Select Action keys: P15 (bit 5) = 0 (selected), P14 (bit 4) = 1 (deselected) -> 0x10
    mmu.write_byte(0xFF00, 0x10);

    // Verify initial state: all buttons unpressed -> 0xDF (0xC0 | 0x10 | 0x0F)
    assert_eq!(mmu.read_byte(0xFF00), 0xDF);

    // Test A (bit 0)
    mmu.press_key(JoypadKey::A);
    assert_eq!(mmu.read_byte(0xFF00) & 0x0F, 0x0E); // Bit 0 = 0
    mmu.release_key(JoypadKey::A);

    // Test B (bit 1)
    mmu.press_key(JoypadKey::B);
    assert_eq!(mmu.read_byte(0xFF00) & 0x0F, 0x0D); // Bit 1 = 0
    mmu.release_key(JoypadKey::B);

    // Test Select (bit 2)
    mmu.press_key(JoypadKey::Select);
    assert_eq!(mmu.read_byte(0xFF00) & 0x0F, 0x0B); // Bit 2 = 0
    mmu.release_key(JoypadKey::Select);

    // Test Start (bit 3)
    mmu.press_key(JoypadKey::Start);
    assert_eq!(mmu.read_byte(0xFF00) & 0x0F, 0x07); // Bit 3 = 0
    mmu.release_key(JoypadKey::Start);

    // Verify Directional buttons have NO effect when Action line selected
    mmu.press_key(JoypadKey::Right);
    mmu.press_key(JoypadKey::Left);
    mmu.press_key(JoypadKey::Up);
    mmu.press_key(JoypadKey::Down);
    assert_eq!(mmu.read_byte(0xFF00) & 0x0F, 0x0F);
}

#[test]
fn challenge_joypad_matrix_both_lines_selected() {
    let mut mmu = Mmu::new();

    // Select BOTH lines: P15 (bit 5) = 0, P14 (bit 4) = 0 -> 0x00
    mmu.write_byte(0xFF00, 0x00);

    // Press Right (Directional bit 0) -> lower nibble becomes 0x0E
    mmu.press_key(JoypadKey::Right);
    assert_eq!(mmu.read_byte(0xFF00) & 0x0F, 0x0E);

    // Release Right, press A (Action bit 0) -> lower nibble becomes 0x0E
    mmu.release_key(JoypadKey::Right);
    mmu.press_key(JoypadKey::A);
    assert_eq!(mmu.read_byte(0xFF00) & 0x0F, 0x0E);

    // Press BOTH Right AND A -> bit 0 remains 0x0E
    mmu.press_key(JoypadKey::Right);
    assert_eq!(mmu.read_byte(0xFF00) & 0x0F, 0x0E);

    // Press Left (Directional bit 1) AND Start (Action bit 3) -> bits 0, 1, 3 cleared -> 0x04
    mmu.press_key(JoypadKey::Left);
    mmu.press_key(JoypadKey::Start);
    assert_eq!(mmu.read_byte(0xFF00) & 0x0F, 0x04);
}

#[test]
fn challenge_joypad_matrix_neither_line_selected() {
    let mut mmu = Mmu::new();

    // Select NEITHER line: P15 (bit 5) = 1, P14 (bit 4) = 1 -> 0x30
    mmu.write_byte(0xFF00, 0x30);

    // Press ALL 8 buttons
    mmu.press_key(JoypadKey::Right);
    mmu.press_key(JoypadKey::Left);
    mmu.press_key(JoypadKey::Up);
    mmu.press_key(JoypadKey::Down);
    mmu.press_key(JoypadKey::A);
    mmu.press_key(JoypadKey::B);
    mmu.press_key(JoypadKey::Select);
    mmu.press_key(JoypadKey::Start);

    // JOYP must read 0xFF (bits 6-7 = 11, bits 4-5 = 11, bits 0-3 = 1111)
    assert_eq!(
        mmu.read_byte(0xFF00),
        0xFF,
        "When neither selection line is active, JOYP must read 0xFF regardless of pressed keys"
    );
}

// ============================================================================
// 3. HIGH-TO-LOW PIN TRANSITION INTERRUPT STRESS TESTS
// ============================================================================

#[test]
fn challenge_joypad_high_to_low_transition_interrupt() {
    let mut mmu = Mmu::new();

    // Clear IF register
    mmu.write_byte(0xFF0F, 0x00);

    // 1. Press key while line is active -> High-to-Low transition on pin -> IF bit 4 set
    mmu.write_byte(0xFF00, 0x20); // Select Directional
    mmu.press_key(JoypadKey::Up);

    assert_ne!(
        mmu.read_byte(0xFF0F) & 0x10,
        0,
        "Pressing key while selection line is active must request Joypad interrupt (IF bit 4)"
    );

    // Clear IF bit 4
    mmu.write_byte(0xFF0F, 0x00);

    // 2. Repeat press while key is already held -> no state change -> IF bit 4 NOT set
    mmu.press_key(JoypadKey::Up);
    assert_eq!(
        mmu.read_byte(0xFF0F) & 0x10,
        0,
        "Repeated key press while already held must NOT re-trigger interrupt"
    );

    // 3. Release key while line is active -> Low-to-High transition -> IF bit 4 NOT set
    mmu.release_key(JoypadKey::Up);
    assert_eq!(
        mmu.read_byte(0xFF0F) & 0x10,
        0,
        "Releasing key must NOT trigger interrupt"
    );

    // 4. Press key while selection line is INACTIVE (0x30) -> no pin change -> IF bit 4 NOT set
    mmu.write_byte(0xFF00, 0x30);
    mmu.press_key(JoypadKey::Down);
    assert_eq!(
        mmu.read_byte(0xFF0F) & 0x10,
        0,
        "Pressing key while selection line is inactive must NOT trigger interrupt"
    );

    // 5. Toggle selection line from inactive (0x30) to active (0x20) while key is held down ->
    // Pin transitions from High (1) to Low (0) -> IF bit 4 SET!
    mmu.write_byte(0xFF00, 0x20);
    assert_ne!(
        mmu.read_byte(0xFF0F) & 0x10,
        0,
        "Activating selection line while button is held must request Joypad interrupt"
    );

    // Clear IF bit 4
    mmu.write_byte(0xFF0F, 0x00);

    // 6. Toggle selection line from active (0x20) to inactive (0x30) while key is held down ->
    // Pin transitions from Low (0) to High (1) -> IF bit 4 NOT set!
    mmu.write_byte(0xFF00, 0x30);
    assert_eq!(
        mmu.read_byte(0xFF0F) & 0x10,
        0,
        "Deactivating selection line must NOT trigger interrupt"
    );
}

#[test]
fn challenge_joypad_line_switch_interrupt_dynamics() {
    let mut mmu = Mmu::new();
    mmu.write_byte(0xFF0F, 0x00);

    // Hold Select button (Action bit 2)
    mmu.press_key(JoypadKey::Select);

    // Currently Directional is selected (0x20). Directional bit 2 (Up) is NOT held.
    // So line bit 2 is currently 1 (High).
    mmu.write_byte(0xFF00, 0x20);
    mmu.write_byte(0xFF0F, 0x00); // Clear IF

    // Now switch from Directional (0x20) to Action (0x10).
    // Action bit 2 (Select) IS held!
    // So line bit 2 transitions from 1 (High) to 0 (Low)!
    mmu.write_byte(0xFF00, 0x10);

    assert_ne!(
        mmu.read_byte(0xFF0F) & 0x10,
        0,
        "Switching selection line to a line with a pressed button must trigger Joypad interrupt"
    );
}

// ============================================================================
// 4. CPU & MMU INTEGRATED INTERRUPT DISPATCH & WAKEUP TESTS
// ============================================================================

#[test]
fn challenge_cpu_halt_wakeup_and_serial_joypad_interrupt_vectors() {
    let mut cpu = Cpu::new();
    let mut mmu = Mmu::new();

    cpu.registers.sp = 0xFFFE;
    cpu.registers.pc = 0x0100;
    cpu.ime_state = ImeState::Enabled;

    // Enable Serial (bit 3) and Joypad (bit 4) interrupts in IE (0xFFFF)
    mmu.write_byte(0xFFFF, 0x18); // 0x08 | 0x10

    // ------------------------------------------------------------------------
    // A. Serial Interrupt Vector Dispatch (0x0058)
    // ------------------------------------------------------------------------
    mmu.write_byte(0xFF01, b'K');
    mmu.write_byte(0xFF02, 0x81); // Request Serial Interrupt

    assert_ne!(mmu.read_byte(0xFF0F) & 0x08, 0);

    let cycles = cpu.step(&mut mmu);
    assert_eq!(cycles, 20, "Interrupt dispatch must take 20 T-cycles");
    assert_eq!(cpu.registers.pc, 0x0058, "Serial interrupt vector must be 0x0058");
    assert_eq!(mmu.read_byte(0xFF0F) & 0x08, 0, "Serial IF bit 3 must be cleared");

    // ------------------------------------------------------------------------
    // B. Joypad Interrupt Vector Dispatch (0x0060) & HALT Wakeup
    // ------------------------------------------------------------------------
    cpu.ime_state = ImeState::Enabled;
    cpu.registers.pc = 0x0200;
    cpu.halted = true; // Put CPU into HALT state

    mmu.write_byte(0xFF00, 0x20); // Select Directional
    mmu.press_key(JoypadKey::A);  // Action key pressed -> no transition for Directional
    assert!(cpu.halted);

    mmu.press_key(JoypadKey::Right); // Directional key pressed -> transition! IF bit 4 set
    assert_ne!(mmu.read_byte(0xFF0F) & 0x10, 0);

    let halt_cycles = cpu.step(&mut mmu);
    assert_eq!(halt_cycles, 20);
    assert!(!cpu.halted, "CPU must wake up from HALT on Joypad interrupt");
    assert_eq!(cpu.registers.pc, 0x0060, "Joypad interrupt vector must be 0x0060");
    assert_eq!(mmu.read_byte(0xFF0F) & 0x10, 0, "Joypad IF bit 4 must be cleared");
}
