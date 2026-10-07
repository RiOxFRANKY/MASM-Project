use crate::hardware::video::{COLUMNS, ROWS};
use crate::machine::{Halt, Machine};

impl Machine {
    pub(crate) fn bios_video(&mut self) -> Result<(), Halt> {
        let regs = self.cpu.regs.clone();
        match regs.ah() {
            0x00 => self.video.set_mode(&mut self.memory, regs.al()),
            0x01 => self.video.set_shape(&mut self.memory, regs.ch(), regs.cl()),
            0x02 => self.video.set_cursor(&mut self.memory, regs.dh() as usize, regs.dl() as usize),
            0x03 => {
                self.cpu.regs.set_dh(self.video.row as u8);
                self.cpu.regs.set_dl(self.video.column as u8);
                self.cpu.regs.set_ch(self.video.cursor_start);
                self.cpu.regs.set_cl(self.video.cursor_end);
            }
            0x06 | 0x07 => {
                let lines = regs.al() as usize;
                let (top, left) = (regs.ch() as usize, regs.cl() as usize);
                let (bottom, right) = (regs.dh() as usize, regs.dl() as usize);
                if regs.ah() == 0x06 {
                    self.video.scroll_up(&mut self.memory, lines, top, left, bottom, right, regs.bh());
                } else {
                    self.video.scroll_down(&mut self.memory, lines, top, left, bottom, right, regs.bh());
                }
            }
            0x08 => {
                let (character, attribute) = self.video.read_cell(&self.memory, self.video.row, self.video.column);
                self.cpu.regs.set_al(character);
                self.cpu.regs.set_ah(attribute);
            }
            0x09 | 0x0A => {
                let attribute = if regs.ah() == 0x09 { Some(regs.bl()) } else { None };
                let start = self.video.row * COLUMNS + self.video.column;
                for index in 0..regs.cx() as usize {
                    let position = start + index;
                    if position >= COLUMNS * ROWS {
                        break;
                    }
                    self.video.write_cell(&mut self.memory, position / COLUMNS, position % COLUMNS, regs.al(), attribute);
                }
            }
            0x0E => self.video.teletype(&mut self.memory, regs.al(), None),
            0x0F => {
                self.cpu.regs.set_al(self.video.mode);
                self.cpu.regs.set_ah(COLUMNS as u8);
                self.cpu.regs.set_bh(0);
            }
            0x10 if regs.al() == 0x03 => self.video.blink = regs.bl() != 0,
            0x11 if regs.al() == 0x30 => {
                self.cpu.regs.set_dl((ROWS - 1) as u8);
                self.cpu.regs.set_cx(16);
            }
            0x12 if regs.bl() == 0x10 => {
                self.cpu.regs.set_bh(0);
                self.cpu.regs.set_bl(3);
            }
            0x13 => self.write_string(),
            0x1A => {
                self.cpu.regs.set_al(0x1A);
                self.cpu.regs.set_bl(0x08);
                self.cpu.regs.set_bh(0);
            }
            _ => {}
        }
        Ok(())
    }

    fn write_string(&mut self) {
        let regs = self.cpu.regs.clone();
        let with_attributes = regs.al() & 2 != 0;
        let update_cursor = regs.al() & 1 != 0;
        let (saved_row, saved_column) = (self.video.row, self.video.column);
        self.video.set_cursor(&mut self.memory, regs.dh() as usize, regs.dl() as usize);
        let mut offset = regs.general[crate::cpu::registers::BP];
        let segment = regs.es();
        for _ in 0..regs.cx() {
            let character = self.memory.read8_at(segment, offset);
            offset = offset.wrapping_add(1);
            let attribute = if with_attributes {
                let value = self.memory.read8_at(segment, offset);
                offset = offset.wrapping_add(1);
                value
            } else {
                regs.bl()
            };
            match character {
                0x07 | 0x08 | 0x0A | 0x0D => self.video.teletype(&mut self.memory, character, None),
                _ => self.video.teletype(&mut self.memory, character, Some(attribute)),
            }
        }
        if !update_cursor {
            self.video.set_cursor(&mut self.memory, saved_row, saved_column);
        }
    }
}
