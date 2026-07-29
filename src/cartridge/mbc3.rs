use super::{save_sram_atomic, Cartridge};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct Rtc {
    pub seconds: u8,
    pub minutes: u8,
    pub hours: u8,
    pub day_low: u8,
    pub day_high: u8,
    pub latched_seconds: u8,
    pub latched_minutes: u8,
    pub latched_hours: u8,
    pub latched_day_low: u8,
    pub latched_day_high: u8,
    pub latch_state: u8,
}

impl Rtc {
    pub fn latch(&mut self) {
        self.latched_seconds = self.seconds;
        self.latched_minutes = self.minutes;
        self.latched_hours = self.hours;
        self.latched_day_low = self.day_low;
        self.latched_day_high = self.day_high;
    }

    pub fn read_reg(&self, reg: u8) -> u8 {
        match reg {
            0x08 => self.latched_seconds,
            0x09 => self.latched_minutes,
            0x0A => self.latched_hours,
            0x0B => self.latched_day_low,
            0x0C => self.latched_day_high,
            _ => 0xFF,
        }
    }

    pub fn write_reg(&mut self, reg: u8, val: u8) {
        match reg {
            0x08 => self.seconds = val % 60,
            0x09 => self.minutes = val % 60,
            0x0A => self.hours = val % 24,
            0x0B => self.day_low = val,
            0x0C => self.day_high = val,
            _ => {}
        }
    }
}

/// MBC3 mapper (up to 2MB ROM, 32KB RAM, Real-Time Clock RTC registers & clock latching).
pub struct Mbc3 {
    rom: Vec<u8>,
    ram: Vec<u8>,
    num_rom_banks: usize,
    num_ram_banks: usize,
    ram_rtc_enabled: bool,
    rom_bank: u8,       // 7 bits (1..=127, 0 maps to 1)
    ram_rtc_select: u8, // 0x00..=0x03 for RAM, 0x08..=0x0C for RTC
    rtc: Rtc,
    has_battery: bool,
    _has_rtc: bool,
    save_path: Option<PathBuf>,
    dirty: bool,
}

impl Mbc3 {
    pub fn new(
        rom: Vec<u8>,
        ram_size: usize,
        has_battery: bool,
        has_rtc: bool,
        save_path: Option<PathBuf>,
        sram_data: Option<Vec<u8>>,
    ) -> Self {
        let num_rom_banks = (rom.len() / 16384).max(1);
        let num_ram_banks = if ram_size > 0 {
            (ram_size / 8192).max(1)
        } else {
            0
        };
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
            ram_rtc_enabled: false,
            rom_bank: 1,
            ram_rtc_select: 0,
            rtc: Rtc::default(),
            has_battery,
            _has_rtc: has_rtc,
            save_path,
            dirty: false,
        }
    }
}

impl Cartridge for Mbc3 {
    fn read_rom(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => self.rom.get(addr as usize).copied().unwrap_or(0xFF),
            0x4000..=0x7FFF => {
                let bank = if self.rom_bank == 0 {
                    1
                } else {
                    self.rom_bank as usize
                };
                let bank = bank % self.num_rom_banks;
                let idx = bank * 16384 + ((addr - 0x4000) as usize);
                self.rom.get(idx).copied().unwrap_or(0xFF)
            }
            _ => 0xFF,
        }
    }

    fn write_rom(&mut self, addr: u16, val: u8) {
        match addr {
            0x0000..=0x1FFF => {
                self.ram_rtc_enabled = (val & 0x0F) == 0x0A;
            }
            0x2000..=0x3FFF => {
                let mut bank = val & 0x7F;
                if bank == 0 {
                    bank = 1;
                }
                self.rom_bank = bank;
            }
            0x4000..=0x5FFF => {
                self.ram_rtc_select = val;
            }
            0x6000..=0x7FFF => {
                if self.rtc.latch_state == 0x00 && val == 0x01 {
                    self.rtc.latch();
                }
                self.rtc.latch_state = val;
            }
            _ => {}
        }
    }

    fn read_ram(&self, addr: u16) -> u8 {
        if !self.ram_rtc_enabled || !(0xA000..=0xBFFF).contains(&addr) {
            return 0xFF;
        }
        match self.ram_rtc_select {
            0x00..=0x03 => {
                if self.num_ram_banks == 0 {
                    return 0xFF;
                }
                let bank = (self.ram_rtc_select as usize) % self.num_ram_banks;
                let idx = bank * 8192 + ((addr - 0xA000) as usize);
                self.ram.get(idx).copied().unwrap_or(0xFF)
            }
            0x08..=0x0C => self.rtc.read_reg(self.ram_rtc_select),
            _ => 0xFF,
        }
    }

    fn write_ram(&mut self, addr: u16, val: u8) {
        if !self.ram_rtc_enabled || !(0xA000..=0xBFFF).contains(&addr) {
            return;
        }
        match self.ram_rtc_select {
            0x00..=0x03 => {
                if self.num_ram_banks == 0 {
                    return;
                }
                let bank = (self.ram_rtc_select as usize) % self.num_ram_banks;
                let idx = bank * 8192 + ((addr - 0xA000) as usize);
                if idx < self.ram.len() {
                    self.ram[idx] = val;
                    self.dirty = true;
                }
            }
            0x08..=0x0C => self.rtc.write_reg(self.ram_rtc_select, val),
            _ => {}
        }
    }

    fn save_sram(&self, path: &Path) -> io::Result<()> {
        if self.ram.is_empty() {
            return Ok(());
        }
        save_sram_atomic(path, &self.ram)
    }
}

impl Drop for Mbc3 {
    fn drop(&mut self) {
        if self.has_battery && self.dirty && !self.ram.is_empty() {
            if let Some(ref path) = self.save_path {
                let _ = save_sram_atomic(path, &self.ram);
            }
        }
    }
}
