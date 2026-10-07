pub const CF: u16 = 0x0001;
pub const PF: u16 = 0x0004;
pub const AF: u16 = 0x0010;
pub const ZF: u16 = 0x0040;
pub const SF: u16 = 0x0080;
pub const TF: u16 = 0x0100;
pub const IF: u16 = 0x0200;
pub const DF: u16 = 0x0400;
pub const OF: u16 = 0x0800;

const WRITABLE: u16 = CF | PF | AF | ZF | SF | TF | IF | DF | OF;
const ALWAYS_SET: u16 = 0xF002;

#[derive(Clone, Copy)]
pub struct Flags {
    bits: u16,
}

impl Default for Flags {
    fn default() -> Self {
        Flags { bits: ALWAYS_SET | IF }
    }
}

impl Flags {
    pub fn bits(&self) -> u16 {
        self.bits
    }

    pub fn load(&mut self, value: u16) {
        self.bits = (value & WRITABLE) | ALWAYS_SET;
    }

    pub fn get(&self, flag: u16) -> bool {
        self.bits & flag != 0
    }

    pub fn set(&mut self, flag: u16, on: bool) {
        if on {
            self.bits |= flag;
        } else {
            self.bits &= !flag;
        }
    }

    pub fn carry(&self) -> bool {
        self.get(CF)
    }

    pub fn zero(&self) -> bool {
        self.get(ZF)
    }

    pub fn set_result(&mut self, value: u32, word: bool) {
        let mask = if word { 0xFFFF } else { 0xFF };
        let sign = if word { 0x8000 } else { 0x80 };
        let value = value & mask;
        self.set(ZF, value == 0);
        self.set(SF, value & sign != 0);
        self.set(PF, (value as u8).count_ones() % 2 == 0);
    }

    pub fn condition(&self, code: u8) -> bool {
        let result = match code >> 1 {
            0 => self.get(OF),
            1 => self.get(CF),
            2 => self.get(ZF),
            3 => self.get(CF) || self.get(ZF),
            4 => self.get(SF),
            5 => self.get(PF),
            6 => self.get(SF) != self.get(OF),
            _ => self.get(ZF) || (self.get(SF) != self.get(OF)),
        };
        if code & 1 == 1 { !result } else { result }
    }
}
