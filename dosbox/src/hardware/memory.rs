pub const MEMORY_SIZE: usize = 0x10_0000;
const ADDRESS_MASK: u32 = 0xF_FFFF;

pub struct Memory {
    data: Vec<u8>,
}

impl Memory {
    pub fn new() -> Self {
        Memory { data: vec![0; MEMORY_SIZE] }
    }

    pub fn linear(segment: u16, offset: u16) -> u32 {
        ((segment as u32) << 4).wrapping_add(offset as u32) & ADDRESS_MASK
    }

    pub fn read8(&self, address: u32) -> u8 {
        self.data[(address & ADDRESS_MASK) as usize]
    }

    pub fn write8(&mut self, address: u32, value: u8) {
        self.data[(address & ADDRESS_MASK) as usize] = value;
    }

    pub fn read16(&self, address: u32) -> u16 {
        let low = self.read8(address) as u16;
        let high = self.read8(address.wrapping_add(1)) as u16;
        low | (high << 8)
    }

    pub fn write16(&mut self, address: u32, value: u16) {
        self.write8(address, value as u8);
        self.write8(address.wrapping_add(1), (value >> 8) as u8);
    }

    pub fn read8_at(&self, segment: u16, offset: u16) -> u8 {
        self.read8(Self::linear(segment, offset))
    }

    pub fn write8_at(&mut self, segment: u16, offset: u16, value: u8) {
        self.write8(Self::linear(segment, offset), value);
    }

    pub fn read16_at(&self, segment: u16, offset: u16) -> u16 {
        let low = self.read8_at(segment, offset) as u16;
        let high = self.read8_at(segment, offset.wrapping_add(1)) as u16;
        low | (high << 8)
    }

    pub fn write16_at(&mut self, segment: u16, offset: u16, value: u16) {
        self.write8_at(segment, offset, value as u8);
        self.write8_at(segment, offset.wrapping_add(1), (value >> 8) as u8);
    }

    pub fn load(&mut self, address: u32, bytes: &[u8]) {
        for (index, byte) in bytes.iter().enumerate() {
            self.write8(address.wrapping_add(index as u32), *byte);
        }
    }

    pub fn slice(&self, address: u32, length: usize) -> &[u8] {
        let start = (address & ADDRESS_MASK) as usize;
        let end = (start + length).min(MEMORY_SIZE);
        &self.data[start..end]
    }

    pub fn read_bytes(&self, segment: u16, offset: u16, length: usize) -> Vec<u8> {
        (0..length)
            .map(|index| self.read8_at(segment, offset.wrapping_add(index as u16)))
            .collect()
    }

    pub fn write_bytes(&mut self, segment: u16, offset: u16, bytes: &[u8]) {
        for (index, byte) in bytes.iter().enumerate() {
            self.write8_at(segment, offset.wrapping_add(index as u16), *byte);
        }
    }

    pub fn read_asciiz(&self, segment: u16, offset: u16) -> String {
        let mut text = String::new();
        let mut cursor = offset;
        loop {
            let byte = self.read8_at(segment, cursor);
            if byte == 0 || text.len() >= 256 {
                break;
            }
            text.push(byte as char);
            cursor = cursor.wrapping_add(1);
        }
        text
    }
}
