use gb_emulator::cartridge::create_cartridge;
use gb_emulator::cpu::{Cpu, ImeState};
use gb_emulator::mmu::bus::Bus;
use gb_emulator::mmu::Mmu;

fn build_dummy_rom_with_vector(vector_addr: usize, opcode: u8) -> Vec<u8> {
    let mut rom = vec![0u8; 32768];
    let title = b"TESTGAME";
    rom[0x0134..0x0134 + title.len()].copy_from_slice(title);
    let mut chk: u8 = 0;
    for &b in &rom[0x0134..=0x014C] {
        chk = chk.wrapping_sub(b).wrapping_sub(1);
    }
    rom[0x014D] = chk;
    rom[vector_addr] = opcode;
    rom
}

// ============================================================================
// 1. TAC Clock Selection Frequencies (4096 Hz, 262144 Hz, 65536 Hz, 16384 Hz)
// ============================================================================

#[test]
fn challenge_timer_frequency_4096_hz() {
    let mut mmu = Mmu::new();
    // TAC bit 2 = 1 (enable), bits 0-1 = 00 (4096 Hz, bit 9, 1024 T-cycles)
    mmu.write_byte(0xFF07, 0x04);
    assert_eq!(mmu.read_byte(0xFF05), 0);

    // Step 1023 T-cycles -> TIMA should remain 0
    mmu.step_components(1023);
    assert_eq!(mmu.read_byte(0xFF05), 0);

    // 1024th T-cycle -> falling edge on DIV bit 9 -> TIMA increments to 1
    mmu.step_components(1);
    assert_eq!(mmu.read_byte(0xFF05), 1);

    // Step another 1024 T-cycles -> TIMA increments to 2
    mmu.step_components(1024);
    assert_eq!(mmu.read_byte(0xFF05), 2);
}

#[test]
fn challenge_timer_frequency_262144_hz() {
    let mut mmu = Mmu::new();
    // TAC bit 2 = 1, bits 0-1 = 01 (262144 Hz, bit 3, 16 T-cycles)
    mmu.write_byte(0xFF07, 0x05);
    assert_eq!(mmu.read_byte(0xFF05), 0);

    // Step 15 T-cycles -> TIMA remains 0
    mmu.step_components(15);
    assert_eq!(mmu.read_byte(0xFF05), 0);

    // 16th T-cycle -> TIMA increments to 1
    mmu.step_components(1);
    assert_eq!(mmu.read_byte(0xFF05), 1);

    // Step 160 T-cycles (10 ticks) -> TIMA becomes 11
    mmu.step_components(160);
    assert_eq!(mmu.read_byte(0xFF05), 11);
}

#[test]
fn challenge_timer_frequency_65536_hz() {
    let mut mmu = Mmu::new();
    // TAC bit 2 = 1, bits 0-1 = 10 (65536 Hz, bit 5, 64 T-cycles)
    mmu.write_byte(0xFF07, 0x06);
    assert_eq!(mmu.read_byte(0xFF05), 0);

    // Step 63 T-cycles -> TIMA remains 0
    mmu.step_components(63);
    assert_eq!(mmu.read_byte(0xFF05), 0);

    // 64th T-cycle -> TIMA increments to 1
    mmu.step_components(1);
    assert_eq!(mmu.read_byte(0xFF05), 1);

    // Step 640 T-cycles (10 ticks) -> TIMA becomes 11
    mmu.step_components(640);
    assert_eq!(mmu.read_byte(0xFF05), 11);
}

#[test]
fn challenge_timer_frequency_16384_hz() {
    let mut mmu = Mmu::new();
    // TAC bit 2 = 1, bits 0-1 = 11 (16384 Hz, bit 7, 256 T-cycles)
    mmu.write_byte(0xFF07, 0x07);
    assert_eq!(mmu.read_byte(0xFF05), 0);

    // Step 255 T-cycles -> TIMA remains 0
    mmu.step_components(255);
    assert_eq!(mmu.read_byte(0xFF05), 0);

    // 256th T-cycle -> TIMA increments to 1
    mmu.step_components(1);
    assert_eq!(mmu.read_byte(0xFF05), 1);

    // Step 2560 T-cycles (10 ticks) -> TIMA becomes 11
    mmu.step_components(2560);
    assert_eq!(mmu.read_byte(0xFF05), 11);
}

