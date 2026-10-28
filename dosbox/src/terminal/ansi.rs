use super::cp437::to_unicode;
use super::palette::escape_for;
use super::Frame;
use std::fmt::Write;

pub const ENTER_SCREEN: &str = "\x1b[?1049h\x1b[8;30;100t\x1b[?25l\x1b[2J";
pub const LEAVE_SCREEN: &str = "\x1b[0m\x1b[?25h\x1b[?1049l";

pub struct AnsiRenderer {
    previous: Vec<u8>,
    blink: bool,
    pub row_offset: usize,
    pub col_offset: usize,
}

impl Default for AnsiRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl AnsiRenderer {
    #[allow(dead_code)]
    pub fn new() -> Self {
        AnsiRenderer { previous: Vec::new(), blink: true, row_offset: 0, col_offset: 0 }
    }

    pub fn with_offset(row_offset: usize, col_offset: usize) -> Self {
        AnsiRenderer { previous: Vec::new(), blink: true, row_offset, col_offset }
    }

    pub fn render(&mut self, frame: &Frame) -> String {
        let mut output = String::new();
        let full = self.previous.len() != frame.cells.len() || self.blink != frame.blink;
        self.blink = frame.blink;
        let mut current_attribute: Option<u8> = None;
        let mut position: Option<(usize, usize)> = None;
        output.push_str("\x1b[?25l");
        for row in 0..frame.rows {
            for column in 0..frame.columns {
                let index = (row * frame.columns + column) * 2;
                let character = frame.cells[index];
                let attribute = frame.cells[index + 1];
                if !full && self.previous[index] == character && self.previous[index + 1] == attribute {
                    continue;
                }
                if position != Some((row, column)) {
                    let _ = write!(output, "\x1b[{};{}H", row + 1 + self.row_offset, column + 1 + self.col_offset);
                }
                if current_attribute != Some(attribute) {
                    output.push_str(&escape_for(attribute, frame.blink));
                    current_attribute = Some(attribute);
                }
                output.push(to_unicode(character));
                position = Some((row, column + 1));
            }
        }
        output.push_str("\x1b[0m");
        let _ = write!(output, "\x1b[{};{}H", frame.cursor_row + 1 + self.row_offset, frame.cursor_column + 1 + self.col_offset);
        if frame.cursor_visible {
            output.push_str("\x1b[?25h");
        }
        self.previous = frame.cells.to_vec();
        output
    }
}
