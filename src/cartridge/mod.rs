pub mod mbc1;
pub mod mbc3;
pub mod mbc5;
pub mod mbcless;

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub use mbc1::Mbc1;
pub use mbc3::Mbc3;
pub use mbc5::Mbc5;
pub use mbcless::NoMbc;

/// Trait implemented by all Game Boy cartridge mappers.
pub trait Cartridge: Send + Sync {
    /// Read a byte from ROM space (0x0000..=0x7FFF).
    fn read_rom(&self, addr: u16) -> u8;

    /// Write a byte to ROM space (0x0000..=0x7FFF) to control mapper registers/banking.
    fn write_rom(&mut self, addr: u16, val: u8);

    /// Read a byte from external RAM space (0xA000..=0xBFFF).
    fn read_ram(&self, addr: u16) -> u8;

    /// Write a byte to external RAM space (0xA000..=0xBFFF).
    fn write_ram(&mut self, addr: u16, val: u8);

    /// Save battery-backed SRAM contents to disk at `path`.
    fn save_sram(&self, path: &Path) -> io::Result<()>;
}

/// Atomic SRAM file saver helper to prevent save corruption.
pub fn save_sram_atomic(path: &Path, data: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let tmp_path = path.with_extension("sav.tmp");
    fs::write(&tmp_path, data)?;
    fs::rename(&tmp_path, path)?;
    Ok(())
}

/// Errors encountered during cartridge parsing or initialization.
#[derive(Debug, PartialEq, Eq)]
pub enum CartridgeError {
    RomTooSmall(usize),
    InvalidHeaderChecksum { calculated: u8, expected: u8 },
    UnsupportedCartridgeType(u8),
    InvalidRomSize(u8),
    InvalidRamSize(u8),
    SaveFileError(String),
}

impl fmt::Display for CartridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CartridgeError::RomTooSmall(size) => {
                write!(f, "ROM data is too small: {size} bytes (minimum 0x0150 required)")
            }
            CartridgeError::InvalidHeaderChecksum { calculated, expected } => {
                write!(
                    f,
                    "Header checksum mismatch: calculated 0x{calculated:02X}, expected 0x{expected:02X}"
                )
            }
            CartridgeError::UnsupportedCartridgeType(cart_type) => {
                write!(f, "Unsupported cartridge type: 0x{cart_type:02X}")
            }
            CartridgeError::InvalidRomSize(code) => write!(f, "Invalid ROM size code: 0x{code:02X}"),
            CartridgeError::InvalidRamSize(code) => write!(f, "Invalid RAM size code: 0x{code:02X}"),
            CartridgeError::SaveFileError(msg) => write!(f, "Save file error: {msg}"),
        }
    }
}

impl std::error::Error for CartridgeError {}

/// Game Boy Cartridge Header structure parsed from ROM bytes 0x0100..=0x014F.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CartridgeHeader {
    pub title: String,
    pub cgb_flag: u8,
    pub sgb_flag: u8,
    pub cartridge_type: u8,
    pub rom_size_code: u8,
    pub ram_size_code: u8,
    pub destination_code: u8,
    pub old_licensee_code: u8,
    pub mask_rom_version: u8,
    pub header_checksum: u8,
    pub global_checksum: u16,
}

impl CartridgeHeader {
    pub fn parse(rom_data: &[u8]) -> Result<Self, CartridgeError> {
        if rom_data.len() < 0x0150 {
            return Err(CartridgeError::RomTooSmall(rom_data.len()));
        }

        // Verify header checksum over 0x0134..=0x014C
        let mut calculated: u8 = 0;
        for &byte in &rom_data[0x0134..=0x014C] {
            calculated = calculated.wrapping_sub(byte).wrapping_sub(1);
        }
        let expected = rom_data[0x014D];
        if calculated != expected {
            return Err(CartridgeError::InvalidHeaderChecksum { calculated, expected });
        }

        let title_bytes = &rom_data[0x0134..=0x0143];
        let title_end = title_bytes.iter().position(|&b| b == 0).unwrap_or(title_bytes.len());
        let title = String::from_utf8_lossy(&title_bytes[..title_end]).to_string();

        let cgb_flag = rom_data[0x0143];
        let sgb_flag = rom_data[0x0146];
        let cartridge_type = rom_data[0x0147];
        let rom_size_code = rom_data[0x0148];
        let ram_size_code = rom_data[0x0149];
        let destination_code = rom_data[0x014A];
        let old_licensee_code = rom_data[0x014B];
        let mask_rom_version = rom_data[0x014C];
        let header_checksum = expected;
        let global_checksum = u16::from_be_bytes([rom_data[0x014E], rom_data[0x014F]]);

        Ok(Self {
            title,
            cgb_flag,
            sgb_flag,
            cartridge_type,
            rom_size_code,
            ram_size_code,
            destination_code,
            old_licensee_code,
            mask_rom_version,
            header_checksum,
            global_checksum,
        })
    }

