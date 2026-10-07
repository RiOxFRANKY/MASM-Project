use super::cp437::to_unicode;
use super::{keymap, screenshot, Frame, Input, Key, Terminal};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Duration;

const IDLE_POLLS_BEFORE_QUIT: u32 = 500;
const BLOCKING_WAIT: Duration = Duration::from_millis(50);

pub struct HeadlessTerminal {
    script: VecDeque<Key>,
    screenshot: Option<PathBuf>,
    idle_polls: u32,
}

impl HeadlessTerminal {
    pub fn new(script: &str, screenshot: Option<PathBuf>) -> Result<Self, String> {
        Ok(HeadlessTerminal { script: parse_script(script)?.into(), screenshot, idle_polls: 0 })
    }
}

pub fn parse_script(script: &str) -> Result<Vec<Key>, String> {
    let mut keys = Vec::new();
    let mut characters = script.chars();
    while let Some(character) = characters.next() {
        if character != '{' {
            keys.push(keymap::from_char(character));
            continue;
        }
        let name: String = characters.by_ref().take_while(|next| *next != '}').collect();
        let key = keymap::named(&name).ok_or_else(|| format!("unknown key name {{{name}}}"))?;
        keys.push(key);
    }
    Ok(keys)
}

impl Terminal for HeadlessTerminal {
    fn present(&mut self, _frame: &Frame) {}

    fn poll(&mut self, timeout: Duration) -> Option<Input> {
        if let Some(key) = self.script.pop_front() {
            self.idle_polls = 0;
            return Some(Input::Key(key));
        }
        if timeout >= BLOCKING_WAIT {
            return Some(Input::Quit);
        }
        if !timeout.is_zero() {
            std::thread::sleep(timeout);
            self.idle_polls = 0;
            return None;
        }
        self.idle_polls += 1;
        if self.idle_polls < IDLE_POLLS_BEFORE_QUIT {
            return None;
        }
        Some(Input::Quit)
    }

    fn finish(&mut self, frame: &Frame) {
        let mut lines: Vec<String> = (0..frame.rows)
            .map(|row| {
                let line: String = (0..frame.columns)
                    .map(|column| to_unicode(frame.cells[(row * frame.columns + column) * 2]))
                    .collect();
                line.trim_end().to_string()
            })
            .collect();
        while lines.last().is_some_and(|line| line.is_empty()) {
            lines.pop();
        }
        println!("{}", lines.join("\n"));
        if let Some(path) = &self.screenshot {
            if let Err(error) = std::fs::write(path, screenshot::html(frame)) {
                eprintln!("dosbox: cannot write screenshot {}: {error}", path.display());
            }
        }
    }
}
