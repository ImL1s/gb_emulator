pub mod alu;
pub mod opcodes;
pub mod registers;
pub mod stress_tests;

use crate::mmu::bus::Bus;
pub use registers::{Flag, Registers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImeState {
    Disabled,
    PendingEnable,
    Enabling,
    Enabled,
}

#[derive(Debug, Clone)]
pub struct Cpu {
    pub registers: Registers,
    pub ime_state: ImeState,
    pub halted: bool,
    pub halt_bug: bool,
    pub stopped: bool,
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            registers: Registers::new(),
            ime_state: ImeState::Disabled,
            halted: false,
            halt_bug: false,
            stopped: false,
        }
    }

    /// Reset CPU registers to Post-BOOT ROM DMG default state.
    pub fn reset(&mut self) {
        self.registers = Registers::new();
        self.ime_state = ImeState::Disabled;
        self.halted = false;
        self.halt_bug = false;
        self.stopped = false;
    }

    /// Execute a single CPU step (instruction fetch-decode-execute or interrupt dispatch).
    /// Returns the number of T-cycles executed.
    pub fn step(&mut self, bus: &mut impl Bus) -> u32 {
        // 1. Service pending interrupts if IME enabled or wake from HALT/STOP
        let interrupt_cycles = self.handle_interrupts(bus);
        if interrupt_cycles > 0 {
            return interrupt_cycles;
        }

        // 2. Handle HALT / STOP idle state
        if self.halted || self.stopped {
            if bus.is_interrupt_requested() {
                self.halted = false;
                self.stopped = false;
            } else {
                bus.step_components(4);
                return 4;
            }
        }

        // Advance EI delay state machine: PendingEnable -> Enabling
        if self.ime_state == ImeState::PendingEnable {
            self.ime_state = ImeState::Enabling;
        }

        // 3. Fetch opcode byte (handling HALT bug if active)
        let pc = self.registers.pc;
        let opcode = bus.read_byte(pc);

        if self.halt_bug {
            self.halt_bug = false; // PC is not incremented for HALT bug fetch
        } else {
            self.registers.pc = self.registers.pc.wrapping_add(1);
        }

        // 4. Decode & execute opcode
        let cycles = if opcode == 0xCB {
            let cb_opcode = self.fetch_byte(bus);
            self.execute_cb(bus, cb_opcode)
        } else if opcode == 0x76 { // HALT
            let ie = bus.read_byte(0xFFFF);
            let if_reg = bus.read_byte(0xFF0F);
            let pending = ie & if_reg & 0x1F;

            if self.ime_state != ImeState::Enabled && pending != 0 {
                // HALT Bug: CPU does not halt, next byte read twice
                self.halted = false;
                self.halt_bug = true;
            } else {
                self.halted = true;
            }
            4
        } else {
            self.execute_unprefixed(bus, opcode)
        };

        // Advance EI delay state machine: Enabling -> Enabled
        if self.ime_state == ImeState::Enabling {
            self.ime_state = ImeState::Enabled;
        }

        // 5. Advance memory-mapped hardware components
        bus.step_components(cycles);

        cycles
    }

    /// Dispatch pending interrupts if requested and IME is enabled.
    pub fn handle_interrupts(&mut self, bus: &mut impl Bus) -> u32 {
        let ie = bus.read_byte(0xFFFF);
        let if_reg = bus.read_byte(0xFF0F);
        let pending = ie & if_reg & 0x1F;

        if pending == 0 {
            return 0;
        }

        // Wake CPU from HALT or STOP state
        if self.halted {
            self.halted = false;
        }
        if self.stopped {
            self.stopped = false;
        }

        // If IME is disabled, CPU wakes up but interrupt routine is not dispatched
        if self.ime_state != ImeState::Enabled {
            return 0;
        }

        // Disable interrupts during service dispatch
        self.ime_state = ImeState::Disabled;

        for bit in 0..5 {
            if (pending & (1 << bit)) != 0 {
                // Clear IF bit
                let new_if = if_reg & !(1 << bit);
                bus.write_byte(0xFF0F, new_if);

                // Push PC to stack
                let pc = self.registers.pc;
                self.push_stack_16(bus, pc);

                // Jump to vector
                let vector = 0x0040 + (bit as u16) * 0x08;
                self.registers.pc = vector;

                // Interrupt service routine dispatch takes 20 T-cycles
                bus.step_components(20);
                return 20;
            }
        }

        0
    }

    /// Fetch 8-bit immediate byte from PC and advance PC by 1.
    pub fn fetch_byte(&mut self, bus: &impl Bus) -> u8 {
        let val = bus.read_byte(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);
        val
    }

    /// Fetch 16-bit immediate word from PC and advance PC by 2.
    pub fn fetch_word(&mut self, bus: &impl Bus) -> u16 {
        let low = self.fetch_byte(bus) as u16;
        let high = self.fetch_byte(bus) as u16;
        (high << 8) | low
    }

    /// Push 16-bit word onto stack (SP -= 2).
    pub fn push_stack_16(&mut self, bus: &mut impl Bus, val: u16) {
        let high = (val >> 8) as u8;
        let low = (val & 0xFF) as u8;
        self.registers.sp = self.registers.sp.wrapping_sub(1);
        bus.write_byte(self.registers.sp, high);
        self.registers.sp = self.registers.sp.wrapping_sub(1);
        bus.write_byte(self.registers.sp, low);
    }

    /// Pop 16-bit word from stack (SP += 2).
    pub fn pop_stack_16(&mut self, bus: &mut impl Bus) -> u16 {
        let low = bus.read_byte(self.registers.sp) as u16;
        self.registers.sp = self.registers.sp.wrapping_add(1);
        let high = bus.read_byte(self.registers.sp) as u16;
        self.registers.sp = self.registers.sp.wrapping_add(1);
        (high << 8) | low
    }
}

