use super::ansi::{AnsiRenderer, ENTER_SCREEN, LEAVE_SCREEN};
use super::{keymap, Frame, Input, Key, Terminal};
use std::ffi::c_void;
use std::io::Write;
use std::time::{Duration, Instant};

type Handle = *mut c_void;

const STD_INPUT_HANDLE: u32 = -10i32 as u32;
const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;
const KEY_EVENT: u16 = 0x0001;
const ENABLE_EXTENDED_FLAGS: u32 = 0x0080;
const ENABLE_PROCESSED_OUTPUT: u32 = 0x0001;
const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;
const RIGHT_ALT: u32 = 0x0001;
const LEFT_ALT: u32 = 0x0002;
const RIGHT_CTRL: u32 = 0x0004;
const LEFT_CTRL: u32 = 0x0008;
const VK_F9: u16 = 0x78;
const VK_CANCEL: u16 = 0x03;
const WAIT_OBJECT_0: u32 = 0;

#[repr(C)]
#[derive(Default)]
struct InputRecord {
    event_type: u16,
    padding: u16,
    key_down: i32,
    repeat_count: u16,
    virtual_key: u16,
    virtual_scan: u16,
    unicode_char: u16,
    control_state: u32,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetStdHandle(which: u32) -> Handle;
    fn GetConsoleMode(handle: Handle, mode: *mut u32) -> i32;
    fn SetConsoleMode(handle: Handle, mode: u32) -> i32;
    fn WaitForSingleObject(handle: Handle, milliseconds: u32) -> u32;
    fn ReadConsoleInputW(handle: Handle, buffer: *mut InputRecord, length: u32, read: *mut u32) -> i32;
}

pub struct WindowsConsole {
    input: Handle,
    output: Handle,
    input_mode: u32,
    output_mode: u32,
    renderer: AnsiRenderer,
}

impl WindowsConsole {
    pub fn open() -> std::io::Result<Self> {
        unsafe {
            let input = GetStdHandle(STD_INPUT_HANDLE);
            let output = GetStdHandle(STD_OUTPUT_HANDLE);
            let mut input_mode = 0;
            let mut output_mode = 0;
            if GetConsoleMode(input, &mut input_mode) == 0 || GetConsoleMode(output, &mut output_mode) == 0 {
                return Err(std::io::Error::other("stdin/stdout is not a console window"));
            }
            SetConsoleMode(input, ENABLE_EXTENDED_FLAGS);
            SetConsoleMode(output, output_mode | ENABLE_PROCESSED_OUTPUT | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
            let console = WindowsConsole { input, output, input_mode, output_mode, renderer: AnsiRenderer::new() };
            print!("{ENTER_SCREEN}");
            let _ = std::io::stdout().flush();
            Ok(console)
        }
    }

    fn translate(record: &InputRecord) -> Option<Input> {
        if record.event_type != KEY_EVENT || record.key_down == 0 {
            return None;
        }
        let control = record.control_state & (LEFT_CTRL | RIGHT_CTRL) != 0;
        let alt = record.control_state & (LEFT_ALT | RIGHT_ALT) != 0;
        if control && record.virtual_key == VK_F9 {
            return Some(Input::Quit);
        }
        if record.virtual_key == VK_CANCEL || (control && !alt && record.virtual_key == b'C' as u16) {
            return Some(Input::Break);
        }
        if matches!(record.virtual_key, 0x10 | 0x11 | 0x12 | 0x14 | 0x5B | 0x5C | 0x90 | 0x91) {
            return None;
        }
        let scan = record.virtual_scan as u8;
        let ascii = if alt && !control {
            0
        } else if record.unicode_char == 0 {
            0
        } else {
            super::cp437::from_unicode(char::from_u32(record.unicode_char as u32).unwrap_or('?'))
        };
        if scan == 0 && ascii == 0 {
            return None;
        }
        let scan = if scan == 0 { keymap::scan_for_char(ascii) } else { scan };
        Some(Input::Key(Key { scan, ascii }))
    }
}

impl Terminal for WindowsConsole {
    fn present(&mut self, frame: &Frame) {
        let text = self.renderer.render(frame);
        let mut stdout = std::io::stdout().lock();
        let _ = stdout.write_all(text.as_bytes());
        let _ = stdout.flush();
    }

    fn poll(&mut self, timeout: Duration) -> Option<Input> {
        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let waited = unsafe { WaitForSingleObject(self.input, remaining.as_millis() as u32) };
            if waited != WAIT_OBJECT_0 {
                return None;
            }
            let mut record = InputRecord::default();
            let mut read = 0;
            let success = unsafe { ReadConsoleInputW(self.input, &mut record, 1, &mut read) };
            if success != 0 && read == 1 {
                if let Some(input) = Self::translate(&record) {
                    return Some(input);
                }
            }
            if Instant::now() >= deadline {
                return None;
            }
        }
    }
}

impl Drop for WindowsConsole {
    fn drop(&mut self) {
        print!("{LEAVE_SCREEN}");
        let _ = std::io::stdout().flush();
        unsafe {
            SetConsoleMode(self.input, self.input_mode);
            SetConsoleMode(self.output, self.output_mode);
        }
    }
}
