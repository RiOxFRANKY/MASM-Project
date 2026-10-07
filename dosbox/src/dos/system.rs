use super::ERROR_INVALID_FUNCTION;
use crate::hardware::clock;
use crate::machine::{Halt, Machine};

impl Machine {
    pub(crate) fn dos_system(&mut self) -> Result<(), Halt> {
        let regs = self.cpu.regs.clone();
        match regs.ah() {
            0x00 => return Err(Halt::Exit(0)),
            0x0D => {}
            0x0E => self.cpu.regs.set_al(3),
            0x19 => self.cpu.regs.set_al(2),
            0x1A => self.dos.dta = (regs.ds(), regs.dx()),
            0x25 => {
                let address = regs.al() as u32 * 4;
                self.memory.write16(address, regs.dx());
                self.memory.write16(address + 2, regs.ds());
            }
            0x2A => {
                let now = clock::now();
                self.cpu.regs.set_cx(now.year);
                self.cpu.regs.set_dh(now.month);
                self.cpu.regs.set_dl(now.day);
                self.cpu.regs.set_al(now.weekday);
            }
            0x2C => {
                let now = clock::now();
                self.cpu.regs.set_ch(now.hour);
                self.cpu.regs.set_cl(now.minute);
                self.cpu.regs.set_dh(now.second);
                self.cpu.regs.set_dl(now.hundredths);
            }
            0x2B | 0x2D => self.cpu.regs.set_al(0),
            0x2F => {
                self.cpu.regs.segment[crate::cpu::registers::ES] = self.dos.dta.0;
                self.cpu.regs.set_bx(self.dos.dta.1);
            }
            0x30 => {
                self.cpu.regs.set_ax(0x0005);
                self.cpu.regs.set_bx(0);
                self.cpu.regs.set_cx(0);
            }
            0x33 => self.cpu.regs.set_dl(0),
            0x35 => {
                let address = regs.al() as u32 * 4;
                let offset = self.memory.read16(address);
                let segment = self.memory.read16(address + 2);
                self.cpu.regs.set_bx(offset);
                self.cpu.regs.segment[crate::cpu::registers::ES] = segment;
            }
            0x4C => return Err(Halt::Exit(regs.al())),
            0x4D => self.cpu.regs.set_ax(self.dos.return_code as u16),
            0x51 | 0x62 => self.cpu.regs.set_bx(self.dos.psp),
            _ => self.dos_error(ERROR_INVALID_FUNCTION),
        }
        Ok(())
    }
}
