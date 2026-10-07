use super::ERROR_NOT_ENOUGH_MEMORY;
use crate::machine::{Halt, Machine};

pub struct Arena {
    owner: u16,
    owner_end: u16,
    next_free: u16,
    top: u16,
}

impl Arena {
    pub fn new(owner: u16, top: u16) -> Self {
        Arena { owner, owner_end: top, next_free: top, top }
    }

    fn resize(&mut self, block: u16, paragraphs: u16) -> Result<(), u16> {
        if block != self.owner {
            return Ok(());
        }
        let limit = self.top - self.owner;
        if paragraphs > limit {
            return Err(limit);
        }
        self.owner_end = self.owner + paragraphs;
        self.next_free = self.next_free.max(self.owner_end);
        if self.next_free == self.top {
            self.next_free = self.owner_end;
        }
        Ok(())
    }

    fn allocate(&mut self, paragraphs: u16) -> Result<u16, u16> {
        let start = self.next_free.max(self.owner_end);
        let available = self.top.saturating_sub(start);
        if paragraphs > available {
            return Err(available);
        }
        self.next_free = start + paragraphs;
        Ok(start)
    }
}

impl Machine {
    pub(crate) fn dos_memory(&mut self) -> Result<(), Halt> {
        let paragraphs = self.cpu.regs.bx();
        let result = match self.cpu.regs.ah() {
            0x48 => self.dos.arena.allocate(paragraphs).map(Some),
            0x49 => Ok(None),
            _ => {
                let block = self.cpu.regs.es();
                self.dos.arena.resize(block, paragraphs).map(|_| None)
            }
        };
        match result {
            Ok(Some(segment)) => {
                self.cpu.regs.set_ax(segment);
                self.dos_ok();
            }
            Ok(None) => self.dos_ok(),
            Err(largest) => {
                self.dos_error(ERROR_NOT_ENOUGH_MEMORY);
                self.cpu.regs.set_bx(largest);
            }
        }
        Ok(())
    }
}
