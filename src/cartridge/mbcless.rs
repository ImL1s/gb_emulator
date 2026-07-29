use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use super::{save_sram_atomic, Cartridge};

/// NoMBC mapper (32KB ROM, optional 8KB RAM, optional battery SRAM save persistence).
pub struct NoMbc {
    rom: Vec<u8>,
    ram: Vec<u8>,
    has_battery: bool,
    save_path: Option<PathBuf>,
    dirty: bool,
}

impl NoMbc {
    pub fn new(
        rom: Vec<u8>,
        ram_size: usize,
        has_battery: bool,
        save_path: Option<PathBuf>,
        sram_data: Option<Vec<u8>>,
    ) -> Self {
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
            has_battery,
            save_path,
            dirty: false,
        }
    }
}

impl Cartridge for NoMbc {
    fn read_rom(&self, addr: u16) -> u8 {
        let idx = addr as usize;
        if idx < self.rom.len() {
            self.rom[idx]
        } else {
            0xFF
        }
    }

    fn write_rom(&mut self, _addr: u16, _val: u8) {
        // ROM is read-only; writes to 0x0000..=0x7FFF on NoMBC are ignored.
    }

    fn read_ram(&self, addr: u16) -> u8 {
        if self.ram.is_empty() || addr < 0xA000 || addr > 0xBFFF {
            return 0xFF;
        }
        let rel_addr = (addr - 0xA000) as usize;
        if rel_addr < self.ram.len() {
            self.ram[rel_addr]
        } else {
            0xFF
        }
    }

    fn write_ram(&mut self, addr: u16, val: u8) {
        if self.ram.is_empty() || addr < 0xA000 || addr > 0xBFFF {
            return;
        }
        let rel_addr = (addr - 0xA000) as usize;
        if rel_addr < self.ram.len() {
            self.ram[rel_addr] = val;
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

impl Drop for NoMbc {
    fn drop(&mut self) {
        if self.has_battery && self.dirty && !self.ram.is_empty() {
            if let Some(ref path) = self.save_path {
                let _ = save_sram_atomic(path, &self.ram);
            }
        }
    }
}
