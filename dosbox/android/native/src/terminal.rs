use crate::bridge::{bridge, CELL_BYTES};
use dosbox::terminal::{Frame, Input, Terminal};
use std::sync::atomic::Ordering;
use std::time::Duration;

pub struct AndroidTerminal;

impl Terminal for AndroidTerminal {
    fn present(&mut self, frame: &Frame) {
        let Ok(mut screen) = bridge().screen.lock() else {
            return;
        };
        let length = frame.cells.len().min(CELL_BYTES);
        screen.cells[..length].copy_from_slice(&frame.cells[..length]);
        screen.cursor_row = frame.cursor_row as u8;
        screen.cursor_column = frame.cursor_column as u8;
        screen.cursor_visible = frame.cursor_visible;
        screen.blink = frame.blink;
        screen.version = screen.version.wrapping_add(1);
    }

    fn poll(&mut self, timeout: Duration) -> Option<Input> {
        let bridge = bridge();
        if bridge.stop.load(Ordering::SeqCst) {
            return Some(Input::Quit);
        }
        let mut keys = bridge.keys.lock().ok()?;
        if keys.is_empty() && !timeout.is_zero() {
            keys = bridge.key_ready.wait_timeout(keys, timeout).ok()?.0;
        }
        if bridge.stop.load(Ordering::SeqCst) {
            return Some(Input::Quit);
        }
        keys.pop_front()
    }

    fn finish(&mut self, frame: &Frame) {
        self.present(frame);
    }
}