    pub fn rom_size_bytes(&self) -> Result<usize, CartridgeError> {
        match self.rom_size_code {
            0x00 => Ok(32 * 1024),
            0x01 => Ok(64 * 1024),
            0x02 => Ok(128 * 1024),
            0x03 => Ok(256 * 1024),
            0x04 => Ok(512 * 1024),
            0x05 => Ok(1024 * 1024),
            0x06 => Ok(2048 * 1024),
            0x07 => Ok(4096 * 1024),
            0x08 => Ok(8192 * 1024),
            0x52 => Ok(1179648),
            0x53 => Ok(1310720),
            0x54 => Ok(1572864),
            other => Err(CartridgeError::InvalidRomSize(other)),
        }
    }

    pub fn ram_size_bytes(&self) -> Result<usize, CartridgeError> {
        match self.ram_size_code {
            0x00 => Ok(0),
            0x01 => Ok(2048),
            0x02 => Ok(8192),
            0x03 => Ok(32768),
            0x04 => Ok(131072),
            0x05 => Ok(65536),
            other => Err(CartridgeError::InvalidRamSize(other)),
        }
    }

    pub fn has_battery(&self) -> bool {
        matches!(
            self.cartridge_type,
            0x03 | 0x06 | 0x09 | 0x0F | 0x10 | 0x13 | 0x1B | 0x1E
        )
    }

    pub fn has_rtc(&self) -> bool {
        matches!(self.cartridge_type, 0x0F | 0x10)
    }
}

