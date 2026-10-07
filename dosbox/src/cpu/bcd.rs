use super::flags::{AF, CF};
use super::muldiv::DivideError;
use super::Cpu;

impl Cpu {
    pub(crate) fn daa(&mut self) {
        let mut al = self.regs.al();
        let old_al = al;
        let old_carry = self.flags.carry();
        if al & 0x0F > 9 || self.flags.get(AF) {
            al = al.wrapping_add(6);
            self.flags.set(AF, true);
        } else {
            self.flags.set(AF, false);
        }
        if old_al > 0x99 || old_carry {
            al = al.wrapping_add(0x60);
            self.flags.set(CF, true);
        } else {
            self.flags.set(CF, false);
        }
        self.regs.set_al(al);
        self.flags.set_result(al as u32, false);
    }

    pub(crate) fn das(&mut self) {
        let mut al = self.regs.al();
        let old_al = al;
        let old_carry = self.flags.carry();
        if al & 0x0F > 9 || self.flags.get(AF) {
            al = al.wrapping_sub(6);
            self.flags.set(AF, true);
        } else {
            self.flags.set(AF, false);
        }
        if old_al > 0x99 || old_carry {
            al = al.wrapping_sub(0x60);
            self.flags.set(CF, true);
        } else {
            self.flags.set(CF, false);
        }
        self.regs.set_al(al);
        self.flags.set_result(al as u32, false);
    }

    pub(crate) fn aaa(&mut self) {
        let adjust = self.regs.al() & 0x0F > 9 || self.flags.get(AF);
        if adjust {
            let ax = self.regs.ax().wrapping_add(0x106);
            self.regs.set_ax(ax);
        }
        self.flags.set(AF, adjust);
        self.flags.set(CF, adjust);
        let al = self.regs.al() & 0x0F;
        self.regs.set_al(al);
    }

    pub(crate) fn aas(&mut self) {
        let adjust = self.regs.al() & 0x0F > 9 || self.flags.get(AF);
        if adjust {
            let al = self.regs.al().wrapping_sub(6);
            let ah = self.regs.ah().wrapping_sub(1);
            self.regs.set_al(al);
            self.regs.set_ah(ah);
        }
        self.flags.set(AF, adjust);
        self.flags.set(CF, adjust);
        let al = self.regs.al() & 0x0F;
        self.regs.set_al(al);
    }

    pub(crate) fn aam(&mut self, base: u8) -> Result<(), DivideError> {
        if base == 0 {
            return Err(DivideError);
        }
        let al = self.regs.al();
        self.regs.set_ah(al / base);
        self.regs.set_al(al % base);
        self.flags.set_result(self.regs.al() as u32, false);
        Ok(())
    }

    pub(crate) fn aad(&mut self, base: u8) {
        let value = self.regs.ah().wrapping_mul(base).wrapping_add(self.regs.al());
        self.regs.set_ax(value as u16);
        self.flags.set_result(value as u32, false);
    }
}