#[test]
fn challenge_timer_disabled_does_not_tick() {
    let mut mmu = Mmu::new();
    // TAC bit 2 = 0 (disabled)
    mmu.write_byte(0xFF07, 0x00);

    mmu.step_components(50000);
    assert_eq!(mmu.read_byte(0xFF05), 0);
    // DIV register (0xFF04) still increments (div_counter >> 8)
    assert_eq!(mmu.read_byte(0xFF04), (50000 >> 8) as u8);
}

// ============================================================================
// 2. DIV Write Reset Glitch Behavior
// ============================================================================

#[test]
fn challenge_div_write_reset_glitch_when_selected_bit_is_high() {
    let mut mmu = Mmu::new();
    mmu.write_byte(0xFF07, 0x04); // Enabled, 4096 Hz (bit 9)

    // Step 512 T-cycles -> DIV internal bit 9 becomes 1 (signal is true)
    mmu.step_components(512);
    assert_eq!(mmu.read_byte(0xFF05), 0);

    // Writing ANY value to DIV (0xFF04) resets internal div_counter to 0.
    // Transition from signal=true to signal=false causes a falling edge -> TIMA increments!
    mmu.write_byte(0xFF04, 0x00);
    assert_eq!(mmu.read_byte(0xFF05), 1);

    // Writing DIV again when div_counter is 0 (bit 9 is 0, signal false -> false)
    // Should NOT cause another increment
    mmu.write_byte(0xFF04, 0xFF);
    assert_eq!(mmu.read_byte(0xFF05), 1);
}

#[test]
fn challenge_div_write_reset_glitch_across_all_frequencies() {
    // Mode 01: bit 3 (16 cycles, high at 8)
    let mut mmu1 = Mmu::new();
    mmu1.write_byte(0xFF07, 0x05);
    mmu1.step_components(8);
    mmu1.write_byte(0xFF04, 0x00);
    assert_eq!(mmu1.read_byte(0xFF05), 1);

    // Mode 10: bit 5 (64 cycles, high at 32)
    let mut mmu2 = Mmu::new();
    mmu2.write_byte(0xFF07, 0x06);
    mmu2.step_components(32);
    mmu2.write_byte(0xFF04, 0x00);
    assert_eq!(mmu2.read_byte(0xFF05), 1);

    // Mode 11: bit 7 (256 cycles, high at 128)
    let mut mmu3 = Mmu::new();
    mmu3.write_byte(0xFF07, 0x07);
    mmu3.step_components(128);
    mmu3.write_byte(0xFF04, 0x00);
    assert_eq!(mmu3.read_byte(0xFF05), 1);
}

#[test]
fn challenge_div_write_reset_glitch_triggers_overflow_and_interrupt() {
    let mut mmu = Mmu::new();
    mmu.write_byte(0xFF07, 0x04); // Enabled, bit 9
    mmu.write_byte(0xFF05, 0xFF); // TIMA = 0xFF
    mmu.write_byte(0xFF06, 0x88); // TMA = 0x88

    // Step 512 cycles -> bit 9 becomes 1
    mmu.step_components(512);
    assert_eq!(mmu.read_byte(0xFF05), 0xFF);
    assert_eq!(mmu.read_byte(0xFF0F) & 0x04, 0);

    // Reset DIV -> glitch triggers TIMA overflow (0xFF -> 0x88) and IF bit 2 assertion
    mmu.write_byte(0xFF04, 0x42);
    assert_eq!(mmu.read_byte(0xFF05), 0x88);
    assert_ne!(mmu.read_byte(0xFF0F) & 0x04, 0);
}

#[test]
fn challenge_div_write_when_selected_bit_is_low_no_glitch() {
    let mut mmu = Mmu::new();
    mmu.write_byte(0xFF07, 0x04); // Enabled, bit 9

    // Step 256 cycles -> bit 9 is 0
    mmu.step_components(256);
    assert_eq!(mmu.read_byte(0xFF05), 0);

    // Reset DIV when bit 9 is low -> signal remains false -> no glitch
    mmu.write_byte(0xFF04, 0x00);
    assert_eq!(mmu.read_byte(0xFF05), 0);
}

