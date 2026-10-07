use super::registers::{BP, BX, DI, DS, SI, SS};
use super::Cpu;
use crate::hardware::memory::Memory;

pub(crate) struct ModRm {
    pub mode: u8,
    pub reg: usize,
    pub rm: usize,
    pub segment: u16,
    pub offset: u16,
}

impl ModRm {
    pub fn is_register(&self) -> bool {
        self.mode == 3
    }
}

impl Cpu {
    pub(crate) fn decode_modrm(&mut self, memory: &Memory) -> ModRm {
        let byte = self.fetch8(memory);
        let mode = byte >> 6;
        let reg = ((byte >> 3) & 7) as usize;
        let rm = (byte & 7) as usize;
        if mode == 3 {
            return ModRm { mode, reg, rm, segment: 0, offset: 0 };
        }
        let general = &self.regs.general;
        let (base, default_segment) = match rm {
            0 => (general[BX].wrapping_add(general[SI]), DS),
            1 => (general[BX].wrapping_add(general[DI]), DS),
            2 => (general[BP].wrapping_add(general[SI]), SS),
            3 => (general[BP].wrapping_add(general[DI]), SS),
            4 => (general[SI], DS),
            5 => (general[DI], DS),
            6 => (general[BP], SS),
            _ => (general[BX], DS),
        };
        let (offset, default_segment) = match mode {
            0 if rm == 6 => (self.fetch16(memory), DS),
            0 => (base, default_segment),
            1 => (base.wrapping_add(self.fetch_signed8(memory)), default_segment),
            _ => (base.wrapping_add(self.fetch16(memory)), default_segment),
        };
        let segment = self.segment_for(default_segment);
        ModRm { mode, reg, rm, segment, offset }
    }

    pub(crate) fn read_rm8(&self, memory: &Memory, modrm: &ModRm) -> u8 {
        if modrm.is_register() {
            self.regs.get8(modrm.rm)
        } else {
            memory.read8_at(modrm.segment, modrm.offset)
        }
    }

    pub(crate) fn write_rm8(&mut self, memory: &mut Memory, modrm: &ModRm, value: u8) {
        if modrm.is_register() {
            self.regs.set8(modrm.rm, value);
        } else {
            memory.write8_at(modrm.segment, modrm.offset, value);
        }
    }

    pub(crate) fn read_rm16(&self, memory: &Memory, modrm: &ModRm) -> u16 {
        if modrm.is_register() {
            self.regs.get16(modrm.rm)
        } else {
            memory.read16_at(modrm.segment, modrm.offset)
        }
    }

    pub(crate) fn write_rm16(&mut self, memory: &mut Memory, modrm: &ModRm, value: u16) {
        if modrm.is_register() {
            self.regs.set16(modrm.rm, value);
        } else {
            memory.write16_at(modrm.segment, modrm.offset, value);
        }
    }

    pub(crate) fn read_rm(&self, memory: &Memory, modrm: &ModRm, word: bool) -> u32 {
        if word {
            self.read_rm16(memory, modrm) as u32
        } else {
            self.read_rm8(memory, modrm) as u32
        }
    }

    pub(crate) fn write_rm(&mut self, memory: &mut Memory, modrm: &ModRm, word: bool, value: u32) {
        if word {
            self.write_rm16(memory, modrm, value as u16);
        } else {
            self.write_rm8(memory, modrm, value as u8);
        }
    }

    pub(crate) fn read_reg(&self, index: usize, word: bool) -> u32 {
        if word {
            self.regs.get16(index) as u32
        } else {
            self.regs.get8(index) as u32
        }
    }

    pub(crate) fn write_reg(&mut self, index: usize, word: bool, value: u32) {
        if word {
            self.regs.set16(index, value as u16);
        } else {
            self.regs.set8(index, value as u8);
        }
    }
}
