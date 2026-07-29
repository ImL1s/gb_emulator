use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "gb_emulator",
    version = "0.1.0",
    author = "LR35902 Team",
    about = "Game Boy LR35902 Emulator"
)]
struct Args {
    /// Path to Game Boy ROM file (.gb)
    #[arg(required = true)]
    rom_path: PathBuf,

    /// Run in headless test execution mode
    #[arg(long)]
    headless: bool,

    /// Path to save screen framebuffer output PPM/PNG image
    #[arg(long)]
    screenshot: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = Args::parse();

    log::info!(
        "Game Boy Emulator initialized (headless: {})",
        args.headless
    );

    if args.headless || args.screenshot.is_some() {
        gb_emulator::frontend::headless::run_with_screenshot(
            &args.rom_path,
            args.screenshot.as_deref(),
        )
    } else {
        gb_emulator::frontend::sdl2_gui::run(&args.rom_path)
    }
}
