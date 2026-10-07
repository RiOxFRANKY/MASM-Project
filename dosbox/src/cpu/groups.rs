use super::registers::CS;
use super::{Cpu, Fault, Step};
use crate::hardware::memory::Memory;

impl Cpu {
    pub(crate) fn group_immediate(&mut self, memory: &mut Memory, opcode: u8) -> Result<Step, Fault> {
        let word = opcode & 1 == 1;
        let modrm = self.decode_modrm(memory);
        let right = match opcode {
            0x81 => self.fetch16(memory) as u32,
            0x83 => self.fetch_signed8(memory) as u32,
            _ => self.fetch8(memory) as u32,
        };
        let left = self.read_rm(memory, &modrm, word);
        let operation = modrm.reg as u8;
        let result = self.arith(operation, left, right, word);
        if operation != super::alu::CMP {
            self.write_rm(memory, &modrm, word, result);
        }
        Ok(Step::Continue)
    }

    pub(crate) fn group_shift(&mut self, memory: &mut Memory, opcode: u8) -> Result<Step, Fault> {
        let word = opcode & 1 == 1;
        let modrm = self.decode_modrm(memory);
        let count = match opcode {
            0xC0 | 0xC1 => self.fetch8(memory),
            0xD0 | 0xD1 => 1,
            _ => self.regs.cl(),
        };
        let value = self.read_rm(memory, &modrm, word);
        let result = self.shift(modrm.reg as u8, value, count, word);
        self.write_rm(memory, &modrm, word, result);
        Ok(Step::Continue)
    }

    pub(crate) fn group_unary(&mut self, memory: &mut Memory, opcode: u8) -> Result<Step, Fault> {
        let word = opcode & 1 == 1;
        let modrm = self.decode_modrm(memory);
        let value = self.read_rm(memory, &modrm, word);
        match modrm.reg {
            0 | 1 => {
                let immediate = if word { self.fetch16(memory) as u32 } else { self.fetch8(memory) as u32 };
                self.logic(value & immediate, word);
            }
            2 => self.write_rm(memory, &modrm, word, !value),
            3 => {
                let result = self.negate(value, word);
                self.write_rm(memory, &modrm, word, result);
            }
            4 => self.multiply(value, word, false),
            5 => self.multiply(value, word, true),
            6 => {
                if self.divide(value, word, false).is_err() {
                    return Ok(Step::Interrupt(0));
                }
            }
            _ => {
                if self.divide(value, word, true).is_err() {
                    return Ok(Step::Interrupt(0));
                }
            }
        }
        Ok(Step::Continue)
    }

    pub(crate) fn group_increment(&mut self, memory: &mut Memory, opcode: u8) -> Result<Step, Fault> {
        let modrm = self.decode_modrm(memory);
        let value = self.read_rm8(memory, &modrm) as u32;
        let result = match modrm.reg {
            0 => self.increment(value, false),
            1 => self.decrement(value, false),
            _ => return Err(self.fault(opcode)),
        };
        self.write_rm8(memory, &modrm, result as u8);
        Ok(Step::Continue)
    }

    pub(crate) fn group_misc(&mut self, memory: &mut Memory, opcode: u8) -> Result<Step, Fault> {
        let modrm = self.decode_modrm(memory);
        match modrm.reg {
            0 => {
                let value = self.read_rm16(memory, &modrm) as u32;
                let result = self.increment(value, true);
                self.write_rm16(memory, &modrm, result as u16);
            }
            1 => {
                let value = self.read_rm16(memory, &modrm) as u32;
                let result = self.decrement(value, true);
                self.write_rm16(memory, &modrm, result as u16);
            }
            2 => {
                let target = self.read_rm16(memory, &modrm);
                let ip = self.regs.ip;
                self.push16(memory, ip);
                self.regs.ip = target;
            }
            3 => {
                if modrm.is_register() {
                    return Err(self.fault(opcode));
                }
                let offset = memory.read16_at(modrm.segment, modrm.offset);
                let segment = memory.read16_at(modrm.segment, modrm.offset.wrapping_add(2));
                let cs = self.regs.segment[CS];
                self.push16(memory, cs);
                let ip = self.regs.ip;
                self.push16(memory, ip);
                self.jump_far(segment, offset);
            }
            4 => self.regs.ip = self.read_rm16(memory, &modrm),
            5 => {
                if modrm.is_register() {
                    return Err(self.fault(opcode));
                }
                let offset = memory.read16_at(modrm.segment, modrm.offset);
                let segment = memory.read16_at(modrm.segment, modrm.offset.wrapping_add(2));
                self.jump_far(segment, offset);
            }
            6 => {
                let value = self.read_rm16(memory, &modrm);
                self.push16(memory, value);
            }
            _ => return Err(self.fault(opcode)),
        }
        Ok(Step::Continue)
    }
}
