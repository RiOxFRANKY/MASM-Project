use super::filesystem::wildcard_match;
use super::ERROR_NO_MORE_FILES;
use crate::hardware::clock;
use crate::machine::{Halt, Machine};

pub struct Found {
    name: String,
    attribute: u8,
    size: u32,
    date: u16,
    time: u16,
}

impl Machine {
    pub(crate) fn dos_find(&mut self) -> Result<(), Halt> {
        if self.cpu.regs.ah() == 0x4E {
            let pattern = self.memory.read_asciiz(self.cpu.regs.ds(), self.cpu.regs.dx());
            let include_directories = self.cpu.regs.cx() & 0x10 != 0;
            self.dos.search = self.search(&pattern, include_directories);
        }
        if self.dos.search.is_empty() {
            self.dos_error(ERROR_NO_MORE_FILES);
            return Ok(());
        }
        let found = self.dos.search.remove(0);
        let (segment, offset) = self.dos.dta;
        self.memory.write8_at(segment, offset.wrapping_add(0x15), found.attribute);
        self.memory.write16_at(segment, offset.wrapping_add(0x16), found.time);
        self.memory.write16_at(segment, offset.wrapping_add(0x18), found.date);
        self.memory.write16_at(segment, offset.wrapping_add(0x1A), found.size as u16);
        self.memory.write16_at(segment, offset.wrapping_add(0x1C), (found.size >> 16) as u16);
        let mut name = found.name.into_bytes();
        name.truncate(12);
        name.resize(13, 0);
        self.memory.write_bytes(segment, offset.wrapping_add(0x1E), &name);
        self.dos_ok();
        Ok(())
    }

    fn search(&self, pattern: &str, include_directories: bool) -> Vec<Found> {
        let normalized = pattern.replace('/', "\\");
        let (directory, mask) = match normalized.rfind(['\\', ':']) {
            Some(position) => (&normalized[..=position], &normalized[position + 1..]),
            None => ("", normalized.as_str()),
        };
        let mask = if mask.is_empty() { "*.*" } else { mask };
        let directory = if directory.is_empty() { "." } else { directory };
        let Ok(entries) = self.drive.list(directory) else {
            return Vec::new();
        };
        entries
            .into_iter()
            .filter(|entry| include_directories || !entry.is_directory)
            .filter(|entry| wildcard_match(mask, &entry.name))
            .map(|entry| {
                let stamp = entry.modified.map(clock::of_system_time).unwrap_or_else(clock::now);
                Found {
                    name: entry.name,
                    attribute: if entry.is_directory { 0x10 } else { 0x20 },
                    size: entry.size.min(u32::MAX as u64) as u32,
                    date: stamp.dos_date(),
                    time: stamp.dos_time(),
                }
            })
            .collect()
    }
}
