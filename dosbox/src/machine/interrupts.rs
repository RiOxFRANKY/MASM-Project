use super::{Halt, Machine, BIOS_SEGMENT, STUB_BASE};

const IRET: u8 = 0xCF;

impl Machine {
    pub(crate) fn install_vectors(&mut self) {
        for vector in 0..=255u16 {
            let stub = STUB_BASE + vector;
            self.memory.write16(vector as u32 * 4, stub);
            self.memory.write16(vector as u32 * 4 + 2, BIOS_SEGMENT);
            self.memory.write8_at(BIOS_SEGMENT, stub, IRET);
        }
    }

    pub(crate) fn service(&mut self, vector: u8) -> Result<(), Halt> {
        match vector {
            0x00 => Err(Halt::Fault("Divide overflow".to_string())),
            0x10 => self.bios_video(),
            0x11 => {
                let equipment = self.memory.read16(0x410);
                self.cpu.regs.set_ax(equipment);
                Ok(())
            }
            0x12 => {
                self.cpu.regs.set_ax(640);
                Ok(())
            }
            0x15 => self.bios_system(),
            0x16 => self.bios_keyboard(),
            0x1A => self.bios_time(),
            0x20 => Err(Halt::Exit(0)),
            0x21 => self.dos_service(),
            0x27 => Err(Halt::Exit(0)),
            0x29 => {
                let character = self.cpu.regs.al();
                self.video.teletype(&mut self.memory, character, None);
                Ok(())
            }
            _ => Ok(()),
        }
    }
}
