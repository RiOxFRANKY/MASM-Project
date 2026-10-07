use crate::cpu::flags::ZF;
use crate::machine::{Halt, Machine};

impl Machine {
    pub(crate) fn bios_keyboard(&mut self) -> Result<(), Halt> {
        match self.cpu.regs.ah() {
            0x00 | 0x10 => {
                let key = self.wait_key()?;
                self.cpu.regs.set_ax(key.code());
            }
            0x01 | 0x11 => match self.peek_key()? {
                Some(key) => {
                    self.cpu.regs.set_ax(key.code());
                    self.cpu.flags.set(ZF, false);
                }
                None => self.cpu.flags.set(ZF, true),
            },
            0x02 | 0x12 => self.cpu.regs.set_al(0),
            0x05 => {
                let code = self.cpu.regs.cx();
                self.keyboard.push(crate::terminal::Key { scan: (code >> 8) as u8, ascii: code as u8 });
                self.cpu.regs.set_al(0);
            }
            _ => {}
        }
        Ok(())
    }
}
