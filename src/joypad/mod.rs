/// Joypad button key identifiers for Game Boy input matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JoypadKey {
    Right,
    Left,
    Up,
    Down,
    A,
    B,
    Select,
    Start,
}

/// Joypad Matrix Subsystem for Game Boy (LR35902).
/// Implements active-low button polling (JOYP 0xFF00) and
/// high-to-low transition interrupt requests (IF bit 4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Joypad {
    /// Bit 5 selection line (true = deselected / 1, false = selected / 0)
    pub select_action: bool,
    /// Bit 4 selection line (true = deselected / 1, false = selected / 0)
    pub select_direction: bool,

    pub a: bool,
    pub b: bool,
    pub select: bool,
    pub start: bool,
    pub right: bool,
    pub left: bool,
    pub up: bool,
    pub down: bool,
}

impl Joypad {
    pub fn new() -> Self {
        Self {
            select_action: true,    // Default 1 (deselected)
            select_direction: true, // Default 1 (deselected)
            a: false,
            b: false,
            select: false,
            start: false,
            right: false,
            left: false,
            up: false,
            down: false,
        }
    }

    /// Read memory-mapped JOYP register (0xFF00).
    pub fn read_joyp(&self) -> u8 {
        let mut line = 0x0F;

        if !self.select_direction {
            if self.right {
                line &= !(1 << 0);
            }
            if self.left {
                line &= !(1 << 1);
            }
            if self.up {
                line &= !(1 << 2);
            }
            if self.down {
                line &= !(1 << 3);
            }
        }

        if !self.select_action {
            if self.a {
                line &= !(1 << 0);
            }
            if self.b {
                line &= !(1 << 1);
            }
            if self.select {
                line &= !(1 << 2);
            }
            if self.start {
                line &= !(1 << 3);
            }
        }

        let select_bits = ((self.select_action as u8) << 5) | ((self.select_direction as u8) << 4);
        0xC0 | select_bits | line
    }

    /// Write memory-mapped JOYP register (0xFF00).
    /// Updates selection bits 4 and 5. Returns `true` if a High-to-Low transition occurred.
    pub fn write_joyp(&mut self, val: u8) -> bool {
        let old_line = self.read_joyp() & 0x0F;

        self.select_action = (val & 0x20) != 0;
        self.select_direction = (val & 0x10) != 0;

        let new_line = self.read_joyp() & 0x0F;
        (old_line & !new_line) & 0x0F != 0
    }

    /// Press a button. Returns `true` if Joypad Interrupt should be requested.
    pub fn press_key(&mut self, key: JoypadKey) -> bool {
        let old_line = self.read_joyp() & 0x0F;

        match key {
            JoypadKey::Right => self.right = true,
            JoypadKey::Left => self.left = true,
            JoypadKey::Up => self.up = true,
            JoypadKey::Down => self.down = true,
            JoypadKey::A => self.a = true,
            JoypadKey::B => self.b = true,
            JoypadKey::Select => self.select = true,
            JoypadKey::Start => self.start = true,
        }

        let new_line = self.read_joyp() & 0x0F;
        (old_line & !new_line) & 0x0F != 0
    }

    /// Release a button. Releasing never triggers a High-to-Low transition (no interrupt).
    pub fn release_key(&mut self, key: JoypadKey) {
        match key {
            JoypadKey::Right => self.right = false,
            JoypadKey::Left => self.left = false,
            JoypadKey::Up => self.up = false,
            JoypadKey::Down => self.down = false,
            JoypadKey::A => self.a = false,
            JoypadKey::B => self.b = false,
            JoypadKey::Select => self.select = false,
            JoypadKey::Start => self.start = false,
        }
    }
}

impl Default for Joypad {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_joypad_action_and_directional_selection() {
        let mut joypad = Joypad::new();
        // Initially neither line selected
        assert_eq!(joypad.read_joyp(), 0xFF);

        joypad.press_key(JoypadKey::A);
        joypad.press_key(JoypadKey::Right);

        // With neither selected, read_joyp still returns 0xFF
        assert_eq!(joypad.read_joyp(), 0xFF);

        // Select Directional keys (bit 5 = 1, bit 4 = 0 -> 0x20)
        joypad.write_joyp(0x20);
        assert_eq!(joypad.read_joyp() & 0x0F, 0x0E); // Bit 0 cleared (Right pressed)

        // Select Action keys (bit 5 = 0, bit 4 = 1 -> 0x10)
        joypad.write_joyp(0x10);
        assert_eq!(joypad.read_joyp() & 0x0F, 0x0E); // Bit 0 cleared (A pressed)

        // Both Action and Directional selected (0x00)
        joypad.write_joyp(0x00);
        joypad.press_key(JoypadKey::B);
        joypad.press_key(JoypadKey::Left);
        assert_eq!(joypad.read_joyp() & 0x0F, 0x0C); // Bits 0 and 1 cleared
    }

    #[test]
    fn test_joypad_interrupt_on_high_to_low_transition() {
        let mut joypad = Joypad::new();
        // Select Directional keys
        joypad.write_joyp(0x20);

        // Pressing Up triggers High-to-Low transition
        let irq1 = joypad.press_key(JoypadKey::Up);
        assert!(irq1);

        // Pressing Up again (already held) -> no transition
        let irq2 = joypad.press_key(JoypadKey::Up);
        assert!(!irq2);

        // Releasing Up -> no interrupt on release
        joypad.release_key(JoypadKey::Up);

        // Hold Down while Directional keys are deselected (0x30)
        joypad.write_joyp(0x30);
        joypad.press_key(JoypadKey::Down);

        // Selecting Directional line while Down is held down -> transition occurs!
        let irq3 = joypad.write_joyp(0x20);
        assert!(irq3);
    }
}
