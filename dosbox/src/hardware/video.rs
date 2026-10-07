use super::memory::Memory;

pub const TEXT_BASE: u32 = 0xB8000;
pub const COLUMNS: usize = 80;
pub const ROWS: usize = 25;
pub const DEFAULT_ATTRIBUTE: u8 = 0x07;

const BDA_MODE: u32 = 0x449;
const BDA_COLUMNS: u32 = 0x44A;
const BDA_PAGE_SIZE: u32 = 0x44C;
const BDA_CURSOR: u32 = 0x450;
const BDA_CURSOR_SHAPE: u32 = 0x460;
const BDA_PAGE: u32 = 0x462;
const BDA_CRTC: u32 = 0x463;
const BDA_ROWS: u32 = 0x484;

pub struct Video {
    pub row: usize,
    pub column: usize,
    pub mode: u8,
    pub cursor_start: u8,
    pub cursor_end: u8,
    pub blink: bool,
}

impl Video {
    pub fn new() -> Self {
        Video { row: 0, column: 0, mode: 3, cursor_start: 6, cursor_end: 7, blink: true }
    }

    fn address(row: usize, column: usize) -> u32 {
        TEXT_BASE + ((row * COLUMNS + column) * 2) as u32
    }

    pub fn cursor_visible(&self) -> bool {
        self.cursor_start & 0x20 == 0 && self.cursor_start <= self.cursor_end
    }

    pub fn read_cell(&self, memory: &Memory, row: usize, column: usize) -> (u8, u8) {
        let address = Self::address(row, column);
        (memory.read8(address), memory.read8(address + 1))
    }

    pub fn write_cell(&self, memory: &mut Memory, row: usize, column: usize, character: u8, attribute: Option<u8>) {
        if row >= ROWS || column >= COLUMNS {
            return;
        }
        let address = Self::address(row, column);
        memory.write8(address, character);
        if let Some(attribute) = attribute {
            memory.write8(address + 1, attribute);
        }
    }

    pub fn set_mode(&mut self, memory: &mut Memory, mode: u8) {
        self.mode = mode & 0x7F;
        if mode & 0x80 == 0 {
            self.scroll_up(memory, 0, 0, 0, ROWS - 1, COLUMNS - 1, DEFAULT_ATTRIBUTE);
        }
        self.cursor_start = 6;
        self.cursor_end = 7;
        self.blink = true;
        self.set_cursor(memory, 0, 0);
        memory.write8(BDA_MODE, self.mode);
        memory.write16(BDA_COLUMNS, COLUMNS as u16);
        memory.write16(BDA_PAGE_SIZE, (COLUMNS * ROWS * 2) as u16);
        memory.write8(BDA_PAGE, 0);
        memory.write16(BDA_CRTC, 0x3D4);
        memory.write8(BDA_ROWS, (ROWS - 1) as u8);
        self.sync_shape(memory);
    }

    pub fn set_cursor(&mut self, memory: &mut Memory, row: usize, column: usize) {
        self.row = row;
        self.column = column;
        memory.write8(BDA_CURSOR, column as u8);
        memory.write8(BDA_CURSOR + 1, row as u8);
    }

    pub fn set_shape(&mut self, memory: &mut Memory, start: u8, end: u8) {
        self.cursor_start = start;
        self.cursor_end = end;
        self.sync_shape(memory);
    }

    fn sync_shape(&self, memory: &mut Memory) {
        memory.write8(BDA_CURSOR_SHAPE, self.cursor_end);
        memory.write8(BDA_CURSOR_SHAPE + 1, self.cursor_start);
    }

    pub fn scroll_up(&self, memory: &mut Memory, lines: usize, top: usize, left: usize, bottom: usize, right: usize, attribute: u8) {
        let bottom = bottom.min(ROWS - 1);
        let right = right.min(COLUMNS - 1);
        if top > bottom || left > right {
            return;
        }
        let height = bottom - top + 1;
        let lines = if lines == 0 || lines > height { height } else { lines };
        for row in top..=bottom {
            for column in left..=right {
                let (character, cell_attribute) = if row + lines <= bottom {
                    self.read_cell(memory, row + lines, column)
                } else {
                    (b' ', attribute)
                };
                self.write_cell(memory, row, column, character, Some(cell_attribute));
            }
        }
    }

    pub fn scroll_down(&self, memory: &mut Memory, lines: usize, top: usize, left: usize, bottom: usize, right: usize, attribute: u8) {
        let bottom = bottom.min(ROWS - 1);
        let right = right.min(COLUMNS - 1);
        if top > bottom || left > right {
            return;
        }
        let height = bottom - top + 1;
        let lines = if lines == 0 || lines > height { height } else { lines };
        for row in (top..=bottom).rev() {
            for column in left..=right {
                let (character, cell_attribute) = if row >= top + lines {
                    self.read_cell(memory, row - lines, column)
                } else {
                    (b' ', attribute)
                };
                self.write_cell(memory, row, column, character, Some(cell_attribute));
            }
        }
    }

    pub fn teletype(&mut self, memory: &mut Memory, character: u8, attribute: Option<u8>) {
        let (mut row, mut column) = (self.row, self.column);
        match character {
            0x07 => {}
            0x08 => column = column.saturating_sub(1),
            0x0A => row += 1,
            0x0D => column = 0,
            0x09 => {
                let target = (column / 8 + 1) * 8;
                while column < target && column < COLUMNS {
                    self.write_cell(memory, row, column, b' ', attribute);
                    column += 1;
                }
            }
            _ => {
                self.write_cell(memory, row, column, character, attribute);
                column += 1;
            }
        }
        if column >= COLUMNS {
            column = 0;
            row += 1;
        }
        if row >= ROWS {
            self.scroll_up(memory, 1, 0, 0, ROWS - 1, COLUMNS - 1, DEFAULT_ATTRIBUTE);
            row = ROWS - 1;
        }
        self.set_cursor(memory, row, column);
    }

    pub fn print(&mut self, memory: &mut Memory, text: &str) {
        for character in text.chars() {
            if character == '\n' {
                self.teletype(memory, b'\r', None);
            }
            self.teletype(memory, crate::terminal::cp437::from_unicode(character), None);
        }
    }

    pub fn text_cells<'a>(&self, memory: &'a Memory) -> &'a [u8] {
        memory.slice(TEXT_BASE, COLUMNS * ROWS * 2)
    }
}
