# Test Infrastructure & Verification Specification

**Project:** Clean-Room Game Boy (LR35902) Rust Emulator  
**Target File Location:** `/Users/iml1s/Documents/mine/gb_emulator/TEST_INFRA.md`  
**Specification Version:** 1.0.0  

---

## 1. Executive Summary & Test Philosophy

The Game Boy (LR35902) emulator test suite follows a strict **opaque-box, requirement-driven verification strategy**. Emulation correctness is validated strictly through hardware interface behavior, cycle timing contracts, state transitions, and objective binary test harness outcomes (Blargg test ROMs).

### Key Test Principles:
1. **Opaque-Box Verification**: Components (`Cpu`, `Mmu`/`Bus`, `Ppu`, `Timer`, `Joypad`, `SerialPort`, `Cartridge`) are tested against publicly exposed hardware memory mappings, register specifications, and signal interfaces. Internal private fields are not relied upon for pass/fail determination.
2. **Requirement-Driven Traceability**: Every unit test, boundary assertion, and integration scenario maps directly to functional requirements (R1–R5) and features (#1–#25) defined in `PROJECT.md`.
3. **Deterministic Cycle-Accurate Timing**: Hardware components step synchronously by M-cycles / T-cycles (4.194304 MHz clock base). Cycle timing is verified down to individual instruction fetch and hardware state machine transitions.
4. **Zero Hardcoded Facades / Anti-Slop Enforcement**: All tests execute real code path implementations. No hardcoded string checks, dummy return facades, or fake test passes are permitted. Verification is audited by independent automated tooling.

---

## 2. Feature Inventory to Test Strategy Mapping

| # | Feature Domain | Description | Test Strategy & Tier Coverage |
|---|----------------|-------------|-------------------------------|
| 1 | CPU Registers & Flags | AF, BC, DE, HL, SP, PC, Flags (Z, N, H, C) | **Tier 1 (T1.CPU.1-5)**: Unit tests for register pair math & flag calculation |
| 2 | Unprefixed Opcodes | 256 LR35902 base opcodes & cycle counts | **Tier 1 (T1.CPU.1, T1.CPU.2)** & **Tier 4 (T4.RWD.1)**: Unit tests + Blargg E2E |
| 3 | CB-Prefixed Opcodes | 256 CB-prefixed bitwise/shift opcodes | **Tier 1 (T1.CPU.3, T1.CPU.4)** & **Tier 4 (T4.RWD.1)**: Bit ops unit tests + Blargg E2E |
| 4 | ALU & DAA Math | Addition/subtraction, DAA BCD adjust, CPL/SCF/CCF | **Tier 1 (T1.CPU.2)**: Dedicated DAA BCD addition & subtraction edge case tests |
| 5 | Interrupt Dispatch & HALT | IME, EI (1 delay), DI, HALT, STOP, Interrupt vectors | **Tier 1 (T1.CPU.5)**, **Tier 2 (T2.BND.3)** & **Tier 3 (T3.XFT.1)**: HALT bug & ISR tests |
| 6 | 64KB Memory Map | Memory routing (ROM, VRAM, SRAM, WRAM, Echo, OAM, I/O, HRAM, IE) | **Tier 1 (T1.MEM.1-5)** & **Tier 2 (T2.BND.5)**: Memory boundary & Echo RAM tests |
| 7 | Cartridge Header Parser | Parse Title, MBC Type, ROM Size, RAM Size, Checksum | **Tier 4 (T4.RWD.2)**: ROM header parser & corrupted header rejection tests |
| 8 | NoMBC Mapper | 32KB ROM cartridge direct mapping | **Tier 1 (T1.CAR.1)**: Fixed 32KB ROM direct address mapping unit tests |
| 9 | MBC1 Mapper | MBC1 ROM/RAM banking, mode switching | **Tier 1 (T1.CAR.2, T1.CAR.3)** & **Tier 2 (T2.BND.4)**: Banking & mask tests |
| 10 | MBC3 Mapper | MBC3 ROM/RAM banking, RTC register stubs | **Tier 1 (T1.CAR.4)**: MBC3 ROM/RAM & RTC register latch selection unit tests |
| 11 | MBC5 Mapper | MBC5 9-bit ROM banking & RAM banking | **Tier 1 (T1.CAR.5)**: MBC5 9-bit bank calculation & Bank 0 access unit tests |
| 12 | Battery SRAM Persistence | Save file (`.sav`) disk read/write persistence | **Tier 1 (T1.MEM.3)**: SRAM enable gating & disk persistence unit tests |
| 13 | Hardware Timer | DIV (16384Hz), TIMA, TMA, TAC bit selector logic | **Tier 1 (T1.TIM.1-5)**: Frequency clocking, overflow & falling edge glitch tests |
| 14 | Interrupt Controller | IF (0xFF0F), IE (0xFFFF), interrupt request flags | **Tier 1 (T1.TIM.4, T1.PPU.2-3, T1.JOY.4)**: Hardware interrupt flag assertions |
| 15 | Serial Output Interceptor | SB (0xFF01), SC (0xFF02) ASCII character capture | **Tier 2 (T2.BND.6)** & **Tier 4 (T4.RWD.1)**: Serial shift clocking & Blargg scraper |
| 16 | Joypad Matrix | JOYP (0xFF00) active-low button matrix polling | **Tier 1 (T1.JOY.1-5)**: Active-low D-Pad & Action button matrix resolution tests |
| 17 | PPU Mode State Machine | Mode 0 (HBlank), 1 (VBlank), 2 (OAM), 3 (Transfer) timing | **Tier 1 (T1.PPU.1)** & **Tier 3 (T3.XFT.2)**: PPU mode cycle step & DMA integration |
| 18 | LCD Control & Status | LCDC, STAT, LY, LYC equality compare & interrupts | **Tier 1 (T1.PPU.3-5)** & **Tier 2 (T2.BND.2)**: STAT blocking & LCD off tests |
| 19 | OAM DMA Transfer | 0xFF46 DMA write triggering fast copy XX00..XX9F to OAM | **Tier 2 (T2.BND.1)** & **Tier 3 (T3.XFT.2)**: Main bus lock & concurrent PPU tests |
| 20 | Framebuffer & Palettes | 160x144 framebuffer, BGP, OBP0, OBP1 grayscale palettes | **Tier 1 (T1.PPU.4)** & **Tier 4 (T4.RWD.3)**: Framebuffer generation & throughput |
| 21 | Scanline Renderer | BG scrolling (SCX/SCY), Window (WX/WY), Sprites | **Tier 1 (T1.PPU.4)**: Scanline tile & sprite rendering unit tests |
| 22 | Headless Runner CLI | `--headless` flag, serial matching, process exit codes | **Tier 4 (T4.RWD.1)** & **Section 5**: CLI runner integration |
| 23 | E2E Test Suite Script | `scripts/run_gb_tests.sh` Blargg automation | **Section 5**: Shell script execution, ROM caching & verification |
| 24 | Interactive SDL2 Window | 60 FPS SDL2 frontend wrapper & keyboard input | Manual & frontend execution tests |
| 25 | APU Stubs | Basic read/write stubs for 0xFF10..0xFF3F | **Tier 1 (T1.APU.1-5)**: NR52 power reset & register mask unit tests |

---

## 3. 4-Tier Test Coverage Methodology

### TIER 1: FEATURE COVERAGE (Unit Tests across 7 Hardware Domains)
*Minimum Requirement: 5 unit tests per hardware domain (35 tests total).*

#### 1. CPU Opcodes Domain (`src/cpu/`)
- **T1.CPU.1**: Unprefixed 8-bit Arithmetic & Flag Computation (`ADD A, r8`, `SUB r8`). Verifies `A`, `Z`, `N`, `H`, `C` flags after arithmetic operations.
- **T1.CPU.2**: DAA Decimal Adjust Addition/Subtraction Edge Cases. Validates BCD adjustments for addition (`N=0`) and subtraction (`N=1`) across boundary conditions.
- **T1.CPU.3**: CB-Prefixed Bitwise Operations (`BIT b, r8`, `SET b, r8`, `RES b, r8`). Verifies bit isolation, flag updates (`H=1`, `N=0`), and targeted bit modifications.
- **T1.CPU.4**: CB-Prefixed Rotates & Shifts (`RLC`, `RRC`, `SLA`, `SRL`, `SWAP`). Asserts carry bit shifting, zero flag setting, and byte nibble swapping.
- **T1.CPU.5**: Control Flow & Stack Pushing/Popping (`CALL`, `RET`, `RST`, `JP cc`). Confirms 16-bit Little-Endian word push/pop onto stack (`SP`) and conditional branching logic.

#### 2. Memory Map Domain (`src/mmu/`)
- **T1.MEM.1**: ROM Bank 0 & Switchable Bank Boundary Reads (`0x0000-0x7FFF`). Verifies immutable ROM Bank 0 (`0x0000-0x3FFF`) and active switchable Bank N (`0x4000-0x7FFF`).
- **T1.MEM.2**: VRAM Read/Write & Mode-Based Access Lockout (`0x8000-0x9FFF`). Asserts VRAM accessibility during VBlank/HBlank and write-protection during PPU Mode 3.
- **T1.MEM.3**: SRAM Gating & Save File Persistence (`0xA000-0xBFFF`). Validates SRAM access enable code (`0x0A`) gating and binary disk `.sav` persistence.
- **T1.MEM.4**: WRAM & High RAM (HRAM) Dual Memory Integrity. Asserts WRAM (`0xC000-0xDFFF`) and HRAM (`0xFF80-0xFFFE`) isolation without cross-talk or address corruption.
- **T1.MEM.5**: Reserved / Unmapped Memory Space Behavior (`0xFEA0-0xFEFF`). Confirms open-bus reads return `0xFF` and unmapped writes are safely ignored.

#### 3. Timer System Domain (`src/timer/`)
- **T1.TIM.1**: DIV Register Constant Clocking & Write Reset. Tests 16384Hz DIV increments every 256 T-cycles and instant reset to `0x00` upon CPU write.
- **T1.TIM.2**: TIMA Frequency Clock Selection via TAC Register. Validates TIMA increment ratios for all 4 TAC clock frequencies (4096Hz, 262144Hz, 65536Hz, 16384Hz).
- **T1.TIM.3**: TIMA Overflow, Delay & TMA Reload Mechanism. Tests 1 M-cycle delay state (`0x00`) following `0xFF` overflow prior to TMA reload.
- **T1.TIM.4**: Timer Interrupt Request Generation (`IF` Bit 2). Asserts Timer interrupt request bit 2 in `IF (0xFF0F)` concurrent with TMA reload.
- **T1.TIM.5**: TAC Disable / Divider Reset Falling Edge Glitch. Verifies hardware edge-detector glitch that increments TIMA when disabling TAC while internal timer tap is high.

#### 4. PPU System Domain (`src/ppu/`)
- **T1.PPU.1**: Mode State Machine Cycle Timing Verification. Validates exact T-cycle offsets for Mode 2 (80 T), Mode 3 (172 T), Mode 0 (204 T), and scanline total (456 T).
- **T1.PPU.2**: VBlank Interrupt Request Generation at `LY = 144`. Confirms `LY` transition to 144 triggers Mode 1 and asserts `IF` bit 0 (VBlank IRQ).
- **T1.PPU.3**: STAT Interrupt Selection Signals (Mode 0, Mode 1, Mode 2, `LY=LYC`). Validates STAT interrupt triggering (`IF` bit 1) for enabled STAT conditions.
- **T1.PPU.4**: LCDC Register Control Bit-by-Bit Functionality. Asserts rendering layer toggles (BG, Window, Sprites), tile map selection (`0x9800`/`0x9C00`), and tile data selection (`0x8000`/`0x8800`).
- **T1.PPU.5**: Scanline LY Counter Increment & LYC Coincidence Flag. Confirms `LY` progression from 0 to 153 and STAT bit 2 coincidence bit assertion when `LY == LYC`.

#### 5. APU Stubs Domain (`src/apu/`)
- **T1.APU.1**: NR52 Master Power Switch Reset Functionality. Tests clearing `NR52` bit 7 clears all APU registers (`0xFF10-0xFF25`) and locks write access.
- **T1.APU.2**: Sound Channel 1 & 2 Frequency / Duty Cycle Register R/W. Verifies Duty Cycle and Volume Envelope register reads match hardware read masks.
- **T1.APU.3**: Sound Channel 3 Wave RAM R/W Access (`0xFF30-0xFF3F`). Confirms 16-byte custom waveform RAM access when Channel 3 is disabled.
- **T1.APU.4**: Sound Channel 4 Noise Polynomial Counter Registers. Tests noise channel polynomial step counter and shift clock frequency register storage.
- **T1.APU.5**: Master Volume & Channel Panning Control (`NR50` & `NR51`). Asserts panning and volume control register write preservation.

#### 6. Joypad Matrix Domain (`src/joypad/`)
- **T1.JOY.1**: Direction Button Selection Mode (`P14 = 0, P15 = 1`). Asserts `JOYP (0xFF00)` lower nibble returns active-low state for D-Pad buttons (Right, Left, Up, Down).
- **T1.JOY.2**: Action Button Selection Mode (`P14 = 1, P15 = 0`). Asserts lower nibble returns active-low state for Action buttons (A, B, Select, Start).
- **T1.JOY.3**: Neither Selection Mode (`P14 = 1, P15 = 1`). Confirms lower nibble returns `0x0F` (all inactive) when neither matrix line is selected.
- **T1.JOY.4**: Joypad Interrupt Triggering on High-to-Low State Transition. Tests high-to-low signal transition on button press asserts `IF` bit 4 (Joypad IRQ).
- **T1.JOY.5**: Simultaneous Multi-Button Selection Active-Low Resolution. Validates bitwise active-low AND logic when both selection lines are grounded (`P14=0, P15=0`).

#### 7. Cartridge Controllers Domain (`src/cartridge/`)
- **T1.CAR.1**: MBC0 (NoMBC) Fixed 32KB ROM Direct Address Mapping. Tests 1:1 mapping of `0x0000-0x7FFF` to 32KB ROM buffer.
- **T1.CAR.2**: MBC1 ROM Bank Selection & Translation. Verifies Bank 0 write translation (`Bank 0 -> Bank 1`) and 5-bit bank register calculations.
- **T1.CAR.3**: MBC1 RAM Enable & Banking Mode Switching. Validates Mode 0 (ROM Banking Mode) vs Mode 1 (RAM Banking Mode) register behavior.
- **T1.CAR.4**: MBC3 ROM & RAM / RTC Register Latch Selection. Asserts RTC register selection (`0x08-0x0C`) mapping to `0xA000-0BFFF`.
- **T1.CAR.5**: MBC5 9-bit ROM Banking & Zero-Bank Unmapped Access. Tests 9-bit ROM bank index calculation and direct Bank 0 mapping at `0x4000-0x7FFF`.

---

### TIER 2: BOUNDARY & CORNER CASES (6 Edge Case Scenarios)

- **T2.BND.1: OAM DMA Transfer Timing & Bus Locking Isolation**
  - *Scenario*: Writing `0xFF46` triggers a 160 M-cycle (640 T-cycle) DMA transfer from source address `XX00-XX9F` to OAM (`0xFE00-0xFE9F`).
  - *Verification*: CPU attempts to read/write main bus (ROM, VRAM, WRAM) during DMA. Main bus access is locked out by MMU. Access to HRAM (`0xFF80-0xFFFE`) remains functional.

- **T2.BND.2: PPU STAT Interrupt Blocking & LCD Turn-Off Behavior**
  - *Scenario*: STAT interrupt line acts as a single OR gate across conditions. If one condition is already active, new conditions fail to generate a rising edge. Turning off LCD (`LCDC` bit 7 = 0) halts PPU state machine.
  - *Verification*: Tests that transition to Mode 0 while Mode 2 STAT IRQ is active does not generate duplicate IRQ. Turning off LCD instantly sets `LY = 0`, resets STAT mode to 0, and grants CPU access to VRAM/OAM.

- **T2.BND.3: CPU HALT Bug Execution under Pending Interrupts (`IME = 0`)**
  - *Scenario*: Executing `HALT` when `IME = 0` and `IF & IE != 0` causes CPU to fail to increment `PC` during the fetch stage of the next instruction.
  - *Verification*: Confirms instruction byte immediately following `HALT` is fetched and executed twice.

- **T2.BND.4: MBC Banking Register Bit Masking & Address Overflow Gating**
  - *Scenario*: Games write unmasked bank values (e.g. `0xFF`) to cartridges with smaller ROM/RAM capacities.
  - *Verification*: Mapper applies bitwise modulo masking based on total bank capacity (`bank_index & (max_banks - 1)`), preventing buffer overflow or out-of-bounds panics.

- **T2.BND.5: Echo RAM Mirroring (`0xE000-0xFDFF` -> `0xC000-0xDDFF`)**
  - *Scenario*: `0xE000-0xFDFF` acts as a 1:1 hardware mirror of WRAM `0xC000-0xDDFF`.
  - *Verification*: Writes to `0xE000-0xFDFF` update `0xC000-0xDDFF` bi-directionally across the entire 7.5KB Echo RAM range.

- **T2.BND.6: Serial Transfer Shift Clocking & Output Buffer Capture**
  - *Scenario*: Writing `0x81` to `SC (0xFF02)` shifts data byte in `SB (0xFF01)` out over 512 T-cycles (8192Hz).
  - *Verification*: Confirms shift completes in 512 T-cycles, sets `SC` bit 7 to 0, asserts Serial IRQ (`IF` bit 3), and appends ASCII character to `SerialPort.output_buffer`.

---

### TIER 3: CROSS-FEATURE INTERACTIONS (3 Multi-Component Scenarios)

- **T3.XFT.1: Timer Interrupt Generation & Execution during CPU HALT State**
  - *Interaction*: CPU is in `HALT` mode while Timer counts down to overflow.
  - *Verification*: TIMA overflow sets `IF` bit 2, wakes CPU from `HALT` (`halted = false`), pushes return `PC` onto stack, clears `IME`, and dispatches CPU to Timer ISR vector `0x0050`.

- **T3.XFT.2: PPU STAT Interrupt Generation during Active OAM DMA Transfers**
  - *Interaction*: OAM DMA transfer is active while PPU steps through Mode transitions (Mode 2 -> 3 -> 0).
  - *Verification*: PPU state machine advances concurrently during DMA, and Mode 0 transition successfully asserts STAT IRQ (`IF` bit 1) at T-cycle ~252.

- **T3.XFT.3: Cartridge Bank Switching within Timer Interrupt Service Routines**
  - *Interaction*: Timer ISR switches ROM Bank (`0x2000` write) before returning (`RETI`) to main loop executing in a different bank.
  - *Verification*: Mapper switches bank during ISR, restores original bank before `RETI`, and main execution loop resumes at exact return `PC` without opcode corruption.

---

### TIER 4: REAL-WORLD SCENARIOS & BLARGG E2E HARNESS

- **T4.RWD.1: Blargg Full CPU Test Suite Execution (`cpu_instrs.gb` & 11 Sub-Tests)**
  - *Scenario*: Headless verification of 13 Blargg test ROM binaries (`cpu_instrs.gb`, `instr_timing.gb`, `01-special.gb` through `11-op a,(hl).gb`).
  - *Verification*: Script `scripts/run_gb_tests.sh` executes binary in headless mode, intercepts serial output, searches for `"Passed"` string, confirms absence of `"Failed"`, and checks process exit code 0.

- **T4.RWD.2: Cartridge ROM Header Parsing & Validation Engine**
  - *Scenario*: Header parser checks entry point (`0x0100`), Nintendo logo graphic (`0x0104-0x0133`), Title, Cartridge Type (`0x0147`), ROM/RAM size, and Header Checksum (`0x014D`).
  - *Verification*: Valid ROM headers instantiate correct mapper backend (`NoMBC`, `MBC1`, `MBC3`, `MBC5`). Corrupted headers return descriptive `CartridgeError` without booting.

- **T4.RWD.3: Frame Generation Rate & Headless Rendering Throughput Metric**
  - *Scenario*: Measures emulation speed and frame generation timing (70,224 T-cycles per frame = 154 scanlines $\times$ 456 T-cycles).
  - *Verification*: Executes 3,600 video frames in headless mode, asserts exact 70,224 T-cycles per frame timing, and verifies throughput exceeds 300 FPS baseline on modern CPU.

---

## 4. Test Summary & Verification Matrix

| Tier | Test Category | Harness / Driver | Verification Mechanism | Pass Criterion | Fail Criterion |
|---|---|---|---|---|---|
| **Tier 1** | CPU Opcodes | `src/cpu/` unit tests (`#[test]`) | Register & flag state inspection post-step | Regs & flags (Z,N,H,C) match exact expected values | Mismatched register values or incorrect flag computations |
| **Tier 1** | Memory Map | `src/mmu/` unit tests (`#[test]`) | Memory R/W boundary assertions | Reads match expected byte pattern; writes locked as specified | Unmapped writes mutate memory or reads return wrong banks |
| **Tier 1** | Timer System | `src/timer/` unit tests (`#[test]`) | Cycle-by-cycle step & IF register bit check | DIV resets on write; TIMA reloads TMA + sets IF bit 2 | Incorrect frequency clocking or missing TIMA reload |
| **Tier 1** | PPU Engine | `src/ppu/` unit tests (`#[test]`) | Mode cycle counter & scanline LY check | Mode 2->3->0->1 timing exact; VBlank/STAT IF flags set | Mode cycle drift or missed LY=LYC STAT interrupts |
| **Tier 1** | APU Stubs | `src/apu/` unit tests (`#[test]`) | Register R/W mask verification | NR52=0 clears registers; Wave RAM accessible when off | Disabling NR52 fails to clear registers or lock writes |
| **Tier 1** | Joypad Input | `src/joypad/` unit tests (`#[test]`) | Active-low matrix polling assertion | JOYP lower nibble correctly isolates D-Pad vs Action | Active-high logic inversion or wrong button matrix bits |
| **Tier 1** | Cartridges | `src/cartridge/` unit tests (`#[test]`) | Bank register write & RAM R/W test | MBC1/3/5 ROM & RAM bank offset calculations accurate | Bank 0 translation missing on MBC1 or wrong MBC5 bits |
| **Tier 2** | OAM DMA | Component integration test | Bus lock read/write attempt during DMA | Main bus locked for 640 T-cycles; HRAM remains readable | Bus read/write succeeds during active DMA transfer |
| **Tier 2** | STAT / LCD Off | Component integration test | STAT OR-line state & LCDC bit 7 toggle | STAT blocking reproduced; LCD off sets LY=0 and Mode 0 | LCD off fails to reset LY or leaves PPU in Mode 3 |
| **Tier 2** | HALT Bug | CPU unit test | Dual instruction execution trace | Inst byte after HALT executed twice when IME=0 & IF!=0 | PC increments normally despite HALT bug trigger condition |
| **Tier 2** | MBC Masking | Cartridge unit test | Out-of-bounds bank write test | Bank selection masked to cartridge bank capacity | Memory out-of-bounds index or array index panic |
| **Tier 2** | Echo RAM | Memory integration test | Bi-directional address read/write | Writes to 0xE000-0xFDFF mirror 0xC000-0xDDFF 1:1 | Echo RAM write does not update base WRAM |
| **Tier 2** | Serial Shift | Serial integration test | T-cycle step counter & output buffer | Shift completes in 512 T-cycles, sets IF bit 3 & buffer | Early/late completion or missing serial interrupt |
| **Tier 3** | Timer in HALT | Full system integration test | Step emulator in HALT state | Timer overflow wakes CPU from HALT & jumps to 0x0050 | CPU remains stuck in HALT or fails to service ISR |
| **Tier 3** | STAT in DMA | Full system integration test | DMA + PPU mode step test | STAT interrupt fires on Mode 0 transition during DMA | STAT interrupt blocked or DMA cycle timing corrupted |
| **Tier 3** | Bank Switch ISR | Full system integration test | Nested ISR bank switch execution | Original bank restored; RETI resumes main loop PC | Stack corruption, crash, or execution of wrong bank |
| **Tier 4** | Blargg CPU ROMs | `scripts/run_gb_tests.sh` runner | Serial output buffer scraper | Serial buffer contains "Passed" for all 11 sub-tests | Buffer contains "Failed", times out, or process crashes |
| **Tier 4** | Header Parser | CLI / Loader test harness | ROM checksum & logo validator | Valid headers load; invalid headers return clean Err | Corrupted ROM boots or valid ROM rejected by parser |
| **Tier 4** | Frame / FPS | Headless benchmark runner | Frame counter & timer duration | 3600 frames at 70,224 cycles/frame executed >300 FPS | Frame cycle drift or throughput < 300 FPS headless |

---

## 5. Automated Test Runner Details (`scripts/run_gb_tests.sh`)

The project includes a production-grade Bash test harness runner at `scripts/run_gb_tests.sh`.

### CLI Options:
- `--download-only`: Automatically fetches and caches all 13 Blargg test ROMs under `tests/roms/` and `tests/roms/individual/`, verifies non-empty binary sizes, and exits cleanly with code 0 without attempting cargo build or emulator execution.
- `--headless`: Executes the emulator in headless mode without SDL2 GUI window rendering (default execution mode).
- `--timeout <N>`: Overrides the default per-ROM execution timeout with `<N>` seconds.
- `--help, -h`: Displays help documentation and command line flags.

### Remote Test ROM Asset Inventory (13 Blargg ROMs):
1. `cpu_instrs.gb` (Umbrella ROM, default timeout: 35s)  
   `https://raw.githubusercontent.com/retrio/gb-test-roms/master/cpu_instrs/cpu_instrs.gb`
2. `instr_timing.gb` (Timing ROM, default timeout: 10s)  
   `https://raw.githubusercontent.com/retrio/gb-test-roms/master/instr_timing/instr_timing.gb`
3. `01-special.gb` (Sub-test, default timeout: 10s)  
   `https://raw.githubusercontent.com/retrio/gb-test-roms/master/cpu_instrs/individual/01-special.gb`
4. `02-interrupts.gb` (Sub-test, default timeout: 10s)  
   `https://raw.githubusercontent.com/retrio/gb-test-roms/master/cpu_instrs/individual/02-interrupts.gb`
5. `03-op sp,hl.gb` (Sub-test, default timeout: 10s)  
   `https://raw.githubusercontent.com/retrio/gb-test-roms/master/cpu_instrs/individual/03-op%20sp,hl.gb`
6. `04-op r,imm.gb` (Sub-test, default timeout: 10s)  
   `https://raw.githubusercontent.com/retrio/gb-test-roms/master/cpu_instrs/individual/04-op%20r,imm.gb`
7. `05-op rp.gb` (Sub-test, default timeout: 10s)  
   `https://raw.githubusercontent.com/retrio/gb-test-roms/master/cpu_instrs/individual/05-op%20rp.gb`
8. `06-ld r,r.gb` (Sub-test, default timeout: 10s)  
   `https://raw.githubusercontent.com/retrio/gb-test-roms/master/cpu_instrs/individual/06-ld%20r,r.gb`
9. `07-jr,jp,call,ret,rst.gb` (Sub-test, default timeout: 10s)  
   `https://raw.githubusercontent.com/retrio/gb-test-roms/master/cpu_instrs/individual/07-jr,jp,call,ret,rst.gb`
10. `08-misc instrs.gb` (Sub-test, default timeout: 10s)  
    `https://raw.githubusercontent.com/retrio/gb-test-roms/master/cpu_instrs/individual/08-misc%20instrs.gb`
11. `09-op r,r.gb` (Sub-test, default timeout: 10s)  
    `https://raw.githubusercontent.com/retrio/gb-test-roms/master/cpu_instrs/individual/09-op%20r,r.gb`
12. `10-bit ops.gb` (Sub-test, default timeout: 10s)  
    `https://raw.githubusercontent.com/retrio/gb-test-roms/master/cpu_instrs/individual/10-bit%20ops.gb`
13. `11-op a,(hl).gb` (Sub-test, default timeout: 10s)  
    `https://raw.githubusercontent.com/retrio/gb-test-roms/master/cpu_instrs/individual/11-op%20a,(hl).gb`

---

## 6. Verification Commands & Execution Guidelines

To independently verify the test infrastructure and run test suites:

### 1. Test Harness ROM Download & Verification
```bash
chmod +x scripts/run_gb_tests.sh
./scripts/run_gb_tests.sh --download-only
```
*Expected Output*: Downloads and caches all 13 Blargg test ROMs under `tests/roms/`, prints success status, and exits with code 0.

### 2. Rust Unit & Integration Test Suites (Tiers 1, 2, 3)
```bash
cargo test --all-targets -- --nocapture
```
*Expected Output*: Executes unit tests across CPU, MMU, PPU, Timer, APU, Joypad, and Cartridge modules, passing cleanly with zero failures.

### 3. Full Headless E2E Test Suite Execution (Tier 4)
```bash
./scripts/run_gb_tests.sh
```
*Expected Output*: Compiles release binary (`cargo build --release`), runs all 13 Blargg test ROMs in headless mode with default timeouts, prints ANSI test progress, verifies serial output contains `"Passed"`, and exits with code 0.

### 4. Verification Invalidation Conditions:
- Any `cargo test` failure invalidates Tier 1, Tier 2, or Tier 3 qualification.
- Missing or corrupted test ROM cache invalidates test runner setup.
- Any Blargg sub-test failing to write `"Passed"` or timing out invalidates Tier 4 qualification.
- Hardcoded test passes or dummy mocks fail independent forensic auditor checks.
