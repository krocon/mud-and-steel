//! LAN peers over real UDP on localhost: lobby, start, lockstep, discovery,
//! rejection and disconnects. All peers run in this thread, interleaved.

mod common;

use common::{Peer, SEED};
use net::lan::{
    DiscoveryBrowser, DiscoveryResponder, GameInfo, LanNode, LanState, PROTOCOL_VERSION,
};
use net::{LockstepSession, Transport, recommended_input_delay};
use sim::scenario::{demo_config, run_demo};
use std::net::{Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant};

const TICKS: u64 = 600;
const TIMEOUT: Duration = Duration::from_secs(20);

fn localhost(port: u16) -> SocketAddr {
    SocketAddr::from((Ipv4Addr::LOCALHOST, port))
}

/// Polls all nodes until `done` holds, failing after `TIMEOUT`.
fn poll_until(nodes: &mut [&mut LanNode], mut done: impl FnMut(&[&mut LanNode]) -> bool) {
    let start = Instant::now();
    while !done(nodes) {
        assert!(start.elapsed() < TIMEOUT, "timed out: {nodes:?}");
        for node in nodes.iter_mut() {
            node.poll();
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// Host plus `clients` clients, all ready and started.
fn started_lobby(clients: usize) -> Vec<LanNode> {
    let mut host = LanNode::host("host", 0, clients + 1).unwrap();
    let address = localhost(host.port());
    let mut nodes = vec![];
    for i in 0..clients {
        let mut client = LanNode::join(&format!("client {i}"), address).unwrap();
        poll_until(&mut [&mut host, &mut client], |n| {
            *n[1].state() == LanState::Lobby && n[0].players().len() == i + 2
        });
        nodes.push(client);
    }
    for client in &mut nodes {
        client.set_ready(true);
    }
    {
        let mut all: Vec<&mut LanNode> = std::iter::once(&mut host).chain(&mut nodes).collect();
        poll_until(&mut all, |n| n[0].can_start());
    }
    let info = host.start(SEED, demo_config(), 3, 10).unwrap();
    nodes.insert(0, host);
    {
        let mut all: Vec<&mut LanNode> = nodes.iter_mut().collect();
        poll_until(&mut all, |n| {
            n.iter().all(|node| *node.state() == LanState::Started)
        });
    }
    for node in &nodes {
        assert_eq!(node.start_info(), Some(&info));
    }
    nodes
}

/// Plays `TICKS` ticks with every peer running its own demo script and
/// adapting its input delay to the measured round trip.
fn play(nodes: Vec<LanNode>) -> Vec<Peer<LanNode>> {
    let mut peers: Vec<Peer<LanNode>> = nodes
        .into_iter()
        .map(|node| {
            let info = node.start_info().unwrap().clone();
            let player = node.local_player().unwrap().0;
            Peer::with_config(player, node, &info.lockstep_config(), info.config)
        })
        .collect();
    let start = Instant::now();
    while peers.iter().any(|p| p.sim.tick() < TICKS) {
        assert!(start.elapsed() < TIMEOUT, "lockstep stalled");
        for peer in &mut peers {
            if peer.sim.tick() < TICKS {
                if peer.sim.tick().is_multiple_of(20)
                    && let Some((rtt, variance)) = peer.transport.round_trip_ms()
                {
                    let delay = recommended_input_delay(rtt, variance, 20);
                    peer.session.set_input_delay(delay);
                }
                peer.poll();
            }
        }
    }
    // Wait for the last hash reports.
    let start = Instant::now();
    while peers.iter().any(|p| p.session.verified_tick() < TICKS) {
        assert!(start.elapsed() < TIMEOUT, "hash reports missing");
        for peer in &mut peers {
            peer.session.receive(&mut peer.transport).unwrap();
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    peers
}

fn assert_in_sync(peers: &[Peer<LanNode>]) {
    let expected = run_demo(SEED, TICKS).state_hash();
    for peer in peers {
        assert_eq!(peer.session.desync(), None);
        assert_eq!(peer.hashes, peers[0].hashes, "per-tick hashes differ");
        assert_eq!(peer.sim.state_hash(), expected);
    }
}

#[test]
fn two_peers_play_over_udp() {
    let peers = play(started_lobby(1));
    assert_in_sync(&peers);
}

#[test]
fn host_relays_between_three_peers() {
    // Player 2 owns no units of the demo and only sends empty frames, but
    // players 0 and 1 still need its frames, relayed by the host.
    let peers = play(started_lobby(2));
    assert_eq!(peers.len(), 3);
    assert_in_sync(&peers);
}

#[test]
fn late_and_excess_joiners_are_rejected() {
    let mut host = LanNode::host("host", 0, 2).unwrap();
    let address = localhost(host.port());
    let mut first = LanNode::join("first", address).unwrap();
    poll_until(&mut [&mut host, &mut first], |n| {
        *n[1].state() == LanState::Lobby
    });
    let mut second = LanNode::join("second", address).unwrap();
    poll_until(&mut [&mut host, &mut first, &mut second], |n| {
        matches!(n[2].state(), LanState::Closed(_))
    });
    let LanState::Closed(reason) = second.state() else {
        unreachable!()
    };
    assert!(reason.contains("full"), "{reason}");
    assert_eq!(host.players().len(), 2);
    // Nobody is ready yet: the host cannot start.
    assert!(!host.can_start());
    assert!(host.start(SEED, demo_config(), 3, 10).is_err());
}

#[test]
fn a_lost_peer_ends_the_match() {
    let mut nodes = started_lobby(1);
    let mut client = nodes.pop().unwrap();
    let mut host = nodes.pop().unwrap();
    client.leave();
    let start = Instant::now();
    loop {
        assert!(start.elapsed() < TIMEOUT, "disconnect not detected");
        client.poll();
        match host.recv() {
            Err(net::TransportError::Disconnected) => break,
            Ok(_) => std::thread::sleep(Duration::from_millis(1)),
        }
    }
    assert!(matches!(host.state(), LanState::Closed(r) if r.contains("player 1 left")));
    // Sending after the loss fails as well.
    let mut session = LockstepSession::new(
        sim::PlayerId(0),
        &host.start_info().unwrap().lockstep_config(),
    );
    assert!(session.submit_local(Vec::new(), &mut host).is_err());
}

#[test]
fn discovery_finds_a_host_on_localhost() {
    // Port 0 picks a free port, so parallel tests do not collide.
    let info = GameInfo {
        protocol: PROTOCOL_VERSION,
        name: "discover me".into(),
        game_port: 4242,
        players: 1,
        max_players: 2,
    };
    let mut responder = DiscoveryResponder::new(0, info.clone()).unwrap();
    let mut browser = DiscoveryBrowser::new(responder.port()).unwrap();
    let start = Instant::now();
    while browser.games().is_empty() {
        assert!(start.elapsed() < TIMEOUT, "no answer");
        browser.query();
        std::thread::sleep(Duration::from_millis(5));
        responder.poll();
        std::thread::sleep(Duration::from_millis(5));
        browser.poll();
    }
    let game = browser
        .games()
        .iter()
        .find(|g| g.address.ip().is_loopback())
        .expect("localhost answer");
    assert_eq!(game.info, info);
    assert_eq!(game.address.port(), 4242);
}
