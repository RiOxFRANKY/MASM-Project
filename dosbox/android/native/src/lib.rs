mod bridge;
mod jni;
mod session;
mod terminal;

use bridge::{push, snapshot, SCREEN_BYTES};
use dosbox::terminal::{keymap, Input, Key};
use jni::{read_bytes, write_bytes, JniEnv, JObject};
use std::path::PathBuf;
use std::sync::atomic::Ordering;

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_rioxfranky_dosbox_NativeBridge_start(env: JniEnv, _class: JObject, root: JObject) {
    let bytes = unsafe { read_bytes(env, root) };
    let path = PathBuf::from(String::from_utf8_lossy(&bytes).into_owned());
    session::start(path);
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_rioxfranky_dosbox_NativeBridge_stop(_env: JniEnv, _class: JObject) {
    session::stop();
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_rioxfranky_dosbox_NativeBridge_isRunning(_env: JniEnv, _class: JObject) -> u8 {
    bridge::bridge().running.load(Ordering::SeqCst) as u8
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_rioxfranky_dosbox_NativeBridge_sendKey(_env: JniEnv, _class: JObject, scan: i32, ascii: i32) {
    push(Input::Key(Key { scan: scan as u8, ascii: ascii as u8 }));
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_rioxfranky_dosbox_NativeBridge_sendChar(_env: JniEnv, _class: JObject, unicode: i32) {
    if let Some(character) = char::from_u32(unicode as u32) {
        push(Input::Key(keymap::from_char(character)));
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_rioxfranky_dosbox_NativeBridge_sendCtrl(_env: JniEnv, _class: JObject, letter: i32) {
    if letter == b'c' as i32 || letter == b'C' as i32 {
        push(Input::Break);
    } else if (letter as u32) < 0x80 && (letter as u8).is_ascii_alphabetic() {
        push(Input::Key(keymap::control(letter as u8)));
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_rioxfranky_dosbox_NativeBridge_sendBreak(_env: JniEnv, _class: JObject) {
    push(Input::Break);
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_rioxfranky_dosbox_NativeBridge_readScreen(env: JniEnv, _class: JObject, output: JObject) -> i32 {
    let mut buffer = [0u8; SCREEN_BYTES];
    let version = snapshot(&mut buffer);
    unsafe { write_bytes(env, output, &buffer) };
    version
}
