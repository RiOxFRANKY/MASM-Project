use crate::cpu::PortBus;

pub struct Ports {
    retrace_toggle: u8,
    timer_counter: u16,
    speaker: u8,
}

impl Ports {
    pub fn new() -> Self {
        Ports { retrace_toggle: 0, timer_counter: 0xFFFF, speaker: 0 }
    }
}

impl PortBus for Ports {
    fn port_in(&mut self, port: u16, _word: bool) -> u16 {
        match port {
            0x3DA | 0x3BA => {
                self.retrace_toggle ^= 0x09;
                self.retrace_toggle as u16
            }
            0x40 => {
                self.timer_counter = self.timer_counter.wrapping_sub(0x1F3);
                self.timer_counter & 0xFF
            }
            0x61 => self.speaker as u16,
            _ => 0xFF,
        }
    }

    fn port_out(&mut self, port: u16, value: u16, _word: bool) {
        if port == 0x61 {
            self.speaker = value as u8;
        }
    }
}
