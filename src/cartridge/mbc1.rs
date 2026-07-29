use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use super::{save_sram_atomic, Cartridge};

/// MBC1 mapper (up to 2MB ROM, 32KB RAM, ROM/RAM mode selection).
pub struct Mbc1 {
    rom: Vec<u8>,
    ram: Vec<u8>,
    num_rom_banks: usize,
    num_ram_banks: usize,
    ram_enabled: bool,
    rom_bank_low: u8,   // 5 bits (1..=31, 0 maps to 1)
    ram_bank_high: u8,  // 2 bits (0..=3)
    banking_mode: u8,   // 0 = ROM mode, 1 = RAM mode
    has_battery: bool,
    save_path: Option<PathBuf>,
    dirty: bool,
}

impl Mbc1 {
    pub fn new(
        rom: Vec<u8>,
        ram_size: usize,
        has_battery: bool,
        save_path: Option<PathBuf>,
        sram_data: Option<Vec<u8>>,
    ) -> Self {
        let num_rom_banks = (rom.len() / 16384).max(1);
        let num_ram_banks = if ram_size > 0 { (ram_size / 8192).max(1) } else { 0 };
        let mut ram = vec![0u8; ram_size];

        if has_battery {
            if let Some(data) = sram_data {
                let len = data.len().min(ram_size);
                ram[..len].copy_from_slice(&data[..len]);
            } else if let Some(ref path) = save_path {
                if path.exists() {
                    if let Ok(data) = fs::read(path) {
                        let len = data.len().min(ram_size);
                        ram[..len].copy_from_slice(&data[..len]);
                    }
                }
            }
        }

        Self {
            rom,
            ram,
            num_rom_banks,
            num_ram_banks,
            ram_enabled: false,
            rom_bank_low: 1,
            ram_bank_high: 0,
            banking_mode: 0,
            has_battery,
            save_path,
            dirty: false,
        }
    }

    fn current_rom_bank_0(&self) -> usize {
        if self.banking_mode == 1 {
            ((self.ram_bank_high << 5) as usize) % self.num_rom_banks
        } else {
            0
        }
    }

    fn current_rom_bank_high(&self) -> usize {
        let low = if self.rom_bank_low == 0 { 1 } else { self.rom_bank_low };
        let bank = ((self.ram_bank_high << 5) | low) as usize;
        bank % self.num_rom_banks
    }

    fn current_ram_bank(&self) -> usize {
        if self.num_ram_banks == 0 {
            return 0;
        }
        if self.banking_mode == 1 {
            (self.ram_bank_high as usize) % self.num_ram_banks
        } else {
            0
        }
    }
}

impl Cartridge for Mbc1 {
    fn read_rom(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => {
                let bank = self.current_rom_bank_0();
                let idx = bank * 16384 + (addr as usize);
                self.rom.get(idx).copied().unwrap_or(0xFF)
            }
            0x4000..=0x7FFF => {
                let bank = self.current_rom_bank_high();
                let idx = bank * 16384 + ((addr - 0x4000) as usize);
                self.rom.get(idx).copied().unwrap_or(0xFF)
            }
            _ => 0xFF,
        }
    }

    fn write_rom(&mut self, addr: u16, val: u8) {
        match addr {
            0x0000..=0x1FFF => {
                self.ram_enabled = (val & 0x0F) == 0x0A;
            }
            0x2000..=0x3FFF => {
                let mut bank = val & 0x1F;
                if bank == 0 {
                    bank = 1;
                }
                self.rom_bank_low = bank;
            }
            0x4000..=0x5FFF => {
                self.ram_bank_high = val & 0x03;
            }
            0x6000..=0x7FFF => {
                self.banking_mode = val & 0x01;
            }
            _ => {}
        }
    }

    fn read_ram(&self, addr: u16) -> u8 {
        if !self.ram_enabled || self.num_ram_banks == 0 || addr < 0xA000 || addr > 0xBFFF {
            return 0xFF;
        }
        let bank = self.current_ram_bank();
        let idx = bank * 8192 + ((addr - 0xA000) as usize);
        self.ram.get(idx).copied().unwrap_or(0xFF)
    }

    fn write_ram(&mut self, addr: u16, val: u8) {
        if !self.ram_enabled || self.num_ram_banks == 0 || addr < 0xA000 || addr > 0xBFFF {
            return;
        }
        let bank = self.current_ram_bank();
        let idx = bank * 8192 + ((addr - 0xA000) as usize);
        if idx < self.ram.len() {
            self.ram[idx] = val;
            self.dirty = true;
        }
    }

    fn save_sram(&self, path: &Path) -> io::Result<()> {
        if self.ram.is_empty() {
            return Ok(());
        }
        save_sram_atomic(path, &self.ram)
    }
}

impl Drop for Mbc1 {
    fn drop(&mut self) {
        if self.has_battery && self.dirty && !self.ram.is_empty() {
            if let Some(ref path) = self.save_path {
                let _ = save_sram_atomic(path, &self.ram);
            }
        }
    }
}