// ============================================================================
// 3. TAC Write Glitch Behavior
// ============================================================================

#[test]
fn challenge_tac_write_disable_timer_glitch() {
    let mut mmu = Mmu::new();
    mmu.write_byte(0xFF07, 0x04); // Enabled, bit 9
    mmu.step_components(512); // Bit 9 is 1 (signal true)
    assert_eq!(mmu.read_byte(0xFF05), 0);

    // Disabling timer (TAC bit 2 = 0) forces signal to false -> falling edge -> TIMA increments!
    mmu.write_byte(0xFF07, 0x00);
    assert_eq!(mmu.read_byte(0xFF05), 1);
}

#[test]
fn challenge_tac_write_mode_change_glitch_true_to_false() {
    let mut mmu = Mmu::new();
    mmu.write_byte(0xFF07, 0x04); // Enabled, bit 9
    mmu.step_components(512); // Bit 9 is 1, bit 3 is 0
    assert_eq!(mmu.read_byte(0xFF05), 0);

    // Change to mode 01 (bit 3). Since bit 3 is 0, signal goes true -> false: falling edge!
    mmu.write_byte(0xFF07, 0x05);
    assert_eq!(mmu.read_byte(0xFF05), 1);
}

#[test]
fn challenge_tac_write_mode_change_no_glitch_true_to_true() {
    let mut mmu = Mmu::new();
    mmu.write_byte(0xFF07, 0x04); // Enabled, bit 9
    mmu.step_components(544); // Bit 9 is 1 (512), bit 5 is 1 (32)
    assert_eq!(mmu.read_byte(0xFF05), 0);

    // Change to mode 10 (bit 5). Since bit 5 is also 1, signal remains true -> no falling edge!
    mmu.write_byte(0xFF07, 0x06);
    assert_eq!(mmu.read_byte(0xFF05), 0);
}

#[test]
fn challenge_tac_write_enable_timer_no_glitch_false_to_true() {
    let mut mmu = Mmu::new();
    mmu.write_byte(0xFF07, 0x00); // Disabled
    mmu.step_components(512); // Bit 9 is 1, but timer is disabled (signal false)
    assert_eq!(mmu.read_byte(0xFF05), 0);

    // Enable timer -> signal false -> true (rising edge!)
    mmu.write_byte(0xFF07, 0x04);
    assert_eq!(mmu.read_byte(0xFF05), 0);
}

#[test]
fn challenge_tac_unused_bits_read_mask() {
    let mut mmu = Mmu::new();
    mmu.write_byte(0xFF07, 0x04);
    // Unused bits 3-7 must read as 1 (0xF8 mask)
    assert_eq!(mmu.read_byte(0xFF07), 0xFC);

    mmu.write_byte(0xFF07, 0xFF);
    assert_eq!(mmu.read_byte(0xFF07), 0xFF);
}

// ============================================================================
// 4. TIMA Overflow & TMA Reload Behavior
// ============================================================================

#[test]
fn challenge_tima_overflow_reloads_tma_and_sets_if_bit_2() {
    let mut mmu = Mmu::new();
    mmu.write_byte(0xFF07, 0x04); // Enabled, 4096 Hz
    mmu.write_byte(0xFF05, 0xFE); // TIMA = 0xFE
    mmu.write_byte(0xFF06, 0x33); // TMA = 0x33

    // Step 1024 cycles -> TIMA increments to 0xFF
    mmu.step_components(1024);
    assert_eq!(mmu.read_byte(0xFF05), 0xFF);
    assert_eq!(mmu.read_byte(0xFF0F) & 0x04, 0);

    // Step 1024 cycles -> TIMA overflows: reloads TMA (0x33) and sets IF bit 2 (0x04)
    mmu.step_components(1024);
    assert_eq!(mmu.read_byte(0xFF05), 0x33);
    assert_ne!(mmu.read_byte(0xFF0F) & 0x04, 0);
}

