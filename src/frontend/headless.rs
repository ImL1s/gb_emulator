use anyhow::{bail, Result};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::Instant;

use crate::cartridge;
use crate::cpu::Cpu;
use crate::mmu::Mmu;

/// Run Game Boy emulator in headless mode driving test execution loops and monitoring serial output.
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

    let max_cycles: u64 = 500_000_000;
    let mut total_cycles: u64 = 0;
    let mut last_output_len = 0;

    let start_time = Instant::now();
    let max_duration = std::time::Duration::from_secs(25);

    while total_cycles < max_cycles && start_time.elapsed() < max_duration {
        let cycles = cpu.step(&mut mmu);
        total_cycles += cycles as u64;

        let output = mmu.get_serial_output();
        if output.len() > last_output_len {
            print!("{}", &output[last_output_len..]);
            io::stdout().flush().ok();
            last_output_len = output.len();

            if output.contains("Passed") {
                println!();
                return Ok(());
            }

            if output.contains("Failed") {
                println!();
                bail!("Test ROM reported Failure:\n{}", output);
            }
        }
    }

    let output = mmu.get_serial_output();
    if output.contains("Passed") {
        println!();
        return Ok(());
    }

    bail!(
        "Test ROM execution timed out after {} cycles ({}s elapsed).\nSerial Output:\n{}",
        total_cycles,
        start_time.elapsed().as_secs(),
        output
    );
}
