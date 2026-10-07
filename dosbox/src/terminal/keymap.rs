use super::Key;

pub const ESCAPE: Key = Key { scan: 0x01, ascii: 0x1B };
pub const ENTER: Key = Key { scan: 0x1C, ascii: 0x0D };
pub const BACKSPACE: Key = Key { scan: 0x0E, ascii: 0x08 };
pub const TAB: Key = Key { scan: 0x0F, ascii: 0x09 };
pub const UP: Key = Key { scan: 0x48, ascii: 0 };
pub const DOWN: Key = Key { scan: 0x50, ascii: 0 };
pub const LEFT: Key = Key { scan: 0x4B, ascii: 0 };
pub const RIGHT: Key = Key { scan: 0x4D, ascii: 0 };
pub const HOME: Key = Key { scan: 0x47, ascii: 0 };
pub const END: Key = Key { scan: 0x4F, ascii: 0 };
pub const PAGE_UP: Key = Key { scan: 0x49, ascii: 0 };
pub const PAGE_DOWN: Key = Key { scan: 0x51, ascii: 0 };
pub const INSERT: Key = Key { scan: 0x52, ascii: 0 };
pub const DELETE: Key = Key { scan: 0x53, ascii: 0 };

const ROWS: [(u8, &str, &str); 4] = [
    (0x02, "1234567890-=", "!@#$%^&*()_+"),
    (0x10, "qwertyuiop[]", "QWERTYUIOP{}"),
    (0x1E, "asdfghjkl;'`", "ASDFGHJKL:\"~"),
    (0x2C, "zxcvbnm,./", "ZXCVBNM<>?"),
];

pub fn scan_for_char(character: u8) -> u8 {
    if character == b' ' {
        return 0x39;
    }
    if character == b'\\' || character == b'|' {
        return 0x2B;
    }
    for (first_scan, plain, shifted) in ROWS {
        if let Some(position) = plain.bytes().position(|byte| byte == character) {
            return first_scan + position as u8;
        }
        if let Some(position) = shifted.bytes().position(|byte| byte == character) {
            return first_scan + position as u8;
        }
    }
    0
}

pub fn from_char(character: char) -> Key {
    match character {
        '\r' | '\n' => ENTER,
        '\x08' | '\x7F' => BACKSPACE,
        '\t' => TAB,
        '\x1B' => ESCAPE,
        _ => {
            let ascii = super::cp437::from_unicode(character);
            Key { scan: scan_for_char(ascii), ascii }
        }
    }
}

pub fn control(letter: u8) -> Key {
    let lower = letter.to_ascii_lowercase();
    Key { scan: scan_for_char(lower), ascii: lower & 0x1F }
}

pub fn function(number: u8) -> Key {
    let scan = match number {
        1..=10 => 0x3A + number,
        11 => 0x85,
        _ => 0x86,
    };
    Key { scan, ascii: 0 }
}

pub fn named(name: &str) -> Option<Key> {
    let name = name.to_ascii_lowercase();
    let key = match name.as_str() {
        "esc" | "escape" => ESCAPE,
        "enter" | "return" => ENTER,
        "backspace" | "bs" => BACKSPACE,
        "tab" => TAB,
        "up" => UP,
        "down" => DOWN,
        "left" => LEFT,
        "right" => RIGHT,
        "home" => HOME,
        "end" => END,
        "pgup" => PAGE_UP,
        "pgdn" => PAGE_DOWN,
        "ins" | "insert" => INSERT,
        "del" | "delete" => DELETE,
        "space" => Key { scan: 0x39, ascii: b' ' },
        "lbrace" => from_char('{'),
        "rbrace" => from_char('}'),
        _ => {
            if let Some(number) = name.strip_prefix('f').and_then(|rest| rest.parse::<u8>().ok()) {
                if (1..=12).contains(&number) {
                    return Some(function(number));
                }
            }
            if let Some(letter) = name.strip_prefix("ctrl+") {
                if letter.len() == 1 {
                    return Some(control(letter.as_bytes()[0]));
                }
            }
            return None;
        }
    };
    Some(key)
}
