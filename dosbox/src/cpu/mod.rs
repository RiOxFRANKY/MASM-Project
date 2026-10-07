mod alu;
mod bcd;
mod execute;
pub mod flags;
mod groups;
mod modrm;
mod muldiv;
pub mod registers;
mod stack;
mod string;

use crate::hardware::memory::Memory;
use flags::Flags;
use registers::{Registers, CS};

pub trait PortBus {
    fn port_in(&mut self, port: u16, word: bool) -> u16;
    fn port_out(&mut self, port: u16, value: u16, word: bool);
}

pub enum Step {
    Continue,
    Interrupt(u8),
    Halt,
}

pub struct Fault {
    pub opcode: u8,
    pub segment: u16,
    pub offset: u16,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Repeat {
    WhileEqual,
    WhileNotEqual,
}

#[derive(Clone, Default)]
pub struct Cpu {
    pub regs: Registers,
    pub flags: Flags,
    pub(crate) segment_override: Option<usize>,
    pub(crate) repeat: Option<Repeat>,
    pub(crate) instruction_ip: u16,
}

impl Cpu {
    pub fn new() -> Self {
        Cpu::default()
    }

    pub fn reset(&mut self) {
        *self = Cpu::default();
    }

    pub fn step(&mut self, memory: &mut Memory, ports: &mut dyn PortBus) -> Result<Step, Fault> {
        self.segment_override = None;
        self.repeat = None;
        self.instruction_ip = self.regs.ip;
        loop {
            let opcode = self.fetch8(memory);
            match opcode {
                0x26 => self.segment_override = Some(0),
                0x2E => self.segment_override = Some(1),
                0x36 => self.segment_override = Some(2),
                0x3E => self.segment_override = Some(3),
                0xF0 => {}
                0xF2 => self.repeat = Some(Repeat::WhileNotEqual),
                0xF3 => self.repeat = Some(Repeat::WhileEqual),
                _ => return self.execute(opcode, memory, ports),
            }
        }
    }

    pub(crate) fn fault(&self, opcode: u8) -> Fault {
        Fault {
            opcode,
            segment: self.regs.segment[CS],
            offset: self.instruction_ip,
        }
    }

    pub(crate) fn fetch8(&mut self, memory: &Memory) -> u8 {
        let value = memory.read8_at(self.regs.segment[CS], self.regs.ip);
        self.regs.ip = self.regs.ip.wrapping_add(1);
        value
    }

    pub(crate) fn fetch16(&mut self, memory: &Memory) -> u16 {
        let low = self.fetch8(memory) as u16;
        let high = self.fetch8(memory) as u16;
        low | (high << 8)
    }

    pub(crate) fn fetch_signed8(&mut self, memory: &Memory) -> u16 {
        self.fetch8(memory) as i8 as i16 as u16
    }

    pub(crate) fn segment_for(&self, default: usize) -> u16 {
        self.regs.segment[self.segment_override.unwrap_or(default)]
    }

    pub fn jump_far(&mut self, segment: u16, offset: u16) {
        self.regs.segment[CS] = segment;
        self.regs.ip = offset;
    }
}
