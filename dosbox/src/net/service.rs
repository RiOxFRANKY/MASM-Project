use crate::cpu::flags::CF;
use crate::cpu::registers::{DI, SI};
use crate::machine::{Halt, Machine};

const SIGNATURE: u16 = 0x4E57;
const VERSION: u16 = 1;
const MAX_MESSAGE: usize = 4096;

impl Machine {
    pub(crate) fn network_service(&mut self) -> Result<(), Halt> {
        let regs = self.cpu.regs.clone();
        let mut failed = false;
        match regs.ah() {
            0x00 => {
                self.cpu.regs.set_ax(SIGNATURE);
                self.cpu.regs.set_bx(VERSION);
            }
            0x01 => {
                let host = self.memory.read_asciiz(regs.ds(), regs.general[SI]);
                if host.trim().is_empty() || regs.dx() == 0 {
                    failed = true;
                } else {
                    self.net.connect(host.trim().to_string(), regs.dx());
                }
            }
            0x02 => self.cpu.regs.set_al(self.net.status()),
            0x03 => {
                let length = (regs.cx() as usize).min(MAX_MESSAGE);
                let bytes = self.memory.read_bytes(regs.ds(), regs.general[SI], length);
                failed = !self.net.send(&bytes);
            }
            0x04 => {
                let count = match self.net.receive() {
                    Some(message) => {
                        let count = message.len().min(regs.cx() as usize);
                        self.memory.write_bytes(regs.ds(), regs.general[DI], &message[..count]);
                        count
                    }
                    None => 0,
                };
                self.cpu.regs.set_cx(count as u16);
                self.cpu.regs.set_al(self.net.status());
            }
            0x05 => self.net.close(),
            _ => failed = true,
        }
        self.cpu.flags.set(CF, failed);
        Ok(())
    }
}
