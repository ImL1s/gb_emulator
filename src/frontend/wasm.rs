use wasm_bindgen::prelude::*;

use crate::cartridge;
use crate::cpu::Cpu;
use crate::joypad::JoypadKey;
use crate::mmu::Mmu;

const SCREEN_WIDTH: usize = 160;
const SCREEN_HEIGHT: usize = 144;
const CYCLES_PER_FRAME: u32 = 70_224;

/// WASM Core Export Layer for Game Boy Emulator.
#[wasm_bindgen]
pub struct WasmEmulator {
    cpu: Cpu,
    mmu: Mmu,
    rgba_buf: Vec<u8>,
}

#[wasm_bindgen]
impl WasmEmulator {
    /// Initialize WasmEmulator with default CPU, MMU, and RGBA pixel buffer.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        #[cfg(target_arch = "wasm32")]
        console_error_panic_hook::set_once();
        Self {
            cpu: Cpu::new(),
            mmu: Mmu::new(),
            rgba_buf: vec![0u8; SCREEN_WIDTH * SCREEN_HEIGHT * 4],
        }
    }


    /// Load raw Game Boy ROM byte slice into emulator cartridge & MMU.
    pub fn load_rom(&mut self, rom_bytes: &[u8]) -> Result<(), String> {
        let cartridge = cartridge::create_cartridge(rom_bytes.to_vec(), None)
            .map_err(|e| e.to_string())?;

        let mut mmu = Mmu::new();
        mmu.attach_cartridge(cartridge);

        self.mmu = mmu;
        self.cpu = Cpu::new();
        Ok(())
    }


    /// Step emulator execution for 1 frame (70,224 T-cycles), unpack PPU framebuffer to RGBA bytes, return true.
    pub fn step_frame(&mut self) -> bool {
        let mut cycles_this_frame = 0;
        while cycles_this_frame < CYCLES_PER_FRAME {
            let cycles = self.cpu.step(&mut self.mmu);
            cycles_this_frame += cycles;
        }

        for (i, &pixel) in self.mmu.ppu.framebuffer.iter().enumerate() {
            self.rgba_buf[i * 4] = (pixel >> 24) as u8;     // Red
            self.rgba_buf[i * 4 + 1] = (pixel >> 16) as u8; // Green
            self.rgba_buf[i * 4 + 2] = (pixel >> 8) as u8;  // Blue
            self.rgba_buf[i * 4 + 3] = pixel as u8;         // Alpha
        }

        true
    }

    /// Return raw pointer to internal RGBA pixel buffer for zero-copy JS Uint8ClampedArray view.
    pub fn get_framebuffer_ptr(&self) -> *const u8 {
        self.rgba_buf.as_ptr()
    }

    /// Press joypad button by key index (0: Right, 1: Left, 2: Up, 3: Down, 4: A, 5: B, 6: Select, 7: Start).
    pub fn press_key(&mut self, key_code: u8) {
        if let Some(key) = map_wasm_key(key_code) {
            self.mmu.press_key(key);
        }
    }

    /// Release joypad button by key index (0: Right, 1: Left, 2: Up, 3: Down, 4: A, 5: B, 6: Select, 7: Start).
    pub fn release_key(&mut self, key_code: u8) {
        if let Some(key) = map_wasm_key(key_code) {
            self.mmu.release_key(key);
        }
    }
}

impl Default for WasmEmulator {
    fn default() -> Self {
        Self::new()
    }
}

/// Map numeric WASM key index to internal JoypadKey enum.
fn map_wasm_key(key_code: u8) -> Option<JoypadKey> {
    match key_code {
        0 => Some(JoypadKey::Right),
        1 => Some(JoypadKey::Left),
        2 => Some(JoypadKey::Up),
        3 => Some(JoypadKey::Down),
        4 => Some(JoypadKey::A),
        5 => Some(JoypadKey::B),
        6 => Some(JoypadKey::Select),
        7 => Some(JoypadKey::Start),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_wasm_key() {
        assert_eq!(map_wasm_key(0), Some(JoypadKey::Right));
        assert_eq!(map_wasm_key(1), Some(JoypadKey::Left));
        assert_eq!(map_wasm_key(2), Some(JoypadKey::Up));
        assert_eq!(map_wasm_key(3), Some(JoypadKey::Down));
        assert_eq!(map_wasm_key(4), Some(JoypadKey::A));
        assert_eq!(map_wasm_key(5), Some(JoypadKey::B));
        assert_eq!(map_wasm_key(6), Some(JoypadKey::Select));
        assert_eq!(map_wasm_key(7), Some(JoypadKey::Start));
        assert_eq!(map_wasm_key(8), None);
    }
}
