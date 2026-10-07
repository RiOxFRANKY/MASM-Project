use std::path::PathBuf;

pub const USAGE: &str = "\
Usage: dosbox [DIRECTORY] [options]

  DIRECTORY          host folder mounted as drive C: (default: current folder)
  -c COMMAND         run a DOS command at startup (can be repeated)
  --exit             quit after the -c commands have run
  --headless         run without a window; print the final screen to stdout
  --keys SCRIPT      keys to type in headless mode, e.g. \"dd{down}{esc}\"
  --screenshot FILE  in headless mode, also save the final screen as coloured HTML
  -h, --help         show this help

Example:
  dosbox Code -c \"masm MAZE.ASM;\" -c \"link MAZE.OBJ;\" -c MAZE";

pub struct Config {
    pub directory: PathBuf,
    pub commands: Vec<String>,
    pub exit_after: bool,
    pub headless: bool,
    pub keys: String,
    pub screenshot: Option<PathBuf>,
}

pub enum Parsed {
    Run(Config),
    Help,
}

pub fn parse(arguments: impl Iterator<Item = String>) -> Result<Parsed, String> {
    let mut config = Config {
        directory: PathBuf::from("."),
        commands: Vec::new(),
        exit_after: false,
        headless: false,
        keys: String::new(),
        screenshot: None,
    };
    let mut directory_given = false;
    let mut arguments = arguments;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "-h" | "--help" | "/?" => return Ok(Parsed::Help),
            "-c" => config.commands.push(arguments.next().ok_or("-c needs a command")?),
            "--exit" | "-exit" => config.exit_after = true,
            "--headless" => config.headless = true,
            "--keys" => config.keys = arguments.next().ok_or("--keys needs a script")?,
            "--screenshot" => {
                config.screenshot = Some(PathBuf::from(arguments.next().ok_or("--screenshot needs a file name")?));
            }
            _ if argument.starts_with('-') => return Err(format!("unknown option {argument}")),
            _ if !directory_given => {
                config.directory = PathBuf::from(argument);
                directory_given = true;
            }
            _ => return Err(format!("unexpected argument {argument}")),
        }
    }
    Ok(Parsed::Run(config))
}