#[test]
fn challenge_tma_modification_affects_subsequent_reloads() {
    let mut mmu = Mmu::new();
    mmu.write_byte(0xFF07, 0x04); // 4096 Hz
    mmu.write_byte(0xFF05, 0xFF);
    mmu.write_byte(0xFF06, 0x10);

    // Trigger first reload -> reloads 0x10
    mmu.step_components(1024);
    assert_eq!(mmu.read_byte(0xFF05), 0x10);

    // Change TMA to 0x20
    mmu.write_byte(0xFF06, 0x20);

    // Step (255 - 0x10 + 1) * 1024 cycles = 240 * 1024 = 245760 cycles to overflow again
    mmu.step_components(245760);
    assert_eq!(mmu.read_byte(0xFF05), 0x20);
}

// ============================================================================
// 5. CPU Interrupt Servicing Integration & Edge Cases
// ============================================================================

#[test]
fn challenge_cpu_timer_interrupt_servicing_full_lifecycle() {
    let mut cpu = Cpu::new();
    let mut mmu = Mmu::new();

    cpu.registers.pc = 0xC000;
    cpu.registers.sp = 0xFFFE;
    cpu.ime_state = ImeState::Enabled;

    // Enable Timer Interrupt in IE (0xFFFF bit 2 = 0x04)
    mmu.write_byte(0xFFFF, 0x04);
    // Assert Timer Interrupt in IF (0xFF0F bit 2 = 0x04)
    mmu.write_byte(0xFF0F, 0x04);

    let cycles = cpu.step(&mut mmu);

    assert_eq!(cycles, 20, "Interrupt dispatch must take 20 T-cycles");
    assert_eq!(cpu.registers.pc, 0x0050, "PC must jump to Timer interrupt vector 0x0050");
    assert_eq!(cpu.registers.sp, 0xFFFC, "SP must be decremented by 2");
    assert_eq!(mmu.read_byte(0xFFFC), 0x00, "Low byte of return PC must be on stack");
    assert_eq!(mmu.read_byte(0xFFFD), 0xC0, "High byte of return PC must be on stack");
    assert_eq!(mmu.read_byte(0xFF0F) & 0x04, 0, "IF bit 2 must be cleared after service");
    assert_eq!(cpu.ime_state, ImeState::Disabled, "IME must be disabled post-dispatch");
}

#[test]
fn challenge_interrupt_priority_order_vblank_stat_timer_serial_joypad() {
    let mut cpu = Cpu::new();
    let mut mmu = Mmu::new();

    cpu.registers.pc = 0xC000;
    cpu.registers.sp = 0xFFFE;
    cpu.ime_state = ImeState::Enabled;

    // Enable all interrupts in IE
    mmu.write_byte(0xFFFF, 0x1F);
    // Assert all 5 interrupts in IF
    mmu.write_byte(0xFF0F, 0x1F);

    // Step 1: VBlank (bit 0) vector 0x0040
    let c1 = cpu.step(&mut mmu);
    assert_eq!(c1, 20);
    assert_eq!(cpu.registers.pc, 0x0040);
    assert_eq!(mmu.read_byte(0xFF0F) & 0x1F, 0x1E); // bit 0 cleared

    // Re-enable IME
    cpu.ime_state = ImeState::Enabled;
    // Step 2: STAT (bit 1) vector 0x0048
    let c2 = cpu.step(&mut mmu);
    assert_eq!(c2, 20);
    assert_eq!(cpu.registers.pc, 0x0048);
    assert_eq!(mmu.read_byte(0xFF0F) & 0x1F, 0x1C); // bit 1 cleared

    // Re-enable IME
    cpu.ime_state = ImeState::Enabled;
    // Step 3: Timer (bit 2) vector 0x0050
    let c3 = cpu.step(&mut mmu);
    assert_eq!(c3, 20);
    assert_eq!(cpu.registers.pc, 0x0050);
    assert_eq!(mmu.read_byte(0xFF0F) & 0x1F, 0x18); // bit 2 cleared

    // Re-enable IME
    cpu.ime_state = ImeState::Enabled;
    // Step 4: Serial (bit 3) vector 0x0058
    let c4 = cpu.step(&mut mmu);
    assert_eq!(c4, 20);
    assert_eq!(cpu.registers.pc, 0x0058);
    assert_eq!(mmu.read_byte(0xFF0F) & 0x1F, 0x10); // bit 3 cleared

    // Re-enable IME
    cpu.ime_state = ImeState::Enabled;
    // Step 5: Joypad (bit 4) vector 0x0060
    let c5 = cpu.step(&mut mmu);
    assert_eq!(c5, 20);
    assert_eq!(cpu.registers.pc, 0x0060);
    assert_eq!(mmu.read_byte(0xFF0F) & 0x1F, 0x00); // bit 4 cleared
}