impl Default for Cpu {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mmu::bus::MockBus;

    #[test]
    fn test_ei_1_instruction_delay_state_machine() {
        let mut cpu = Cpu::new();
        let mut bus = MockBus::new();

        // Write EI (0xFB) at 0x0100, NOP (0x00) at 0x0101, NOP (0x00) at 0x0102
        bus.write_byte(0x0100, 0xFB); // EI
        bus.write_byte(0x0101, 0x00); // NOP
        bus.write_byte(0x0102, 0x00); // NOP

        assert_eq!(cpu.ime_state, ImeState::Disabled);

        // Step 1: Execute EI (0xFB)
        let cycles1 = cpu.step(&mut bus);
        assert_eq!(cycles1, 4);
        assert_eq!(cpu.ime_state, ImeState::PendingEnable);

        // Step 2: Execute NOP (0x0101) directly following EI
        let cycles2 = cpu.step(&mut bus);
        assert_eq!(cycles2, 4);
        // After executing instruction following EI, IME is Enabled
        assert_eq!(cpu.ime_state, ImeState::Enabled);
    }

    #[test]
    fn test_interrupt_dispatch_vectors_and_20_tcycles() {
        let vectors = [
            (0, 0x0040), // VBlank
            (1, 0x0048), // STAT
            (2, 0x0050), // Timer
            (3, 0x0058), // Serial
            (4, 0x0060), // Joypad
        ];

        for (bit, expected_vector) in vectors {
            let mut cpu = Cpu::new();
            let mut bus = MockBus::new();

            cpu.registers.pc = 0x0200;
            cpu.registers.sp = 0xFFFE;
            cpu.ime_state = ImeState::Enabled;

            // Set IE and IF for target interrupt bit
            bus.write_byte(0xFFFF, 1 << bit); // IE
            bus.write_byte(0xFF0F, 1 << bit); // IF

            let cycles = cpu.step(&mut bus);

            assert_eq!(cycles, 20, "Interrupt dispatch must take 20 T-cycles");
            assert_eq!(cpu.registers.pc, expected_vector, "PC must jump to vector {:#06X}", expected_vector);
            assert_eq!(cpu.registers.sp, 0xFFFC, "SP must be decremented by 2");
            assert_eq!(bus.read_word(0xFFFC), 0x0200, "Stack must store return PC 0x0200");
            assert_eq!(bus.read_byte(0xFF0F) & (1 << bit), 0, "IF bit must be cleared");
            assert_eq!(cpu.ime_state, ImeState::Disabled, "IME must be disabled post-dispatch");
        }
    }

    #[test]
    fn test_halt_and_interrupt_wakeup() {
        let mut cpu = Cpu::new();
        let mut bus = MockBus::new();

        // Execute HALT (0x76) at 0x0100 with IME Enabled and no pending interrupts
        cpu.registers.pc = 0x0100;
        cpu.ime_state = ImeState::Enabled;
        bus.write_byte(0x0100, 0x76);

        let cycles = cpu.step(&mut bus);
        assert_eq!(cycles, 4);
        assert_eq!(cpu.halted, true);

        // While halted and no interrupts, step() advances 4 T-cycles
        let idle_cycles = cpu.step(&mut bus);
        assert_eq!(idle_cycles, 4);

        // Trigger VBlank interrupt (bit 0)
        bus.write_byte(0xFFFF, 0x01);
        bus.write_byte(0xFF0F, 0x01);

        let wake_cycles = cpu.step(&mut bus);
        assert_eq!(wake_cycles, 20); // Dispatches interrupt
        assert_eq!(cpu.halted, false);
        assert_eq!(cpu.registers.pc, 0x0040);
    }

    #[test]
    fn test_halt_bug_pc_duplication() {
        let mut cpu = Cpu::new();
        let mut bus = MockBus::new();

        // Setup HALT bug condition: IME Disabled, IE=0x01, IF=0x01
        cpu.registers.pc = 0x0100;
        cpu.ime_state = ImeState::Disabled;
        bus.write_byte(0xFFFF, 0x01);
        bus.write_byte(0xFF0F, 0x01);

        // Opcode at 0x0100 is HALT (0x76), next opcode at 0x0101 is INC B (0x04)
        bus.write_byte(0x0100, 0x76);
        bus.write_byte(0x0101, 0x04); // INC B

        // Step 1: HALT opcode executes. Triggers HALT bug (halted stays false, halt_bug becomes true)
        let cycles1 = cpu.step(&mut bus);
        assert_eq!(cycles1, 4);
        assert_eq!(cpu.halted, false);
        assert_eq!(cpu.halt_bug, true);
        assert_eq!(cpu.registers.pc, 0x0101);

        // Step 2: Next step reads opcode at 0x0101 (0x04 INC B), but due to halt_bug, PC is NOT incremented
        let cycles2 = cpu.step(&mut bus);
        assert_eq!(cycles2, 4);
        assert_eq!(cpu.registers.b, 1);
        assert_eq!(cpu.registers.pc, 0x0101); // PC failed to increment! HALT bug byte duplicated!

        // Step 3: Next step reads opcode at 0x0101 (0x04 INC B) again! Now PC increments normally.
        let cycles3 = cpu.step(&mut bus);
        assert_eq!(cycles3, 4);
        assert_eq!(cpu.registers.b, 2);
        assert_eq!(cpu.registers.pc, 0x0102);
    }
}

