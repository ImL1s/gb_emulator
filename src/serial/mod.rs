/// Serial Port Interceptor for Game Boy (LR35902).
/// Captures ASCII output characters written to SB (0xFF01) when transfer start bit
/// is written to SC (0xFF02), used primarily for test ROM output interception.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SerialPort {
    pub sb: u8,
    pub sc: u8,
    pub output_buffer: String,
}

impl SerialPort {
    pub fn new() -> Self {
        Self {
            sb: 0,
            sc: 0,
            output_buffer: String::new(),
        }
    }

    pub fn read_sb(&self) -> u8 {
        self.sb
    }

    pub fn write_sb(&mut self, val: u8) {
        self.sb = val;
    }

    pub fn read_sc(&self) -> u8 {
        self.sc | 0x7E // Bits 1..=6 are always 1 on DMG
    }

    /// Write to SC register. Returns `true` if Serial Interrupt (IF bit 3) should be set.
    pub fn write_sc(&mut self, val: u8) -> bool {
        self.sc = val;
        if (val & 0x80) != 0 {
            // Capture byte to string buffer
            self.output_buffer.push(self.sb as char);
            // Reset transfer start bit
            self.sc &= !0x80;
            true // Request interrupt
        } else {
            false
        }
    }

    /// Advance serial port clock components.
    pub fn step(&mut self, _cycles: u32) -> bool {
        false
    }

    pub fn get_output(&self) -> &str {
        &self.output_buffer
    }

    pub fn clear_output(&mut self) {
        self.output_buffer.clear();
    }

    pub fn take_output(&mut self) -> String {
        std::mem::take(&mut self.output_buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serial_character_capture_and_interrupt() {
        let mut serial = SerialPort::new();
        serial.write_sb(0x41); // 'A'
        let interrupt_triggered = serial.write_sc(0x81);

        assert!(interrupt_triggered);
        assert_eq!(serial.read_sc(), 0x01 | 0x7E);
        assert_eq!(serial.get_output(), "A");
    }

    #[test]
    fn test_serial_string_accumulation() {
        let mut serial = SerialPort::new();

        serial.write_sb(b'P');
        serial.write_sc(0x81);

        serial.write_sb(b'a');
        serial.write_sc(0x81);

        serial.write_sb(b's');
        serial.write_sc(0x81);

        serial.write_sb(b's');
        serial.write_sc(0x81);

        assert_eq!(serial.get_output(), "Pass");
    }
}
