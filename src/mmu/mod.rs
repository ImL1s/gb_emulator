pub mod bus;
pub mod ram;

pub use bus::{Bus, MockBus};
pub use ram::{Hram, Wram};

use crate::cartridge::Cartridge;
use crate::joypad::{Joypad, JoypadKey};
use crate::ppu::Ppu;
use crate::serial::SerialPort;
use crate::timer::Timer;

/// Memory Management Unit (MMU) handling full 64KB Game Boy memory map routing.
pub struct Mmu {
    pub cartridge: Option<Box<dyn Cartridge>>,
    pub wram: Wram,
    pub hram: Hram,
    pub vram: Box<[u8; 0x2000]>,
    pub oam: Box<[u8; 0xA0]>,
    pub timer: Timer,
    pub serial: SerialPort,
    pub joypad: Joypad,
    pub ppu: Ppu,
    pub interrupt_flag: u8,
    pub interrupt_enable: u8,
}

impl Mmu {
    pub fn new() -> Self {
        Self {
            cartridge: None,
            wram: Wram::new(),
            hram: Hram::new(),
            vram: Box::new([0; 0x2000]),
            oam: Box::new([0; 0xA0]),
            timer: Timer::new(),
            serial: SerialPort::new(),
            joypad: Joypad::new(),
            ppu: Ppu::new(),
            interrupt_flag: 0,
            interrupt_enable: 0,
        }
    }

    pub fn attach_cartridge(&mut self, cartridge: Box<dyn Cartridge>) {
        self.cartridge = Some(cartridge);
    }

    fn perform_oam_dma(&mut self, val: u8) {
        self.ppu.write_reg(0xFF46, val);
        let src_base = (val as u16) << 8;
        for i in 0..160 {
            let byte = self.read_byte(src_base + i as u16);
            self.oam[i] = byte;
        }
    }

    /// Press a joypad key and request Joypad interrupt (IF bit 4) if line transition occurred.
    pub fn press_key(&mut self, key: JoypadKey) {
        if self.joypad.press_key(key) {
            self.interrupt_flag |= 0x10;
        }
    }

    /// Release a joypad key.
    pub fn release_key(&mut self, key: JoypadKey) {
        self.joypad.release_key(key);
    }

    /// Retrieve serial port ASCII output buffer.
    pub fn get_serial_output(&self) -> &str {
        self.serial.get_output()
    }
}

impl Default for Mmu {
    fn default() -> Self {
        Self::new()
    }
}

impl Bus for Mmu {
    fn read_byte(&self, addr: u16) -> u8 {
        match addr {
            // ROM (0x0000-0x7FFF)
            0x0000..=0x7FFF => {
                if let Some(ref cart) = self.cartridge {
                    cart.read_rom(addr)
                } else {
                    0xFF
                }
            }
            // VRAM (0x8000-0x9FFF)
            0x8000..=0x9FFF => self.vram[(addr - 0x8000) as usize],
            // External SRAM (0xA000-0xBFFF)
            0xA000..=0xBFFF => {
                if let Some(ref cart) = self.cartridge {
                    cart.read_ram(addr)
                } else {
                    0xFF
                }
            }
            // WRAM (0xC000-0xDFFF)
            0xC000..=0xDFFF => self.wram.read(addr),
            // Echo RAM (0xE000-0xFDFF)
            0xE000..=0xFDFF => self.wram.read_echo(addr),
            // OAM (0xFE00-0xFE9F)
            0xFE00..=0xFE9F => self.oam[(addr - 0xFE00) as usize],
            // Unusable (0xFEA0-0xFEFF)
            0xFEA0..=0xFEFF => 0xFF,
            // I/O Registers (0xFF00-0xFF7F)
            0xFF00..=0xFF7F => match addr {
                0xFF00 => self.joypad.read_joyp(),
                0xFF01 => self.serial.read_sb(),
                0xFF02 => self.serial.read_sc(),
                0xFF04 => self.timer.read_reg(0xFF04),
                0xFF05 => self.timer.read_reg(0xFF05),
                0xFF06 => self.timer.read_reg(0xFF06),
                0xFF07 => self.timer.read_reg(0xFF07),
                0xFF0F => self.interrupt_flag | 0xE0,
                0xFF10..=0xFF3F => match addr {
                    0xFF26 => 0xF1,
                    _ => 0xFF,
                },
                0xFF40..=0xFF4B => self.ppu.read_reg(addr),
                _ => 0xFF,
            },
            // HRAM (0xFF80-0xFFFE)
            0xFF80..=0xFFFE => self.hram.read(addr),
            // IE Interrupt Enable (0xFFFF)
            0xFFFF => self.interrupt_enable & 0x1F,
        }
    }

