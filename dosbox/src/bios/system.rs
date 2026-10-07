use crate::cpu::flags::CF;
use crate::hardware::clock;
use crate::machine::{Halt, Machine};
use std::time::Duration;

fn bcd(value: u8) -> u8 {
    ((value / 10) << 4) | (value % 10)
}

impl Machine {
    pub(crate) fn bios_system(&mut self) -> Result<(), Halt> {
        match self.cpu.regs.ah() {
            0x86 => {
                let micros = ((self.cpu.regs.cx() as u64) << 16) | self.cpu.regs.dx() as u64;
                self.present();
                std::thread::sleep(Duration::from_micros(micros.min(5_000_000)));
                self.cpu.flags.set(CF, false);
            }
            _ => {
                self.cpu.regs.set_ah(0x86);
                self.cpu.flags.set(CF, true);
            }
        }
        Ok(())
    }

    pub(crate) fn bios_time(&mut self) -> Result<(), Halt> {
        match self.cpu.regs.ah() {
            0x00 => {
                let ticks = clock::ticks_since_midnight();
                self.cpu.regs.set_cx((ticks >> 16) as u16);
                self.cpu.regs.set_dx(ticks as u16);
                self.cpu.regs.set_al(0);
            }
            0x02 => {
                let now = clock::now();
                self.cpu.regs.set_ch(bcd(now.hour));
                self.cpu.regs.set_cl(bcd(now.minute));
                self.cpu.regs.set_dh(bcd(now.second));
                self.cpu.regs.set_dl(0);
                self.cpu.flags.set(CF, false);
            }
            0x04 => {
                let now = clock::now();
                self.cpu.regs.set_ch(bcd((now.year / 100) as u8));
                self.cpu.regs.set_cl(bcd((now.year % 100) as u8));
                self.cpu.regs.set_dh(bcd(now.month));
                self.cpu.regs.set_dl(bcd(now.day));
                self.cpu.flags.set(CF, false);
            }
            _ => {}
        }
        Ok(())
    }
}
