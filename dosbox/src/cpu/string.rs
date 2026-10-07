use super::alu::CMP;
use super::flags::DF;
use super::registers::{AX, CX, DI, DS, ES, SI};
use super::{Cpu, Repeat};
use crate::hardware::memory::Memory;

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum StringOp {
    Movs,
    Cmps,
    Stos,
    Lods,
    Scas,
}

impl Cpu {
    pub(crate) fn string_instruction(&mut self, memory: &mut Memory, operation: StringOp, word: bool) {
        let compares = matches!(operation, StringOp::Cmps | StringOp::Scas);
        match self.repeat {
            None => self.string_once(memory, operation, word),
            Some(repeat) => loop {
                if self.regs.general[CX] == 0 {
                    break;
                }
                self.string_once(memory, operation, word);
                self.regs.general[CX] = self.regs.general[CX].wrapping_sub(1);
                if compares {
                    let zero = self.flags.zero();
                    if (repeat == Repeat::WhileEqual && !zero) || (repeat == Repeat::WhileNotEqual && zero) {
                        break;
                    }
                }
            },
        }
    }

    fn string_once(&mut self, memory: &mut Memory, operation: StringOp, word: bool) {
        let size: u16 = if word { 2 } else { 1 };
        let delta = if self.flags.get(DF) { size.wrapping_neg() } else { size };
        let source_segment = self.segment_for(DS);
        let destination_segment = self.regs.segment[ES];
        let si = self.regs.general[SI];
        let di = self.regs.general[DI];
        let read = |memory: &Memory, segment: u16, offset: u16| -> u32 {
            if word {
                memory.read16_at(segment, offset) as u32
            } else {
                memory.read8_at(segment, offset) as u32
            }
        };
        match operation {
            StringOp::Movs => {
                let value = read(memory, source_segment, si);
                write_value(memory, destination_segment, di, value, word);
                self.advance(SI, delta);
                self.advance(DI, delta);
            }
            StringOp::Cmps => {
                let left = read(memory, source_segment, si);
                let right = read(memory, destination_segment, di);
                self.arith(CMP, left, right, word);
                self.advance(SI, delta);
                self.advance(DI, delta);
            }
            StringOp::Stos => {
                let value = self.read_reg(AX, word);
                write_value(memory, destination_segment, di, value, word);
                self.advance(DI, delta);
            }
            StringOp::Lods => {
                let value = read(memory, source_segment, si);
                self.write_reg(AX, word, value);
                self.advance(SI, delta);
            }
            StringOp::Scas => {
                let left = self.read_reg(AX, word);
                let right = read(memory, destination_segment, di);
                self.arith(CMP, left, right, word);
                self.advance(DI, delta);
            }
        }
    }

    fn advance(&mut self, register: usize, delta: u16) {
        self.regs.general[register] = self.regs.general[register].wrapping_add(delta);
    }
}

fn write_value(memory: &mut Memory, segment: u16, offset: u16, value: u32, word: bool) {
    if word {
        memory.write16_at(segment, offset, value as u16);
    } else {
        memory.write8_at(segment, offset, value as u8);
    }
}
