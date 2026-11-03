use dosbox::terminal::Input;
use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;
use std::sync::{Condvar, Mutex, OnceLock};

pub const COLUMNS: usize = 80;
pub const ROWS: usize = 25;
pub const CELL_BYTES: usize = COLUMNS * ROWS * 2;
pub const SCREEN_BYTES: usize = CELL_BYTES + 4;

pub struct Screen {
    pub cells: Vec<u8>,
    pub cursor_row: u8,
    pub cursor_column: u8,
    pub cursor_visible: bool,
    pub blink: bool,
    pub version: i32,
}

pub struct Bridge {
    pub keys: Mutex<VecDeque<Input>>,
    pub key_ready: Condvar,
    pub screen: Mutex<Screen>,
    pub running: AtomicBool,
    pub stop: AtomicBool,
}

pub fn bridge() -> &'static Bridge {
    static BRIDGE: OnceLock<Bridge> = OnceLock::new();
    BRIDGE.get_or_init(|| Bridge {
        keys: Mutex::new(VecDeque::new()),
        key_ready: Condvar::new(),
        screen: Mutex::new(Screen {
            cells: vec![0; CELL_BYTES],
            cursor_row: 0,
            cursor_column: 0,
            cursor_visible: true,
            blink: true,
            version: 0,
        }),
        running: AtomicBool::new(false),
        stop: AtomicBool::new(false),
    })
}

pub fn push(input: Input) {
    let bridge = bridge();
    if let Ok(mut keys) = bridge.keys.lock() {
        if keys.len() < 64 {
            keys.push_back(input);
        }
    }
    bridge.key_ready.notify_all();
}

pub fn snapshot(output: &mut [u8; SCREEN_BYTES]) -> i32 {
    let Ok(screen) = bridge().screen.lock() else {
        return 0;
    };
    output[..CELL_BYTES].copy_from_slice(&screen.cells);
    output[CELL_BYTES] = screen.cursor_row;
    output[CELL_BYTES + 1] = screen.cursor_column;
    output[CELL_BYTES + 2] = screen.cursor_visible as u8;
    output[CELL_BYTES + 3] = screen.blink as u8;
    screen.version
}
