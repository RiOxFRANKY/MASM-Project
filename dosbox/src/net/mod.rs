mod service;
mod websocket;

use std::collections::VecDeque;
use std::io;
use std::net::{Shutdown, TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use websocket::{
    client_handshake, read_frame, write_masked, OPCODE_BINARY, OPCODE_CLOSE, OPCODE_CONTINUATION, OPCODE_PING,
    OPCODE_PONG, OPCODE_TEXT,
};

pub const STATUS_CLOSED: u8 = 0;
pub const STATUS_CONNECTING: u8 = 1;
pub const STATUS_OPEN: u8 = 2;
pub const STATUS_FAILED: u8 = 3;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(4);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
const INBOX_LIMIT: usize = 512;

struct Inner {
    status: u8,
    writer: Option<TcpStream>,
    inbox: VecDeque<Vec<u8>>,
    generation: u64,
}

pub struct Network {
    inner: Arc<Mutex<Inner>>,
}

impl Network {
    pub fn new() -> Self {
        Network {
            inner: Arc::new(Mutex::new(Inner {
                status: STATUS_CLOSED,
                writer: None,
                inbox: VecDeque::new(),
                generation: 0,
            })),
        }
    }

    pub fn connect(&mut self, host: String, port: u16) {
        let generation = {
            let Ok(mut inner) = self.inner.lock() else {
                return;
            };
            inner.generation += 1;
            if let Some(writer) = inner.writer.take() {
                let _ = writer.shutdown(Shutdown::Both);
            }
            inner.inbox.clear();
            inner.status = STATUS_CONNECTING;
            inner.generation
        };
        let shared = Arc::clone(&self.inner);
        std::thread::spawn(move || run(shared, generation, host, port));
    }

    pub fn status(&self) -> u8 {
        self.inner.lock().map(|inner| inner.status).unwrap_or(STATUS_FAILED)
    }

    pub fn send(&self, payload: &[u8]) -> bool {
        let Ok(mut inner) = self.inner.lock() else {
            return false;
        };
        match inner.writer.as_mut() {
            Some(writer) => write_masked(writer, OPCODE_BINARY, payload).is_ok(),
            None => false,
        }
    }

    pub fn receive(&self) -> Option<Vec<u8>> {
        self.inner.lock().ok()?.inbox.pop_front()
    }

    pub fn close(&mut self) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        inner.generation += 1;
        if let Some(mut writer) = inner.writer.take() {
            let _ = write_masked(&mut writer, OPCODE_CLOSE, &[]);
            let _ = writer.shutdown(Shutdown::Both);
        }
        inner.inbox.clear();
        inner.status = STATUS_CLOSED;
    }
}

impl Default for Network {
    fn default() -> Self {
        Self::new()
    }
}

fn open(host: &str, port: u16) -> io::Result<TcpStream> {
    let address = (host, port)
        .to_socket_addrs()?
        .find(|address| address.is_ipv4())
        .ok_or_else(|| io::Error::other("no IPv4 address for that host"))?;
    let mut stream = TcpStream::connect_timeout(&address, CONNECT_TIMEOUT)?;
    stream.set_nodelay(true)?;
    stream.set_read_timeout(Some(HANDSHAKE_TIMEOUT))?;
    client_handshake(&mut stream, host, port)?;
    stream.set_read_timeout(None)?;
    Ok(stream)
}

fn run(shared: Arc<Mutex<Inner>>, generation: u64, host: String, port: u16) {
    let stream = match open(&host, port).and_then(|stream| stream.try_clone().map(|writer| (stream, writer))) {
        Ok(pair) => pair,
        Err(_) => {
            if let Ok(mut inner) = shared.lock() {
                if inner.generation == generation {
                    inner.status = STATUS_FAILED;
                }
            }
            return;
        }
    };
    let (mut reader, writer) = stream;
    {
        let Ok(mut inner) = shared.lock() else {
            return;
        };
        if inner.generation != generation {
            let _ = writer.shutdown(Shutdown::Both);
            return;
        }
        inner.writer = Some(writer);
        inner.status = STATUS_OPEN;
    }
    let mut partial: Vec<u8> = Vec::new();
    while let Ok(frame) = read_frame(&mut reader) {
        let Ok(mut inner) = shared.lock() else {
            return;
        };
        if inner.generation != generation {
            return;
        }
        match frame.opcode {
            OPCODE_BINARY | OPCODE_TEXT | OPCODE_CONTINUATION => {
                partial.extend_from_slice(&frame.payload);
                if inner.inbox.len() < INBOX_LIMIT {
                    inner.inbox.push_back(std::mem::take(&mut partial));
                }
            }
            OPCODE_PING => {
                if let Some(writer) = inner.writer.as_mut() {
                    let _ = write_masked(writer, OPCODE_PONG, &frame.payload);
                }
            }
            OPCODE_CLOSE => break,
            _ => {}
        }
    }
    if let Ok(mut inner) = shared.lock() {
        if inner.generation == generation {
            inner.status = STATUS_CLOSED;
            inner.writer = None;
        }
    }
}
