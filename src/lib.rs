pub mod cartridge;
pub mod cpu;
pub mod frontend;
pub mod joypad;
pub mod mmu;
pub mod ppu;
pub mod serial;
pub mod timer;

#[cfg(target_arch = "wasm32")]
pub use frontend::wasm::WasmEmulator;
