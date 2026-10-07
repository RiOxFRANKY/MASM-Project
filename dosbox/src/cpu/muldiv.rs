use super::flags::{CF, OF};
use super::registers::{AX, DX};
use super::Cpu;

pub(crate) struct DivideError;

impl Cpu {
    pub(crate) fn multiply(&mut self, operand: u32, word: bool, signed: bool) {
        let overflow = if word {
            let ax = self.regs.general[AX];
            let product = if signed {
                (ax as i16 as i32 * operand as u16 as i16 as i32) as u32
            } else {
                ax as u32 * operand
            };
            self.regs.general[AX] = product as u16;
            self.regs.general[DX] = (product >> 16) as u16;
            if signed {
                (product as i32) != (product as u16 as i16 as i32)
            } else {
                product >> 16 != 0
            }
        } else {
            let al = self.regs.al();
            let product = if signed {
                (al as i8 as i16 * operand as u8 as i8 as i16) as u16
            } else {
                al as u16 * operand as u16
            };
            self.regs.general[AX] = product;
            if signed {
                (product as i16) != (product as u8 as i8 as i16)
            } else {
                product >> 8 != 0
            }
        };
        self.flags.set(CF, overflow);
        self.flags.set(OF, overflow);
    }

    pub(crate) fn signed_multiply3(&mut self, left: u16, right: u16) -> u16 {
        let product = left as i16 as i32 * right as i16 as i32;
        let overflow = product != product as i16 as i32;
        self.flags.set(CF, overflow);
        self.flags.set(OF, overflow);
        product as u16
    }

    pub(crate) fn divide(&mut self, operand: u32, word: bool, signed: bool) -> Result<(), DivideError> {
        if operand == 0 {
            return Err(DivideError);
        }
        if word {
            let dividend = ((self.regs.general[DX] as u32) << 16) | self.regs.general[AX] as u32;
            let (quotient, remainder) = if signed {
                let dividend = dividend as i32 as i64;
                let divisor = operand as u16 as i16 as i64;
                let quotient = dividend / divisor;
                if quotient > i16::MAX as i64 || quotient < i16::MIN as i64 {
                    return Err(DivideError);
                }
                (quotient as u16, (dividend % divisor) as u16)
            } else {
                let quotient = dividend / operand;
                if quotient > 0xFFFF {
                    return Err(DivideError);
                }
                (quotient as u16, (dividend % operand) as u16)
            };
            self.regs.general[AX] = quotient;
            self.regs.general[DX] = remainder;
        } else {
            let dividend = self.regs.general[AX];
            let (quotient, remainder) = if signed {
                let dividend = dividend as i16 as i32;
                let divisor = operand as u8 as i8 as i32;
                let quotient = dividend / divisor;
                if quotient > i8::MAX as i32 || quotient < i8::MIN as i32 {
                    return Err(DivideError);
                }
                (quotient as u8, (dividend % divisor) as u8)
            } else {
                let quotient = dividend as u32 / operand;
                if quotient > 0xFF {
                    return Err(DivideError);
                }
                (quotient as u8, (dividend as u32 % operand) as u8)
            };
            self.regs.set_al(quotient);
            self.regs.set_ah(remainder);
        }
        Ok(())
    }
}
