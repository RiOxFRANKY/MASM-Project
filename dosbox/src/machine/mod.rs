mod input;
mod interrupts;

use crate::cpu::flags::{AF, CF, OF, PF, SF, ZF};
use crate::cpu::registers::CS;
use crate::cpu::{Cpu, Step};
use crate::dos::filesystem::Drive;
use crate::dos::Dos;
use crate::hardware::clock;
use crate::hardware::keyboard::Keyboard;
use crate::hardware::memory::Memory;
use crate::hardware::ports::Ports;
use crate::hardware::video::{Video, COLUMNS, ROWS};
use crate::terminal::{Frame, Terminal};
use std::time::{Duration, Instant};

pub const BIOS_SEGMENT: u16 = 0xF000;
pub const STUB_BASE: u16 = 0x1000;
const HOUSEKEEPING_INTERVAL: u32 = 20_000;
const PRESENT_INTERVAL: Duration = Duration::from_millis(40);
const RESULT_FLAGS: u16 = CF | ZF | SF | OF | PF | AF;

pub enum Halt {
    Exit(u8),
    Break,
    Quit,
    Fault(String),
}

pub struct Machine {
    pub cpu: Cpu,
    pub memory: Memory,
    pub video: Video,
    pub keyboard: Keyboard,
    pub ports: Ports,
    pub dos: Dos,
    pub drive: Drive,
    terminal: Box<dyn Terminal>,
    last_present: Instant,
}

fn build_frame<'a>(video: &Video, memory: &'a Memory) -> Frame<'a> {
    Frame {
        cells: video.text_cells(memory),
        columns: COLUMNS,
        rows: ROWS,
        cursor_row: video.row.min(ROWS - 1),
        cursor_column: video.column.min(COLUMNS - 1),
        cursor_visible: video.cursor_visible(),
        blink: video.blink,
    }
}

impl Machine {
    pub fn new(terminal: Box<dyn Terminal>, drive: Drive) -> Self {
        let mut machine = Machine {
            cpu: Cpu::new(),
            memory: Memory::new(),
            video: Video::new(),
            keyboard: Keyboard::new(),
            ports: Ports::new(),
            dos: Dos::new(),
            drive,
            terminal,
            last_present: Instant::now(),
        };
        machine.install_vectors();
        machine.memory.write16(0x410, 0x0021);
        machine.memory.write16(0x413, 640);
        clock::store_ticks(&mut machine.memory);
        machine.video.set_mode(&mut machine.memory, 3);
        machine
    }

    pub fn print(&mut self, text: &str) {
        self.video.print(&mut self.memory, text);
    }

    pub fn present(&mut self) {
        let frame = build_frame(&self.video, &self.memory);
        self.terminal.present(&frame);
        self.last_present = Instant::now();
    }

    pub fn finish(&mut self) {
        let frame = build_frame(&self.video, &self.memory);
        self.terminal.finish(&frame);
    }

    pub fn run(&mut self) -> Halt {
        let mut counter = 0u32;
        loop {
            let segment = self.cpu.regs.segment[CS];
            let offset = self.cpu.regs.ip;
            if segment == BIOS_SEGMENT && (STUB_BASE..STUB_BASE + 0x100).contains(&offset) {
                let vector = (offset - STUB_BASE) as u8;
                if let Err(halt) = self.service(vector) {
                    return halt;
                }
                self.cpu.return_from_interrupt_keeping(&self.memory, RESULT_FLAGS);
                continue;
            }
            match self.cpu.step(&mut self.memory, &mut self.ports) {
                Ok(Step::Continue) => {}
                Ok(Step::Interrupt(vector)) => self.cpu.enter_interrupt(&mut self.memory, vector),
                Ok(Step::Halt) => {
                    if let Err(halt) = self.idle() {
                        return halt;
                    }
                }
                Err(fault) => {
                    return Halt::Fault(format!(
                        "Illegal instruction {:02X}h at {:04X}:{:04X}",
                        fault.opcode, fault.segment, fault.offset
                    ));
                }
            }
            counter += 1;
            if counter >= HOUSEKEEPING_INTERVAL {
                counter = 0;
                if let Err(halt) = self.housekeeping() {
                    return halt;
                }
            }
        }
    }

    fn housekeeping(&mut self) -> Result<(), Halt> {
        clock::store_ticks(&mut self.memory);
        if self.last_present.elapsed() >= PRESENT_INTERVAL {
            self.present();
        }
        if self.keyboard.is_empty() {
            self.pump(Duration::ZERO)?;
        }
        Ok(())
    }
}
