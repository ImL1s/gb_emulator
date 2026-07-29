/// Work RAM (8KB: 0xC000..=0xDFFF) and Echo RAM mapping (0xE000..=0xFDFF).
#[derive(Debug, Clone)]
pub struct Wram {
    pub data: Box<[u8; 0x2000]>,
}

impl Wram {
    pub fn new() -> Self {
        Self {
            data: Box::new([0; 0x2000]),
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        let idx = (addr - 0xC000) as usize;
        self.data.get(idx).copied().unwrap_or(0xFF)
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        let idx = (addr - 0xC000) as usize;
        if idx < self.data.len() {
            self.data[idx] = val;
        }
    }

    pub fn read_echo(&self, addr: u16) -> u8 {
        let idx = (addr - 0xE000) as usize;
        self.data.get(idx).copied().unwrap_or(0xFF)
    }

    pub fn write_echo(&mut self, addr: u16, val: u8) {
        let idx = (addr - 0xE000) as usize;
        if idx < self.data.len() {
            self.data[idx] = val;
        }
    }
}

impl Default for Wram {
    fn default() -> Self {
        Self::new()
    }
}

/// High RAM (127B: 0xFF80..=0xFFFE).
#[derive(Debug, Clone)]
pub struct Hram {
    pub data: Box<[u8; 0x7F]>,
}

impl Hram {
    pub fn new() -> Self {
        Self {
            data: Box::new([0; 0x7F]),
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        let idx = (addr - 0xFF80) as usize;
        self.data.get(idx).copied().unwrap_or(0xFF)
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        let idx = (addr - 0xFF80) as usize;
        if idx < self.data.len() {
            self.data[idx] = val;
        }
    }
}

impl Default for Hram {
    fn default() -> Self {
        Self::new()
    }
}
