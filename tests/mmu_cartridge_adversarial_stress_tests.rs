use gb_emulator::cartridge::{create_cartridge, save_sram_atomic, CartridgeError, CartridgeHeader};
use gb_emulator::mmu::bus::Bus;
use gb_emulator::mmu::Mmu;
use std::fs;

fn build_test_rom(cart_type: u8, rom_size_code: u8, ram_size_code: u8) -> Vec<u8> {
    let size = match rom_size_code {
        0x00 => 32 * 1024,
        0x01 => 64 * 1024,
        0x02 => 128 * 1024,
        0x03 => 256 * 1024,
        0x04 => 512 * 1024,
        0x05 => 1024 * 1024,
        0x06 => 2048 * 1024,
        0x07 => 4096 * 1024,
        0x08 => 8192 * 1024,
        _ => 32 * 1024,
    };
    let mut rom = vec![0u8; size];
    let title = b"STRESSTEST";
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
fn stress_test_unusable_memory_region_fea0_feff() {
    let mut mmu = Mmu::new();
    for addr in 0xFEA0..=0xFEFF {
        // Writes should be ignored
        mmu.write_byte(addr, 0x42);
        assert_eq!(
            mmu.read_byte(addr),
            0xFF,
            "Unusable address 0x{addr:04X} should read 0xFF"
        );
    }
}

#[test]
fn stress_test_unmapped_io_registers_read_write() {
    let mut mmu = Mmu::new();
    let unmapped_addrs = [
        0xFF03, 0xFF08, 0xFF09, 0xFF0A, 0xFF0B, 0xFF0C, 0xFF0D, 0xFF0E, 0xFF10, 0xFF20, 0xFF3F,
        0xFF4C, 0xFF4D, 0xFF50, 0xFF70, 0xFF7F,
    ];

    for &addr in &unmapped_addrs {
        mmu.write_byte(addr, 0xA5);
        assert_eq!(
            mmu.read_byte(addr),
            0xFF,
            "Unmapped IO register 0x{addr:04X} should read 0xFF"
        );
    }
}

#[test]
fn stress_test_io_register_read_masks() {
    let mut mmu = Mmu::new();

    // Joypad (0xFF00): upper 2 bits (6-7) are always 1
    mmu.write_byte(0xFF00, 0x00);
    assert_eq!(mmu.read_byte(0xFF00) & 0xC0, 0xC0);

    // Serial SC (0xFF02): bits 1-6 are always 1
    mmu.write_byte(0xFF02, 0x00);
    assert_eq!(mmu.read_byte(0xFF02) & 0x7E, 0x7E);

    // TAC (0xFF07): upper 5 bits (3-7) are always 1
    mmu.write_byte(0xFF07, 0x00);
    assert_eq!(mmu.read_byte(0xFF07) & 0xF8, 0xF8);

    // IF (0xFF0F): upper 3 bits (5-7) are always 1
    mmu.write_byte(0xFF0F, 0x00);
    assert_eq!(mmu.read_byte(0xFF0F) & 0xE0, 0xE0);

    // STAT (0xFF41): bit 7 is always 1
    mmu.write_byte(0xFF41, 0x00);
    assert_eq!(mmu.read_byte(0xFF41) & 0x80, 0x80);

    // IE (0xFFFF): upper 3 bits are 0 (masked with 0x1F)
    mmu.write_byte(0xFFFF, 0xFF);
    assert_eq!(mmu.read_byte(0xFFFF), 0x1F);
}

#[test]
fn stress_test_echo_ram_full_span_bidirectional_mirroring() {
    let mut mmu = Mmu::new();

    // Fill Echo RAM (0xE000..=0xFDFF) with pattern
    for addr in 0xE000..=0xFDFF {
        let val = (addr & 0xFF) as u8;
        mmu.write_byte(addr, val);
    }

    // Verify reading corresponding WRAM (0xC000..=0xDDFF)
    for i in 0..0x1E00 {
        let wram_addr = 0xC000 + i as u16;
        let echo_addr = 0xE000 + i as u16;
        let expected = (echo_addr & 0xFF) as u8;
        assert_eq!(
            mmu.read_byte(wram_addr),
            expected,
            "WRAM address 0x{wram_addr:04X} mismatch"
        );
    }

    // Now overwrite via WRAM and verify Echo RAM reflects change
    for addr in 0xC000..=0xDDFF {
        let val = (!addr & 0xFF) as u8;
        mmu.write_byte(addr, val);
    }

    for i in 0..0x1E00 {
        let wram_addr = 0xC000 + i as u16;
        let echo_addr = 0xE000 + i as u16;
        let expected = (!wram_addr & 0xFF) as u8;
        assert_eq!(
            mmu.read_byte(echo_addr),
            expected,
            "Echo RAM address 0x{echo_addr:04X} mismatch"
        );
    }

    // Verify 0xFE00 (OAM start) does NOT mirror WRAM 0xDE00
    mmu.write_byte(0xDE00, 0x77);
    mmu.write_byte(0xFE00, 0x88);
    assert_eq!(mmu.read_byte(0xDE00), 0x77);
    assert_eq!(mmu.read_byte(0xFE00), 0x88);
}

#[test]
fn stress_test_oam_dma_from_various_memory_regions() {
    let mut mmu = Mmu::new();

    // 1. From WRAM (0xC000)
    for i in 0..160 {
        mmu.write_byte(0xC000 + i as u16, (i as u8).wrapping_add(10));
    }
    mmu.write_byte(0xFF46, 0xC0);
    for i in 0..160 {
        assert_eq!(mmu.read_byte(0xFE00 + i as u16), (i as u8).wrapping_add(10));
    }

    // 2. From VRAM (0x8000)
    for i in 0..160 {
        mmu.write_byte(0x8000 + i as u16, (i as u8).wrapping_add(50));
    }
    mmu.write_byte(0xFF46, 0x80);
    for i in 0..160 {
        assert_eq!(mmu.read_byte(0xFE00 + i as u16), (i as u8).wrapping_add(50));
    }

    // 3. From page 0xFF (0xFF00..0xFF9F: I/O + HRAM)
    for i in 0..32 {
        mmu.write_byte(0xFF80 + i as u16, (i as u8).wrapping_add(90));
    }
    mmu.write_byte(0xFF46, 0xFF); // DMA from 0xFF00
    for i in 0..32 {
        assert_eq!(
            mmu.read_byte(0xFE00 + 128 + i as u16),
            (i as u8).wrapping_add(90)
        );
    }
}

#[test]
fn stress_test_div_reset_and_tcycle_stepping() {
    let mut mmu = Mmu::new();

    // Initial DIV should be 0
    assert_eq!(mmu.read_byte(0xFF04), 0);

    // Step 255 cycles: DIV still 0
    mmu.step_components(255);
    assert_eq!(mmu.read_byte(0xFF04), 0);

    // Step 1 more cycle: DIV becomes 1 (256 T-cycles)
    mmu.step_components(1);
    assert_eq!(mmu.read_byte(0xFF04), 1);

    // Step 256 * 10 cycles: DIV becomes 11
    mmu.step_components(256 * 10);
    assert_eq!(mmu.read_byte(0xFF04), 11);

    // Write to DIV resets it back to 0
    mmu.write_byte(0xFF04, 0x42);
    assert_eq!(mmu.read_byte(0xFF04), 0);

    // Step 256 cycles after reset: DIV becomes 1
    mmu.step_components(256);
    assert_eq!(mmu.read_byte(0xFF04), 1);
}

#[test]
fn stress_test_mbc1_large_rom_64_banks_mode0_and_mode1() {
    let mut rom = build_test_rom(0x03, 0x05, 0x03); // MBC1 + 1MB ROM (64 banks) + 32KB RAM

    // Mark each 16KB ROM bank with unique signature: bank index
    for bank in 0..64 {
        rom[bank * 16384] = bank as u8;
    }

    let mut cart = create_cartridge(rom, None).expect("Factory failed");

    // Enable RAM
    cart.write_rom(0x0000, 0x0A);

    // In Mode 0 (default): 0x0000..=0x3FFF is ALWAYS Bank 0
    assert_eq!(cart.read_rom(0x0000), 0);

    // Select Low Bank 5 -> 0x4000..=0x7FFF displays Bank 5
    cart.write_rom(0x2000, 0x05);
    assert_eq!(cart.read_rom(0x4000), 5);
    assert_eq!(cart.read_rom(0x0000), 0);

    // Select Low Bank 0 -> remaps to Bank 1
    cart.write_rom(0x2000, 0x00);
    assert_eq!(cart.read_rom(0x4000), 1);

    // Select High Bank bits (RAM bank high = 1, i.e. bit 5 set -> bank + 32)
    cart.write_rom(0x4000, 0x01);
    // Now 0x4000 displays Bank (1 | 32) = 33
    assert_eq!(cart.read_rom(0x4000), 33);
    // But 0x0000 still displays Bank 0 in Mode 0!
    assert_eq!(cart.read_rom(0x0000), 0);

    // Switch to Banking Mode 1 (RAM Banking Mode)
    cart.write_rom(0x6000, 0x01);
    // In Mode 1, 0x0000..=0x3FFF displays Bank (0 | 32) = 32!
    assert_eq!(cart.read_rom(0x0000), 32);

    // Test RAM bank switching in Mode 1
    cart.write_rom(0x4000, 0x02); // Select RAM Bank 2
    cart.write_ram(0xA000, 0x77);

    cart.write_rom(0x4000, 0x03); // Select RAM Bank 3
    cart.write_ram(0xA000, 0x88);

    cart.write_rom(0x4000, 0x02); // Read RAM Bank 2
    assert_eq!(cart.read_ram(0xA000), 0x77);

    cart.write_rom(0x4000, 0x03); // Read RAM Bank 3
    assert_eq!(cart.read_ram(0xA000), 0x88);
}

#[test]
fn stress_test_mbc3_7bit_banking_and_rtc_latching() {
    let mut rom = build_test_rom(0x10, 0x06, 0x03); // MBC3 + 2MB ROM (128 banks) + 32KB RAM + RTC

    for bank in 0..128 {
        rom[bank * 16384] = (bank & 0xFF) as u8;
    }

    let mut cart = create_cartridge(rom, None).expect("Factory failed");

    // Enable RAM & RTC
    cart.write_rom(0x0000, 0x0A);

    // Select Bank 0x7F (127)
    cart.write_rom(0x2000, 0x7F);
    assert_eq!(cart.read_rom(0x4000), 127);

    // Select Bank 0 -> remaps to 1
    cart.write_rom(0x2000, 0x00);
    assert_eq!(cart.read_rom(0x4000), 1);

    // Set RTC registers
    cart.write_rom(0x4000, 0x08); // Seconds
    cart.write_ram(0xA000, 59);

    cart.write_rom(0x4000, 0x09); // Minutes
    cart.write_ram(0xA000, 42);

    cart.write_rom(0x4000, 0x0A); // Hours
    cart.write_ram(0xA000, 14);

    // Before latching, reads return latched values (default 0)
    assert_eq!(cart.read_ram(0xA000), 0);

    // Perform Latch sequence: write 0x00 then 0x01 to 0x6000
    cart.write_rom(0x6000, 0x00);
    cart.write_rom(0x6000, 0x01);

    // Now latched value for Hours (reg 0x0A) should be 14
    assert_eq!(cart.read_ram(0xA000), 14);

    // Switch to Seconds (reg 0x08)
    cart.write_rom(0x4000, 0x08);
    assert_eq!(cart.read_ram(0xA000), 59);
}

#[test]
fn stress_test_mbc5_9bit_banking_and_bank0_selection() {
    let mut rom = build_test_rom(0x19, 0x07, 0x04); // MBC5 + 4MB ROM (256 banks) + 128KB RAM

    for bank in 0..256 {
        rom[bank * 16384] = (bank & 0xFF) as u8;
    }

    let mut cart = create_cartridge(rom, None).expect("Factory failed");

    // Enable RAM
    cart.write_rom(0x0000, 0x0A);

    // In MBC5, Bank 0 can be selected at 0x4000..=0x7FFF
    cart.write_rom(0x2000, 0x00);
    cart.write_rom(0x3000, 0x00);
    assert_eq!(cart.read_rom(0x4000), 0);

    // Select Bank 255 (0xFF)
    cart.write_rom(0x2000, 0xFF);
    assert_eq!(cart.read_rom(0x4000), 255);

    // Test RAM bank 0..15 switching
    for ram_bank in 0..16 {
        cart.write_rom(0x4000, ram_bank);
        cart.write_ram(0xA000, ram_bank + 0x10);
    }

    for ram_bank in 0..16 {
        cart.write_rom(0x4000, ram_bank);
        assert_eq!(cart.read_ram(0xA000), ram_bank + 0x10);
    }
}

#[test]
fn stress_test_cartridge_header_validation_errors() {
    // 1. ROM too small
    let err = CartridgeHeader::parse(&[0u8; 100]).unwrap_err();
    assert!(matches!(err, CartridgeError::RomTooSmall(100)));

    // 2. Invalid header checksum
    let mut rom = build_test_rom(0x00, 0x00, 0x00);
    rom[0x014D] ^= 0xFF;
    let err = CartridgeHeader::parse(&rom).unwrap_err();
    assert!(matches!(err, CartridgeError::InvalidHeaderChecksum { .. }));

    // 3. Unsupported cartridge type
    let rom = build_test_rom(0xFF, 0x00, 0x00);
    let err = match create_cartridge(rom, None) {
        Err(e) => e,
        Ok(_) => panic!("Expected error for unsupported cartridge type"),
    };
    assert_eq!(err, CartridgeError::UnsupportedCartridgeType(0xFF));
}

#[test]
fn stress_test_atomic_sram_file_creation() {
    let temp_dir = std::env::temp_dir().join("gb_emulator_stress_sram");
    let save_path = temp_dir.join("save_file.sav");
    let _ = fs::remove_file(&save_path);

    let data = vec![0xDE, 0xAD, 0xBE, 0xEF];
    save_sram_atomic(&save_path, &data).unwrap();

    assert!(save_path.exists());
    let read_back = fs::read(&save_path).unwrap();
    assert_eq!(read_back, data);

    let _ = fs::remove_file(&save_path);
    let _ = fs::remove_dir_all(&temp_dir);
}
