# LAN multiplayer

Two or more machines in the local network play the same match in deterministic
lockstep (see [ARCHITECTURE.md](ARCHITECTURE.md#data-flow-multiplayer)). One player
hosts, the others find the game with a LAN search, join, get ready, and everyone
starts together. Built in prompt 025; since prompt 027 the lobby opens from the
main menu (*LAN game*), uses the menu look and has *Back to main menu*.

## Quick start

```bash
just lan            # on every machine: opens the LAN lobby
```

- **Host:** enter a name, press *Host game*. The host is always player 0.
- **Others:** press *Search LAN*, double-click the game (or type the host's IP into
  the address field and press *Join address*), then tick *Ready*.
  A host can appear once per network address it has (e.g. `127.0.0.1` on the
  same machine plus its LAN address); every entry leads to the same game.
- **Host:** *Start match* is enabled once everyone is ready.

The demo scenario has two players, so a game takes one host and one client. The
host picks the map in the lobby (or with `just lan --map=test_large`); every peer
must have the same map files (checked by content hash).

**End of a match:** Esc opens the in-match menu (the game keeps running: a lockstep
match cannot pause for one player) with *Surrender* and *Leave match*. A surrender,
or a player without units, decides the match in the simulation on every peer; each
player then sees the result screen and returns to the main menu. A player leaving
after the result is shown as "Match over", not as an error.

During the match a status line shows the player, the input delay, the round trip
and the last tick whose state hash every peer confirmed. It turns yellow while the
game waits for another player's commands and red when the match stopped.

## Choices

**ENet in Rust (`rusty_enet`), not Godot's ENet.** The transport only moves bytes, so
both would be equally deterministic. The Rust one wins on everything else:

- *Testable without Godot:* `cargo test` plays real UDP matches (in-process and two
  `cli` processes) on macOS, Linux and Windows CI.
- *Platforms and build:* `rusty_enet` is ENet 1.3.18 transpiled to Rust on top of
  `std::net::UdpSocket`; no C compiler, no `bindgen`/`libclang` (which the C bindings
  `enet-sys` need).
- *Maintenance:* one small dependency behind `net::lan`; Godot's ENet would tie the
  network code to Godot types and to the paired Godot/godot-rust updates.
- *Steam later:* GodotSteam brings its own networking; a `SteamTransport` implements
  the same `Transport` trait, independent of where ENet lives.
- *Risk:* the crate's version number is low. Everything ENet-specific is in
  `crates/net/src/lan/`, so it can be swapped without touching the lockstep code.

**Star topology.** The host is the ENet server; clients only connect to it. The host
relays the lockstep packets of each client to all other clients unchanged. Order per
sender is preserved, which is all `LockstepSession` needs.

**The host's config wins.** The start message carries the host's fully resolved
`SimConfig` (as in a replay), the seed, the players and the initial input delay.
Peers with a different unit catalog therefore cannot diverge; the map is loaded
locally and `Sim::new` rejects a different content hash.

## Protocol

| Port (UDP) | Default | Purpose |
|------------|---------|---------|
| Game       | 47810   | ENet, two channels, both reliable and ordered |
| Search     | 47811   | Discovery queries and answers |

ENet channel 0, lobby messages (postcard, `net::lan::wire`):

| Message | Direction | Meaning |
|---------|-----------|---------|
| `Hello { protocol, name }` | client → host | first message; wrong protocol version is rejected (`net::lan::PROTOCOL_VERSION`, 2 since prompt 017 changed config and commands) |
| `Welcome { player }` | host → client | accepted as player id `player` (smallest free id) |
| `Reject { reason }` | host → client | game full, already started, wrong version; then disconnect |
| `Lobby { players }` | host → all | names and ready flags |
| `Ready(bool)` | client → host | |
| `Start(StartInfo)` | host → all | seed, resolved `SimConfig`, players, initial input delay, hash interval |
| `PeerLeft(player)` | host → all | a player left the running match; it ends |

ENet channel 1 carries the opaque lockstep messages of `net::Transport`
(`CommandFrame`, `HashReport`).

Discovery: a client sends `"MSLAN?" + version` to `255.255.255.255`, to the broadcast
address of every IPv4 interface (needed on Windows with several adapters) and to
`127.0.0.1`, all on the search port. A host in the lobby answers by unicast with
`"MSLAN!" + GameInfo` (name, game port, players). Only the host binds the search
port, so several searching clients can run on one machine. A second host on the same
machine cannot bind it and is only reachable by address.

Timeouts: ENet disconnects a peer that did not answer for 3 to 10 s; pings every
250 ms measure the round trip for the dynamic input delay.

## Latency, disconnects, desyncs

- **Input delay:** starts at 3 ticks (150 ms) for everyone, then each peer follows its
  measured round trip: `ceil((rtt + variance) / 2 / 50 ms) + 1`, between 1 and 12
  ticks. Raised at once, lowered after three checks in a row (one check per
  second) to the largest value those checks recommended. ENet starts every
  connection with an assumed round trip of 500 ms that converges within about a
  second of traffic, so the delay can be high for the first seconds of a match.
- **Lost connection:** the match halts with the reason ("player 1 left the game",
  "connection to the host lost"); there is no reconnect yet. The replay is written on
  exit as usual.
- **Desync:** the match halts, the status line names tick and player, and the replay
  is saved at once to `user://replays/desync-p<player>-<date-time>.msreplay`
  (macOS: `~/Library/Application Support/Godot/app_userdata/Mud and Steel/replays/`).
  Collect the replays of all peers and run `just replay <file>` on each: every replay
  re-simulates to its own end hash; comparing their command frames shows which peer
  applied something the other did not.

**Computer players** (prompt 024, [AI.md](AI.md)): a peer can host further
players (`LockstepSession::host_player` / `submit_hosted`) and sends their frames
and hash reports as if they had their own peer. The lobby offers no AI slots yet.

## Tests

| Command | What it does |
|---------|--------------|
| `just check` | includes `crates/net/tests/lan.rs` (two and three peers over UDP on localhost, rejection, lost peer, discovery) and `crates/cli/tests/lan.rs` (two `cli` processes; the client finds the host by search; end hash equals the single-process `run_demo`; a provoked desync is reported by both, with replays) |
| `just lan-selftest` | two headless Godot instances: host, search, join, ready, start, both hold fire and order their tanks until tick 400 is confirmed by both; then the client surrenders and both must show the result (host victory, client defeat) at a confirmed tick. Logs in `build/lan-selftest/` |
| `just lan-host` / `just lan-join [address]` | headless CLI peers playing the demo, e.g. one per machine |

## Testing with two machines

Both machines need the same commit (`git pull`), a Rust toolchain and Godot 4.7.2,
built and imported once (`just build`, `just import`). They must be in the same
network segment (same Wi-Fi or switch; guest networks often block traffic between
devices).

1. **Firewall.**
   - macOS: on the first `just lan` (or `just lan-host`) macOS asks whether Godot
     (or `cli`) may accept incoming connections: *Allow*. If you missed it: System
     Settings → Network → Firewall → Options.
   - Windows: allow Godot (and `cli.exe`) for *private* networks when asked, or add
     inbound rules for UDP 47810 and 47811:
     ```powershell
     New-NetFirewallRule -DisplayName "Mud and Steel LAN" -Direction Inbound -Protocol UDP -LocalPort 47810,47811 -Action Allow -Profile Private
     ```
     Make sure the network is marked *private*, not *public*.
   - Linux: e.g. `sudo ufw allow 47810:47811/udp`.
2. **Headless check first** (no graphics involved):
   - Machine A: `just lan-host` (prints "hosting on port 47810 …").
   - Machine B: `just lan-join` (searches) or `just lan-join 192.168.1.20` (A's IP).
   - Both print `player=… ticks=600 hash=0x… verified=600`. The hashes must be equal
     and equal to `just sim 600` on either machine. Exit code 0 on both.
3. **The game:** `just lan` on both. A hosts, B searches and joins (or types A's IP),
   B ticks *Ready*, A starts. Each player orders their own tanks (box-select, right
   click); the other machine must show the same movements. The status line should
   stay white with a round trip of a few milliseconds and an input delay of 1–2.
4. **Disconnect:** close the game on B. A's status line turns red with "player 1
   left the game" within a few seconds.
5. **What to report back** if something fails: both terminal outputs, the desync
   replays if any, OS of both machines, and whether the search or only the IP worked.

If the search finds nothing but joining by IP works, broadcasts are blocked (common
in office and guest networks, or with VPN adapters on Windows). Joining by IP is the
supported fallback.
