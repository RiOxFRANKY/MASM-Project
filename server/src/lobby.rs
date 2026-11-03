use crate::protocol::{self, BY_EXIT, BY_TIME, DRAW};
use crate::websocket::{write_frame, OPCODE_BINARY};
use std::collections::HashMap;
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const TIMEOUT_GRACE: Duration = Duration::from_secs(2);

pub struct Client {
    pub id: u64,
    pub address: SocketAddr,
    writer: Mutex<TcpStream>,
}

impl Client {
    pub fn new(id: u64, address: SocketAddr, writer: TcpStream) -> Self {
        Client { id, address, writer: Mutex::new(writer) }
    }

    pub fn send(&self, message: &[u8]) -> bool {
        match self.writer.lock() {
            Ok(mut writer) => write_frame(&mut writer, OPCODE_BINARY, message).is_ok(),
            Err(_) => false,
        }
    }

    pub fn send_raw(&self, opcode: u8, payload: &[u8]) {
        if let Ok(mut writer) = self.writer.lock() {
            let _ = write_frame(&mut writer, opcode, payload);
        }
    }

    pub fn shutdown(&self) {
        if let Ok(writer) = self.writer.lock() {
            let _ = writer.shutdown(Shutdown::Both);
        }
    }
}

struct Match {
    players: [Arc<Client>; 2],
    level: u8,
    ready: [bool; 2],
    decided: bool,
    distance: [Option<u16>; 2],
}

impl Match {
    fn broadcast(&self, message: &[u8]) {
        for player in &self.players {
            player.send(message);
        }
    }

    fn enter_level(&mut self, level: u8) {
        if level != self.level {
            self.level = level;
            self.ready = [false; 2];
            self.decided = false;
            self.distance = [None; 2];
        }
    }

    fn decide(&mut self, winner: u8, reason: u8) {
        self.decided = true;
        self.broadcast(&protocol::result(self.level, winner, reason));
        println!(
            "match {} - {}: level {} won by {} ({})",
            self.players[0].address,
            self.players[1].address,
            self.level + 1,
            match winner {
                DRAW => "nobody, a draw".to_string(),
                slot => self.players[slot as usize].address.to_string(),
            },
            if reason == BY_EXIT { "reached the exit" } else { "time up" }
        );
    }

    fn decide_by_distance(&mut self) {
        let winner = match self.distance {
            [Some(first), Some(second)] if first < second => 0,
            [Some(first), Some(second)] if second < first => 1,
            [Some(_), None] => 0,
            [None, Some(_)] => 1,
            _ => DRAW,
        };
        self.decide(winner, BY_TIME);
    }
}

#[derive(Default)]
pub struct Lobby {
    waiting: Option<Arc<Client>>,
    matches: HashMap<u64, Match>,
    seats: HashMap<u64, (u64, usize)>,
    next_match: u64,
}

fn new_seed() -> u16 {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|elapsed| elapsed.subsec_nanos()).unwrap_or(1);
    let mixed = nanos ^ (nanos >> 16) ^ 0x5A5A;
    (mixed as u16) | 1
}

impl Lobby {
    pub fn queue(&mut self, client: &Arc<Client>) {
        if self.seats.contains_key(&client.id) {
            return;
        }
        match self.waiting.take() {
            Some(other) if other.id != client.id => self.start_match(other, Arc::clone(client)),
            _ => {
                println!("{} is looking for an opponent", client.address);
                self.waiting = Some(Arc::clone(client));
            }
        }
    }

    fn start_match(&mut self, first: Arc<Client>, second: Arc<Client>) {
        let id = self.next_match;
        self.next_match += 1;
        let seed = new_seed();
        first.send(&protocol::matched(0, seed));
        second.send(&protocol::matched(1, seed));
        println!("match {} - {} started (maze seed {seed})", first.address, second.address);
        self.seats.insert(first.id, (id, 0));
        self.seats.insert(second.id, (id, 1));
        self.matches.insert(
            id,
            Match { players: [first, second], level: u8::MAX, ready: [false; 2], decided: false, distance: [None; 2] },
        );
    }

    fn seat(&mut self, client: &Client) -> Option<(&mut Match, usize)> {
        let (match_id, slot) = *self.seats.get(&client.id)?;
        Some((self.matches.get_mut(&match_id)?, slot))
    }

    pub fn relay_move(&mut self, client: &Client, message: &[u8]) {
        if let Some((game, slot)) = self.seat(client) {
            game.players[1 - slot].send(message);
        }
    }

    pub fn ready(&mut self, client: &Client, level: u8) {
        if let Some((game, slot)) = self.seat(client) {
            game.enter_level(level);
            game.ready[slot] = true;
            if game.ready == [true, true] {
                game.broadcast(&protocol::go(level));
            }
        }
    }

    pub fn finished(&mut self, client: &Client, level: u8) {
        if let Some((game, slot)) = self.seat(client) {
            if game.level == level && !game.decided {
                game.decide(slot as u8, BY_EXIT);
            }
        }
    }

    pub fn distance(lobby: &Arc<Mutex<Lobby>>, client: &Client, level: u8, distance: u16) {
        let Ok(mut guard) = lobby.lock() else {
            return;
        };
        let Some((game, slot)) = guard.seat(client) else {
            return;
        };
        if game.level != level || game.decided {
            return;
        }
        game.distance[slot] = Some(distance);
        if game.distance.iter().all(Option::is_some) {
            game.decide_by_distance();
            return;
        }
        let Some(&(match_id, _)) = guard.seats.get(&client.id) else {
            return;
        };
        drop(guard);
        let lobby = Arc::clone(lobby);
        std::thread::spawn(move || {
            std::thread::sleep(TIMEOUT_GRACE);
            if let Ok(mut guard) = lobby.lock() {
                if let Some(game) = guard.matches.get_mut(&match_id) {
                    if game.level == level && !game.decided {
                        game.decide_by_distance();
                    }
                }
            }
        });
    }

    pub fn leave(&mut self, client: &Client) {
        if self.waiting.as_ref().is_some_and(|waiting| waiting.id == client.id) {
            self.waiting = None;
        }
        let Some((match_id, slot)) = self.seats.remove(&client.id) else {
            return;
        };
        if let Some(game) = self.matches.remove(&match_id) {
            let other = &game.players[1 - slot];
            self.seats.remove(&other.id);
            other.send(&protocol::left());
            println!("match {} - {} ended: {} left", game.players[0].address, game.players[1].address, client.address);
        }
    }
}
