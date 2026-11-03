pub const QUEUE: u8 = b'Q';
pub const MATCHED: u8 = b'M';
pub const READY: u8 = b'Y';
pub const GO: u8 = b'G';
pub const MOVE: u8 = b'O';
pub const FINISHED: u8 = b'F';
pub const DISTANCE: u8 = b'D';
pub const RESULT: u8 = b'R';
pub const LEFT: u8 = b'X';

pub const DRAW: u8 = 2;
pub const BY_EXIT: u8 = 0;
pub const BY_TIME: u8 = 1;

pub fn matched(slot: u8, seed: u16) -> Vec<u8> {
    let seed = seed.to_le_bytes();
    vec![MATCHED, slot, seed[0], seed[1]]
}

pub fn go(level: u8) -> Vec<u8> {
    vec![GO, level]
}

pub fn result(level: u8, winner: u8, reason: u8) -> Vec<u8> {
    vec![RESULT, level, winner, reason]
}

pub fn left() -> Vec<u8> {
    vec![LEFT]
}
