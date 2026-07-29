# TEST_READY — Game Boy (LR35902) Emulator Test Infrastructure & Specification

**Project:** Clean-Room Game Boy (LR35902) Rust Emulator  
**File Location:** `/Users/iml1s/Documents/mine/gb_emulator/TEST_READY.md`  
**Status:** E2E Test Suite Specification & Runner Harness Operational | Baseline: Unit Tests Passing | Pending Implementation Track Completion (0/13 ROMs Passing)  

---

## 1. Executive Summary & Repository Status Baseline

The test infrastructure for the Game Boy emulator is fully established. It consists of an automated E2E test runner script (`scripts/run_gb_tests.sh`), asset caching mechanisms, and a 4-Tier verification specification.

### Current Codebase Baseline (M3 Remediation Baseline)
- **Rust Unit Tests**: **Passing** (`cargo test --all-targets`) covering core CPU registers, flags, ALU arithmetic/DAA, unprefixed & CB opcodes, cycle timing, interrupt delays, HALT bug handling, and memory bus structure in `src/cpu/` & `src/mmu/`.
- **E2E Test Harness Infrastructure**: **100% Operational** (`scripts/run_gb_tests.sh`). Downloading, caching, CLI flag parsing, binary compilation, and serial log matching are fully implemented.
- **Headless ROM Harness Execution**: **0 / 13 Passing** (`./scripts/run_gb_tests.sh`). Blargg test ROMs currently fail because the Implementation Track has not yet wired emulator execution and serial output interception into `src/main.rs`.

---

## 2. Quick Verification Commands & Execution Matrix

### Primary Test Commands

#### 1. Download & Verify Blargg Test ROM Assets
Downloads and verifies all 13 Blargg test ROM binaries in `tests/roms/` without building or executing the emulator binary.
```bash
./scripts/run_gb_tests.sh --download-only
```
- **Expected Exit Code**: `0` (All 13 ROMs cached and non-empty).

#### 2. Run Rust Unit & Integration Test Suites
Executes all unit tests in the codebase.
```bash
cargo test --all-targets -- --nocapture
```
- **Expected Exit Code**: `0` (All unit tests passing cleanly).

#### 3. Execute Headless E2E Test Runner
Builds the emulator in release mode and attempts headless execution of all 13 Blargg ROMs.
```bash
./scripts/run_gb_tests.sh
```
- **Current Exit Code**: `1` (0/13 passed — expected until Implementation Track completes core emulator loop).
- **Target Exit Code (M5 Completion)**: `0` (13/13 ROMs pass with `"Passed"` in serial output).

#### 4. Test Runner Options & Flags
```bash
./scripts/run_gb_tests.sh --timeout 20 # Custom per-ROM timeout in seconds
./scripts/run_gb_tests.sh --headless   # Explicit headless execution mode
./scripts/run_gb_tests.sh --help       # Display command-line usage help
```

---

## 3. 4-Tier E2E & Unit Test Suite Specification (47 Specifications)

The complete project test plan comprises **47 planned test specifications** across 4 verification tiers designed to validate full emulator correctness across Milestones M1 through M5:

| Tier Level | Verification Scope | Specification Count | Implementation Status | Primary Pass Criteria |
|---|---|:---:|---|---|
| **Tier 1: Feature Coverage** | Unit tests across 7 hardware subsystems (CPU, MMU, Timer, PPU, APU, Joypad, Cartridge Mappers) | **35 specs** (5 per domain) | **CPU & MMU unit tests passing**; remaining MMU/Timer/PPU/APU/Joypad/Cartridge specs target Implementation Track | Register, flag, timing, memory, and register mask state exact matches |
| **Tier 2: Boundary & Corner Cases** | Edge cases (OAM DMA lock, STAT blocking / LCD off, HALT bug, MBC masking, Echo RAM, Serial shift) | **6 specs** | HALT bug test passing in `src/cpu/`; remaining specs target M2–M4 | Bus isolation, flag preservation, instruction repetition, address masking |
| **Tier 3: Cross-Feature Interactions** | Multi-component scenarios (Timer in HALT, STAT in DMA, Bank switch in ISR) | **3 specs** | Target Implementation Track M3–M4 | Concurrent state machine transitions & execution flow integrity |
| **Tier 4: Real-World Scenarios** | End-to-end Blargg ROM execution (13 ROMs), Header parser, FPS benchmark | **3 specs** | E2E Harness Operational (`scripts/run_gb_tests.sh`); 0/13 passing until emulator loop wired | Serial output matches `"Passed"`, 70,224 cycles/frame, valid header load |
| **TOTAL** | **Full Project Verification Plan** | **47 Specifications** | **Infrastructure Ready** | **Clean execution, exit code 0 upon M5 completion** |

