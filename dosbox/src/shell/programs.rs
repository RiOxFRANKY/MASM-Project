use super::{Quit, Shell};
use crate::machine::Halt;
use std::fs;
use std::path::PathBuf;

const EXTENSIONS: [&str; 3] = ["COM", "EXE", "BAT"];

impl Shell {
    fn find_program(&self, name: &str) -> Option<(PathBuf, String)> {
        let has_extension = name.rsplit('\\').next().is_some_and(|file| file.contains('.'));
        let candidates: Vec<String> = if has_extension {
            vec![name.to_string()]
        } else {
            EXTENSIONS.iter().map(|extension| format!("{name}.{extension}")).collect()
        };
        let roots = ["".to_string(), "C:\\".to_string()];
        for root in &roots {
            for candidate in &candidates {
                let dos_name = if candidate.contains('\\') { candidate.clone() } else { format!("{root}{candidate}") };
                if let Ok(path) = self.machine.drive.resolve(&dos_name) {
                    if path.is_file() {
                        return Some((path, candidate.to_ascii_uppercase()));
                    }
                }
            }
        }
        None
    }

    pub(super) fn launch(&mut self, name: &str, arguments: &str) -> Result<(), Quit> {
        let Some((path, file_name)) = self.find_program(name) else {
            self.println("Bad command or file name");
            return Ok(());
        };
        if file_name.ends_with(".BAT") {
            return self.run_batch(&path, arguments);
        }
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(_) => {
                self.println("Unable to read program file");
                return Ok(());
            }
        };
        let full_name = format!("{}\\{}", self.machine.drive.current_text().trim_end_matches('\\'), file_name);
        if let Err(message) = self.machine.load_program(&bytes, &full_name, arguments) {
            self.println(&message);
            return Ok(());
        }
        let outcome = self.machine.run();
        self.machine.dos.files.close_all();
        self.machine.keyboard.clear();
        if self.machine.video.mode != 3 && self.machine.video.mode != 2 && self.machine.video.mode != 7 {
            let memory = &mut self.machine.memory;
            self.machine.video.set_mode(memory, 3);
        }
        match outcome {
            Halt::Exit(code) => self.machine.dos.return_code = code,
            Halt::Break => {
                self.machine.dos.return_code = 0;
                self.print("\n^C\n");
            }
            Halt::Quit => return Err(Quit),
            Halt::Fault(message) => {
                self.print("\n");
                self.println(&message);
            }
        }
        Ok(())
    }
}