/// Factory function to parse header and instantiate the appropriate Cartridge mapper.
pub fn create_cartridge(
    rom_data: Vec<u8>,
    save_path: Option<PathBuf>,
) -> Result<Box<dyn Cartridge>, CartridgeError> {
    let header = CartridgeHeader::parse(&rom_data)?;
    let ram_size = header.ram_size_bytes()?;
    let has_battery = header.has_battery();
    let has_rtc = header.has_rtc();

    let sram_data = if has_battery {
        if let Some(ref path) = save_path {
            if path.exists() {
                match fs::read(path) {
                    Ok(bytes) => Some(bytes),
                    Err(e) => return Err(CartridgeError::SaveFileError(e.to_string())),
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    match header.cartridge_type {
        0x00 | 0x08 | 0x09 => Ok(Box::new(NoMbc::new(
            rom_data,
            ram_size,
            has_battery,
            save_path,
            sram_data,
        ))),
        0x01 | 0x02 | 0x03 => Ok(Box::new(Mbc1::new(
            rom_data,
            ram_size,
            has_battery,
            save_path,
            sram_data,
        ))),
        0x0F | 0x10 | 0x11 | 0x12 | 0x13 => Ok(Box::new(Mbc3::new(
            rom_data,
            ram_size,
            has_battery,
            has_rtc,
            save_path,
            sram_data,
        ))),
        0x19 | 0x1A | 0x1B | 0x1C | 0x1D | 0x1E => Ok(Box::new(Mbc5::new(
            rom_data,
            ram_size,
            has_battery,
            save_path,
            sram_data,
        ))),
        other => Err(CartridgeError::UnsupportedCartridgeType(other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn build_test_rom(cart_type: u8, rom_size_code: u8, ram_size_code: u8) -> Vec<u8> {
        let size = match rom_size_code {
            0x00 => 32 * 1024,
            0x01 => 64 * 1024,
            0x02 => 128 * 1024,
            0x03 => 256 * 1024,
            0x04 => 512 * 1024,
            _ => 32 * 1024,
        };
        let mut rom = vec![0u8; size];
        let title = b"TESTGAME";
        rom[0x0134..0x0134 + title.len()].copy_from_slice(title);
        rom[0x0147] = cart_type;
        rom[0x0148] = rom_size_code;
        rom[0x0149] = ram_size_code;

        // Compute valid checksum
        let mut chk: u8 = 0;
        for &b in &rom[0x0134..=0x014C] {
            chk = chk.wrapping_sub(b).wrapping_sub(1);
        }
        rom[0x014D] = chk;
        rom
    }

    #[test]
    fn test_header_parsing_valid_and_invalid_checksum() {
        let mut rom = build_test_rom(0x00, 0x00, 0x00);
        let header = CartridgeHeader::parse(&rom).expect("Header parsing failed");
        assert_eq!(header.title, "TESTGAME");
        assert_eq!(header.cartridge_type, 0x00);

        // Corrupt checksum
        rom[0x014D] = rom[0x014D].wrapping_add(1);
        let err = CartridgeHeader::parse(&rom).unwrap_err();
        assert!(matches!(err, CartridgeError::InvalidHeaderChecksum { .. }));
    }

    #[test]
    fn test_header_rom_too_small() {
        let rom = vec![0u8; 0x100];
        let err = CartridgeHeader::parse(&rom).unwrap_err();
        assert_eq!(err, CartridgeError::RomTooSmall(0x100));
    }

    #[test]
    fn test_nombc_functionality() {
        let mut rom = build_test_rom(0x08, 0x00, 0x02); // NoMBC with RAM
        rom[0x0000] = 0xAA;
        rom[0x7FFF] = 0xBB;
        let mut cart = create_cartridge(rom, None).expect("Factory failed");

        assert_eq!(cart.read_rom(0x0000), 0xAA);
        assert_eq!(cart.read_rom(0x7FFF), 0xBB);

        // RAM write and read
        cart.write_ram(0xA000, 0x42);
        assert_eq!(cart.read_ram(0xA000), 0x42);
    }

    #[test]
    fn test_mbc1_banking_and_ram() {
        let mut rom = build_test_rom(0x03, 0x03, 0x03); // MBC1 + RAM + BATTERY (256KB ROM, 32KB RAM)
        // Put distinctive bytes in bank 1 and bank 2
        rom[1 * 16384] = 0x11;
        rom[2 * 16384] = 0x22;

        let mut cart = create_cartridge(rom, None).expect("Factory failed");

        // Initial Bank 1 at 0x4000
        assert_eq!(cart.read_rom(0x4000), 0x11);

        // Switch to Bank 2
        cart.write_rom(0x2000, 0x02);
        assert_eq!(cart.read_rom(0x4000), 0x22);

        // Bank 0 selection is remapped to Bank 1
        cart.write_rom(0x2000, 0x00);
        assert_eq!(cart.read_rom(0x4000), 0x11);

        // Enable RAM (write 0x0A to 0x0000..0x1FFF)
        cart.write_ram(0xA000, 0x99);
        assert_eq!(cart.read_ram(0xA000), 0xFF); // RAM is disabled!

        cart.write_rom(0x0000, 0x0A); // Enable RAM
        cart.write_ram(0xA000, 0x99);
        assert_eq!(cart.read_ram(0xA000), 0x99);
    }

    #[test]
    fn test_mbc3_banking_and_rtc() {
        let mut rom = build_test_rom(0x10, 0x03, 0x03); // MBC3 + TIMER + RAM + BATTERY
        rom[1 * 16384] = 0x31;
        rom[5 * 16384] = 0x35;

        let mut cart = create_cartridge(rom, None).expect("Factory failed");

        // Enable RAM & RTC
        cart.write_rom(0x0000, 0x0A);

        // Select ROM Bank 5
        cart.write_rom(0x2000, 0x05);
        assert_eq!(cart.read_rom(0x4000), 0x35);

        // Select RTC register 0x08 (Seconds)
        cart.write_rom(0x4000, 0x08);
        cart.write_ram(0xA000, 45); // Set seconds to 45

        // Latch clock: write 0x00 then 0x01 to 0x6000
        cart.write_rom(0x6000, 0x00);
        cart.write_rom(0x6000, 0x01);

        assert_eq!(cart.read_ram(0xA000), 45);
    }

    #[test]
    fn test_mbc5_banking_allowing_bank_0() {
        let mut rom = build_test_rom(0x19, 0x03, 0x02); // MBC5
        rom[0 * 16384] = 0x50;
        rom[1 * 16384] = 0x51;

        let mut cart = create_cartridge(rom, None).expect("Factory failed");

        // MBC5 allows selecting Bank 0 at 0x4000..=0x7FFF
        cart.write_rom(0x2000, 0x00);
        assert_eq!(cart.read_rom(0x4000), 0x50);

        cart.write_rom(0x2000, 0x01);
        assert_eq!(cart.read_rom(0x4000), 0x51);
    }

    #[test]
    fn test_sram_persistence_saving_and_loading() {
        let temp_dir = std::env::temp_dir().join("gb_emulator_tests_sram");
        let _ = fs::create_dir_all(&temp_dir);
        let save_path = temp_dir.join("test_game.sav");
        let _ = fs::remove_file(&save_path);

        let rom = build_test_rom(0x03, 0x00, 0x02); // MBC1 + RAM + BATTERY

        {
            let mut cart = create_cartridge(rom.clone(), Some(save_path.clone())).unwrap();
            cart.write_rom(0x0000, 0x0A); // Enable RAM
            cart.write_ram(0xA000, 0xDE);
            cart.write_ram(0xA001, 0xAD);
            cart.save_sram(&save_path).unwrap();
        }

        assert!(save_path.exists());

        // Load cartridge with existing save file
        let mut loaded_cart = create_cartridge(rom, Some(save_path.clone())).unwrap();
        loaded_cart.write_rom(0x0000, 0x0A); // Enable RAM
        assert_eq!(loaded_cart.read_ram(0xA000), 0xDE);
        assert_eq!(loaded_cart.read_ram(0xA001), 0xAD);

        let _ = fs::remove_file(&save_path);
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
