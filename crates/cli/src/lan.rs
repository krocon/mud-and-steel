//! Headless LAN peers: `lan-host` and `lan-join` play the demo scenario over
//! ENet, each peer running only its own player's command script.

use net::lan::{
    DiscoveryBrowser, DiscoveryResponder, GameInfo, LanNode, LanState, PROTOCOL_VERSION,
};
use net::lobby::{Link, LobbyNode};
use net::{LockstepSession, ReplayRecorder, Transport, recommended_input_delay};
use sim::scenario::{CommandScript, builtin_map, demo_config};
use sim::{Command, PlayerCommand, Sim, Vec2};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// State hashes are exchanged every this many ticks (like the game).
pub const HASH_INTERVAL: u64 = 20;
/// The input delay is re-evaluated every this many ticks.
const DELAY_INTERVAL: u64 = 20;

pub struct MatchOptions {
    pub ticks: u64,
    pub record: Option<PathBuf>,
    /// Applies an extra command locally only at this tick, to provoke a
    /// desync (testing).
    pub cheat_tick: Option<u64>,
    pub timeout: Duration,
}

pub struct HostOptions {
    pub name: String,
    pub port: u16,
    pub discovery_port: u16,
    pub players: usize,
    pub seed: u64,
    pub input_delay: u32,
}

pub struct JoinOptions {
    pub name: String,
    /// Address of the host; searched on the LAN if `None`.
    pub address: Option<SocketAddr>,
    pub discovery_port: u16,
}

