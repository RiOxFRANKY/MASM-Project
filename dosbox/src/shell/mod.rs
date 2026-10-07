mod batch;
mod commands;
mod programs;
mod tools;

use crate::machine::{Halt, Machine};

const HISTORY_LIMIT: usize = 50;
const LINE_LIMIT: usize = 126;

pub struct Quit;

pub struct Shell {
    machine: Machine,
    history: Vec<String>,
    echo: bool,
}

pub fn split_command(line: &str) -> (String, String) {
    let line = line.trim();
    let end = line.find(|character: char| character.is_whitespace() || "/;,=".contains(character)).unwrap_or(line.len());
    let (name, rest) = line.split_at(end);
    let upper = name.to_ascii_uppercase();
    for prefix in ["CD", "CHDIR", "MD", "MKDIR", "RD", "RMDIR", "DIR", "ECHO"] {
        if let Some(tail) = upper.strip_prefix(prefix) {
            if tail.starts_with('.') || tail.starts_with('\\') {
                return (prefix.to_string(), format!("{}{}", &name[prefix.len()..], rest));
            }
        }
    }
    (upper, rest.trim_start().to_string())
}

impl Shell {
    pub fn new(machine: Machine) -> Self {
        Shell { machine, history: Vec::new(), echo: true }
    }

    pub fn print(&mut self, text: &str) {
        self.machine.print(text);
    }

    pub fn println(&mut self, text: &str) {
        self.machine.print(text);
        self.machine.print("\n");
    }

    pub fn banner(&mut self) {
        let root = self.machine.drive.root().display().to_string();
        self.println("DOSBox-RS 8086 emulator with built-in MASM and LINK");
        self.println(&format!("Drive C: is mounted at {root}"));
        self.println("Type HELP for commands. Ctrl+C stops a program, Ctrl+F9 quits.");
        self.println("");
    }

    fn prompt(&mut self) {
        if self.machine.video.column != 0 {
            self.print("\n");
        }
        let text = format!("{}>", self.machine.drive.current_text());
        self.print(&text);
    }

    pub fn run_startup(&mut self, commands: &[String]) -> Result<(), Quit> {
        for command in commands {
            self.prompt();
            self.println(command);
            self.execute(command)?;
        }
        Ok(())
    }

    pub fn interactive(&mut self) -> Result<(), Quit> {
        loop {
            self.prompt();
            let line = match self.machine.read_line(LINE_LIMIT, &self.history) {
                Ok(line) => line,
                Err(Halt::Quit) => return Err(Quit),
                Err(_) => {
                    self.println("^C");
                    continue;
                }
            };
            self.print("\n");
            if !line.trim().is_empty() && self.history.last() != Some(&line) {
                self.history.push(line.clone());
                if self.history.len() > HISTORY_LIMIT {
                    self.history.remove(0);
                }
            }
            self.execute(&line)?;
        }
    }

    pub fn execute(&mut self, line: &str) -> Result<(), Quit> {
        let line = line.trim();
        let line = line.strip_prefix('@').unwrap_or(line);
        if line.is_empty() {
            return Ok(());
        }
        let (name, arguments) = split_command(line);
        if self.internal(&name, &arguments)? {
            return Ok(());
        }
        match name.as_str() {
            "MASM" | "ML" => self.masm(&arguments),
            "LINK" => self.link(&arguments),
            _ => self.launch(&name, &arguments),
        }
    }

    pub fn finish(&mut self) {
        self.machine.present();
        self.machine.finish();
    }
}
