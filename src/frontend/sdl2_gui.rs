use anyhow::{bail, Result};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use std::fs;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use crate::cartridge;
use crate::cpu::Cpu;
use crate::joypad::JoypadKey;
use crate::mmu::Mmu;

const SCREEN_WIDTH: u32 = 160;
const SCREEN_HEIGHT: u32 = 144;
const SCALE_FACTOR: u32 = 4;
const CYCLES_PER_FRAME: u32 = 70_224;
const TARGET_FRAME_DURATION: Duration = Duration::from_nanos(16_742_706); // ~60 FPS (59.73 Hz)

fn map_keycode(kc: Keycode) -> Option<JoypadKey> {
    match kc {
        Keycode::Up => Some(JoypadKey::Up),
        Keycode::Down => Some(JoypadKey::Down),
        Keycode::Left => Some(JoypadKey::Left),
        Keycode::Right => Some(JoypadKey::Right),
        Keycode::Z => Some(JoypadKey::A),
        Keycode::X => Some(JoypadKey::B),
        Keycode::Return | Keycode::KpEnter => Some(JoypadKey::Start),
        Keycode::RShift | Keycode::LShift => Some(JoypadKey::Select),
        _ => None,
    }
}

/// Run Game Boy emulator in interactive SDL2 GUI mode (60 FPS, RGBA framebuffer blitting, keyboard input).
pub fn run(rom_path: &Path) -> Result<()> {
    if !rom_path.exists() {
        bail!("ROM file does not exist: {}", rom_path.display());
    }

    let rom_data = fs::read(rom_path)?;
    let save_path = rom_path.with_extension("sav");
    let cartridge = cartridge::create_cartridge(rom_data, Some(save_path))?;

    let mut mmu = Mmu::new();
    mmu.attach_cartridge(cartridge);

    let mut cpu = Cpu::new();

    let sdl_context = sdl2::init().map_err(|e| anyhow::anyhow!("SDL2 init error: {e}"))?;
    let video_subsystem = sdl_context
        .video()
        .map_err(|e| anyhow::anyhow!("SDL2 video error: {e}"))?;

    let rom_name = rom_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("ROM");
    let window_title = format!("Game Boy Emulator - {rom_name}");

    let window = video_subsystem
        .window(&window_title, SCREEN_WIDTH * SCALE_FACTOR, SCREEN_HEIGHT * SCALE_FACTOR)
        .position_centered()
        .resizable()
        .build()
        .map_err(|e| anyhow::anyhow!("Window creation error: {e}"))?;

    let mut canvas = window
        .into_canvas()
        .accelerated()
        .present_vsync()
        .build()
        .or_else(|_| {
            video_subsystem
                .window(&window_title, SCREEN_WIDTH * SCALE_FACTOR, SCREEN_HEIGHT * SCALE_FACTOR)
                .position_centered()
                .build()
                .map_err(|e| anyhow::anyhow!("Window creation fallback error: {e}"))?
                .into_canvas()
                .build()
                .map_err(|e| anyhow::anyhow!("Canvas creation error: {e}"))
        })?;

    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator
        .create_texture_streaming(PixelFormatEnum::RGBA8888, SCREEN_WIDTH, SCREEN_HEIGHT)
        .map_err(|e| anyhow::anyhow!("Texture creation error: {e}"))?;

    let mut event_pump = sdl_context
        .event_pump()
        .map_err(|e| anyhow::anyhow!("Event pump error: {e}"))?;

    let mut pixel_bytes = vec![0u8; (SCREEN_WIDTH * SCREEN_HEIGHT * 4) as usize];

    'main_loop: loop {
        let frame_start = Instant::now();

        // 1. Poll SDL2 Keyboard & Window events
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'main_loop,

                Event::KeyDown {
                    keycode: Some(kc),
                    repeat: false,
                    ..
                } => {
                    if let Some(key) = map_keycode(kc) {
                        mmu.press_key(key);
                    }
                }

                Event::KeyUp {
                    keycode: Some(kc),
                    ..
                } => {
                    if let Some(key) = map_keycode(kc) {
                        mmu.release_key(key);
                    }
                }

                _ => {}
            }
        }

        // 2. Step emulator core for 1 frame (~70,224 T-cycles)
        let mut cycles_this_frame = 0;
        while cycles_this_frame < CYCLES_PER_FRAME {
            let cycles = cpu.step(&mut mmu);
            cycles_this_frame += cycles;
        }

        // 3. Unpack PPU u32 RGBA framebuffer to pixel bytes and update texture
        for (i, &pixel) in mmu.ppu.framebuffer.iter().enumerate() {
            pixel_bytes[i * 4] = (pixel >> 24) as u8;     // Red
            pixel_bytes[i * 4 + 1] = (pixel >> 16) as u8; // Green
            pixel_bytes[i * 4 + 2] = (pixel >> 8) as u8;  // Blue
            pixel_bytes[i * 4 + 3] = pixel as u8;         // Alpha
        }

        texture
            .update(None, &pixel_bytes, (SCREEN_WIDTH * 4) as usize)
            .map_err(|e| anyhow::anyhow!("Texture update error: {e}"))?;

        canvas.clear();
        canvas
            .copy(&texture, None, None)
            .map_err(|e| anyhow::anyhow!("Canvas copy error: {e}"))?;
        canvas.present();

        // 4. Cap frame rate at 60 FPS
        let elapsed = frame_start.elapsed();
        if elapsed < TARGET_FRAME_DURATION {
            thread::sleep(TARGET_FRAME_DURATION - elapsed);
        }
    }

    Ok(())
}
