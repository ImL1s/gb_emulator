/// Hardware Timer Subsystem for Game Boy (LR35902).
/// Manages DIV (0xFF04), TIMA (0xFF05), TMA (0xFF06), TAC (0xFF07),
/// bit multiplexing, falling-edge detection, and timer interrupts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timer {
    /// Internal 16-bit DIV counter incremented every T-cycle
    pub div_counter: u16,
    /// TIMA counter register (0xFF05)
    pub tima: u8,
    /// TMA reload modulo register (0xFF06)
    pub tma: u8,
    /// TAC control register (0xFF07)
    pub tac: u8,
    /// Timer interrupt pending flag (bit 2 of IF 0xFF0F)
    pub interrupt_pending: bool,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            div_counter: 0,
            tima: 0,
            tma: 0,
            tac: 0,
            interrupt_pending: false,
        }
    }

    /// Reset timer state to default state.
    pub fn reset(&mut self) {
        self.div_counter = 0;
        self.tima = 0;
        self.tma = 0;
        self.tac = 0;
        self.interrupt_pending = false;
    }

    /// Evaluates current multiplexed signal state (div_bit AND enable).
    #[inline]
    fn get_signal(&self) -> bool {
        if (self.tac & 0x04) == 0 {
            return false;
        }
        let bit_index = match self.tac & 0x03 {
            0b00 => 9,
            0b01 => 3,
            0b10 => 5,
            0b11 => 7,
            _ => unreachable!(),
        };
        (self.div_counter & (1 << bit_index)) != 0
    }

    /// Increment TIMA and handle overflow reload & interrupt request.
    #[inline]
    fn increment_tima(&mut self) {
        if self.tima == 0xFF {
            self.tima = self.tma;
            self.interrupt_pending = true;
        } else {
            self.tima = self.tima.wrapping_add(1);
        }
    }

    /// Advance timer by specified T-cycles.
    pub fn step(&mut self, cycles: u32) {
        for _ in 0..cycles {
            let old_signal = self.get_signal();
            self.div_counter = self.div_counter.wrapping_add(1);
            let new_signal = self.get_signal();

            if old_signal && !new_signal {
                self.increment_tima();
            }
        }
    }

    /// Read memory-mapped timer register.
    pub fn read_reg(&self, addr: u16) -> u8 {
        match addr {
            0xFF04 => (self.div_counter >> 8) as u8,
            0xFF05 => self.tima,
            0xFF06 => self.tma,
            0xFF07 => self.tac | 0xF8, // Unused bits 3-7 read as 1
            _ => 0xFF,
        }
    }

    /// Write memory-mapped timer register.
    pub fn write_reg(&mut self, addr: u16, val: u8) {
        match addr {
            0xFF04 => {
                // Writing to DIV resets internal 16-bit counter to 0.
                let old_signal = self.get_signal();
                self.div_counter = 0;
                let new_signal = self.get_signal();

                // Resetting DIV can trigger falling edge if bit was 1
                if old_signal && !new_signal {
                    self.increment_tima();
                }
            }
            0xFF05 => self.tima = val,
            0xFF06 => self.tma = val,
            0xFF07 => {
                let old_signal = self.get_signal();
                self.tac = val & 0x07;
                let new_signal = self.get_signal();

                // Changing TAC enable or clock bit can trigger falling edge
                if old_signal && !new_signal {
                    self.increment_tima();
                }
            }
            _ => {}
        }
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_div_increment_and_read() {
        let mut timer = Timer::new();
        timer.step(256);
        assert_eq!(timer.div_counter, 256);
        assert_eq!(timer.read_reg(0xFF04), 1);
    }

    #[test]
    fn test_tima_frequency_selection() {
        // TAC 0b00 -> bit 9 (period 1024 cycles)
        let mut timer = Timer::new();
        timer.write_reg(0xFF07, 0x04); // Enabled, mode 00
        timer.step(1024);
        assert_eq!(timer.read_reg(0xFF05), 1);

        // TAC 0b01 -> bit 3 (period 16 cycles)
        let mut timer = Timer::new();
        timer.write_reg(0xFF07, 0x05); // Enabled, mode 01
        timer.step(16);
        assert_eq!(timer.read_reg(0xFF05), 1);

        // TAC 0b10 -> bit 5 (period 64 cycles)
        let mut timer = Timer::new();
        timer.write_reg(0xFF07, 0x06); // Enabled, mode 10
        timer.step(64);
        assert_eq!(timer.read_reg(0xFF05), 1);

        // TAC 0b11 -> bit 7 (period 256 cycles)
        let mut timer = Timer::new();
        timer.write_reg(0xFF07, 0x07); // Enabled, mode 11
        timer.step(256);
        assert_eq!(timer.read_reg(0xFF05), 1);
    }

    #[test]
    fn test_timer_disabled_does_not_increment_tima() {
        let mut timer = Timer::new();
        // TAC enable is 0 (disabled)
        timer.write_reg(0xFF07, 0x00);
        timer.step(10000);
        assert_eq!(timer.div_counter, 10000);
        assert_eq!(timer.tima, 0);
    }

    #[test]
    fn test_div_write_glitch_triggers_tima_increment() {
        let mut timer = Timer::new();
        timer.write_reg(0xFF07, 0x04); // Enabled, bit 9
        timer.step(512); // Bit 9 becomes 1 (signal is true)
        assert_eq!(timer.tima, 0);

        // Resetting DIV forces bit 9 to 0 (signal true -> false: falling edge!)
        timer.write_reg(0xFF04, 0x00);
        assert_eq!(timer.tima, 1);
    }

    #[test]
    fn test_tac_write_glitch_disable_timer() {
        let mut timer = Timer::new();
        timer.write_reg(0xFF07, 0x04); // Enabled, bit 9
        timer.step(512); // Bit 9 is 1 (signal true)
        assert_eq!(timer.tima, 0);

        // Disable timer (TAC enable bit 2 -> 0 => signal true -> false: falling edge!)
        timer.write_reg(0xFF07, 0x00);
        assert_eq!(timer.tima, 1);
    }

    #[test]
    fn test_tac_write_glitch_change_mode() {
        let mut timer = Timer::new();
        timer.write_reg(0xFF07, 0x04); // Enabled, bit 9
        timer.step(512); // Bit 9 is 1, bit 3 is 0
        assert_eq!(timer.tima, 0);

        // Switch to mode 01 (bit 3). Since bit 3 is 0, signal goes true -> false: falling edge!
        timer.write_reg(0xFF07, 0x05);
        assert_eq!(timer.tima, 1);
    }

    #[test]
    fn test_tima_overflow_reloads_tma_and_sets_interrupt() {
        let mut timer = Timer::new();
        timer.write_reg(0xFF05, 0xFF);
        timer.write_reg(0xFF06, 0x42);
        timer.write_reg(0xFF07, 0x04); // Enabled, bit 9

        assert!(!timer.interrupt_pending);

        // Step 1024 cycles to trigger TIMA increment on bit 9 falling edge
        timer.step(1024);

        assert_eq!(timer.read_reg(0xFF05), 0x42);
        assert!(timer.interrupt_pending);
    }
}
