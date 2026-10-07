use super::eval::Value;
use super::registers::{Register, BP, BX, DI, SI};
use super::segments::Frame;
use super::types::TypeKind;

#[derive(Clone, Debug)]
pub struct MemoryOperand {
    pub rm: u8,
    pub direct: bool,
    pub displacement: i64,
    pub kind: Option<TypeKind>,
    pub segment: Option<u8>,
}

#[derive(Clone, Debug)]
pub struct Immediate {
    pub value: i64,
    pub frame: Option<(Frame, u16)>,
    pub unknown: bool,
}

#[derive(Clone, Debug)]
pub enum Operand {
    Register(Register),
    Memory(MemoryOperand),
    Immediate(Immediate),
}

impl Operand {
    pub fn size(&self) -> Option<u32> {
        match self {
            Operand::Register(Register::Byte(_)) => Some(1),
            Operand::Register(_) => Some(2),
            Operand::Memory(memory) => memory.kind.map(TypeKind::size),
            Operand::Immediate(_) => None,
        }
    }
}

fn rm_for(base: Option<u8>, index: Option<u8>) -> Option<u8> {
    Some(match (base, index) {
        (Some(BX), Some(SI)) => 0,
        (Some(BX), Some(DI)) => 1,
        (Some(BP), Some(SI)) => 2,
        (Some(BP), Some(DI)) => 3,
        (None, Some(SI)) => 4,
        (None, Some(DI)) => 5,
        (Some(BP), None) => 6,
        (Some(BX), None) => 7,
        _ => return None,
    })
}

pub fn to_operand(value: &Value) -> Result<Operand, String> {
    if let Some(register) = value.register {
        if value.memory || value.base.is_some() || value.index.is_some() || value.number != 0 {
            return Err("invalid use of register".to_string());
        }
        return Ok(Operand::Register(register));
    }
    if value.memory || value.base.is_some() || value.index.is_some() {
        let direct = value.base.is_none() && value.index.is_none();
        let rm = if direct { 6 } else { rm_for(value.base, value.index).ok_or("invalid addressing mode")? };
        return Ok(Operand::Memory(MemoryOperand {
            rm,
            direct,
            displacement: value.number,
            kind: value.kind,
            segment: value.segment_override,
        }));
    }
    Ok(Operand::Immediate(Immediate { value: value.number, frame: value.frame, unknown: value.unknown }))
}
