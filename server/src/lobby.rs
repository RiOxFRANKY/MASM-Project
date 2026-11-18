use crate::leaderboard::Leaderboard;
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
    pub name: Mutex<Option<String>>,
    writer: Mutex<TcpStream>,
}

impl Client {
    pub fn new(id: u64, address: SocketAddr, writer: TcpStream) -> Self {
        Client {
            id,
            address,
            name: Mutex::new(None),
            writer: Mutex::new(writer),
        }
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
    start_time: Option<SystemTime>,
    level_wins: [u8; 2],
    level_draws: u8,
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
            self.start_time = None;
        }
    }

    fn decide(&mut self, leaderboard: &Arc<Mutex<Leaderboard>>, winner: u8, reason: u8) {
        self.decided = true;
        self.broadcast(&protocol::result(self.level, winner, reason));

        let elapsed_secs = self
            .start_time
            .and_then(|t| t.elapsed().ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let time_limit: u32 = match self.level {
            0 => 30,
            1 => 45,
            _ => 60,
        };
        let time_bonus = if reason == BY_EXIT {
            time_limit.saturating_sub(elapsed_secs as u32)
        } else {
            0
        };

        let p0_ip = self.players[0].address.ip();
        let p1_ip = self.players[1].address.ip();

        if winner == DRAW {
            self.level_draws += 1;
            if let Ok(mut board) = leaderboard.lock() {
                board.record_level_result(Some(p0_ip), Some(p1_ip), true, false, 0);
            }
        } else if (winner as usize) < 2 {
            let w = winner as usize;
            let l = 1 - w;
            self.level_wins[w] += 1;
            let w_ip = self.players[w].address.ip();
            let l_ip = self.players[l].address.ip();
            if let Ok(mut board) = leaderboard.lock() {
                board.record_level_result(Some(w_ip), Some(l_ip), false, reason == BY_EXIT, time_bonus);
            }
        }

        println!(
            "match {} - {}: level {} won by {} ({}){}",
            self.players[0].address,
            self.players[1].address,
            self.level + 1,
            match winner {
                DRAW => "nobody, a draw".to_string(),
                slot => self.players[slot as usize].address.to_string(),
            },
            if reason == BY_EXIT {
                "reached the exit"
            } else {
                "time up"
            },
            if time_bonus > 0 {
                format!(" [Speed Bonus: +{time_bonus} pts]")
            } else {
                String::new()
            }
        );

        // If level 2 is complete, the 3-level match is concluded!
        if self.level == 2 {
            println!(
                "\n--- MATCH COMPLETE: {} ({}) vs {} ({}) ---",
                self.players[0].address, self.level_wins[0], self.players[1].address, self.level_wins[1]
            );
            if let Ok(mut board) = leaderboard.lock() {
                board.record_match_result(p0_ip, p1_ip, self.level_wins[0], self.level_wins[1]);
                print!("{}", board.generate_terminal_table());
            }
        }
    }

    fn decide_by_distance(&mut self, leaderboard: &Arc<Mutex<Leaderboard>>) {
        let winner = match self.distance {
            [Some(first), Some(second)] if first < second => 0,
            [Some(first), Some(second)] if second < first => 1,
            [Some(_), None] => 0,
            [None, Some(_)] => 1,
            _ => DRAW,
        };
        self.decide(leaderboard, winner, BY_TIME);
    }
}

pub struct Lobby {
    waiting: Option<Arc<Client>>,
    matches: HashMap<u64, Match>,
    seats: HashMap<u64, (u64, usize)>,
    next_match: u64,
    pub leaderboard: Arc<Mutex<Leaderboard>>,
}

fn new_seed() -> u16 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.subsec_nanos())
        .unwrap_or(1);
    let mixed = nanos ^ (nanos >> 16) ^ 0x5A5A;
    (mixed as u16) | 1
}

impl Lobby {
    pub fn new(leaderboard: Arc<Mutex<Leaderboard>>) -> Self {
        Lobby {
            waiting: None,
            matches: HashMap::new(),
            seats: HashMap::new(),
            next_match: 0,
            leaderboard,
        }
    }

    pub fn set_name(&mut self, client: &Client, name: String) {
        if let Ok(mut n) = client.name.lock() {
            *n = Some(name.clone());
        }
        if let Ok(mut board) = self.leaderboard.lock() {
            board.set_player_name(client.address.ip(), name);
        }
    }

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
        println!(
            "match {} - {} started (maze seed {seed})",
            first.address, second.address
        );
        self.seats.insert(first.id, (id, 0));
        self.seats.insert(second.id, (id, 1));
        self.matches.insert(
            id,
            Match {
                players: [first, second],
                level: u8::MAX,
                ready: [false; 2],
                decided: false,
                distance: [None; 2],
                start_time: None,
                level_wins: [0, 0],
                level_draws: 0,
            },
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
                game.start_time = Some(SystemTime::now());
                game.broadcast(&protocol::go(level));
            }
        }
    }

    pub fn finished(&mut self, client: &Client, level: u8) {
        let lb = Arc::clone(&self.leaderboard);
        if let Some((game, slot)) = self.seat(client) {
            if game.level == level && !game.decided {
                game.decide(&lb, slot as u8, BY_EXIT);
            }
        }
    }

    pub fn distance(lobby: &Arc<Mutex<Lobby>>, client: &Client, level: u8, distance: u16) {
        let Ok(mut guard) = lobby.lock() else {
            return;
        };
        let lb = Arc::clone(&guard.leaderboard);
        let Some((game, slot)) = guard.seat(client) else {
            return;
        };
        if game.level != level || game.decided {
            return;
        }
        game.distance[slot] = Some(distance);
        if game.distance.iter().all(Option::is_some) {
            game.decide_by_distance(&lb);
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
                let lb = Arc::clone(&guard.leaderboard);
                if let Some(game) = guard.matches.get_mut(&match_id) {
                    if game.level == level && !game.decided {
                        game.decide_by_distance(&lb);
                    }
                }
            }
        });
    }

    pub fn leave(&mut self, client: &Client) {
        if self
            .waiting
            .as_ref()
            .is_some_and(|waiting| waiting.id == client.id)
        {
            self.waiting = None;
        }
        let Some((match_id, slot)) = self.seats.remove(&client.id) else {
            return;
        };
        if let Some(game) = self.matches.remove(&match_id) {
            let other = &game.players[1 - slot];
            self.seats.remove(&other.id);
            other.send(&protocol::left());
            println!(
                "match {} - {} ended: {} left",
                game.players[0].address, game.players[1].address, client.address
            );

            // If a game was in progress, award forfeit to the remaining player
            if game.level != u8::MAX && !game.decided {
                let winner_ip = other.address.ip();
                let leaver_ip = client.address.ip();
                if let Ok(mut board) = self.leaderboard.lock() {
                    board.record_forfeit(winner_ip, leaver_ip);
                    println!("Awarded forfeit victory to {winner_ip}.");
                    print!("{}", board.generate_terminal_table());
                }
            }
        }
    }
}
