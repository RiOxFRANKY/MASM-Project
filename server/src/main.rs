mod leaderboard;
mod lobby;
mod protocol;
mod sha1;
mod websocket;

use leaderboard::Leaderboard;
use lobby::{Client, Lobby};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::process::ExitCode;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use websocket::{
    read_frame, server_handshake, HandshakeResult, OPCODE_BINARY, OPCODE_CLOSE, OPCODE_CONTINUATION, OPCODE_PING,
    OPCODE_PONG, OPCODE_TEXT,
};

const USAGE: &str = "Usage: maze <port>\n\nStarts the MAZE PvP server with Live Leaderboard.\nPlayers press 2 in MAZE.EXE and enter this computer's IP address and the port.\nOpen http://<this-ip>:<port> in any browser to view the Live Leaderboard.";

fn lan_address() -> Option<IpAddr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    Some(socket.local_addr().ok()?.ip())
}

fn handle_message(lobby: &Arc<Mutex<Lobby>>, client: &Arc<Client>, message: &[u8]) {
    let Some(&kind) = message.first() else {
        return;
    };
    let level = message.get(1).copied().unwrap_or(0);
    if kind == protocol::DISTANCE {
        let distance = u16::from_le_bytes([
            message.get(2).copied().unwrap_or(0xFF),
            message.get(3).copied().unwrap_or(0xFF),
        ]);
        Lobby::distance(lobby, client, level, distance);
        return;
    }
    if kind == protocol::NAME {
        if let Ok(name_str) = std::str::from_utf8(&message[1..]) {
            if let Ok(mut lobby) = lobby.lock() {
                lobby.set_name(client, name_str.to_string());
            }
        }
        return;
    }
    let Ok(mut lobby) = lobby.lock() else {
        return;
    };
    match kind {
        protocol::QUEUE => lobby.queue(client),
        protocol::MOVE => lobby.relay_move(client, message),
        protocol::READY => lobby.ready(client, level),
        protocol::FINISHED => lobby.finished(client, level),
        _ => {}
    }
}

fn serve(
    lobby: Arc<Mutex<Lobby>>,
    leaderboard: Arc<Mutex<Leaderboard>>,
    mut stream: TcpStream,
    address: SocketAddr,
    id: u64,
) {
    let _ = stream.set_nodelay(true);

    let handshake_res = {
        let Ok(lb) = leaderboard.lock() else {
            return;
        };
        server_handshake(&mut stream, &lb)
    };

    match handshake_res {
        Ok(HandshakeResult::WebSocket) => {
            // Proceed to WebSocket game loop
        }
        _ => {
            // Handled as regular HTTP request (e.g. browser viewing the leaderboard)
            return;
        }
    }

    let Ok(writer) = stream.try_clone() else {
        return;
    };
    let client = Arc::new(Client::new(id, address, writer));
    println!("{address} connected (player joined session)");

    let mut partial = Vec::new();
    while let Ok(frame) = read_frame(&mut stream) {
        match frame.opcode {
            OPCODE_BINARY | OPCODE_TEXT | OPCODE_CONTINUATION => {
                partial.extend_from_slice(&frame.payload);
                let message = std::mem::take(&mut partial);
                handle_message(&lobby, &client, &message);
            }
            OPCODE_PING => client.send_raw(OPCODE_PONG, &frame.payload),
            OPCODE_CLOSE => break,
            _ => {}
        }
    }

    if let Ok(mut lobby) = lobby.lock() {
        lobby.leave(&client);
    }
    client.shutdown();
    println!("{address} disconnected");
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let port = match arguments.as_slice() {
        [port] => match port.parse::<u16>() {
            Ok(port) if port > 0 => port,
            _ => {
                eprintln!("maze: '{port}' is not a valid port (1-65535)\n\n{USAGE}");
                return ExitCode::FAILURE;
            }
        },
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::FAILURE;
        }
    };

    let listener = match TcpListener::bind(("0.0.0.0", port)) {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("maze: cannot listen on port {port}: {error}");
            return ExitCode::FAILURE;
        }
    };

    let leaderboard = Arc::new(Mutex::new(Leaderboard::new()));
    let lobby = Arc::new(Mutex::new(Lobby::new(Arc::clone(&leaderboard))));

    println!("============================================================");
    println!("       MAZE PvP Server with Live Tournament Leaderboard     ");
    println!("============================================================");
    println!("Server listening on port {port}");
    match lan_address() {
        Some(address) => {
            println!("  • Players in MAZE.EXE: server IP {address}   port {port}");
            println!("  • Web Browser Dashboard: http://{address}:{port}/");
        }
        None => {
            println!("  • Web Browser Dashboard: http://localhost:{port}/");
        }
    }
    println!("  • Persistent stats: leaderboard.json & leaderboard.txt");
    println!("Press Ctrl+C to stop.\n");

    if let Ok(board) = leaderboard.lock() {
        if !board.players.is_empty() {
            println!("Loaded existing leaderboard from disk:");
            print!("{}", board.generate_terminal_table());
        }
    }

    let next_id = AtomicU64::new(1);
    for connection in listener.incoming() {
        let Ok(stream) = connection else {
            continue;
        };
        let Ok(address) = stream.peer_addr() else {
            continue;
        };
        let id = next_id.fetch_add(1, Ordering::Relaxed);
        let lobby = Arc::clone(&lobby);
        let leaderboard = Arc::clone(&leaderboard);
        std::thread::spawn(move || serve(lobby, leaderboard, stream, address, id));
    }
    ExitCode::SUCCESS
}
