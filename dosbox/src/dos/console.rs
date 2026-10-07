use crate::cpu::flags::ZF;
use crate::machine::{Halt, Machine};
use crate::terminal::keymap;

impl Machine {
    pub(crate) fn dos_console(&mut self) -> Result<(), Halt> {
        match self.cpu.regs.ah() {
            0x01 => {
                let character = self.read_character()?;
                self.video.teletype(&mut self.memory, character, None);
                self.cpu.regs.set_al(character);
            }
            0x02 => {
                let character = self.cpu.regs.dl();
                self.video.teletype(&mut self.memory, character, None);
                self.cpu.regs.set_al(character);
            }
            0x06 => {
                if self.cpu.regs.dl() == 0xFF {
                    self.direct_input()?;
                } else {
                    let character = self.cpu.regs.dl();
                    self.video.teletype(&mut self.memory, character, None);
                    self.cpu.regs.set_al(character);
                }
            }
            0x07 | 0x08 => {
                let character = self.read_character()?;
                self.cpu.regs.set_al(character);
            }
            0x09 => {
                let segment = self.cpu.regs.ds();
                let mut offset = self.cpu.regs.dx();
                for _ in 0..0xFFFF {
                    let character = self.memory.read8_at(segment, offset);
                    if character == b'$' {
                        break;
                    }
                    self.video.teletype(&mut self.memory, character, None);
                    offset = offset.wrapping_add(1);
                }
                self.cpu.regs.set_al(b'$');
            }
            0x0A => self.buffered_input()?,
            0x0B => {
                let ready = self.dos.pending_scan.is_some() || self.peek_key()?.is_some();
                self.cpu.regs.set_al(if ready { 0xFF } else { 0x00 });
            }
            _ => {
                self.keyboard.clear();
                self.dos.pending_scan = None;
                let function = self.cpu.regs.al();
                if matches!(function, 0x01 | 0x06 | 0x07 | 0x08 | 0x0A) {
                    self.cpu.regs.set_ah(function);
                    return self.dos_console();
                }
            }
        }
        Ok(())
    }

    fn read_character(&mut self) -> Result<u8, Halt> {
        if let Some(scan) = self.dos.pending_scan.take() {
            return Ok(scan);
        }
        let key = self.wait_key()?;
        if key.ascii == 0 || key.ascii == 0xE0 {
            self.dos.pending_scan = Some(key.scan);
            return Ok(0);
        }
        Ok(key.ascii)
    }

    fn direct_input(&mut self) -> Result<(), Halt> {
        if self.dos.pending_scan.is_none() && self.peek_key()?.is_none() {
            self.cpu.regs.set_al(0);
            self.cpu.flags.set(ZF, true);
            return Ok(());
        }
        let character = self.read_character()?;
        self.cpu.regs.set_al(character);
        self.cpu.flags.set(ZF, false);
        Ok(())
    }

    fn buffered_input(&mut self) -> Result<(), Halt> {
        let segment = self.cpu.regs.ds();
        let offset = self.cpu.regs.dx();
        let capacity = self.memory.read8_at(segment, offset) as usize;
        if capacity == 0 {
            return Ok(());
        }
        let line = self.read_line(capacity - 1, &[])?;
        let bytes: Vec<u8> = line.bytes().collect();
        self.memory.write8_at(segment, offset.wrapping_add(1), bytes.len() as u8);
        self.memory.write_bytes(segment, offset.wrapping_add(2), &bytes);
        self.memory.write8_at(segment, offset.wrapping_add(2 + bytes.len() as u16), 0x0D);
        self.video.teletype(&mut self.memory, b'\r', None);
        Ok(())
    }

    pub fn read_line(&mut self, limit: usize, history: &[String]) -> Result<String, Halt> {
        let mut line: Vec<u8> = Vec::new();
        let mut recall = history.len();
        loop {
            let key = self.wait_key()?;
            match key {
                _ if key == keymap::ENTER => return Ok(line.iter().map(|byte| *byte as char).collect()),
                _ if key == keymap::BACKSPACE => {
                    if line.pop().is_some() {
                        self.erase_characters(1);
                    }
                }
                _ if key == keymap::ESCAPE => {
                    self.erase_characters(line.len());
                    line.clear();
                }
                _ if (key == keymap::UP || key == keymap::DOWN) && !history.is_empty() => {
                    recall = if key == keymap::UP {
                        recall.saturating_sub(1)
                    } else {
                        (recall + 1).min(history.len())
                    };
                    self.erase_characters(line.len());
                    line = history.get(recall).map(|text| text.bytes().collect()).unwrap_or_default();
                    line.truncate(limit);
                    for byte in line.clone() {
                        self.video.teletype(&mut self.memory, byte, None);
                    }
                }
                _ if key.ascii >= 0x20 && key.ascii != 0x7F && key.ascii != 0xE0 => {
                    if line.len() < limit {
                        line.push(key.ascii);
                        self.video.teletype(&mut self.memory, key.ascii, None);
                    }
                }
                _ => {}
            }
        }
    }

    fn erase_characters(&mut self, count: usize) {
        for _ in 0..count {
            if self.video.column == 0 && self.video.row > 0 {
                let row = self.video.row - 1;
                self.video.set_cursor(&mut self.memory, row, crate::hardware::video::COLUMNS - 1);
                self.video.write_cell(&mut self.memory, row, crate::hardware::video::COLUMNS - 1, b' ', None);
                continue;
            }
            self.video.teletype(&mut self.memory, 0x08, None);
            let (row, column) = (self.video.row, self.video.column);
            self.video.write_cell(&mut self.memory, row, column, b' ', None);
        }
    }
}
