use super::{split_command, Quit, Shell};
use std::fs;
use std::path::Path;

fn substitute(line: &str, parameters: &[&str]) -> String {
    let mut output = String::new();
    let mut characters = line.chars().peekable();
    while let Some(character) = characters.next() {
        if character != '%' {
            output.push(character);
            continue;
        }
        match characters.peek().copied() {
            Some(digit) if digit.is_ascii_digit() => {
                characters.next();
                let index = digit.to_digit(10).unwrap_or(0) as usize;
                output.push_str(parameters.get(index).copied().unwrap_or(""));
            }
            Some('%') => {
                characters.next();
                output.push('%');
            }
            _ => output.push('%'),
        }
    }
    output
}

impl Shell {
    pub(super) fn run_batch(&mut self, path: &Path, arguments: &str) -> Result<(), Quit> {
        let Ok(content) = fs::read(path) else {
            self.println("Unable to read batch file");
            return Ok(());
        };
        let text: String = content.iter().map(|byte| *byte as char).collect();
        let name = path.file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_default();
        let mut parameters = vec![name.as_str()];
        parameters.extend(arguments.split_whitespace());
        let saved_echo = self.echo;
        for raw in text.lines() {
            let line = substitute(raw.trim(), &parameters);
            if line.is_empty() || line.starts_with(':') {
                continue;
            }
            let silent = line.starts_with('@');
            let (command, _) = split_command(line.trim_start_matches('@'));
            if command == "REM" {
                continue;
            }
            if self.echo && !silent {
                let prompt = format!("{}>{}", self.machine.drive.current_text(), line);
                self.println(&prompt);
            }
            self.execute(&line)?;
        }
        self.echo = saved_echo;
        Ok(())
    }
}
