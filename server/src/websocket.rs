use crate::sha1::sha1;
use std::io::{self, Read, Write};
use std::net::TcpStream;

pub const OPCODE_CONTINUATION: u8 = 0x0;
pub const OPCODE_TEXT: u8 = 0x1;
pub const OPCODE_BINARY: u8 = 0x2;
pub const OPCODE_CLOSE: u8 = 0x8;
pub const OPCODE_PING: u8 = 0x9;
pub const OPCODE_PONG: u8 = 0xA;

const ACCEPT_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
const MAX_PAYLOAD: u64 = 64 * 1024;
const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub struct Frame {
    pub opcode: u8,
    pub payload: Vec<u8>,
}

fn base64(bytes: &[u8]) -> String {
    let mut text = String::new();
    for chunk in bytes.chunks(3) {
        let value = (chunk[0] as u32) << 16 | (*chunk.get(1).unwrap_or(&0) as u32) << 8 | *chunk.get(2).unwrap_or(&0) as u32;
        for position in 0..4 {
            if position <= chunk.len() {
                text.push(BASE64[(value >> (18 - position * 6)) as usize & 63] as char);
            } else {
                text.push('=');
            }
        }
    }
    text
}

pub fn accept_key(key: &str) -> String {
    base64(&sha1(format!("{}{ACCEPT_GUID}", key.trim()).as_bytes()))
}

pub fn server_handshake(stream: &mut TcpStream) -> io::Result<()> {
    let mut request = Vec::new();
    let mut byte = [0u8; 1];
    while !request.ends_with(b"\r\n\r\n") {
        stream.read_exact(&mut byte)?;
        request.push(byte[0]);
        if request.len() > 8192 {
            return Err(io::Error::other("request too long"));
        }
    }
    let text = String::from_utf8_lossy(&request);
    let key = text.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.trim().eq_ignore_ascii_case("sec-websocket-key").then(|| value.trim().to_string())
    });
    let Some(key) = key else {
        let body = "MAZE PvP server is running. Connect from MAZE.EXE (menu option 2).\n";
        let reply = format!(
            "HTTP/1.1 400 Bad Request\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = stream.write_all(reply.as_bytes());
        return Err(io::Error::other("not a WebSocket request"));
    };
    let reply = format!(
        "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {}\r\n\r\n",
        accept_key(&key)
    );
    stream.write_all(reply.as_bytes())
}

pub fn write_frame(stream: &mut TcpStream, opcode: u8, payload: &[u8]) -> io::Result<()> {
    let mut frame = Vec::with_capacity(payload.len() + 10);
    frame.push(0x80 | opcode);
    let length = payload.len();
    if length < 126 {
        frame.push(length as u8);
    } else if length <= 0xFFFF {
        frame.push(126);
        frame.extend_from_slice(&(length as u16).to_be_bytes());
    } else {
        frame.push(127);
        frame.extend_from_slice(&(length as u64).to_be_bytes());
    }
    frame.extend_from_slice(payload);
    stream.write_all(&frame)
}

pub fn read_frame(stream: &mut impl Read) -> io::Result<Frame> {
    let mut head = [0u8; 2];
    stream.read_exact(&mut head)?;
    let opcode = head[0] & 0x0F;
    let masked = head[1] & 0x80 != 0;
    let mut length = (head[1] & 0x7F) as u64;
    if length == 126 {
        let mut extended = [0u8; 2];
        stream.read_exact(&mut extended)?;
        length = u16::from_be_bytes(extended) as u64;
    } else if length == 127 {
        let mut extended = [0u8; 8];
        stream.read_exact(&mut extended)?;
        length = u64::from_be_bytes(extended);
    }
    if length > MAX_PAYLOAD {
        return Err(io::Error::other("frame too large"));
    }
    let mut mask = [0u8; 4];
    if masked {
        stream.read_exact(&mut mask)?;
    }
    let mut payload = vec![0u8; length as usize];
    stream.read_exact(&mut payload)?;
    if masked {
        payload.iter_mut().enumerate().for_each(|(index, byte)| *byte ^= mask[index % 4]);
    }
    Ok(Frame { opcode, payload })
}

#[cfg(test)]
mod tests {
    use super::{accept_key, base64};
    use crate::sha1::sha1;

    #[test]
    fn sha1_of_abc() {
        let hex: String = sha1(b"abc").iter().map(|byte| format!("{byte:02x}")).collect();
        assert_eq!(hex, "a9993e364706816aba3e25717850c26c9cd0d89d");
    }

    #[test]
    fn base64_examples() {
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(b"Ma"), "TWE=");
        assert_eq!(base64(b"M"), "TQ==");
    }

    #[test]
    fn accept_key_matches_rfc_example() {
        assert_eq!(accept_key("dGhlIHNhbXBsZSBub25jZQ=="), "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    }
}
