use super::super::operand::{Immediate, MemoryOperand, Operand};
use super::super::registers::{segment_prefix, Register};
use super::super::segments::Frame;

#[derive(Default)]
pub struct Encoded {
    pub bytes: Vec<u8>,
    pub relocations: Vec<(usize, Frame)>,
    pub warnings: Vec<String>,
}

#[derive(Default)]
pub struct Emitter {
    pub bytes: Vec<u8>,
    pub relocations: Vec<(usize, Frame)>,
    pub warnings: Vec<String>,
}

pub fn register_code(register: Register) -> u8 {
    match register {
        Register::Byte(code) | Register::Word(code) | Register::Segment(code) => code,
    }
}

pub fn fits_signed_byte(value: i64) -> bool {
    let truncated = value as u16 as i16;
    (-128..=127).contains(&truncated) && (-0x8000..=0xFFFF).contains(&value)
}

pub fn pair_size(left: &Operand, right: &Operand) -> Result<u32, String> {
    match (left.size(), right.size()) {
        (Some(a), Some(b)) if a != b => Err("operand types must match".to_string()),
        (Some(a), _) => Ok(a),
        (None, Some(b)) => Ok(b),
        (None, None) => Err("operand must have a size (use BYTE PTR or WORD PTR)".to_string()),
    }
}

pub fn single_size(operand: &Operand) -> Result<u32, String> {
    operand.size().ok_or_else(|| "operand must have a size (use BYTE PTR or WORD PTR)".to_string())
}

pub fn word_bit(size: u32) -> Result<u8, String> {
    match size {
        1 => Ok(0),
        2 => Ok(1),
        _ => Err("operand must be a byte or a word".to_string()),
    }
}

impl Emitter {
    pub fn finish(self) -> Encoded {
        Encoded { bytes: self.bytes, relocations: self.relocations, warnings: self.warnings }
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn byte(&mut self, value: u8) {
        self.bytes.push(value);
    }

    pub fn word(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub fn segment_prefix(&mut self, operand: &Operand) {
        if let Operand::Memory(MemoryOperand { segment: Some(segment), .. }) = operand {
            self.byte(segment_prefix(*segment));
        }
    }

    pub fn modrm(&mut self, reg_field: u8, operand: &Operand) -> Result<(), String> {
        match operand {
            Operand::Register(register) => {
                self.byte(0xC0 | (reg_field << 3) | register_code(*register));
                Ok(())
            }
            Operand::Memory(memory) => {
                self.memory(reg_field, memory);
                Ok(())
            }
            Operand::Immediate(_) => Err("register or memory operand expected".to_string()),
        }
    }

    pub fn memory(&mut self, reg_field: u8, memory: &MemoryOperand) {
        let displacement = memory.displacement as u16;
        if memory.direct {
            self.byte((reg_field << 3) | 6);
            self.word(displacement);
        } else if displacement == 0 && memory.rm != 6 {
            self.byte((reg_field << 3) | memory.rm);
        } else if fits_signed_byte(memory.displacement) {
            self.byte(0x40 | (reg_field << 3) | memory.rm);
            self.byte(displacement as u8);
        } else {
            self.byte(0x80 | (reg_field << 3) | memory.rm);
            self.word(displacement);
        }
    }

    pub fn immediate(&mut self, immediate: &Immediate, size: u32) -> Result<(), String> {
        if let Some((frame, paragraph)) = immediate.frame {
            if size != 2 {
                return Err("segment value needs a word operand".to_string());
            }
            self.relocations.push((self.bytes.len(), frame));
            self.word(paragraph);
            return Ok(());
        }
        let value = immediate.value;
        match size {
            1 => {
                if !immediate.unknown && !(-256..=255).contains(&value) {
                    return Err(format!("value {value} does not fit in a byte"));
                }
                self.byte(value as u8);
            }
            _ => {
                if !immediate.unknown && !(-0x10000..=0xFFFF).contains(&value) {
                    return Err(format!("value {value} does not fit in a word"));
                }
                self.word(value as u16);
            }
        }
        Ok(())
    }
}
