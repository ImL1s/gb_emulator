use gb_emulator::frontend::wasm::WasmEmulator;

fn build_valid_dummy_rom() -> Vec<u8> {
    let mut rom = vec![0u8; 32 * 1024];
    let title = b"WASMTEST";
    rom[0x0134..0x0134 + title.len()].copy_from_slice(title);
    rom[0x0147] = 0x00; // NoMBC
    rom[0x0148] = 0x00; // 32KB
    rom[0x0149] = 0x00; // 0 RAM

    // Calculate header checksum
    let mut chk: u8 = 0;
    for &b in &rom[0x0134..=0x014C] {
        chk = chk.wrapping_sub(b).wrapping_sub(1);
    }
    rom[0x014D] = chk;
    rom
}

#[test]
fn test_wasm_emulator_new_defaults() {
    let emu = WasmEmulator::new();
    let ptr = emu.get_framebuffer_ptr();
    assert!(!ptr.is_null(), "Framebuffer pointer must not be null");
}

#[test]
fn test_wasm_load_rom_invalid_and_valid() {
    let mut emu = WasmEmulator::new();

    // 1. Empty ROM
    assert!(emu.load_rom(&[]).is_err(), "Empty ROM should return Err");

    // 2. Truncated ROM
    assert!(
        emu.load_rom(&[0u8; 100]).is_err(),
        "Truncated ROM < 0x150 bytes should return Err"
    );

    // 3. Corrupt Checksum
    assert!(
        emu.load_rom(&[0u8; 0x150]).is_err(),
        "ROM with invalid header checksum should return Err"
    );

    // 4. Valid ROM
    let valid_rom = build_valid_dummy_rom();
    assert!(
        emu.load_rom(&valid_rom).is_ok(),
        "Valid 32KB ROM should load successfully"
    );
}

#[test]
fn test_wasm_key_input_boundary_and_invalid_codes() {
    let mut emu = WasmEmulator::new();
    let valid_rom = build_valid_dummy_rom();
    emu.load_rom(&valid_rom).unwrap();

    // Test valid keys (0..=7)
    for key in 0..=7 {
        emu.press_key(key);
        emu.release_key(key);
    }

    // Test out-of-bounds key codes (8..=255)
    let invalid_keys = [8, 9, 10, 16, 32, 64, 128, 254, 255];
    for &invalid in &invalid_keys {
        emu.press_key(invalid);
        emu.release_key(invalid);
    }
}

#[test]
fn test_wasm_frame_stepping_stability() {
    let mut emu = WasmEmulator::new();
    let valid_rom = build_valid_dummy_rom();
    emu.load_rom(&valid_rom).unwrap();

    // Run 120 consecutive frame steps (2 seconds of 60FPS execution)
    for frame in 0..120 {
        let res = emu.step_frame();
        assert!(res, "step_frame must return true at frame {}", frame);
    }

    // Framebuffer pointer must remain valid and non-null
    let ptr = emu.get_framebuffer_ptr();
    assert!(!ptr.is_null());
}

#[test]
fn test_wasm_framebuffer_rgba_unpacking() {
    let mut emu = WasmEmulator::new();
    let valid_rom = build_valid_dummy_rom();
    emu.load_rom(&valid_rom).unwrap();

    emu.step_frame();

    let ptr = emu.get_framebuffer_ptr();
    unsafe {
        // Sample first pixel RGBA bytes (160 * 144 * 4 = 92,160 total bytes)
        let _r = *ptr;
        let _g = *ptr.add(1);
        let _b = *ptr.add(2);
        let a = *ptr.add(3);

        // Standard default Game Boy palette shade 0 (White 0xFFFFFFFF)
        assert_eq!(a, 0xFF, "Alpha channel of pixel 0 should be 0xFF");
    }
}

#[test]
fn test_wasm_reload_rom_resets_emulator_state() {
    let mut emu = WasmEmulator::new();
    let valid_rom = build_valid_dummy_rom();
    emu.load_rom(&valid_rom).unwrap();

    // Step 60 frames
    for _ in 0..60 {
        emu.step_frame();
    }

    // Reload ROM mid-execution
    assert!(emu.load_rom(&valid_rom).is_ok());

    // Continue stepping
    for _ in 0..60 {
        assert!(emu.step_frame());
    }
}
