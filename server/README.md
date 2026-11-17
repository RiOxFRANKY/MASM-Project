# MAZE PvP Server & Live Leaderboard

A high-performance WebSocket server with an integrated **Live Tournament Leaderboard** for MAZE's two-player network mode (menu option **2**). Built in Rust with zero third-party dependencies.

---

## Features

- **Matchmaking & Relay**: Automatically pairs competitors on the same LAN and synchronizes random maze seeds.
- **Authoritative Referee**: Arbitrates race finishes, time-out BFS distance measurements, and forfeits.
- **Live Leaderboard System**:
  - **Console Display**: Real-time ASCII leaderboard printed in the server terminal after every match and level.
  - **Web Dashboard**: Open `http://<server-ip>:<port>/` in any browser (laptop, phone, tablet) for a live, auto-refreshing arcade-styled dashboard with ranks, medals, win-rate graphs, and recent match histories.
  - **Persistent Storage**: Player ratings and match histories automatically save to and load from `leaderboard.json` and `leaderboard.txt`.

---

## The Score Metric

To evaluate competitive maze navigation skill objectively in classroom and tournament play, the server uses a composite performance score:

$$\text{Score} = (W_{\text{match}} \times 150) + (D_{\text{match}} \times 50) + (W_{\text{exit}} \times 100) + (W_{\text{time}} \times 60) + (D_{\text{level}} \times 30) + (N_{\text{levels}} \times 10) + \text{SpeedBonuses}$$

### Scoring Components:

| Metric | Points | Why It Matters |
|---|---|---|
| **Match Win** | **+150 pts** | Awarded for winning the overall best-of-3 level match. |
| **Match Draw** | **+50 pts** | Both competitors won an equal number of levels. |
| **Level Win (Exit)** | **+100 pts** | Primary objective: navigated through the maze and reached the exit first. |
| **Speed Bonus** | **+1 pt / sec left** | Clock seconds remaining when reaching the exit (rewards fast pathfinding & memory). |
| **Level Win (Distance)** | **+60 pts** | When time expires, closer to the exit along real BFS maze distance. |
| **Level Draw** | **+30 pts** | Both players equidistant from the exit at timeout. |
| **Participation** | **+10 pts** | Awarded for every level played to encourage continuous learning. |

### Ranking & Tie-Breaker Hierarchy:
1. **Total Points** (descending)
2. **Total Match Wins** (descending)
3. **Exit Finishes** (descending)
4. **Match Win Rate %** (descending)
5. **Fewer Losses** (ascending)

---

## Run

On 64-bit Windows, run the prebuilt `server.exe` from the repository root:

```cmd
bin\windows-x86_64\server.exe 9000
```

To build from source:

```bash
cargo build --release
target/release/maze 9000        # (maze.exe on Windows)
```

The server outputs its LAN IP address, e.g.:
```
============================================================
       MAZE PvP Server with Live Tournament Leaderboard     
============================================================
Server listening on port 9000
  • Players in MAZE.EXE: server IP 192.168.1.34   port 9000
  • Web Browser Dashboard: http://192.168.1.34:9000/
  • Persistent stats: leaderboard.json & leaderboard.txt
```

---

## Web Dashboard & API

- **Web Browser**: Anyone on the local network (teacher, spectators, players) can visit `http://<server-ip>:<port>/` in Chrome, Edge, Safari, or mobile browsers to view real-time rankings and match logs (auto-refreshes every 5s).
- **JSON API**: Requesting `http://<server-ip>:<port>/leaderboard.json` or passing `Accept: application/json` returns the complete leaderboard data structure in JSON for custom tools or projectors.

---

## How to Play

Every player runs `MAZE` in DOSBox-RS, on a PC or the Android app, on the same Wi-Fi as the server:

1. Press **2** and type the server IP and port. MAZE remembers them in `C:\MAZE.CFG`.
2. MAZE connects and displays "Searching for an opponent…".
3. The server pairs the first two searching players and transmits an identical maze seed.
4. Before each level, both players report ready. The server issues the synchronized start countdown.
5. Every move is relayed instantly over WebSocket (`TCP_NODELAY`).
6. The server records level and match results, updates ratings, updates `leaderboard.json`, and prints the leaderboard table in the console.

---

## Protocol (Binary WebSocket Messages)

| From | Message | Meaning |
|---|---|---|
| client | `Q` | Queue for matchmaking |
| client | `N <name>` | *(Optional)* Register a player nickname |
| server | `M slot seedLo seedHi` | Matched: your player slot (0/1) and the maze seed |
| client | `Y level` | Ready for this level |
| server | `G level` | Both players ready: start countdown |
| both | `O squareLo squareHi` | Real-time move coordinate (relayed to opponent) |
| client | `F level` | Reached the exit square |
| client | `D level distLo distHi` | Time up: remaining BFS distance to exit |
| server | `R level winner reason` | Verdict: winner slot (0/1) or 2 for draw; reason 0 = exit, 1 = time |
| server | `X` | Opponent disconnected (awards forfeit win) |
