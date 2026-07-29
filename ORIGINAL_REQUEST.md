# Original User Request

## 2026-07-28T17:11:31Z

<USER_REQUEST>
Build a clean-room Game Boy (LR35902) emulator in Rust driven by objective test ROM harness validation and interactive SDL2 window rendering.

Working directory: /Users/iml1s/Documents/mine/gb_emulator
Integrity mode: development

## Requirements

### R1. Core LR35902 CPU Implementation
Implement all 256 base opcodes and 256 CB-prefixed opcodes for the LR35902 CPU. Include 8-bit registers (A, B, C, D, E, H, L, F flags Z/N/H/C) and 16-bit register pairs (AF, BC, DE, HL, SP, PC), cycle counting, interrupts (VBlank, LCD STAT, Timer, Serial, Joypad), and HALT/STOP states.

### R2. Memory Map & Cartridge Bank Controllers (MBC1 / MBC3)
Implement the 64KB Game Boy memory map:
- `0000-3FFF`: ROM Bank 0
- `4000-7FFF`: Switchable ROM Bank
- `8000-9FFF`: VRAM
- `A000-BFFF`: External RAM (SRAM) with save file (`.sav`) persistence
- `C000-DFFF`: Work RAM (WRAM)
- `E000-FDFF`: Echo RAM
- `FE00-FE9F`: OAM (Object Attribute Memory)
- `FF00-FF7F`: I/O Registers (Joypad 0xFF00, Timer 0xFF04-0xFF07, IF 0xFF0F, IE 0xFFFF)
- `FF80-FFFE`: High RAM (HRAM)
Implement MBC1 and MBC3 bank switching to support larger commercial ROMs (e.g., Tetris, Pokémon Red/Blue).

### R3. PPU (Picture Processing Unit) 2D Graphics Engine
Implement the PPU rendering pipeline producing a 160x144 2-bit grayscale (4-color palette) framebuffer:
- Mode 0 (HBlank), Mode 1 (VBlank), Mode 2 (OAM Search), Mode 3 (Pixel Transfer) with correct cycle timing
- Background layer rendering with SCX/SCY scrolling
- Window layer rendering with WX/WY positioning
- Sprite (OBJ) layer rendering (8x8 and 8x16 modes, priority, X/Y flip, transparency)

### R4. Joypad Input & Interactive SDL2 UI Window
Implement Joypad register (`0xFF00`) polling. Add a frontend wrapper using `sdl2` (or `minifb`) that:
- Opens a window (scalable 160x144) displaying the 60FPS video buffer
- Maps keyboard inputs: Arrow Keys (D-Pad), Z (A), X (B), Enter (Start), Right Shift (Select)
- Accepts a ROM filepath argument: `cargo run --release -- path/to/game.gb`

### R5. Test Harness & Headless Verification Runner
Provide a headless test execution mode. Integrate a verification script `scripts/run_gb_tests.sh` that:
1. Downloads Blargg's GB test ROMs (`cpu_instrs.gb`, `instr_timing.gb`) if not present
2. Executes the emulator in headless mode against these test ROMs
3. Validates output either by checking serial output (`0xFF01`) for "Passed" or comparing register logs against `gameboy-doctor`
4. Exits with code 0 when all CPU test suites pass

## Acceptance Criteria

### Test Harness Verification
- [ ] All 11 Blargg `cpu_instrs` sub-tests execute and pass in headless mode
- [ ] `scripts/run_gb_tests.sh` exits with code 0
- [ ] `cargo test` runs all unit tests and passes clean

### Core Functionality & Playability
- [ ] `cargo run --release -- path/to/game.gb` launches a graphical window without crashing
- [ ] Test ROMs (`cpu_instrs.gb`) render "Passed" text on screen
- [ ] Commercial ROMs (`Tetris.gb`, `Super_Mario_Land.gb`, `Pokemon_Red.gb`) boot, display title screens, accept player controller input, and run smoothly at 60 FPS
</USER_REQUEST>
