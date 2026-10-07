#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Register {
    Byte(u8),
    Word(u8),
    Segment(u8),
}

pub const BX: u8 = 3;
pub const BP: u8 = 5;
pub const SI: u8 = 6;
pub const DI: u8 = 7;

const BYTE_NAMES: [&str; 8] = ["AL", "CL", "DL", "BL", "AH", "CH", "DH", "BH"];
const WORD_NAMES: [&str; 8] = ["AX", "CX", "DX", "BX", "SP", "BP", "SI", "DI"];
const SEGMENT_NAMES: [&str; 4] = ["ES", "CS", "SS", "DS"];

pub fn lookup(name: &str) -> Option<Register> {
    let find = |names: &[&str]| names.iter().position(|candidate| *candidate == name).map(|index| index as u8);
    find(&BYTE_NAMES)
        .map(Register::Byte)
        .or_else(|| find(&WORD_NAMES).map(Register::Word))
        .or_else(|| find(&SEGMENT_NAMES).map(Register::Segment))
}

pub fn segment_prefix(segment: u8) -> u8 {
    [0x26, 0x2E, 0x36, 0x3E][segment as usize & 3]
}