#[test]
fn challenge_interrupt_servicing_with_ime_disabled_does_not_jump() {
    let mut cpu = Cpu::new();
    let mut mmu = Mmu::new();

    cpu.registers.pc = 0xC000;
    cpu.ime_state = ImeState::Disabled;

    // Enable and request Timer interrupt
    mmu.write_byte(0xFFFF, 0x04);
    mmu.write_byte(0xFF0F, 0x04);
    mmu.write_byte(0xC000, 0x00); // NOP at 0xC000

    let cycles = cpu.step(&mut mmu);
    assert_eq!(cycles, 4, "NOP should execute 4 cycles when IME is disabled");
    assert_eq!(cpu.registers.pc, 0xC001, "PC should advance to next instruction, not vector");
    assert_ne!(mmu.read_byte(0xFF0F) & 0x04, 0, "IF bit 2 should remain pending");
}

#[test]
fn challenge_cpu_halt_wakeup_and_timer_interrupt_dispatch() {
    let mut cpu = Cpu::new();
    let mut mmu = Mmu::new();

    cpu.registers.pc = 0xC000;
    cpu.ime_state = ImeState::Enabled;
    mmu.write_byte(0xC000, 0x76); // HALT opcode
    mmu.write_byte(0xFFFF, 0x04); // IE Timer bit 2

    // Execute HALT opcode
    let c1 = cpu.step(&mut mmu);
    assert_eq!(c1, 4);
    assert!(cpu.halted);

    // Trigger Timer interrupt via timer step
    mmu.write_byte(0xFF07, 0x05); // 262144 Hz (16 cycles)
    mmu.write_byte(0xFF05, 0xFF);
    mmu.step_components(16); // Overflows TIMA -> sets IF bit 2

    assert_ne!(mmu.read_byte(0xFF0F) & 0x04, 0);

    // Next CPU step should wake from HALT and service interrupt
    let c2 = cpu.step(&mut mmu);
    assert_eq!(c2, 20);
    assert!(!cpu.halted);
    assert_eq!(cpu.registers.pc, 0x0050);
}

#[test]
fn challenge_halt_wakeup_with_ime_disabled_no_vector_jump() {
    let mut cpu = Cpu::new();
    let mut mmu = Mmu::new();

    cpu.registers.pc = 0xC000;
    cpu.ime_state = ImeState::Disabled;
    mmu.write_byte(0xC000, 0x76); // HALT
    mmu.write_byte(0xC001, 0x00); // NOP after HALT
    mmu.write_byte(0xFFFF, 0x04); // IE Timer bit 2

    // Step 1: HALT executes
    let c1 = cpu.step(&mut mmu);
    assert_eq!(c1, 4);
    assert!(cpu.halted);

    // Request Timer interrupt
    mmu.write_byte(0xFF0F, 0x04);

    // Step 2: CPU wakes up from HALT because pending interrupt matches IE,
    // but because IME is Disabled, it does NOT jump to vector!
    // Instead it resumes execution at PC 0xC001 (NOP)
    let c2 = cpu.step(&mut mmu);
    assert_eq!(c2, 4); // NOP executed
    assert!(!cpu.halted, "CPU must wake up from HALT");
    assert_eq!(cpu.registers.pc, 0xC002, "PC must execute instruction after HALT");
    assert_ne!(mmu.read_byte(0xFF0F) & 0x04, 0, "IF bit 2 must remain pending");
}

#[test]
fn challenge_div_counter_wrapping_65536_cycles() {
    let mut mmu = Mmu::new();
    mmu.write_byte(0xFF07, 0x04); // 4096 Hz (bit 9)

    // Step 65535 cycles (div_counter reaches 65535 = 0xFFFF)
    mmu.step_components(65535);
    let tima_before = mmu.read_byte(0xFF05);

    // 65536th cycle (div_counter wraps from 0xFFFF -> 0x0000)
    // Bit 9 transitions 1 -> 0, causing a falling edge and incrementing TIMA
    mmu.step_components(1);
    let tima_after = mmu.read_byte(0xFF05);

    assert_eq!(tima_after, tima_before.wrapping_add(1));
    assert_eq!(mmu.read_byte(0xFF04), 0);
}

