use super::ansi::{AnsiRenderer, ENTER_SCREEN, LEAVE_SCREEN};
use super::{keymap, Frame, Input, Terminal};
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError};
use std::time::Duration;

const SEQUENCE_WAIT: Duration = Duration::from_millis(30);

pub struct UnixConsole {
    saved_mode: String,
    bytes: Receiver<u8>,
    renderer: AnsiRenderer,
}

fn stty(arguments: &[&str]) -> std::io::Result<String> {
    let output = Command::new("stty").args(arguments).stdin(Stdio::inherit()).output()?;
    if !output.status.success() {
        return Err(std::io::Error::other("stty failed: stdin is not a terminal"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

impl UnixConsole {
    pub fn open() -> std::io::Result<Self> {
        let saved_mode = stty(&["-g"])?;
        stty(&["raw", "-echo"])?;
        let (sender, bytes) = channel();
        std::thread::spawn(move || {
            let mut stdin = std::io::stdin();
            let mut buffer = [0u8; 64];
            while let Ok(count) = stdin.read(&mut buffer) {
                if count == 0 {
                    break;
                }
                for byte in &buffer[..count] {
                    if sender.send(*byte).is_err() {
                        return;
                    }
                }
            }
        });
        print!("{ENTER_SCREEN}");
        let _ = std::io::stdout().flush();
        Ok(UnixConsole { saved_mode, bytes, renderer: AnsiRenderer::new() })
    }

    fn read_escape(&mut self) -> Option<Input> {
        let introducer = match self.bytes.recv_timeout(SEQUENCE_WAIT) {
            Ok(byte) => byte,
            Err(_) => return Some(Input::Key(keymap::ESCAPE)),
        };
        if introducer != b'[' && introducer != b'O' {
            return Some(Input::Key(keymap::from_char(introducer as char)));
        }
        let mut body = String::new();
        loop {
            let byte = self.bytes.recv_timeout(SEQUENCE_WAIT).ok()?;
            if (0x40..=0x7E).contains(&byte) {
                body.push(byte as char);
                break;
            }
            body.push(byte as char);
        }
        let key = match body.as_str() {
            "A" => keymap::UP,
            "B" => keymap::DOWN,
            "C" => keymap::RIGHT,
            "D" => keymap::LEFT,
            "H" | "1~" => keymap::HOME,
            "F" | "4~" => keymap::END,
            "2~" => keymap::INSERT,
            "3~" => keymap::DELETE,
            "5~" => keymap::PAGE_UP,
            "6~" => keymap::PAGE_DOWN,
            "P" => keymap::function(1),
            "Q" => keymap::function(2),
            "R" => keymap::function(3),
            "S" => keymap::function(4),
            "15~" => keymap::function(5),
            "17~" => keymap::function(6),
            "18~" => keymap::function(7),
            "19~" => keymap::function(8),
            "20~" => keymap::function(9),
            "21~" => keymap::function(10),
            "23~" => keymap::function(11),
            "24~" => keymap::function(12),
            "20;5~" => return Some(Input::Quit),
            _ => return None,
        };
        Some(Input::Key(key))
    }
}

impl Terminal for UnixConsole {
    fn present(&mut self, frame: &Frame) {
        let text = self.renderer.render(frame);
        let mut stdout = std::io::stdout().lock();
        let _ = stdout.write_all(text.as_bytes());
        let _ = stdout.flush();
    }

    fn poll(&mut self, timeout: Duration) -> Option<Input> {
        let byte = match self.bytes.recv_timeout(timeout) {
            Ok(byte) => byte,
            Err(RecvTimeoutError::Timeout) => return None,
            Err(RecvTimeoutError::Disconnected) => return Some(Input::Quit),
        };
        match byte {
            0x1B => self.read_escape(),
            0x03 => Some(Input::Break),
            0x1C => Some(Input::Quit),
            0x0D | 0x0A => Some(Input::Key(keymap::ENTER)),
            0x7F | 0x08 => Some(Input::Key(keymap::BACKSPACE)),
            0x01..=0x1A => Some(Input::Key(keymap::control(byte + 0x60))),
            0x80..=0xFF => None,
            _ => Some(Input::Key(keymap::from_char(byte as char))),
        }
    }
}

impl Drop for UnixConsole {
    fn drop(&mut self) {
        print!("{LEAVE_SCREEN}");
        let _ = std::io::stdout().flush();
        let _ = stty(&[self.saved_mode.as_str()]);
    }
}
