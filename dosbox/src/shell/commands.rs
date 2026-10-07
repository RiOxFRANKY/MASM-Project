use super::{Quit, Shell};
use crate::dos::filesystem::wildcard_match;
use crate::hardware::clock;
use std::fs;

const HELP: &str = "\
Internal commands:
  DIR [pattern]      list files            CD [dir]       change directory
  TYPE file          show a text file      CLS            clear the screen
  DEL file           delete files          REN old new    rename a file
  MD dir / RD dir    make / remove dir     ECHO text      print text
  VER                show version          EXIT           quit the emulator
Tools:
  MASM file[.ASM][,obj][;]                 assemble to an .OBJ file
  LINK file[.OBJ][,exe][,map][;] [/T] [/M] link to .EXE (/T makes a .COM)
Programs:
  name               run name.COM, name.EXE or name.BAT
Keys: Ctrl+C stops the running program, Ctrl+F9 quits the emulator.";

impl Shell {
    pub(super) fn internal(&mut self, name: &str, arguments: &str) -> Result<bool, Quit> {
        let argument = arguments.trim();
        match name {
            "EXIT" => return Err(Quit),
            "CLS" => {
                let memory = &mut self.machine.memory;
                self.machine.video.set_mode(memory, 3);
            }
            "VER" => self.println("\nDOSBox-RS version 0.1 (reports DOS 5.0)\n"),
            "HELP" | "?" => self.println(HELP),
            "ECHO" => match argument.to_ascii_uppercase().as_str() {
                "" => {
                    let state = if self.echo { "ON" } else { "OFF" };
                    self.println(&format!("ECHO is {state}"));
                }
                "OFF" => self.echo = false,
                "ON" => self.echo = true,
                _ => self.println(argument.strip_prefix('.').unwrap_or(argument)),
            },
            "REM" => {}
            "C:" => {}
            _ if name.len() == 2 && name.ends_with(':') => self.println("Invalid drive specification"),
            "CD" | "CHDIR" => {
                if argument.is_empty() {
                    let current = self.machine.drive.current_text();
                    self.println(&current);
                } else if self.machine.drive.change_directory(argument).is_err() {
                    self.println("Invalid directory");
                }
            }
            "MD" | "MKDIR" => match self.machine.drive.resolve(argument) {
                Ok(path) if !argument.is_empty() && fs::create_dir(&path).is_ok() => {}
                _ => self.println("Unable to create directory"),
            },
            "RD" | "RMDIR" => match self.machine.drive.resolve(argument) {
                Ok(path) if !argument.is_empty() && fs::remove_dir(&path).is_ok() => {}
                _ => self.println("Invalid path, not directory, or directory not empty"),
            },
            "TYPE" => self.type_file(argument),
            "DEL" | "ERASE" => self.delete(argument),
            "REN" | "RENAME" => self.rename(argument),
            "DIR" => self.directory(argument),
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn type_file(&mut self, argument: &str) {
        let content = self.machine.drive.resolve(argument).ok().and_then(|path| fs::read(path).ok());
        match content {
            Some(bytes) => {
                for byte in bytes {
                    if byte == 0x1A {
                        break;
                    }
                    let memory = &mut self.machine.memory;
                    self.machine.video.teletype(memory, byte, None);
                }
                self.print("\n");
            }
            None => self.println("File not found"),
        }
    }

    fn split_pattern(argument: &str) -> (String, String) {
        let normalized = argument.replace('/', "\\");
        match normalized.rfind(['\\', ':']) {
            Some(position) => (normalized[..=position].to_string(), normalized[position + 1..].to_string()),
            None => (String::new(), normalized),
        }
    }

    fn delete(&mut self, argument: &str) {
        if argument.is_empty() {
            self.println("Required parameter missing");
            return;
        }
        let (directory, mask) = Self::split_pattern(argument);
        let listing = self.machine.drive.list(if directory.is_empty() { "." } else { &directory });
        let mut removed = 0;
        for entry in listing.unwrap_or_default() {
            if entry.is_directory || !wildcard_match(&mask, &entry.name) {
                continue;
            }
            let target = format!("{directory}{}", entry.name);
            if let Ok(path) = self.machine.drive.resolve(&target) {
                if fs::remove_file(path).is_ok() {
                    removed += 1;
                }
            }
        }
        if removed == 0 {
            self.println("File not found");
        }
    }

    fn rename(&mut self, argument: &str) {
        let parts: Vec<&str> = argument.split_whitespace().collect();
        if parts.len() != 2 {
            self.println("Required parameter missing");
            return;
        }
        let from = self.machine.drive.resolve(parts[0]);
        let to = self.machine.drive.resolve(parts[1]);
        match (from, to) {
            (Ok(from), Ok(to)) if from.exists() && !to.exists() && fs::rename(&from, &to).is_ok() => {}
            _ => self.println("Duplicate file name or file not found"),
        }
    }

    fn directory(&mut self, argument: &str) {
        let argument = argument.split('/').next().unwrap_or("").trim();
        let (directory, mut mask) = Self::split_pattern(argument);
        let mut directory = directory;
        if !mask.is_empty() && !mask.contains(['*', '?']) {
            let candidate = format!("{directory}{mask}");
            if self.machine.drive.resolve(&candidate).is_ok_and(|path| path.is_dir()) {
                directory = format!("{candidate}\\");
                mask.clear();
            }
        }
        if mask.is_empty() {
            mask = "*.*".to_string();
        }
        let listing = match self.machine.drive.list(if directory.is_empty() { "." } else { &directory }) {
            Ok(listing) => listing,
            Err(_) => {
                self.println("Path not found");
                return;
            }
        };
        let shown = self.machine.drive.current_text();
        self.println(&format!("\n Directory of {shown}\n"));
        let (mut files, mut folders, mut bytes) = (0, 0, 0u64);
        for entry in listing.iter().filter(|entry| wildcard_match(&mask, &entry.name)) {
            let (base, extension) = entry.name.split_once('.').unwrap_or((&entry.name, ""));
            let stamp = entry.modified.map(clock::of_system_time).unwrap_or_else(clock::now);
            let size = if entry.is_directory { "<DIR>    ".to_string() } else { format!("{:>9}", entry.size) };
            let line = format!(
                "{:<8} {:<3} {} {:02}-{:02}-{:04} {:02}:{:02}",
                base, extension, size, stamp.month, stamp.day, stamp.year, stamp.hour, stamp.minute
            );
            self.println(&line);
            if entry.is_directory {
                folders += 1;
            } else {
                files += 1;
                bytes += entry.size;
            }
        }
        self.println(&format!("{files:>9} file(s) {bytes:>12} bytes"));
        self.println(&format!("{folders:>9} dir(s)"));
    }
}