/// Hosts a game, waits until the lobby is full and everyone is ready, then
/// plays. Returns the process exit code.
pub fn host(options: &HostOptions, play: &MatchOptions) -> Result<i32, String> {
    let mut node = LanNode::host(&options.name, options.port, options.players)
        .map_err(|e| format!("cannot host on port {}: {e}", options.port))?;
    let info = |node: &LanNode| GameInfo {
        protocol: PROTOCOL_VERSION,
        name: options.name.clone(),
        game_port: node.port(),
        players: u8::try_from(node.players().len()).unwrap_or(u8::MAX),
        max_players: u8::try_from(options.players).unwrap_or(u8::MAX),
    };
    let mut responder = match DiscoveryResponder::new(options.discovery_port, info(&node)) {
        Ok(responder) => Some(responder),
        Err(e) => {
            eprintln!(
                "warning: discovery port {} unavailable ({e}); join by address",
                options.discovery_port
            );
            None
        }
    };
    println!(
        "hosting on port {} (discovery {}), waiting for {} player(s)",
        node.port(),
        options.discovery_port,
        options.players - 1
    );
    let deadline = Instant::now() + play.timeout;
    while !(node.can_start() && node.players().len() == options.players) {
        if Instant::now() > deadline {
            return Err("timed out waiting for players".into());
        }
        node.poll();
        if let Some(responder) = &mut responder {
            responder.set_info(info(&node));
            responder.poll();
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    drop(responder);
    node.start(
        options.seed,
        demo_config(),
        options.input_delay,
        HASH_INTERVAL,
    )?;
    play_match(node, play)
}

/// Pause before joining again when a join by address got no answer.
const RETRY_MS: u64 = 250;

/// Joins a game (by address or the first one found), gets ready and plays.
/// Returns the process exit code.
pub fn join(options: &JoinOptions, play: &MatchOptions) -> Result<i32, String> {
    let deadline = Instant::now() + play.timeout;
    let address = match options.address {
        Some(address) => address,
        None => discover(options.discovery_port, deadline)?,
    };
    println!("joining {address}");
    let mut node =
        LanNode::join(&options.name, address).map_err(|e| format!("cannot join: {e}"))?;
    let mut ready = false;
    let mut in_lobby = false;
    loop {
        if Instant::now() > deadline {
            return Err("timed out waiting for the start".into());
        }
        node.poll();
        match node.state() {
            LanState::Lobby if !ready => {
                node.set_ready(true);
                ready = true;
                in_lobby = true;
            }
            LanState::Started => break,
            // Joined by address before the host listens (both started
            // together, a slow host start): try again until the deadline.
            LanState::Closed(reason) if !in_lobby && options.address.is_some() => {
                println!("no answer from {address} ({reason}), trying again");
                std::thread::sleep(Duration::from_millis(RETRY_MS));
                node = LanNode::join(&options.name, address)
                    .map_err(|e| format!("cannot join: {e}"))?;
            }
            LanState::Closed(reason) => return Err(reason.clone()),
            _ => {}
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    play_match(node, play)
}

fn discover(port: u16, deadline: Instant) -> Result<SocketAddr, String> {
    let mut browser =
        DiscoveryBrowser::new(port).map_err(|e| format!("cannot search the LAN: {e}"))?;
    println!("searching the LAN (discovery port {port})");
    loop {
        if Instant::now() > deadline {
            return Err("no game found".into());
        }
        browser.query();
        for _ in 0..50 {
            std::thread::sleep(Duration::from_millis(4));
            browser.poll();
            if let Some(game) = browser.games().first() {
                println!(
                    "found '{}' at {} ({}/{} players)",
                    game.info.name, game.address, game.info.players, game.info.max_players
                );
                return Ok(game.address);
            }
        }
    }
}

/// Plays the started match as fast as the lockstep allows (LAN or Steam).
pub fn play_match<L: Link>(mut node: LobbyNode<L>, options: &MatchOptions) -> Result<i32, String> {
    let info = node.start_info().ok_or("match not started")?.clone();
    let local = node.local_player().ok_or("no player id")?;
    let map = builtin_map();
    let mut sim = Sim::new(&info.config, map, info.seed).map_err(|e| e.to_string())?;
    let mut session = LockstepSession::new(local, &info.lockstep_config());
    let mut recorder = ReplayRecorder::new(info.seed, info.config.clone(), &info.players);
    let mut script = CommandScript::new(info.seed, local, &info.config);
    let own_unit = info
        .config
        .spawns
        .iter()
        .position(|s| s.owner == local)
        .and_then(|i| u32::try_from(i).ok())
        .map(sim::UnitId);
    let net = |e: net::NetError| e.to_string();
    // A desync already found wins over a following disconnect: the peer that
    // found it first leaves right away, so its disconnect often arrives in
    // the same receive as the hash report that shows the desync here.
    let tolerate = |result: Result<(), net::NetError>, session: &net::LockstepSession| match result
    {
        Err(_) if session.desync().is_some() => Ok(()),
        other => other.map_err(net),
    };

    let deadline = Instant::now() + options.timeout;
    let mut max_delay = session.input_delay();
    while sim.tick() < options.ticks && session.desync().is_none() {
        if Instant::now() > deadline {
            return Err(format!("lockstep stalled at tick {}", sim.tick()));
        }
        if sim.tick().is_multiple_of(DELAY_INTERVAL)
            && let Some((rtt, variance)) = node.round_trip_ms()
        {
            session.set_input_delay(recommended_input_delay(rtt, variance, info.config.tick_hz));
            max_delay = max_delay.max(session.input_delay());
        }
        while session.needs_local_input() {
            let commands = script.commands_for_tick(session.next_local_tick());
            let sent = session.submit_local(commands, &mut node).map(|_| ());
            tolerate(sent, &session)?;
            if session.desync().is_some() {
                break;
            }
        }
        let received = session.receive(&mut node);
        tolerate(received, &session)?;
        if session.desync().is_some() {
            break;
        }
        let Some(mut commands) = session.next_commands() else {
            std::thread::sleep(Duration::from_micros(500));
            continue;
        };
        if options.cheat_tick == Some(sim.tick())
            && let Some(unit) = own_unit
        {
            commands.push(PlayerCommand::new(
                local,
                Command::MoveTo {
                    units: vec![unit],
                    target: Vec2::from_meters(99, 99),
                },
            ));
        }
        recorder.record(&commands);
        sim.step(&commands);
        let reported = session.record_hash_with(sim.tick(), || sim.state_hash(), &mut node);
        tolerate(reported, &session)?;
    }
    // Wait until every peer confirmed the final hash (or a desync shows up).
    while session.verified_tick() < sim.tick() && session.desync().is_none() {
        if Instant::now() > deadline {
            return Err(format!(
                "final hash not confirmed (verified up to tick {})",
                session.verified_tick()
            ));
        }
        let received = session.receive(&mut node);
        tolerate(received, &session)?;
        std::thread::sleep(Duration::from_millis(1));
    }

    if let Some(path) = &options.record {
        std::fs::write(path, recorder.finish(&sim).encode())
            .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        println!("wrote {}", path.display());
    }
    let rtt = node.round_trip_ms().map_or(0, |(rtt, _)| rtt);
    // Let the disconnect reach the others.
    node.leave();
    for _ in 0..50 {
        node.poll();
        std::thread::sleep(Duration::from_millis(2));
    }
    if let Some(desync) = session.desync() {
        println!(
            "DESYNC player={} tick={} local={:#018x} remote player {}={:#018x}",
            local.0, desync.tick, desync.local_hash, desync.player.0, desync.remote_hash
        );
        return Ok(1);
    }
    println!(
        "player={} ticks={} hash={:#018x} verified={} input_delay={} max_input_delay={max_delay} rtt_ms={rtt}",
        local.0,
        sim.tick(),
        sim.state_hash(),
        session.verified_tick(),
        session.input_delay(),
    );
    Ok(0)
}
