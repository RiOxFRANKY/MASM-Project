# MAZE PvP server

A small WebSocket server for MAZE's two-player network mode (menu option **2**). It has no dependencies.

## Run

```
cargo build --release
target/release/maze <port>        (maze.exe on Windows)
```

It prints this computer's LAN address, e.g. `server IP 192.168.1.34   port 9000`.

## Play

Every player runs `MAZE` in DOSBox-RS, on a PC or the Android app, on the same Wi-Fi as the server:

1. Press **2** and type the server IP and port. MAZE remembers them in `C:\MAZE.CFG`, because the IP changes from network to network.
2. MAZE connects and shows "Searching for an opponent…" with a seconds counter.
3. The server pairs the first two players who are searching. Both get the same maze seed, so they play identical mazes.
4. Before each level both send "ready". The server starts both at the same moment, so the 3-2-1 countdowns line up.
5. Every move is relayed at once. TCP_NODELAY is set on both ends, so there is no Nagle delay, and a message is 3 bytes.
6. The server is the referee:
   - the first one to report reaching the exit wins;
   - at timeout, the one with the smaller remaining distance wins.
   - If one side doesn't report within 2 seconds, the side that did wins.
7. If a player leaves, the other is told.

Network play needs the DOSBox-RS network service (`INT 61h`). Real DOS and other DOSBox builds don't have it, and MAZE says so.

## Protocol (binary WebSocket messages)

| From | Message | Meaning |
|---|---|---|
| client | `Q` | look for an opponent |
| server | `M slot seedLo seedHi` | matched: your side (0/1) and the maze seed |
| client | `Y level` | ready for this level |
| server | `G level` | both are ready: start |
| both | `O squareLo squareHi` | a move (relayed to the opponent) |
| client | `F level` | reached the exit |
| client | `D level distLo distHi` | time is up, this is my distance |
| server | `R level winner reason` | verdict: winner is a side, or 2 for a draw; reason 0 = exit, 1 = time |
| server | `X` | the opponent left |
