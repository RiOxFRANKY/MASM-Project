mod ansi;
pub mod cp437;
pub mod headless;
pub mod keymap;
mod palette;
mod screenshot;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Key {
    pub scan: u8,
    pub ascii: u8,
}

impl Key {
    pub fn code(self) -> u16 {
        ((self.scan as u16) << 8) | self.ascii as u16
    }
}

pub enum Input {
    Key(Key),
    Break,
    Quit,
}

pub struct Frame<'a> {
    pub cells: &'a [u8],
    pub columns: usize,
    pub rows: usize,
    pub cursor_row: usize,
    pub cursor_column: usize,
    pub cursor_visible: bool,
    pub blink: bool,
}

pub trait Terminal {
    fn present(&mut self, frame: &Frame);
    fn poll(&mut self, timeout: Duration) -> Option<Input>;
    fn finish(&mut self, _frame: &Frame) {}
}

pub fn open_console() -> std::io::Result<Box<dyn Terminal>> {
    #[cfg(windows)]
    {
        Ok(Box::new(windows::WindowsConsole::open()?))
    }
    #[cfg(unix)]
    {
        Ok(Box::new(unix::UnixConsole::open()?))
    }
}