    fn write_byte(&mut self, addr: u16, val: u8) {
        match addr {
            // ROM (0x0000-0x7FFF)
            0x0000..=0x7FFF => {
                if let Some(ref mut cart) = self.cartridge {
                    cart.write_rom(addr, val);
                }
            }
            // VRAM (0x8000-0x9FFF)
            0x8000..=0x9FFF => self.vram[(addr - 0x8000) as usize] = val,
            // External SRAM (0xA000-0xBFFF)
            0xA000..=0xBFFF => {
                if let Some(ref mut cart) = self.cartridge {
                    cart.write_ram(addr, val);
                }
            }
            // WRAM (0xC000-0xDFFF)
            0xC000..=0xDFFF => self.wram.write(addr, val),
            // Echo RAM (0xE000-0xFDFF)
            0xE000..=0xFDFF => self.wram.write_echo(addr, val),
            // OAM (0xFE00-0xFE9F)
            0xFE00..=0xFE9F => self.oam[(addr - 0xFE00) as usize] = val,
            // Unusable (0xFEA0-0xFEFF)
            0xFEA0..=0xFEFF => {}
            // I/O Registers (0xFF00-0xFF7F)
            0xFF00..=0xFF7F => match addr {
                0xFF00 => {
                    if self.joypad.write_joyp(val) {
                        self.interrupt_flag |= 0x10;
                    }
                }
                0xFF01 => self.serial.write_sb(val),
                0xFF02 => {
                    if self.serial.write_sc(val) {
                        self.interrupt_flag |= 0x08;
                    }
                }
                0xFF04 => {
                    self.timer.write_reg(0xFF04, val);
                    if self.timer.interrupt_pending {
                        self.interrupt_flag |= 0x04;
                        self.timer.interrupt_pending = false;
                    }
                }
                0xFF05 => self.timer.write_reg(0xFF05, val),
                0xFF06 => self.timer.write_reg(0xFF06, val),
                0xFF07 => {
                    self.timer.write_reg(0xFF07, val);
                    if self.timer.interrupt_pending {
                        self.interrupt_flag |= 0x04;
                        self.timer.interrupt_pending = false;
                    }
                }
                0xFF0F => self.interrupt_flag = val & 0x1F,
                0xFF10..=0xFF3F => {}
                0xFF46 => self.perform_oam_dma(val),
                0xFF40..=0xFF4B => self.ppu.write_reg(addr, val),
                _ => {}
            },
            // HRAM (0xFF80-0xFFFE)
            0xFF80..=0xFFFE => self.hram.write(addr, val),
            // IE Interrupt Enable (0xFFFF)
            0xFFFF => self.interrupt_enable = val & 0x1F,
        }
    }

    fn step_components(&mut self, cycles: u32) {
        self.timer.step(cycles);
        if self.timer.interrupt_pending {
            self.interrupt_flag |= 0x04;
            self.timer.interrupt_pending = false;
        }
        if self.serial.step(cycles) {
            self.interrupt_flag |= 0x08;
        }
        self.ppu.step(cycles, &self.vram, &self.oam);
        if self.ppu.vblank_interrupt {
            self.interrupt_flag |= 0x01;
            self.ppu.vblank_interrupt = false;
        }
        if self.ppu.stat_interrupt {
            self.interrupt_flag |= 0x02;
            self.ppu.stat_interrupt = false;
        }
    }

