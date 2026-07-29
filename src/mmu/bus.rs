/// Memory Bus interface trait connecting the CPU to memory and peripherals.
pub trait Bus {
    /// Read a single byte from the 16-bit address space.
    fn read_byte(&self, addr: u16) -> u8;

    /// Write a single byte to the 16-bit address space.
    fn write_byte(&mut self, addr: u16, val: u8);

    /// Advance memory-mapped hardware components (PPU, Timers, APU, etc.) by specified T-cycles.
    fn step_components(&mut self, cycles: u32);

    /// Check if any enabled interrupt is currently requested.
    fn is_interrupt_requested(&self) -> bool {
        (self.read_byte(0xFF0F) & self.read_byte(0xFFFF) & 0x1F) != 0
    }

    /// Read a 16-bit word in little-endian order.
    fn read_word(&self, addr: u16) -> u16 {
        let low = self.read_byte(addr) as u16;
        let high = self.read_byte(addr.wrapping_add(1)) as u16;
        (high << 8) | low
    }

    /// Write a 16-bit word in little-endian order.
    fn write_word(&mut self, addr: u16, val: u16) {
        self.write_byte(addr, (val & 0xFF) as u8);
        self.write_byte(addr.wrapping_add(1), (val >> 8) as u8);
    }
}

/// MockBus for isolated unit testing of CPU instruction execution and timing.
#[derive(Debug, Clone)]
pub struct MockBus {
    pub memory: Box<[u8; 0x10000]>,
    pub cycles: u32,
}

impl MockBus {
    pub fn new() -> Self {
        Self {
            memory: Box::new([0; 0x10000]),
            cycles: 0,
        }
    }

    pub fn load_code(&mut self, start_addr: u16, code: &[u8]) {
        for (i, &byte) in code.iter().enumerate() {
            let addr = start_addr.wrapping_add(i as u16);
            self.memory[addr as usize] = byte;
        }
    }
}

impl Default for MockBus {
    fn default() -> Self {
        Self::new()
    }
}

impl Bus for MockBus {
    fn read_byte(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }

    fn write_byte(&mut self, addr: u16, val: u8) {
        self.memory[addr as usize] = val;
    }

    fn step_components(&mut self, cycles: u32) {
        self.cycles = self.cycles.wrapping_add(cycles);
    }
}