---

## 4. Hardware Subsystem Feature & Verification Mapping

Mappings between Hardware Subsystems, Requirements (R1–R5), Project Features (#1–#25), Implementation Module Targets, and Verification Tiers:

| # | Hardware Subsystem / Feature | Feature Description | Requirement | Target Module Path (Implementation Track Target) | Current Verification Status |
|---|------------------------------|---------------------|-------------|--------------------------------------------------|-----------------------------|
| 1 | **LR35902 CPU**: Registers & Flags | AF, BC, DE, HL, SP, PC, Flags (Z, N, H, C, lower 4 bits zero) | R1 | `src/cpu/registers.rs` | **VERIFIED**: Unit tests passing (`test_default_registers`, `test_16bit_accessors`, `test_lower_4_bits_zero_invariant`) |
| 2 | **LR35902 CPU**: Unprefixed Opcodes | All 256 base opcodes & cycle counts | R1 | `src/cpu/opcodes.rs` | **VERIFIED**: Unit tests passing (`test_all_256_unprefixed_decoding`, `test_conditional_jump_call_ret_cycles`) |
| 3 | **LR35902 CPU**: CB-Prefixed Opcodes | All 256 CB-prefixed bitwise/shift opcodes | R1 | `src/cpu/opcodes.rs` | **VERIFIED**: Unit tests passing (`test_all_256_cb_decoding`, `test_cb_hl_timing`) |
| 4 | **LR35902 CPU**: ALU & DAA Math | Addition/subtraction, DAA BCD adjust, CPL/SCF/CCF | R1 | `src/cpu/alu.rs` | **VERIFIED**: Unit tests passing (`test_add_8`, `test_adc_8`, `test_sub_8`, `test_sbc_8`, `test_daa_scenarios`, etc.) |
| 5 | **LR35902 CPU**: Interrupts & HALT | IME, EI (1 delay), DI, HALT, STOP, Interrupt vectors | R1 | `src/cpu/mod.rs` | **VERIFIED**: Unit tests passing (`test_ei_1_instruction_delay_state_machine`, `test_halt_and_interrupt_wakeup`, `test_halt_bug_pc_duplication`) |
| 6 | **MMU**: 64KB Memory Map | Memory routing (ROM, VRAM, SRAM, WRAM, Echo, OAM, I/O, HRAM, IE) | R2 | `src/mmu/mod.rs` | **PARTIAL**: Basic bus struct present; detailed memory routing targets M2 |
| 7 | **Cartridge**: Header Parser | Parse Title, Cartridge Type, ROM Size, RAM Size, Checksum | R2 | `src/cartridge/mod.rs` *(Target)* | **PLANNED**: Specified under Tier 4 (T4.RWD.2); targets M2 |
| 8 | **Cartridge**: NoMBC Mapper | 32KB ROM cartridge direct address mapping | R2 | `src/cartridge/mbcless.rs` *(Target)* | **PLANNED**: Specified under Tier 1 (T1.CAR.1); targets M2 |
| 9 | **Cartridge**: MBC1 Mapper | MBC1 ROM/RAM banking, mode 0/1 switching | R2 | `src/cartridge/mbc1.rs` *(Target)* | **PLANNED**: Specified under Tier 1 (T1.CAR.2, T1.CAR.3); targets M2 |
| 10 | **Cartridge**: MBC3 Mapper | MBC3 ROM/RAM banking, RTC register stubs | R2 | `src/cartridge/mbc3.rs` *(Target)* | **PLANNED**: Specified under Tier 1 (T1.CAR.4); targets M2 |
| 11 | **Cartridge**: MBC5 Mapper | MBC5 9-bit ROM banking & RAM banking | R2 | `src/cartridge/mbc5.rs` *(Target)* | **PLANNED**: Specified under Tier 1 (T1.CAR.5); targets M2 |
| 12 | **Cartridge**: Save File Persistence | Battery-backed SRAM `.sav` file disk save/load | R2 | `src/cartridge/mod.rs` *(Target)* | **PLANNED**: Specified under Tier 1 (T1.MEM.3); targets M2 |
| 13 | **Timer**: Hardware Clocking | DIV (16384Hz), TIMA, TMA, TAC selector logic | R4 | `src/timer/mod.rs` *(Target)* | **PLANNED**: Specified under Tier 1 (T1.TIM.1-5); targets M3 |
| 14 | **Interrupts**: Controller | IF (0xFF0F), IE (0xFFFF), interrupt request flags | R4 | `src/timer/mod.rs` / `src/cpu/` *(Target)* | **PLANNED**: Specified under Tier 1 (T1.TIM.4, T1.PPU.2); targets M3 |
| 15 | **Serial**: Interceptor | SB (0xFF01), SC (0xFF02) ASCII character capture | R4/R5 | `src/serial/mod.rs` *(Target)* | **PLANNED**: Specified under Tier 2 (T2.BND.6) & Tier 4 (T4.RWD.1); targets M3 |
| 16 | **Joypad**: Matrix Input | JOYP (0xFF00) active-low button matrix polling | R4 | `src/joypad/mod.rs` *(Target)* | **PLANNED**: Specified under Tier 1 (T1.JOY.1-5); targets M3 |
| 17 | **PPU**: State Machine Timing | Mode 0 (HBlank), 1 (VBlank), 2 (OAM), 3 (Transfer) cycles | R3 | `src/ppu/mod.rs` *(Target)* | **PLANNED**: Specified under Tier 1 (T1.PPU.1); targets M4 |
| 18 | **PPU**: LCD Control & Status | LCDC, STAT, LY, LYC compare & interrupts | R3 | `src/ppu/lcd.rs` *(Target)* | **PLANNED**: Specified under Tier 1 (T1.PPU.3-5); targets M4 |
| 19 | **PPU**: OAM DMA Transfer | 0xFF46 DMA write fast copy XX00..XX9F to OAM | R3 | `src/mmu/mod.rs` / `src/ppu/` *(Target)* | **PLANNED**: Specified under Tier 2 (T2.BND.1); targets M4 |
| 20 | **PPU**: Framebuffer & Palettes | 160x144 framebuffer, BGP, OBP0, OBP1 grayscale palettes | R3 | `src/ppu/framebuffer.rs` *(Target)* | **PLANNED**: Specified under Tier 1 (T1.PPU.4); targets M4 |
| 21 | **PPU**: Scanline Renderer | BG scrolling (SCX/SCY), Window (WX/WY), Sprites (8x8/8x16) | R3 | `src/ppu/renderer.rs` *(Target)* | **PLANNED**: Specified under Tier 1 (T1.PPU.4); targets M4 |
| 22 | **CLI**: Headless Runner Mode | `--headless` CLI flag, serial match, process exit codes | R5 | `src/frontend/headless.rs` *(Target)* | **PLANNED**: Specified under Tier 4 (T4.RWD.1); targets M5 |
| 23 | **Test Harness**: E2E Runner | `scripts/run_gb_tests.sh` Blargg ROM runner | R5 | `scripts/run_gb_tests.sh` | **VERIFIED**: Harness script fully operational |
| 24 | **Frontend**: Interactive SDL2 UI | 60 FPS SDL2 renderer, windowing & keyboard mappings | R4 | `src/frontend/sdl2_gui.rs` *(Target)* | **PLANNED**: Interactive GUI launch; targets M5 |
| 25 | **APU**: Sound Register Stubs | Basic read/write stubs & masks for 0xFF10..0xFF3F | R4 | `src/apu/mod.rs` *(Target)* | **PLANNED**: Specified under Tier 1 (T1.APU.1-5); targets M5 |

---

## 5. Verification Matrix & Exit Code Expectations

| Verification Command | Execution Scope | Success Condition | Current Exit Code | Milestone M5 Target Exit Code |
|---|---|---|:---:|:---:|
| `./scripts/run_gb_tests.sh --download-only` | Asset Cache | All 13 Blargg ROMs downloaded & non-empty | **`0`** | **`0`** |
| `cargo test --all-targets` | Unit Tests | 100% unit tests pass cleanly | **`0`** (Unit tests passed) | **`0`** (All unit tests passed) |
| `./scripts/run_gb_tests.sh` | Headless E2E | All 13 Blargg ROMs log `"Passed"` | `1` (0/13 passed, stub main) | **`0`** (13/13 ROMs passed) |
| `cargo run --release -- path/to/game.gb` | Interactive GUI | Graphical window opens and runs at 60 FPS | N/A (Stub main) | **`0`** (Interactive window running) |

---

## 6. Verification Invalidation Conditions

Independent auditors will invalidate project verification if any of the following occur:
1. **Hardcoded Test Facades**: Any code returning pre-baked pass strings or hardcoded register outputs without executing real emulation logic.
2. **Missing Test Assets**: Failure of `./scripts/run_gb_tests.sh --download-only` due to invalid network URLs or zero-byte cached files.
3. **Unit Test Failure**: Any failing test during `cargo test`.
4. **Blargg Harness Unresolved Failure (at M5)**: Any of the 13 Blargg test ROMs failing or timing out once Implementation Track M5 is marked complete.