#[test]
fn challenge_manual_if_register_manipulation() {
    let mut cpu = Cpu::new();
    let mut mmu = Mmu::new();

    cpu.registers.pc = 0xC000;
    cpu.registers.sp = 0xFFFE;
    cpu.ime_state = ImeState::Enabled;
    mmu.write_byte(0xFFFF, 0x04); // IE Timer

    // Manually set IF bit 2 via write_byte
    mmu.write_byte(0xFF0F, 0x04);
    assert_eq!(mmu.read_byte(0xFF0F) & 0x04, 0x04);

    // Step CPU: services interrupt
    cpu.step(&mut mmu);
    assert_eq!(cpu.registers.pc, 0x0050);
    assert_eq!(mmu.read_byte(0xFF0F) & 0x04, 0);

    // Manually clear IF bit 2 while another request is set
    mmu.write_byte(0xFF0F, 0x04);
    mmu.write_byte(0xFF0F, 0x00);
    cpu.ime_state = ImeState::Enabled;

    let c2 = cpu.step(&mut mmu);
    assert_ne!(c2, 20, "Should not dispatch interrupt when IF was cleared");
}

#[test]
fn challenge_reti_inside_timer_interrupt_handler() {
    let mut cpu = Cpu::new();
    let mut mmu = Mmu::new();

    let rom_data = build_dummy_rom_with_vector(0x0050, 0xD9); // 0xD9 = RETI at 0x0050
    let cart = create_cartridge(rom_data, None).expect("Failed to build cart");
    mmu.attach_cartridge(cart);

    cpu.registers.pc = 0xC000;
    cpu.registers.sp = 0xFFFE;
    cpu.ime_state = ImeState::Enabled;

    mmu.write_byte(0xFFFF, 0x04); // IE Timer
    mmu.write_byte(0xFF0F, 0x04); // IF Timer

    // Step 1: Dispatch to 0x0050 (20 cycles, IME disabled)
    let c1 = cpu.step(&mut mmu);
    assert_eq!(c1, 20);
    assert_eq!(cpu.registers.pc, 0x0050);
    assert_eq!(cpu.ime_state, ImeState::Disabled);

    // Step 2: Execute RETI at 0x0050 (16 cycles)
    let c2 = cpu.step(&mut mmu);
    assert_eq!(c2, 16);
    assert_eq!(cpu.registers.pc, 0xC000, "RETI must pop return address 0xC000 from stack");
    assert_eq!(cpu.registers.sp, 0xFFFE, "SP must be restored");
    assert_eq!(cpu.ime_state, ImeState::Enabled, "RETI must enable IME immediately");
}

#[test]
fn challenge_rapid_div_write_glitch_tima_overflow() {
    let mut mmu = Mmu::new();
    mmu.write_byte(0xFF07, 0x04); // Enabled, bit 9
    mmu.write_byte(0xFF05, 0xFD); // TIMA = 253
    mmu.write_byte(0xFF06, 0x10); // TMA = 16

    // Glitch 1: step 512 (bit 9 high) -> write DIV -> TIMA becomes 254
    mmu.step_components(512);
    mmu.write_byte(0xFF04, 0x00);
    assert_eq!(mmu.read_byte(0xFF05), 254);

    // Glitch 2: step 512 -> write DIV -> TIMA becomes 255
    mmu.step_components(512);
    mmu.write_byte(0xFF04, 0x00);
    assert_eq!(mmu.read_byte(0xFF05), 255);

    // Glitch 3: step 512 -> write DIV -> TIMA overflows! Reloads 0x10 and asserts IF bit 2
    mmu.step_components(512);
    mmu.write_byte(0xFF04, 0x00);
    assert_eq!(mmu.read_byte(0xFF05), 0x10);
    assert_ne!(mmu.read_byte(0xFF0F) & 0x04, 0);
}