    fn is_interrupt_requested(&self) -> bool {
        (self.interrupt_flag & self.interrupt_enable & 0x1F) != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::{Cpu, ImeState};

    #[test]
    fn test_wram_and_echo_ram_mirroring() {
        let mut mmu = Mmu::new();
        mmu.write_byte(0xC000, 0xAB);
        assert_eq!(mmu.read_byte(0xC000), 0xAB);
        assert_eq!(mmu.read_byte(0xE000), 0xAB);

        mmu.write_byte(0xFDFF, 0xCD);
        assert_eq!(mmu.read_byte(0xDDFF), 0xCD);
        assert_eq!(mmu.read_byte(0xFDFF), 0xCD);
    }

    #[test]
    fn test_hram_vram_oam_access() {
        let mut mmu = Mmu::new();

        mmu.write_byte(0x8000, 0x12);
        assert_eq!(mmu.read_byte(0x8000), 0x12);

        mmu.write_byte(0xFE00, 0x34);
        assert_eq!(mmu.read_byte(0xFE00), 0x34);

        mmu.write_byte(0xFF80, 0x56);
        assert_eq!(mmu.read_byte(0xFF80), 0x56);
    }

    #[test]
    fn test_unusable_memory_returns_0xff() {
        let mut mmu = Mmu::new();
        mmu.write_byte(0xFEA0, 0x99);
        assert_eq!(mmu.read_byte(0xFEA0), 0xFF);
    }

    #[test]
    fn test_div_reset_on_write() {
        let mut mmu = Mmu::new();
        mmu.step_components(0x0500);
        assert!(mmu.read_byte(0xFF04) > 0);

        mmu.write_byte(0xFF04, 0xFF);
        assert_eq!(mmu.read_byte(0xFF04), 0);
    }

    #[test]
    fn test_oam_dma_transfer() {
        let mut mmu = Mmu::new();
        for i in 0..160 {
            mmu.write_byte(0xC000 + i as u16, (i + 1) as u8);
        }

        mmu.write_byte(0xFF46, 0xC0); // Trigger DMA from 0xC000

        for i in 0..160 {
            assert_eq!(mmu.read_byte(0xFE00 + i as u16), (i + 1) as u8);
        }
    }

    #[test]
    fn test_interrupt_flag_and_enable() {
        let mut mmu = Mmu::new();
        mmu.write_byte(0xFF0F, 0x01); // Request VBlank
        mmu.write_byte(0xFFFF, 0x01); // Enable VBlank

        assert!(mmu.is_interrupt_requested());

        mmu.write_byte(0xFFFF, 0x02); // Enable STAT only
        assert!(!mmu.is_interrupt_requested());
    }

    #[test]
    fn test_mmu_peripheral_read_masks() {
        let mmu = Mmu::new();
        assert_eq!(mmu.read_byte(0xFF00) & 0xC0, 0xC0);
        assert_eq!(mmu.read_byte(0xFF02) & 0x7E, 0x7E);
        assert_eq!(mmu.read_byte(0xFF07) & 0xF8, 0xF8);
        assert_eq!(mmu.read_byte(0xFF0F) & 0xE0, 0xE0);
    }

    #[test]
    fn test_mmu_timer_step_and_interrupt_trigger() {
        let mut mmu = Mmu::new();
        mmu.write_byte(0xFF07, 0x04); // Timer Enable, 4096Hz (bit 9)
        mmu.write_byte(0xFF05, 0xFF); // TIMA = 0xFF
        mmu.write_byte(0xFF06, 0x55); // TMA = 0x55

        mmu.step_components(1024);

        assert_eq!(mmu.read_byte(0xFF05), 0x55);
        assert_ne!(mmu.read_byte(0xFF0F) & 0x04, 0); // Bit 2 set
    }

    #[test]
    fn test_mmu_serial_capture_and_interrupt() {
        let mut mmu = Mmu::new();
        mmu.write_byte(0xFF01, b'H');
        mmu.write_byte(0xFF02, 0x81);

        mmu.step_components(4096);

        assert_eq!(mmu.get_serial_output(), "H");
        assert_ne!(mmu.read_byte(0xFF0F) & 0x08, 0); // Bit 3 set
    }

    #[test]
    fn test_mmu_joypad_key_press_interrupt() {
        let mut mmu = Mmu::new();
        mmu.write_byte(0xFF00, 0x20); // Select Directional keys

        mmu.press_key(JoypadKey::Right);

        assert_eq!(mmu.read_byte(0xFF00) & 0x01, 0);
        assert_ne!(mmu.read_byte(0xFF0F) & 0x10, 0); // Bit 4 set
    }

    #[test]
    fn test_full_cpu_mmu_timer_interrupt_dispatch() {
        let mut cpu = Cpu::new();
        let mut mmu = Mmu::new();

        cpu.registers.sp = 0xFFFE;
        cpu.registers.pc = 0x0100;
        cpu.ime_state = ImeState::Enabled;

        mmu.write_byte(0xFFFF, 0x04); // Enable Timer Interrupt
        mmu.write_byte(0xFF07, 0x04); // Enable Timer 4096Hz
        mmu.write_byte(0xFF05, 0xFF);
        mmu.write_byte(0xFF06, 0x55);

        // Step components to trigger timer interrupt
        mmu.step_components(1024);
        assert_ne!(mmu.read_byte(0xFF0F) & 0x04, 0);

        // Step CPU: should service timer interrupt
        let cycles = cpu.step(&mut mmu);

        assert_eq!(cycles, 20);
        assert_eq!(cpu.registers.pc, 0x0050); // Timer vector
        assert_eq!(mmu.read_byte(0xFF0F) & 0x04, 0); // Bit 2 cleared
        assert_eq!(cpu.ime_state, ImeState::Disabled);
        assert_eq!(mmu.read_byte(0xFFFC), 0x00); // Low byte PC
        assert_eq!(mmu.read_byte(0xFFFD), 0x01); // High byte PC
    }

    #[test]
    fn test_mmu_ppu_vblank_and_stat_interrupt_routing() {
        let mut mmu = Mmu::new();

        // Step 144 lines to trigger VBlank
        mmu.step_components(144 * 456);
        assert_ne!(mmu.read_byte(0xFF0F) & 0x01, 0); // Bit 0 VBlank set

        // Set LYC = 5, enable STAT LYC interrupt
        let mut mmu2 = Mmu::new();
        mmu2.write_byte(0xFF45, 5);
        mmu2.write_byte(0xFF41, 0x40);

        mmu2.step_components(5 * 456);
        assert_ne!(mmu2.read_byte(0xFF0F) & 0x02, 0); // Bit 1 STAT set
    }

    #[test]
    fn test_apu_register_stubs() {
        let mut mmu = Mmu::new();
        assert_eq!(mmu.read_byte(0xFF26), 0xF1);
        assert_eq!(mmu.read_byte(0xFF10), 0xFF);
        assert_eq!(mmu.read_byte(0xFF3F), 0xFF);

        mmu.write_byte(0xFF10, 0x12);
        mmu.write_byte(0xFF26, 0x00);
        assert_eq!(mmu.read_byte(0xFF26), 0xF1);
        assert_eq!(mmu.read_byte(0xFF10), 0xFF);
    }
}
