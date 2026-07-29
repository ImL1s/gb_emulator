use gb_emulator::cartridge::{
    create_cartridge, save_sram_atomic, CartridgeError, CartridgeHeader,
};
use gb_emulator::mmu::bus::Bus;
use gb_emulator::mmu::Mmu;
use std::fs;

/// Helper function to construct a valid Game Boy ROM header with given parameters.
fn make_test_rom(cart_type: u8, rom_size_code: u8, ram_size_code: u8) -> Vec<u8> {
    let rom_bytes = match rom_size_code {
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
    let mut rom = vec![0u8; rom_bytes];

    // Write title
    let title = b"CHALLENGER_GB";
    rom[0x0134..0x0134 + title.len()].copy_from_slice(title);

    rom[0x0147] = cart_type;
    rom[0x0148] = rom_size_code;
    rom[0x0149] = ram_size_code;

    // Calculate valid header checksum
    let mut chk: u8 = 0;
    for &b in &rom[0x0134..=0x014C] {
        chk = chk.wrapping_sub(b).wrapping_sub(1);
    }
    rom[0x014D] = chk;

    rom
}

// ============================================================================
// 1. CARTRIDGE HEADER PARSING CHALLENGES
// ============================================================================

#[test]
fn challenge_header_parsing_and_checksum_validation() {
    let rom = make_test_rom(0x01, 0x01, 0x02);
    let header = CartridgeHeader::parse(&rom).expect("Valid header parse failed");

    assert_eq!(header.title, "CHALLENGER_GB");
    assert_eq!(header.cartridge_type, 0x01);
    assert_eq!(header.rom_size_code, 0x01);
    assert_eq!(header.ram_size_code, 0x02);
    assert_eq!(header.rom_size_bytes().unwrap(), 64 * 1024);
    assert_eq!(header.ram_size_bytes().unwrap(), 8192);

    // Corrupt header checksum
    let mut bad_rom = rom.clone();
    bad_rom[0x014D] ^= 0xFF;
    let err = CartridgeHeader::parse(&bad_rom).unwrap_err();
    assert!(
        matches!(err, CartridgeError::InvalidHeaderChecksum { .. }),
        "Corrupted checksum did not return InvalidHeaderChecksum"
    );
}

#[test]
fn challenge_header_null_terminated_title_and_short_rom() {
    let mut rom = make_test_rom(0x00, 0x00, 0x00);
    // Insert null byte in middle of title and recompute header checksum
    rom[0x0134 + 4] = 0;
    let mut chk: u8 = 0;
    for &b in &rom[0x0134..=0x014C] {
        chk = chk.wrapping_sub(b).wrapping_sub(1);
    }
    rom[0x014D] = chk;

    let header = CartridgeHeader::parse(&rom).expect("Header parse failed");
    assert_eq!(header.title, "CHAL", "Title must be truncated at null byte");

    // Short ROM < 0x0150
    let short_rom = vec![0u8; 0x014F];
    let err = CartridgeHeader::parse(&short_rom).unwrap_err();
    assert_eq!(err, CartridgeError::RomTooSmall(0x014F));
}

#[test]
fn challenge_header_unsupported_mappers_and_invalid_sizes() {
    let mut rom = make_test_rom(0xFF, 0x00, 0x00);
    // Fix checksum after changing cart type
    let mut chk: u8 = 0;
    for &b in &rom[0x0134..=0x014C] {
        chk = chk.wrapping_sub(b).wrapping_sub(1);
    }
    rom[0x014D] = chk;

    let err = match create_cartridge(rom, None) {
        Err(e) => e,
        Ok(_) => panic!("Expected error for unsupported cartridge type"),
    };
    assert_eq!(err, CartridgeError::UnsupportedCartridgeType(0xFF));
}

// ============================================================================
// 2. NOMBC MAPPER CHALLENGES
// ============================================================================

#[test]
fn challenge_nombc_rom_ram_access_and_bounds() {
    let mut rom = make_test_rom(0x08, 0x00, 0x02); // NoMBC + RAM + BATTERY
    rom[0x0000] = 0x11;
    rom[0x3FFF] = 0x22;
    rom[0x4000] = 0x33;
    rom[0x7FFF] = 0x44;

    let mut cart = create_cartridge(rom, None).expect("NoMBC creation failed");

    // Read ROM
    assert_eq!(cart.read_rom(0x0000), 0x11);
    assert_eq!(cart.read_rom(0x3FFF), 0x22);
    assert_eq!(cart.read_rom(0x4000), 0x33);
    assert_eq!(cart.read_rom(0x7FFF), 0x44);

    // Writes to ROM ignored
    cart.write_rom(0x0000, 0x99);
    assert_eq!(cart.read_rom(0x0000), 0x11);

    // Read & Write RAM
    cart.write_ram(0xA000, 0x77);
    cart.write_ram(0xBFFF, 0x88);
    assert_eq!(cart.read_ram(0xA000), 0x77);
    assert_eq!(cart.read_ram(0xBFFF), 0x88);
}

// ============================================================================
// 3. MBC1 MAPPER BANK SWITCHING & BANKING MODE CHALLENGES
// ============================================================================

#[test]
fn challenge_mbc1_bank_0_remapping_and_zero_bank_writes() {
    // MBC1 with 512KB ROM (32 banks), 32KB RAM (4 banks), Battery
    let mut rom = make_test_rom(0x03, 0x04, 0x03);

    // Populate each 16KB bank with its bank number
    for bank in 0..32 {
        rom[bank * 16384] = bank as u8;
    }

    let mut cart = create_cartridge(rom, None).expect("MBC1 creation failed");

    // Initial state: Bank 0 at 0x0000..=0x3FFF, Bank 1 at 0x4000..=0x7FFF
    assert_eq!(cart.read_rom(0x0000), 0x00);
    assert_eq!(cart.read_rom(0x4000), 0x01);

    // Write 0 to ROM Bank Low register (0x2000) -> MUST remap 0 to 1
    cart.write_rom(0x2000, 0x00);
    assert_eq!(cart.read_rom(0x4000), 0x01, "Bank 0 write must remap to Bank 1");

    // Write 0x20 (32) to ROM Bank Low register (0x2000) -> 0x20 & 0x1F = 0 -> MUST remap to 1 (Bank 1)
    cart.write_rom(0x2000, 0x20);
    assert_eq!(cart.read_rom(0x4000), 0x01, "Bank 0x20 write must remap to Bank 1");

    // Select Bank 5
    cart.write_rom(0x2000, 0x05);
    assert_eq!(cart.read_rom(0x4000), 0x05);
}

#[test]
fn challenge_mbc1_mode_0_vs_mode_1_rom_ram_banking() {
    // 1MB ROM (64 banks), 32KB RAM (4 banks)
    let mut rom = make_test_rom(0x03, 0x05, 0x03);

    for bank in 0..64 {
        rom[bank * 16384] = bank as u8;
    }

    let mut cart = create_cartridge(rom, None).expect("MBC1 creation failed");

    // Enable RAM
    cart.write_rom(0x0000, 0x0A);

    // Select ram_bank_high = 1 (bits 5-6 for ROM, bank 1 for RAM)
    cart.write_rom(0x4000, 0x01);

    // --- Mode 0 (ROM Mode, default) ---
    cart.write_rom(0x6000, 0x00); // Set Mode 0

    // In Mode 0: 0x0000..=0x3FFF is ALWAYS Bank 0
    assert_eq!(cart.read_rom(0x0000), 0x00, "Mode 0: 0x0000 must be Bank 0");
    // 0x4000..=0x7FFF is Bank (1 << 5) | 1 = Bank 33
    cart.write_rom(0x2000, 0x01);
    assert_eq!(cart.read_rom(0x4000), 33, "Mode 0: 0x4000 must be Bank 33");
    // RAM is locked to RAM Bank 0 in Mode 0
    cart.write_ram(0xA000, 0xAA);
    assert_eq!(cart.read_ram(0xA000), 0xAA);

    // --- Mode 1 (RAM / Advanced Banking Mode) ---
    cart.write_rom(0x6000, 0x01); // Set Mode 1

    // In Mode 1: 0x0000..=0x3FFF maps Bank (ram_bank_high << 5) = Bank 32!
    assert_eq!(
        cart.read_rom(0x0000),
        32,
        "Mode 1: 0x0000 must map Bank 32 when ram_bank_high=1"
    );

    // In Mode 1: RAM accesses RAM Bank 1 (selected by ram_bank_high = 1)
    cart.write_ram(0xA000, 0xBB);
    assert_eq!(cart.read_ram(0xA000), 0xBB);

    // Switch back to Mode 0 and check RAM Bank 0 has 0xAA, while RAM Bank 1 has 0xBB
    cart.write_rom(0x6000, 0x00);
    assert_eq!(cart.read_ram(0xA000), 0xAA, "Mode 0 must access RAM Bank 0");
}

#[test]
fn challenge_mbc1_ram_enable_disable_protection() {
    let rom = make_test_rom(0x03, 0x02, 0x03); // MBC1 + RAM + BATTERY
    let mut cart = create_cartridge(rom, None).unwrap();

    // RAM initially disabled
    cart.write_ram(0xA000, 0x42);
    assert_eq!(cart.read_ram(0xA000), 0xFF, "Disabled RAM read must return 0xFF");

    // Enable RAM with 0x0A
    cart.write_rom(0x0000, 0x0A);
    cart.write_ram(0xA000, 0x42);
    assert_eq!(cart.read_ram(0xA000), 0x42);

    // Disable RAM with 0x00
    cart.write_rom(0x0000, 0x00);
    assert_eq!(cart.read_ram(0xA000), 0xFF, "Disabled RAM read must return 0xFF");
    cart.write_ram(0xA000, 0x99); // Attempt write while disabled

    // Re-enable RAM and verify 0x99 was ignored
    cart.write_rom(0x0000, 0x0A);
    assert_eq!(cart.read_ram(0xA000), 0x42, "Write while RAM disabled must be ignored");
}

// ============================================================================
// 4. MBC3 MAPPER & RTC CHALLENGES
// ============================================================================

#[test]
fn challenge_mbc3_rom_banking_7bit_and_ram_rtc_latching() {
    let mut rom = make_test_rom(0x10, 0x05, 0x03); // MBC3 + TIMER + RAM + BATTERY (1MB ROM, 32KB RAM)

    for bank in 0..64 {
        rom[bank * 16384] = bank as u8;
    }

    let mut cart = create_cartridge(rom, None).expect("MBC3 creation failed");

    // Enable RAM & RTC
    cart.write_rom(0x0000, 0x0A);

    // Select ROM Bank 63 (7 bits)
    cart.write_rom(0x2000, 0x3F);
    assert_eq!(cart.read_rom(0x4000), 63);

    // Writing Bank 0 remaps to Bank 1 in MBC3
    cart.write_rom(0x2000, 0x00);
    assert_eq!(cart.read_rom(0x4000), 1, "MBC3 Bank 0 write must remap to Bank 1");

    // --- RTC Register Selection & Latching ---
    // Select RTC Seconds register (0x08)
    cart.write_rom(0x4000, 0x08);
    cart.write_ram(0xA000, 42); // Set live RTC seconds to 42

    // Select RTC Minutes register (0x09)
    cart.write_rom(0x4000, 0x09);
    cart.write_ram(0xA000, 15); // Set live RTC minutes to 15

    // Perform Latch sequence: Write 0x00 then 0x01 to 0x6000..=0x7FFF
    cart.write_rom(0x6000, 0x00);
    cart.write_rom(0x6000, 0x01);

    // Read back latched Seconds
    cart.write_rom(0x4000, 0x08);
    assert_eq!(cart.read_ram(0xA000), 42, "Latched RTC Seconds mismatch");

    // Read back latched Minutes
    cart.write_rom(0x4000, 0x09);
    assert_eq!(cart.read_ram(0xA000), 15, "Latched RTC Minutes mismatch");
}

// ============================================================================
// 5. MBC5 MAPPER CHALLENGES (ALLOWING BANK 0 & 9-BIT BANKING)
// ============================================================================

#[test]
fn challenge_mbc5_bank_0_selection_and_9bit_banking() {
    let mut rom = make_test_rom(0x19, 0x06, 0x04); // MBC5 (2MB ROM = 128 banks, 128KB RAM = 16 banks)

    for bank in 0..128 {
        rom[bank * 16384] = (bank & 0xFF) as u8;
    }

    let mut cart = create_cartridge(rom, None).expect("MBC5 creation failed");

    // Enable RAM
    cart.write_rom(0x0000, 0x0A);

    // MBC5 ALLOWS Bank 0 at 0x4000..=0x7FFF!
    cart.write_rom(0x2000, 0x00);
    assert_eq!(cart.read_rom(0x4000), 0x00, "MBC5 MUST allow Bank 0 at 0x4000");

    // 9-bit bank selection: low = 0x00, high = 1 (bit 8) -> Bank 256 % 128 = Bank 0
    cart.write_rom(0x2000, 0x00);
    cart.write_rom(0x3000, 0x01);
    assert_eq!(cart.read_rom(0x4000), 0x00, "Bank 256 % 128 must be Bank 0");

    // 9-bit bank selection: low = 0x20 (32), high = 1 (bit 8) -> Bank 288 % 128 = Bank 32
    cart.write_rom(0x2000, 0x20);
    cart.write_rom(0x3000, 0x01);
    assert_eq!(cart.read_rom(0x4000), 32, "Bank 288 % 128 must be Bank 32");

    // Select RAM Bank 15
    cart.write_rom(0x4000, 0x0F);
    cart.write_ram(0xA000, 0xE0);
    assert_eq!(cart.read_ram(0xA000), 0xE0);
}

// ============================================================================
// 6. BATTERY-BACKED SRAM PERSISTENCE LIFECYCLE HARNESS
// ============================================================================

#[test]
fn challenge_sram_persistence_full_lifecycle() {
    let temp_dir = std::env::temp_dir().join("gb_emulator_m2_challenger_sram");
    let save_path = temp_dir.join("pokemon_test.sav");
    let _ = fs::remove_file(&save_path);

    let rom = make_test_rom(0x03, 0x03, 0x03); // MBC1 + RAM + BATTERY (256KB ROM, 32KB RAM)

    // Phase 1: Write multi-bank pattern to RAM & Drop instance to trigger auto-save
    {
        let mut cart = create_cartridge(rom.clone(), Some(save_path.clone())).unwrap();
        cart.write_rom(0x0000, 0x0A); // Enable RAM
        cart.write_rom(0x6000, 0x01); // Set RAM Banking Mode (Mode 1)

        // Write to RAM Bank 0
        cart.write_rom(0x4000, 0x00);
        cart.write_ram(0xA000, 0xAA);
        cart.write_ram(0xA001, 0xBB);

        // Write to RAM Bank 3
        cart.write_rom(0x4000, 0x03);
        cart.write_ram(0xA000, 0xCC);
        cart.write_ram(0xA001, 0xDD);

        // Explicit save or Drop
        cart.save_sram(&save_path).unwrap();
    }

    assert!(save_path.exists(), "Save file .sav must exist on disk");

    // Verify raw file contents on disk
    let raw_bytes = fs::read(&save_path).expect("Failed to read save file");
    assert_eq!(raw_bytes.len(), 32768, "Save file size must match RAM size (32KB)");
    assert_eq!(raw_bytes[0], 0xAA);
    assert_eq!(raw_bytes[1], 0xBB);
    assert_eq!(raw_bytes[3 * 8192], 0xCC);
    assert_eq!(raw_bytes[3 * 8192 + 1], 0xDD);

    // Phase 2: Reload Cartridge from disk and verify data integrity
    {
        let mut loaded_cart = create_cartridge(rom, Some(save_path.clone())).unwrap();
        loaded_cart.write_rom(0x0000, 0x0A);
        loaded_cart.write_rom(0x6000, 0x01);

        loaded_cart.write_rom(0x4000, 0x00);
        assert_eq!(loaded_cart.read_ram(0xA000), 0xAA);
        assert_eq!(loaded_cart.read_ram(0xA001), 0xBB);

        loaded_cart.write_rom(0x4000, 0x03);
        assert_eq!(loaded_cart.read_ram(0xA000), 0xCC);
        assert_eq!(loaded_cart.read_ram(0xA001), 0xDD);
    }

    // Cleanup
    let _ = fs::remove_file(&save_path);
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn challenge_sram_atomic_save_helper() {
    let temp_dir = std::env::temp_dir().join("gb_emulator_m2_challenger_atomic");
    let save_path = temp_dir.join("test_atomic.sav");
    let tmp_path = temp_dir.join("test_atomic.sav.tmp");

    let test_data = vec![0x12, 0x34, 0x56, 0x78];
    save_sram_atomic(&save_path, &test_data).unwrap();

    assert!(save_path.exists(), "Final .sav file must exist");
    assert!(!tmp_path.exists(), "Temporary .sav.tmp file must be cleaned up");

    let read_back = fs::read(&save_path).unwrap();
    assert_eq!(read_back, test_data);

    let _ = fs::remove_file(&save_path);
    let _ = fs::remove_dir_all(&temp_dir);
}

// ============================================================================
// 7. MMU 64KB MEMORY MAP & BUS ROUTING CHALLENGES
// ============================================================================

#[test]
fn challenge_mmu_echo_ram_bidirectional_mirroring() {
    let mut mmu = Mmu::new();

    // Write to WRAM -> Read Echo RAM
    mmu.write_byte(0xC000, 0x12);
    mmu.write_byte(0xDDFF, 0x34);
    assert_eq!(mmu.read_byte(0xE000), 0x12);
    assert_eq!(mmu.read_byte(0xFDFF), 0x34);

    // Write to Echo RAM -> Read WRAM
    mmu.write_byte(0xE001, 0x56);
    mmu.write_byte(0xFDFE, 0x78);
    assert_eq!(mmu.read_byte(0xC001), 0x56);
    assert_eq!(mmu.read_byte(0xDDFE), 0x78);
}

#[test]
fn challenge_mmu_unusable_memory_and_io_unused_bits() {
    let mut mmu = Mmu::new();

    // Unusable memory 0xFEA0..=0xFEFF
    for addr in 0xFEA0..=0xFEFF {
        mmu.write_byte(addr, 0x00);
        assert_eq!(mmu.read_byte(addr), 0xFF, "Unusable memory at {:#06X} must return 0xFF", addr);
    }

    // IF (0xFF0F) top 3 bits must read as 1 (0xE0 mask)
    mmu.write_byte(0xFF0F, 0x01);
    assert_eq!(mmu.read_byte(0xFF0F), 0xE1);

    // IE (0xFFFF) top 3 bits ignored
    mmu.write_byte(0xFFFF, 0xFF);
    assert_eq!(mmu.read_byte(0xFFFF), 0x1F);

    // TAC (0xFF07) top 5 bits read as 1 (0xF8 mask)
    mmu.write_byte(0xFF07, 0x04);
    assert_eq!(mmu.read_byte(0xFF07), 0xFC);
}

#[test]
fn challenge_mmu_oam_dma_transfer_from_various_sources() {
    let mut mmu = Mmu::new();

    // Populate ROM/RAM/WRAM source
    let mut rom = make_test_rom(0x00, 0x00, 0x00);
    for i in 0..160 {
        rom[0x0200 + i] = (i + 10) as u8;
    }
    let cart = create_cartridge(rom, None).unwrap();
    mmu.attach_cartridge(cart);

    // Trigger DMA from ROM 0x0200
    mmu.write_byte(0xFF46, 0x02);
    for i in 0..160 {
        assert_eq!(mmu.read_byte(0xFE00 + i as u16), (i + 10) as u8);
    }
}
