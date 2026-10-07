use super::alu::CMP;
use super::flags::{AF, CF, DF, IF, OF, PF, SF, ZF};
use super::registers::{AX, BP, CS, CX, DS, DX, SP};
use super::string::StringOp;
use super::{Cpu, Fault, PortBus, Step};
use crate::hardware::memory::Memory;

impl Cpu {
    pub(crate) fn execute(&mut self, opcode: u8, memory: &mut Memory, ports: &mut dyn PortBus) -> Result<Step, Fault> {
        match opcode {
            0x00..=0x3F if opcode & 7 < 6 => self.arith_form(memory, opcode),
            0x06 | 0x0E | 0x16 | 0x1E => {
                let value = self.regs.segment[(opcode >> 3) as usize];
                self.push16(memory, value);
            }
            0x07 | 0x17 | 0x1F => {
                let value = self.pop16(memory);
                self.regs.segment[(opcode >> 3) as usize] = value;
            }
            0x27 => self.daa(),
            0x2F => self.das(),
            0x37 => self.aaa(),
            0x3F => self.aas(),
            0x40..=0x47 => {
                let index = (opcode & 7) as usize;
                let result = self.increment(self.regs.get16(index) as u32, true);
                self.regs.set16(index, result as u16);
            }
            0x48..=0x4F => {
                let index = (opcode & 7) as usize;
                let result = self.decrement(self.regs.get16(index) as u32, true);
                self.regs.set16(index, result as u16);
            }
            0x50..=0x57 => {
                let value = self.regs.get16((opcode & 7) as usize);
                self.push16(memory, value);
            }
            0x58..=0x5F => {
                let value = self.pop16(memory);
                self.regs.set16((opcode & 7) as usize, value);
            }
            0x60 => {
                let original_sp = self.regs.general[SP];
                for index in 0..8 {
                    let value = if index == SP { original_sp } else { self.regs.general[index] };
                    self.push16(memory, value);
                }
            }
            0x61 => {
                for index in (0..8).rev() {
                    let value = self.pop16(memory);
                    if index != SP {
                        self.regs.general[index] = value;
                    }
                }
            }
            0x68 => {
                let value = self.fetch16(memory);
                self.push16(memory, value);
            }
            0x6A => {
                let value = self.fetch_signed8(memory);
                self.push16(memory, value);
            }
            0x69 | 0x6B => {
                let modrm = self.decode_modrm(memory);
                let left = self.read_rm16(memory, &modrm);
                let right = if opcode == 0x69 { self.fetch16(memory) } else { self.fetch_signed8(memory) };
                let result = self.signed_multiply3(left, right);
                self.regs.set16(modrm.reg, result);
            }
            0x70..=0x7F => {
                let displacement = self.fetch_signed8(memory);
                if self.flags.condition(opcode & 0x0F) {
                    self.regs.ip = self.regs.ip.wrapping_add(displacement);
                }
            }
            0x80..=0x83 => return self.group_immediate(memory, opcode),
            0x84 | 0x85 => {
                let word = opcode & 1 == 1;
                let modrm = self.decode_modrm(memory);
                let left = self.read_rm(memory, &modrm, word);
                let right = self.read_reg(modrm.reg, word);
                self.logic(left & right, word);
            }
            0x86 | 0x87 => {
                let word = opcode & 1 == 1;
                let modrm = self.decode_modrm(memory);
                let left = self.read_rm(memory, &modrm, word);
                let right = self.read_reg(modrm.reg, word);
                self.write_rm(memory, &modrm, word, right);
                self.write_reg(modrm.reg, word, left);
            }
            0x88..=0x8B => {
                let word = opcode & 1 == 1;
                let modrm = self.decode_modrm(memory);
                if opcode & 2 == 0 {
                    let value = self.read_reg(modrm.reg, word);
                    self.write_rm(memory, &modrm, word, value);
                } else {
                    let value = self.read_rm(memory, &modrm, word);
                    self.write_reg(modrm.reg, word, value);
                }
            }
            0x8C => {
                let modrm = self.decode_modrm(memory);
                let value = self.regs.segment[modrm.reg & 3];
                self.write_rm16(memory, &modrm, value);
            }
            0x8D => {
                let modrm = self.decode_modrm(memory);
                if modrm.is_register() {
                    return Err(self.fault(opcode));
                }
                self.regs.set16(modrm.reg, modrm.offset);
            }
            0x8E => {
                let modrm = self.decode_modrm(memory);
                let value = self.read_rm16(memory, &modrm);
                self.regs.segment[modrm.reg & 3] = value;
            }
            0x8F => {
                let modrm = self.decode_modrm(memory);
                let value = self.pop16(memory);
                self.write_rm16(memory, &modrm, value);
            }
            0x90 => {}
            0x91..=0x97 => {
                let index = (opcode & 7) as usize;
                let value = self.regs.get16(index);
                self.regs.set16(index, self.regs.general[AX]);
                self.regs.general[AX] = value;
            }
            0x98 => {
                let value = self.regs.al() as i8 as i16 as u16;
                self.regs.set_ax(value);
            }
            0x99 => {
                let value = if self.regs.ax() & 0x8000 != 0 { 0xFFFF } else { 0 };
                self.regs.set_dx(value);
            }
            0x9A => {
                let offset = self.fetch16(memory);
                let segment = self.fetch16(memory);
                let cs = self.regs.segment[CS];
                self.push16(memory, cs);
                let ip = self.regs.ip;
                self.push16(memory, ip);
                self.jump_far(segment, offset);
            }
            0x9B => {}
            0x9C => {
                let value = self.flags.bits();
                self.push16(memory, value);
            }
            0x9D => {
                let value = self.pop16(memory);
                self.flags.load(value);
            }
            0x9E => {
                let ah = self.regs.ah() as u16;
                let mask = SF | ZF | AF | PF | CF;
                let value = (self.flags.bits() & !mask) | (ah & mask);
                self.flags.load(value);
            }
            0x9F => {
                let value = self.flags.bits() as u8;
                self.regs.set_ah(value);
            }
            0xA0..=0xA3 => {
                let word = opcode & 1 == 1;
                let offset = self.fetch16(memory);
                let segment = self.segment_for(DS);
                if opcode & 2 == 0 {
                    let value = if word { memory.read16_at(segment, offset) as u32 } else { memory.read8_at(segment, offset) as u32 };
                    self.write_reg(AX, word, value);
                } else if word {
                    memory.write16_at(segment, offset, self.regs.ax());
                } else {
                    memory.write8_at(segment, offset, self.regs.al());
                }
            }
            0xA4 | 0xA5 => self.string_instruction(memory, StringOp::Movs, opcode & 1 == 1),
            0xA6 | 0xA7 => self.string_instruction(memory, StringOp::Cmps, opcode & 1 == 1),
            0xA8 => {
                let value = self.fetch8(memory) as u32;
                self.logic(self.regs.al() as u32 & value, false);
            }
            0xA9 => {
                let value = self.fetch16(memory) as u32;
                self.logic(self.regs.ax() as u32 & value, true);
            }
            0xAA | 0xAB => self.string_instruction(memory, StringOp::Stos, opcode & 1 == 1),
            0xAC | 0xAD => self.string_instruction(memory, StringOp::Lods, opcode & 1 == 1),
            0xAE | 0xAF => self.string_instruction(memory, StringOp::Scas, opcode & 1 == 1),
            0xB0..=0xB7 => {
                let value = self.fetch8(memory);
                self.regs.set8((opcode & 7) as usize, value);
            }
            0xB8..=0xBF => {
                let value = self.fetch16(memory);
                self.regs.set16((opcode & 7) as usize, value);
            }
            0xC0 | 0xC1 | 0xD0..=0xD3 => return self.group_shift(memory, opcode),
            0xC2 => {
                let release = self.fetch16(memory);
                self.regs.ip = self.pop16(memory);
                self.regs.general[SP] = self.regs.general[SP].wrapping_add(release);
            }
            0xC3 => self.regs.ip = self.pop16(memory),
            0xC4 | 0xC5 => {
                let modrm = self.decode_modrm(memory);
                if modrm.is_register() {
                    return Err(self.fault(opcode));
                }
                let offset = memory.read16_at(modrm.segment, modrm.offset);
                let segment = memory.read16_at(modrm.segment, modrm.offset.wrapping_add(2));
                self.regs.set16(modrm.reg, offset);
                let target = if opcode == 0xC4 { 0 } else { DS };
                self.regs.segment[target] = segment;
            }
            0xC6 => {
                let modrm = self.decode_modrm(memory);
                let value = self.fetch8(memory);
                self.write_rm8(memory, &modrm, value);
            }
            0xC7 => {
                let modrm = self.decode_modrm(memory);
                let value = self.fetch16(memory);
                self.write_rm16(memory, &modrm, value);
            }
            0xC8 => {
                let size = self.fetch16(memory);
                let level = self.fetch8(memory) & 0x1F;
                let bp = self.regs.general[BP];
                self.push16(memory, bp);
                let frame = self.regs.general[SP];
                for depth in 1..level {
                    let address = bp.wrapping_sub(depth as u16 * 2);
                    let value = memory.read16_at(self.regs.segment[2], address);
                    self.push16(memory, value);
                }
                if level > 0 {
                    self.push16(memory, frame);
                }
                self.regs.general[BP] = frame;
                self.regs.general[SP] = self.regs.general[SP].wrapping_sub(size);
            }
            0xC9 => {
                self.regs.general[SP] = self.regs.general[BP];
                self.regs.general[BP] = self.pop16(memory);
            }
            0xCA | 0xCB => {
                let release = if opcode == 0xCA { self.fetch16(memory) } else { 0 };
                let ip = self.pop16(memory);
                let cs = self.pop16(memory);
                self.jump_far(cs, ip);
                self.regs.general[SP] = self.regs.general[SP].wrapping_add(release);
            }
            0xCC => return Ok(Step::Interrupt(3)),
            0xCD => {
                let vector = self.fetch8(memory);
                return Ok(Step::Interrupt(vector));
            }
            0xCE => {
                if self.flags.get(OF) {
                    return Ok(Step::Interrupt(4));
                }
            }
            0xCF => self.return_from_interrupt(memory),
            0xD4 => {
                let base = self.fetch8(memory);
                if self.aam(base).is_err() {
                    return Ok(Step::Interrupt(0));
                }
            }
            0xD5 => {
                let base = self.fetch8(memory);
                self.aad(base);
            }
            0xD6 => {
                let value = if self.flags.carry() { 0xFF } else { 0 };
                self.regs.set_al(value);
            }
            0xD7 => {
                let offset = self.regs.bx().wrapping_add(self.regs.al() as u16);
                let value = memory.read8_at(self.segment_for(DS), offset);
                self.regs.set_al(value);
            }
            0xD8..=0xDF => {
                self.decode_modrm(memory);
            }
            0xE0..=0xE3 => {
                let displacement = self.fetch_signed8(memory);
                let take = if opcode == 0xE3 {
                    self.regs.general[CX] == 0
                } else {
                    self.regs.general[CX] = self.regs.general[CX].wrapping_sub(1);
                    let remaining = self.regs.general[CX] != 0;
                    match opcode {
                        0xE0 => remaining && !self.flags.zero(),
                        0xE1 => remaining && self.flags.zero(),
                        _ => remaining,
                    }
                };
                if take {
                    self.regs.ip = self.regs.ip.wrapping_add(displacement);
                }
            }
            0xE4 | 0xE5 => {
                let port = self.fetch8(memory) as u16;
                let word = opcode & 1 == 1;
                let value = ports.port_in(port, word);
                self.write_reg(AX, word, value as u32);
            }
            0xE6 | 0xE7 => {
                let port = self.fetch8(memory) as u16;
                let word = opcode & 1 == 1;
                ports.port_out(port, self.read_reg(AX, word) as u16, word);
            }
            0xE8 => {
                let displacement = self.fetch16(memory);
                let ip = self.regs.ip;
                self.push16(memory, ip);
                self.regs.ip = ip.wrapping_add(displacement);
            }
            0xE9 => {
                let displacement = self.fetch16(memory);
                self.regs.ip = self.regs.ip.wrapping_add(displacement);
            }
            0xEA => {
                let offset = self.fetch16(memory);
                let segment = self.fetch16(memory);
                self.jump_far(segment, offset);
            }
            0xEB => {
                let displacement = self.fetch_signed8(memory);
                self.regs.ip = self.regs.ip.wrapping_add(displacement);
            }
            0xEC | 0xED => {
                let word = opcode & 1 == 1;
                let value = ports.port_in(self.regs.general[DX], word);
                self.write_reg(AX, word, value as u32);
            }
            0xEE | 0xEF => {
                let word = opcode & 1 == 1;
                ports.port_out(self.regs.general[DX], self.read_reg(AX, word) as u16, word);
            }
            0xF4 => return Ok(Step::Halt),
            0xF5 => {
                let carry = self.flags.carry();
                self.flags.set(CF, !carry);
            }
            0xF6 | 0xF7 => return self.group_unary(memory, opcode),
            0xF8 => self.flags.set(CF, false),
            0xF9 => self.flags.set(CF, true),
            0xFA => self.flags.set(IF, false),
            0xFB => self.flags.set(IF, true),
            0xFC => self.flags.set(DF, false),
            0xFD => self.flags.set(DF, true),
            0xFE => return self.group_increment(memory, opcode),
            0xFF => return self.group_misc(memory, opcode),
            _ => return Err(self.fault(opcode)),
        }
        Ok(Step::Continue)
    }

    fn arith_form(&mut self, memory: &mut Memory, opcode: u8) {
        let operation = opcode >> 3;
        let form = opcode & 7;
        let word = form & 1 == 1;
        match form {
            0..=3 => {
                let modrm = self.decode_modrm(memory);
                let to_register = form & 2 != 0;
                let rm_value = self.read_rm(memory, &modrm, word);
                let reg_value = self.read_reg(modrm.reg, word);
                let (left, right) = if to_register { (reg_value, rm_value) } else { (rm_value, reg_value) };
                let result = self.arith(operation, left, right, word);
                if operation != CMP {
                    if to_register {
                        self.write_reg(modrm.reg, word, result);
                    } else {
                        self.write_rm(memory, &modrm, word, result);
                    }
                }
            }
            _ => {
                let right = if word { self.fetch16(memory) as u32 } else { self.fetch8(memory) as u32 };
                let left = self.read_reg(AX, word);
                let result = self.arith(operation, left, right, word);
                if operation != CMP {
                    self.write_reg(AX, word, result);
                }
            }
        }
    }
}
