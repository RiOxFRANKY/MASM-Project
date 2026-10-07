use super::{Arena, FileTable};
use crate::cpu::registers::{CS, DS, ES, SP, SS};
use crate::formats::mz;
use crate::machine::Machine;

pub const ENVIRONMENT_SEGMENT: u16 = 0x0800;
pub const PSP_SEGMENT: u16 = 0x0820;
pub const TOP_SEGMENT: u16 = 0xA000;
const COM_LIMIT: usize = 0xFF00;

impl Machine {
    pub fn load_program(&mut self, bytes: &[u8], program_name: &str, tail: &str) -> Result<(), String> {
        for segment in ENVIRONMENT_SEGMENT..TOP_SEGMENT {
            self.memory.load((segment as u32) << 4, &[0u8; 16]);
        }
        self.write_environment(program_name);
        self.write_psp(tail);
        self.cpu.reset();
        self.dos.psp = PSP_SEGMENT;
        self.dos.dta = (PSP_SEGMENT, 0x80);
        self.dos.pending_scan = None;
        self.dos.files = FileTable::new();
        self.dos.arena = Arena::new(PSP_SEGMENT, TOP_SEGMENT);
        if mz::is_executable(bytes) {
            self.load_executable(bytes)
        } else {
            self.load_com(bytes)
        }
    }

    fn write_environment(&mut self, program_name: &str) {
        let mut block = b"COMSPEC=C:\\COMMAND.COM\0PATH=C:\\\0\0".to_vec();
        block.extend_from_slice(&1u16.to_le_bytes());
        block.extend_from_slice(program_name.as_bytes());
        block.push(0);
        self.memory.write_bytes(ENVIRONMENT_SEGMENT, 0, &block);
    }

    fn write_psp(&mut self, tail: &str) {
        let psp = PSP_SEGMENT;
        self.memory.write_bytes(psp, 0x00, &[0xCD, 0x20]);
        self.memory.write16_at(psp, 0x02, TOP_SEGMENT);
        self.memory.write16_at(psp, 0x2C, ENVIRONMENT_SEGMENT);
        self.memory.write_bytes(psp, 0x50, &[0xCD, 0x21, 0xCB]);
        self.memory.write_bytes(psp, 0x5C, &[0, b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ']);
        let mut text: Vec<u8> = tail.bytes().take(126).collect();
        if !text.is_empty() && text[0] != b' ' {
            text.insert(0, b' ');
            text.truncate(126);
        }
        self.memory.write8_at(psp, 0x80, text.len() as u8);
        self.memory.write_bytes(psp, 0x81, &text);
        self.memory.write8_at(psp, 0x81 + text.len() as u16, 0x0D);
    }

    fn load_executable(&mut self, bytes: &[u8]) -> Result<(), String> {
        let executable = mz::parse(bytes)?;
        let load_segment = PSP_SEGMENT + 0x10;
        let available = ((TOP_SEGMENT - load_segment) as usize) << 4;
        let required = executable.image.len() + ((executable.min_alloc as usize) << 4);
        if required > available {
            return Err("Program too big to fit in memory".to_string());
        }
        self.memory.load((load_segment as u32) << 4, &executable.image);
        for (offset, segment) in &executable.relocations {
            let target = segment.wrapping_add(load_segment);
            let value = self.memory.read16_at(target, *offset).wrapping_add(load_segment);
            self.memory.write16_at(target, *offset, value);
        }
        let regs = &mut self.cpu.regs;
        regs.segment[CS] = executable.cs.wrapping_add(load_segment);
        regs.ip = executable.ip;
        regs.segment[SS] = executable.ss.wrapping_add(load_segment);
        regs.general[SP] = executable.sp;
        regs.segment[DS] = PSP_SEGMENT;
        regs.segment[ES] = PSP_SEGMENT;
        Ok(())
    }

    fn load_com(&mut self, bytes: &[u8]) -> Result<(), String> {
        if bytes.len() > COM_LIMIT {
            return Err("Program too big to fit in memory".to_string());
        }
        self.memory.write_bytes(PSP_SEGMENT, 0x100, bytes);
        let regs = &mut self.cpu.regs;
        regs.segment = [PSP_SEGMENT; 4];
        regs.ip = 0x100;
        regs.general[SP] = 0xFFFE;
        self.memory.write16_at(PSP_SEGMENT, 0xFFFE, 0);
        Ok(())
    }
}
