# Online multiplayer over Steam

Players with Steam accounts play the same match over the internet in
deterministic lockstep (see [ARCHITECTURE.md](ARCHITECTURE.md#data-flow-multiplayer)).
Steam finds and joins the lobby, invites friends and relays the packets
through its network (NAT traversal included). Built in prompt 026.

Steam is **optional**: single player, LAN, the map editor and every self-test
run without Steam. Steam is only started when a player opens *Online
(Steam)*; without a running, logged-in Steam client that screen says so and
offers *Try again*.

## Quick start

Steam must be running and logged in on every machine, one account per
player (two accounts cannot share one Steam client).

```bash
just online         # opens the online lobby
```

- **Host:** pick the map, leave *Listed for everyone* on (or off for friends
  only), press *Host game*. The lobby code appears; *Copy code* puts it on the
  clipboard.
- **Others:** pick the game from *Open games* (refreshed every 5 s), or enter
  the code and press *Join code*, or use *Join game* on the host in the Steam
  friends list. Then tick *Ready*.
- **Host:** *Start match* is enabled once everyone is ready.

The match itself is the same as on the LAN ([LAN.md](LAN.md)): status line
(now "Online · …") with input delay, round trip and confirmed tick, Esc menu
with *Surrender* and *Leave match*, result screen, desync replays.

## App id

`steam::APP_ID` in `crates/steam/src/lib.rs` is the only place the Steam app
id is set. It is **480**, Valve's public test app ("Spacewar"), until Mud and
Steel has its own. With 480 Steam shows the player as "playing Spacewar", and
every Steamworks developer shares the lobby list; ours are filtered by the
lobby data `game = mud-and-steel`.

When the real app id arrives: change the constant, then set up rich presence
localization and (optionally) the launch option `+connect_lobby %lobby%`
in the Steamworks partner site. No `steam_appid.txt` is needed; the game sets
the id itself (`Client::init_app`).

## Choices

**steamworks-rs in Rust, not GodotSteam.** As on the LAN (ENet in Rust
instead of Godot's), the Rust side wins:

- *Testable:* the lobby protocol and the match transport are the same
  `net::lobby::LobbyNode` as on the LAN, tested in `cargo test` over in-process
  links (`MemoryLink`). With GodotSteam the network code would be GDScript and
  every packet would pass between GDScript and `SimHost` each frame.
- *No binaries in the repository:* the crate
  [`steamworks`](https://crates.io/crates/steamworks) 0.13.1 (MIT/Apache-2.0,
  Steamworks SDK 1.64) brings Valve's redistributable `steam_api` libraries
  through `steamworks-sys` from crates.io, pinned with a checksum in
  `Cargo.lock`. GodotSteam would add an addon with libraries per platform.
- *One GDExtension:* no second binary that must match the Godot version.
- *Headless:* `just steam-host` / `just steam-join` play over Steam without
  graphics, like `just lan-host` / `lan-join`.
- *Cost:* the bridge links `steam_api` dynamically, so the library must sit
  next to it (see [Build](#build)).

**Steam Networking Messages**, not Networking Sockets: connectionless and
addressed by Steam id, reliable and ordered, with channels like ENet, relayed
by Steam Datagram Relay. Channel 0 carries the lobby messages, channel 1 the
lockstep packets, as on the LAN. Messages are sent without Nagle delay.

**The LAN lobby protocol.** `LobbyNode` (hello, welcome, reject, lobby, ready,
start, peer left; star with the host relaying, see [LAN.md](LAN.md#protocol))
runs over a `steam::SteamLink` instead of ENet. The Steam lobby is used to
find, invite and as the list of members: the lobby owner is the host and
player 0. A member that leaves the Steam lobby (or whose session fails) is
a lost peer, exactly like an ENet disconnect.

**Steam starts with the online screen**, not with the game. Consequence: the
Steam overlay is not available in development (Steam only hooks it into games
it started), so *Invite friends* explains that and the other ways remain.

**Round trip:** the ping of the Steam session (`GetSessionConnectionInfo`).
Steam reports no variance; a quarter of the ping is assumed. The dynamic input
delay of the LAN applies unchanged (`ceil((rtt + variance) / 2 / 50 ms) + 1`,
1 to 12 ticks), so an internet round trip of 120 ms gives 3 ticks (150 ms).

## Protocol details

| What | Value |
|------|-------|
| Lobby data | `game = mud-and-steel`, `protocol = <net::lobby::PROTOCOL_VERSION>`, `name = <host's Steam name>`, `started = 1` once the match runs (the lobby is then also set not joinable) |
| Lobby size | 6 members (prompt 037: the slots follow the map's starts, [TEAMS.md](TEAMS.md)) |
| Lobby list | filter `game`, distance worldwide; incompatible protocol versions are shown and refused by the host |
| Rich presence | `status` ("In a lobby" / "In a match"), `connect = +connect_lobby <code>` while the host waits for players, `steam_player_group = <code>` |
| Invites | overlay invite dialog (when Steam started the game), *Join game* in the friends list (`GameRichPresenceJoinRequested`), lobby invitations (`GameLobbyJoinRequested`), the code |
| Launch option | `+connect_lobby <code>`: the main menu opens the online screen and joins |
| Sessions | accepted from anyone while we are in a lobby; the lobby protocol decides who becomes a player (protocol version, free slot, not started) |
| Message size | Steam allows 512 KB per message; the start message (full `SimConfig`) is far smaller (checked by a test) |
| Timeouts | a client gives up after 20 s without an answer from the host; lost peers are seen through the Steam lobby at once, or through a failed session |

## Build

- `crates/steam` holds everything Steam-specific. The bridge uses it through
  the cargo feature `steam`, on by default. `cargo build -p bridge
  --no-default-features` builds without Steam; `SteamLobby` then reports
  "This build has no Steam support" and the rest of the game is unchanged.
- **`just build` copies the Steam library** (`libsteam_api.dylib` / `.so`,
  `steam_api64.dll`) from the `steamworks-sys` build to `target/debug/`, next
  to the bridge. Without it Godot cannot load the bridge at all ("Library not
  loaded: @loader_path/libsteam_api.dylib"); so build with `just build` (all
  `just` Godot commands do), not with a bare `cargo build -p bridge`. On
  Linux the bridge has `rpath=$ORIGIN` for it; on Windows Godot adds the
  extension's folder to the DLL search path.
- The CLI's Steam peers need `--features steam` (the `just steam-*` recipes
  pass it); `cargo run` finds the library by itself.
- CI builds and lints the bridge with and without Steam and the CLI with
  Steam on macOS, Linux and Windows. No Steam client is needed anywhere in CI.
- An exported game ships the Steam library next to the bridge: the
  `[dependencies]` of `godot/mud_and_steel.gdextension` (prompt 044, see
  [RELEASE.md](RELEASE.md)).

## Tests

| Command | What it does |
|---------|--------------|
| `just check` | `crates/net/tests/lobby.rs`: the lobby protocol over in-process links, connectionless like Steam: two and three peers play through the lobby to the end hash of `run_demo` (with a measured round trip driving the input delay), host relay, full / started / wrong version rejected, lost peer ends the match for everyone, leaving frees the slot, unreachable host, start message size. The LAN tests run the same `LobbyNode` over ENet. `crates/steam` tests the launch option and connect string parsing (Steam itself is not started). |
| `just selftest` | the flow self-test also opens *Online (Steam)* and goes back, without starting Steam |
| `just steam-host` / `just steam-join [code]` | headless CLI peers over Steam (needs Steam; see below) |
| `just online-selftest host` / `just online-selftest join` | two games play an automatic match over Steam like `just lan-selftest` (needs Steam) |

Everything that needs a real Steam client is a manual test, below.

## Testing with two Steam accounts

Needed: two machines (macOS, Linux or Windows in any combination), each with
Steam installed, running and logged in with a **different** account, the same
commit, a Rust toolchain and Godot 4.7.2, built and imported once
(`just build`, `just import`). The accounts do not need to be friends for
the lobby list or the code; they need to be friends for invites and the
friends list. Different networks are fine: that is the point of the test.

1. **Headless check first** (no graphics involved):
   - Machine A: `just steam-host`. It prints "Steam: signed in as …" and
     "lobby code 1097…".
   - Machine B: `just steam-join` (takes the first listed lobby) or
     `just steam-join 1097…` (the code from A).
   - Both print `player=… ticks=600 hash=0x… verified=600 … rtt_ms=…`. The
     hashes must be equal and equal to `just sim 600`. Exit code 0 on both.
2. **Automatic match in the game:** A `just online-selftest host`, B
   `just online-selftest join`. Both play until tick 400 is confirmed, the
   client surrenders, both quit with "LAN self-test PASSED" (the name of the
   shared test) and exit code 0.
3. **The game:** `just online` on both. A hosts; B finds the game under
   *Open games* (or types the code) and joins, ticks *Ready*; A starts. Each
   player orders their own tanks; the other machine must show the same
   movements. The status line should stay white; note the RTT and the input
   delay.
4. **Invites:** with the accounts befriended, B quits the online screen; A
   hosts. In B's Steam friends list A shows "In a lobby" and *Join game*:
   clicking it while the game shows the online screen joins A's lobby.
5. **Disconnect:** close the game on B during a match. A's status line turns
   red with "player 1 left the game" within a few seconds.
6. **Without Steam:** quit Steam on one machine and open *Online (Steam)*: it
   must say that Steam is not running; *Try again* works after starting Steam.
   Single player and LAN must work as before.
7. **What to report back** if something fails: the terminal output of both
   machines, the desync replays if any, OS of both machines, and whether the
   list, the code or the friends list was used.

## Limits

- Slots, teams and computer players as on the LAN (prompt 037,
  [TEAMS.md](TEAMS.md)); tested without Steam only (the lobby protocol over
  memory links and ENet), a real test with three or more accounts is open.
  No spectators, no reconnect.
- No kick: Steam lobbies cannot remove a member; a refused peer leaves by
  itself after the host's rejection.
- Lobby membership is not enforced for sessions (anyone in contact with the
  host could say hello); the protocol version and the player limit are.
- The overlay (and so the invite dialog) only works when Steam starts the
  game, i.e. after there is a real app id and a Steam build.
