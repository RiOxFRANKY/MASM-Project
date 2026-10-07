use super::flags::{IF, TF};
use super::registers::{CS, SP, SS};
use super::Cpu;
use crate::hardware::memory::Memory;

impl Cpu {
    pub fn push16(&mut self, memory: &mut Memory, value: u16) {
        let sp = self.regs.general[SP].wrapping_sub(2);
        self.regs.general[SP] = sp;
        memory.write16_at(self.regs.segment[SS], sp, value);
    }

    pub fn pop16(&mut self, memory: &Memory) -> u16 {
        let sp = self.regs.general[SP];
        let value = memory.read16_at(self.regs.segment[SS], sp);
        self.regs.general[SP] = sp.wrapping_add(2);
        value
    }

    pub fn enter_interrupt(&mut self, memory: &mut Memory, vector: u8) {
        let flags = self.flags.bits();
        self.push16(memory, flags);
        let cs = self.regs.segment[CS];
        self.push16(memory, cs);
        let ip = self.regs.ip;
        self.push16(memory, ip);
        self.flags.set(IF, false);
        self.flags.set(TF, false);
        let offset = memory.read16(vector as u32 * 4);
        let segment = memory.read16(vector as u32 * 4 + 2);
        self.jump_far(segment, offset);
    }

    pub fn return_from_interrupt(&mut self, memory: &Memory) {
        let ip = self.pop16(memory);
        let cs = self.pop16(memory);
        let flags = self.pop16(memory);
        self.jump_far(cs, ip);
        self.flags.load(flags);
    }

    pub fn return_from_interrupt_keeping(&mut self, memory: &Memory, keep_mask: u16) {
        let current = self.flags.bits();
        self.return_from_interrupt(memory);
        let merged = (self.flags.bits() & !keep_mask) | (current & keep_mask);
        self.flags.load(merged);
    }
}
