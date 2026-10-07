mod arena;
mod console;
mod directory;
mod files;
pub mod filesystem;
pub mod loader;
mod system;

use crate::cpu::flags::CF;
use crate::machine::{Halt, Machine};
pub use arena::Arena;
pub use files::FileTable;

pub const ERROR_INVALID_FUNCTION: u16 = 0x01;
pub const ERROR_FILE_NOT_FOUND: u16 = 0x02;
pub const ERROR_PATH_NOT_FOUND: u16 = 0x03;
pub const ERROR_TOO_MANY_FILES: u16 = 0x04;
pub const ERROR_ACCESS_DENIED: u16 = 0x05;
pub const ERROR_INVALID_HANDLE: u16 = 0x06;
pub const ERROR_NOT_ENOUGH_MEMORY: u16 = 0x08;
pub const ERROR_NO_MORE_FILES: u16 = 0x12;

pub struct Dos {
    pub psp: u16,
    pub dta: (u16, u16),
    pub return_code: u8,
    pub pending_scan: Option<u8>,
    pub files: FileTable,
    pub arena: Arena,
    pub search: Vec<directory::Found>,
}

impl Dos {
    pub fn new() -> Self {
        Dos {
            psp: 0,
            dta: (0, 0x80),
            return_code: 0,
            pending_scan: None,
            files: FileTable::new(),
            arena: Arena::new(0, 0),
            search: Vec::new(),
        }
    }
}

impl Machine {
    pub(crate) fn dos_service(&mut self) -> Result<(), Halt> {
        match self.cpu.regs.ah() {
            0x01 | 0x02 | 0x06 | 0x07 | 0x08 | 0x09 | 0x0A | 0x0B | 0x0C => self.dos_console(),
            0x39 | 0x3A | 0x3B | 0x3C | 0x3D | 0x3E | 0x3F | 0x40 | 0x41 | 0x42 | 0x43 | 0x44 | 0x45 | 0x47 | 0x56 => {
                self.dos_files()
            }
            0x4E | 0x4F => self.dos_find(),
            0x48 | 0x49 | 0x4A => self.dos_memory(),
            _ => self.dos_system(),
        }
    }

    pub(crate) fn dos_ok(&mut self) {
        self.cpu.flags.set(CF, false);
    }

    pub(crate) fn dos_error(&mut self, code: u16) {
        self.cpu.regs.set_ax(code);
        self.cpu.flags.set(CF, true);
    }
}
