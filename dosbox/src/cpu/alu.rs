use super::flags::{AF, CF, OF};
use super::Cpu;

pub(crate) const ADD: u8 = 0;
pub(crate) const OR: u8 = 1;
pub(crate) const ADC: u8 = 2;
pub(crate) const SBB: u8 = 3;
pub(crate) const AND: u8 = 4;
pub(crate) const SUB: u8 = 5;
pub(crate) const CMP: u8 = 7;

fn limits(word: bool) -> (u32, u32, u32) {
    if word { (0xFFFF, 0x8000, 16) } else { (0xFF, 0x80, 8) }
}

impl Cpu {
    pub(crate) fn arith(&mut self, operation: u8, left: u32, right: u32, word: bool) -> u32 {
        match operation {
            ADD => self.add(left, right, 0, word),
            ADC => {
                let carry = self.flags.carry() as u32;
                self.add(left, right, carry, word)
            }
            SUB | CMP => self.sub(left, right, 0, word),
            SBB => {
                let carry = self.flags.carry() as u32;
                self.sub(left, right, carry, word)
            }
            AND => self.logic(left & right, word),
            OR => self.logic(left | right, word),
            _ => self.logic(left ^ right, word),
        }
    }

    fn add(&mut self, left: u32, right: u32, carry: u32, word: bool) -> u32 {
        let (mask, sign, _) = limits(word);
        let full = left + right + carry;
        let result = full & mask;
        self.flags.set(CF, full > mask);
        self.flags.set(OF, (left ^ result) & (right ^ result) & sign != 0);
        self.flags.set(AF, (left ^ right ^ result) & 0x10 != 0);
        self.flags.set_result(result, word);
        result
    }

    fn sub(&mut self, left: u32, right: u32, borrow: u32, word: bool) -> u32 {
        let (mask, sign, _) = limits(word);
        let result = left.wrapping_sub(right).wrapping_sub(borrow) & mask;
        self.flags.set(CF, (left as u64) < right as u64 + borrow as u64);
        self.flags.set(OF, (left ^ right) & (left ^ result) & sign != 0);
        self.flags.set(AF, (left ^ right ^ result) & 0x10 != 0);
        self.flags.set_result(result, word);
        result
    }

    pub(crate) fn logic(&mut self, value: u32, word: bool) -> u32 {
        let (mask, _, _) = limits(word);
        let result = value & mask;
        self.flags.set(CF, false);
        self.flags.set(OF, false);
        self.flags.set(AF, false);
        self.flags.set_result(result, word);
        result
    }

    pub(crate) fn increment(&mut self, value: u32, word: bool) -> u32 {
        let carry = self.flags.carry();
        let result = self.add(value, 1, 0, word);
        self.flags.set(CF, carry);
        result
    }

    pub(crate) fn decrement(&mut self, value: u32, word: bool) -> u32 {
        let carry = self.flags.carry();
        let result = self.sub(value, 1, 0, word);
        self.flags.set(CF, carry);
        result
    }

    pub(crate) fn negate(&mut self, value: u32, word: bool) -> u32 {
        let result = self.sub(0, value, 0, word);
        self.flags.set(CF, value != 0);
        result
    }

    pub(crate) fn shift(&mut self, operation: u8, value: u32, count: u8, word: bool) -> u32 {
        let (mask, sign, bits) = limits(word);
        let count = count & 0x1F;
        if count == 0 {
            return value;
        }
        let mut result = value & mask;
        let mut carry = self.flags.carry();
        match operation {
            0 => {
                for _ in 0..count {
                    carry = result & sign != 0;
                    result = ((result << 1) | carry as u32) & mask;
                }
                self.flags.set(OF, (result & sign != 0) != carry);
            }
            1 => {
                for _ in 0..count {
                    carry = result & 1 != 0;
                    result = (result >> 1) | ((carry as u32) << (bits - 1));
                }
                self.flags.set(OF, (result ^ (result << 1)) & sign != 0);
            }
            2 => {
                for _ in 0..count {
                    let out = result & sign != 0;
                    result = ((result << 1) | carry as u32) & mask;
                    carry = out;
                }
                self.flags.set(OF, (result & sign != 0) != carry);
            }
            3 => {
                for _ in 0..count {
                    let out = result & 1 != 0;
                    result = (result >> 1) | ((carry as u32) << (bits - 1));
                    carry = out;
                }
                self.flags.set(OF, (result ^ (result << 1)) & sign != 0);
            }
            4 | 6 => {
                for _ in 0..count {
                    carry = result & sign != 0;
                    result = (result << 1) & mask;
                }
                self.flags.set(OF, (result & sign != 0) != carry);
                self.flags.set_result(result, word);
            }
            5 => {
                self.flags.set(OF, value & sign != 0);
                for _ in 0..count {
                    carry = result & 1 != 0;
                    result >>= 1;
                }
                self.flags.set_result(result, word);
            }
            _ => {
                for _ in 0..count {
                    carry = result & 1 != 0;
                    result = (result >> 1) | (result & sign);
                }
                self.flags.set(OF, false);
                self.flags.set_result(result, word);
            }
        }
        self.flags.set(CF, carry);
        result
    }
}
