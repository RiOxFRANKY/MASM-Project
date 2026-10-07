pub const AX: usize = 0;
pub const CX: usize = 1;
pub const DX: usize = 2;
pub const BX: usize = 3;
pub const SP: usize = 4;
pub const BP: usize = 5;
pub const SI: usize = 6;
pub const DI: usize = 7;

pub const ES: usize = 0;
pub const CS: usize = 1;
pub const SS: usize = 2;
pub const DS: usize = 3;

#[derive(Clone, Default)]
pub struct Registers {
    pub general: [u16; 8],
    pub segment: [u16; 4],
    pub ip: u16,
}

impl Registers {
    pub fn get16(&self, index: usize) -> u16 {
        self.general[index & 7]
    }

    pub fn set16(&mut self, index: usize, value: u16) {
        self.general[index & 7] = value;
    }

    pub fn get8(&self, index: usize) -> u8 {
        let index = index & 7;
        if index < 4 {
            self.general[index] as u8
        } else {
            (self.general[index - 4] >> 8) as u8
        }
    }

    pub fn set8(&mut self, index: usize, value: u8) {
        let index = index & 7;
        if index < 4 {
            self.general[index] = (self.general[index] & 0xFF00) | value as u16;
        } else {
            let slot = index - 4;
            self.general[slot] = (self.general[slot] & 0x00FF) | ((value as u16) << 8);
        }
    }

    pub fn ax(&self) -> u16 {
        self.general[AX]
    }
    pub fn bx(&self) -> u16 {
        self.general[BX]
    }
    pub fn cx(&self) -> u16 {
        self.general[CX]
    }
    pub fn dx(&self) -> u16 {
        self.general[DX]
    }
    pub fn al(&self) -> u8 {
        self.get8(0)
    }
    pub fn ah(&self) -> u8 {
        self.get8(4)
    }
    pub fn bl(&self) -> u8 {
        self.get8(3)
    }
    pub fn bh(&self) -> u8 {
        self.get8(7)
    }
    pub fn cl(&self) -> u8 {
        self.get8(1)
    }
    pub fn ch(&self) -> u8 {
        self.get8(5)
    }
    pub fn dl(&self) -> u8 {
        self.get8(2)
    }
    pub fn dh(&self) -> u8 {
        self.get8(6)
    }

    pub fn set_ax(&mut self, value: u16) {
        self.general[AX] = value;
    }
    pub fn set_bx(&mut self, value: u16) {
        self.general[BX] = value;
    }
    pub fn set_cx(&mut self, value: u16) {
        self.general[CX] = value;
    }
    pub fn set_dx(&mut self, value: u16) {
        self.general[DX] = value;
    }
    pub fn set_al(&mut self, value: u8) {
        self.set8(0, value);
    }
    pub fn set_ah(&mut self, value: u8) {
        self.set8(4, value);
    }
    pub fn set_bl(&mut self, value: u8) {
        self.set8(3, value);
    }
    pub fn set_bh(&mut self, value: u8) {
        self.set8(7, value);
    }
    pub fn set_cl(&mut self, value: u8) {
        self.set8(1, value);
    }
    pub fn set_ch(&mut self, value: u8) {
        self.set8(5, value);
    }
    pub fn set_dl(&mut self, value: u8) {
        self.set8(2, value);
    }
    pub fn set_dh(&mut self, value: u8) {
        self.set8(6, value);
    }

    pub fn ds(&self) -> u16 {
        self.segment[DS]
    }
    pub fn es(&self) -> u16 {
        self.segment[ES]
    }
}
