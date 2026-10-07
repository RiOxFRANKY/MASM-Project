mod bios;
mod config;
mod cpu;
mod dos;
mod formats;
mod hardware;
mod linker;
mod machine;
mod masm;
mod shell;
mod terminal;

use config::Parsed;
use dos::filesystem::Drive;
use machine::Machine;
use shell::Shell;
use std::process::ExitCode;
use terminal::headless::HeadlessTerminal;
use terminal::Terminal;

fn main() -> ExitCode {
    let config = match config::parse(std::env::args().skip(1)) {
        Ok(Parsed::Run(config)) => config,
        Ok(Parsed::Help) => {
            println!("{}", config::USAGE);
            return ExitCode::SUCCESS;
        }
        Err(message) => {
            eprintln!("dosbox: {message}\n\n{}", config::USAGE);
            return ExitCode::FAILURE;
        }
    };
    let drive = match Drive::mount(&config.directory) {
        Ok(drive) => drive,
        Err(error) => {
            eprintln!("dosbox: cannot mount {}: {error}", config.directory.display());
            return ExitCode::FAILURE;
        }
    };
    let terminal: Box<dyn Terminal> = if config.headless {
        match HeadlessTerminal::new(&config.keys, config.screenshot.clone()) {
            Ok(terminal) => Box::new(terminal),
            Err(message) => {
                eprintln!("dosbox: {message}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        match terminal::open_console() {
            Ok(terminal) => terminal,
            Err(error) => {
                eprintln!("dosbox: cannot open the console: {error}");
                return ExitCode::FAILURE;
            }
        }
    };
    let mut shell = Shell::new(Machine::new(terminal, drive));
    shell.banner();
    let finished = shell.run_startup(&config.commands);
    if finished.is_ok() && !config.exit_after {
        let _ = shell.interactive();
    }
    shell.finish();
    ExitCode::SUCCESS
}
