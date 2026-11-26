//! The lobby protocol over in-process links (`MemoryLink`): the same
//! `LobbyNode` that runs over ENet on the LAN and over Steam online.

mod common;

use common::{Peer, SEED};
use net::lobby::{LobbyNode, LobbyState, MemoryLink, MemoryNetwork, PROTOCOL_VERSION};
use net::{LockstepSession, Transport, TransportError, recommended_input_delay};
use sim::scenario::{demo_config, run_demo};

type Node = LobbyNode<MemoryLink>;

const TICKS: u64 = 600;
const HOST: u64 = 76_561_000_000_000_000;

/// Steam Networking Messages refuses larger messages
/// (`k_cbMaxSteamNetworkingSocketsMessageSizeSend`).
const STEAM_MAX_MESSAGE: usize = 512 * 1024;

fn poll_until(nodes: &mut [&mut Node], mut done: impl FnMut(&[&mut Node]) -> bool) {
    for _ in 0..1000 {
        if done(nodes) {
            return;
        }
        for node in nodes.iter_mut() {
            node.poll();
        }
    }
    panic!("no progress: {nodes:?}");
}

fn client(network: &MemoryNetwork, id: u64, name: &str) -> Node {
    let mut link = network.endpoint(id);
    link.connect(HOST);
    LobbyNode::new_client(link, HOST, name)
}

/// Host plus `clients` clients, all ready and started.
fn started_lobby(network: &MemoryNetwork, clients: usize) -> Vec<Node> {
    let mut host = LobbyNode::new_host(network.endpoint(HOST), "host", clients + 1);
    let mut nodes = vec![];
    for i in 0..clients {
        let mut client = client(network, HOST + 1 + i as u64, &format!("client {i}"));
        poll_until(&mut [&mut host, &mut client], |n| {
            *n[1].state() == LobbyState::Lobby && n[0].players().len() == i + 2
        });
        nodes.push(client);
    }
    for client in &mut nodes {
        client.set_ready(true);
    }
    {
        let mut all: Vec<&mut Node> = std::iter::once(&mut host).chain(&mut nodes).collect();
        poll_until(&mut all, |n| n[0].can_start());
        // Everyone sees everyone, with the right ready flags.
        poll_until(&mut all, |n| {
            n.iter().all(|node| {
                node.players().len() == clients + 1 && node.players().iter().all(|p| p.ready)
            })
        });
    }
    let info = host.start(SEED, demo_config(), 3, 10).unwrap();
    nodes.insert(0, host);
    {
        let mut all: Vec<&mut Node> = nodes.iter_mut().collect();
        poll_until(&mut all, |n| {
            n.iter().all(|node| *node.state() == LobbyState::Started)
        });
    }
    for node in &nodes {
        assert_eq!(node.start_info(), Some(&info));
    }
    nodes
}

fn play(nodes: Vec<Node>) -> Vec<Peer<Node>> {
    let mut peers: Vec<Peer<Node>> = nodes
        .into_iter()
        .map(|node| {
            let info = node.start_info().unwrap().clone();
            let player = node.local_player().unwrap().0;
            Peer::with_config(player, node, &info.lockstep_config(), info.config)
        })
        .collect();
    let mut rounds = 0;
    while peers.iter().any(|p| p.sim.tick() < TICKS) {
        rounds += 1;
        assert!(rounds < 20 * TICKS, "lockstep stalled");
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
    for _ in 0..10 {
        for peer in &mut peers {
            peer.session.receive(&mut peer.transport).unwrap();
        }
    }
    peers
}

fn assert_in_sync(peers: &[Peer<Node>]) {
    let expected = run_demo(SEED, TICKS).state_hash();
    for peer in peers {
        assert_eq!(peer.session.desync(), None);
        assert_eq!(peer.session.verified_tick(), TICKS);
        assert_eq!(peer.hashes, peers[0].hashes, "per-tick hashes differ");
        assert_eq!(peer.sim.state_hash(), expected);
    }
}

#[test]
fn two_peers_play_over_memory_links() {
    let network = MemoryNetwork::new();
    // A measured round trip drives the dynamic input delay like on Steam.
    network.set_round_trip_ms(Some((120, 30)));
    let peers = play(started_lobby(&network, 1));
    assert_eq!(peers[0].transport.round_trip_ms(), Some((120, 30)));
    assert_eq!(
        peers[0].session.input_delay(),
        recommended_input_delay(120, 30, 20)
    );
    assert_in_sync(&peers);
}

#[test]
fn host_relays_between_three_peers() {
    let network = MemoryNetwork::new();
    let peers = play(started_lobby(&network, 2));
    assert_eq!(peers.len(), 3);
    assert_in_sync(&peers);
}

#[test]
fn full_games_and_running_matches_reject_joiners() {
    let network = MemoryNetwork::new();
    let mut host = LobbyNode::new_host(network.endpoint(HOST), "host", 2);
    let mut first = client(&network, 1, "first");
    poll_until(&mut [&mut host, &mut first], |n| {
        *n[1].state() == LobbyState::Lobby
    });
    let mut second = client(&network, 2, "second");
    poll_until(&mut [&mut host, &mut first, &mut second], |n| {
        matches!(n[2].state(), LobbyState::Closed(_))
    });
    assert!(matches!(second.state(), LobbyState::Closed(r) if r.contains("full")));
    assert_eq!(host.players().len(), 2);
    assert!(!host.can_start());
    assert!(host.start(SEED, demo_config(), 3, 10).is_err());

    // Once started, nobody can join any more.
    first.set_ready(true);
    poll_until(&mut [&mut host, &mut first], |n| n[0].can_start());
    host.start(SEED, demo_config(), 3, 10).unwrap();
    let mut late = client(&network, 3, "late");
    poll_until(&mut [&mut host, &mut first, &mut late], |n| {
        matches!(n[2].state(), LobbyState::Closed(_))
    });
    assert!(matches!(late.state(), LobbyState::Closed(r) if r.contains("already started")));
}

#[test]
fn a_different_protocol_version_is_rejected() {
    use net::lobby::Link;
    let network = MemoryNetwork::new();
    let mut host = LobbyNode::new_host(network.endpoint(HOST), "host", 4);
    let mut raw = network.endpoint(9);
    // `Control::Hello { protocol: 99, name: "old" }` in postcard: variant 0,
    // varint 99, string "old".
    raw.send(HOST, 0, &[0, 99, 3, b'o', b'l', b'd']);
    host.poll();
    assert_eq!(host.players().len(), 1);
    let mut events = vec![];
    while let Some(event) = raw.poll().unwrap() {
        events.push(event);
    }
    // A reject on the control channel, then the disconnect.
    assert!(
        matches!(
            &events[..],
            [
                net::lobby::LinkEvent::Received { channel: 0, .. },
                net::lobby::LinkEvent::Disconnected(HOST)
            ]
        ),
        "{events:?}"
    );
    assert_eq!(PROTOCOL_VERSION, net::lan::PROTOCOL_VERSION);
}

#[test]
fn a_lost_peer_ends_the_match_for_everyone() {
    let network = MemoryNetwork::new();
    let mut nodes = started_lobby(&network, 2);
    let lost = nodes.pop().unwrap();
    drop(lost); // Like a crashed process.
    let mut client = nodes.pop().unwrap();
    let mut host = nodes.pop().unwrap();
    assert_eq!(host.recv(), Err(TransportError::Disconnected));
    assert!(matches!(host.state(), LobbyState::Closed(r) if r.contains("player 2 left")));
    // The host tells the remaining client.
    assert_eq!(client.recv(), Err(TransportError::Disconnected));
    assert!(matches!(client.state(), LobbyState::Closed(r) if r.contains("player 2 left")));
    let mut session = LockstepSession::new(
        sim::PlayerId(0),
        &host.start_info().unwrap().lockstep_config(),
    );
    assert!(session.submit_local(Vec::new(), &mut host).is_err());
}

#[test]
fn a_client_leaving_the_lobby_frees_its_slot() {
    let network = MemoryNetwork::new();
    let mut host = LobbyNode::new_host(network.endpoint(HOST), "host", 2);
    let mut first = client(&network, 1, "first");
    poll_until(&mut [&mut host, &mut first], |n| n[0].players().len() == 2);
    first.leave();
    host.poll();
    assert_eq!(host.players().len(), 1);
    assert_eq!(*host.state(), LobbyState::Lobby);
    let mut second = client(&network, 2, "second");
    poll_until(&mut [&mut host, &mut second], |n| {
        *n[1].state() == LobbyState::Lobby
    });
    assert_eq!(second.local_player(), Some(sim::PlayerId(1)));
}

#[test]
fn an_unreachable_host_closes_the_client() {
    let network = MemoryNetwork::new();
    let mut node = client(&network, 1, "alone");
    node.poll();
    assert!(matches!(node.state(), LobbyState::Closed(r) if r.contains("could not connect")));
}

#[test]
fn the_start_message_fits_into_a_steam_message() {
    let network = MemoryNetwork::new();
    let nodes = started_lobby(&network, 1);
    let info = nodes[0].start_info().unwrap();
    let size = postcard::to_allocvec(info).unwrap().len();
    assert!(
        size < STEAM_MAX_MESSAGE / 4,
        "start message has {size} bytes"
    );
}

/// Prompt 182: the host's start variant reaches the client with the setup,
/// and the start message carries the match built in it.
#[test]
fn the_start_variant_travels_with_the_setup_and_the_config() {
    let network = MemoryNetwork::new();
    let mut host = LobbyNode::new_host(network.endpoint(HOST), "host", 2);
    let mut joiner = client(&network, HOST + 1, "client");
    poll_until(&mut [&mut host, &mut joiner], |n| {
        *n[1].state() == LobbyState::Lobby
    });
    host.edit_setup(|s| "west_east".clone_into(&mut s.variant));
    poll_until(&mut [&mut host, &mut joiner], |n| {
        n[1].setup().variant == "west_east"
    });
    joiner.set_ready(true);
    poll_until(&mut [&mut host, &mut joiner], |n| n[0].can_start());
    let mut config = demo_config();
    config.variant = Some(sim::VariantSetup {
        name: "west_east".into(),
        supply: Some(vec!["supply_village".into()]),
    });
    host.start(SEED, config.clone(), 3, 10).unwrap();
    poll_until(&mut [&mut host, &mut joiner], |n| {
        *n[1].state() == LobbyState::Started
    });
    assert_eq!(joiner.start_info().map(|i| &i.config), Some(&config));
}

/// Prompt 212: the host authenticates each game packet's `PlayerId` against the
/// sending peer's assigned slot, dropping spoofed frames/hashes and malformed
/// bytes without relaying them to other peers, while still relaying genuine
/// packets and allowing the host to send frames for hosted AI slots.
#[test]
fn host_blocks_and_does_not_relay_spoofed_player_id() {
    use net::{CommandFrame, HashReport, Message, decode, encode};
    use sim::{Command, PlayerId};

    let network = MemoryNetwork::new();
    let mut nodes = started_lobby(&network, 2);
    let mut c2 = nodes.pop().unwrap(); // PlayerId(2)
    let mut c1 = nodes.pop().unwrap(); // PlayerId(1)
    let mut host = nodes.pop().unwrap(); // PlayerId(0)
    assert_eq!(host.local_player(), Some(PlayerId(0)));
    assert_eq!(c1.local_player(), Some(PlayerId(1)));
    assert_eq!(c2.local_player(), Some(PlayerId(2)));

    // Client 1 tries to spoof PlayerId(2)'s CommandFrame, PlayerId(0)'s
    // HashReport, and malformed garbage bytes.
    let spoof_frame = encode(&Message::Frame(CommandFrame {
        tick: 3,
        player: PlayerId(2),
        commands: vec![Command::Surrender],
    }));
    let spoof_hash = encode(&Message::Hash(HashReport {
        tick: 20,
        player: PlayerId(0),
        hash: 0xbad0_0bad,
    }));
    c1.send(&spoof_frame).unwrap();
    c1.send(&spoof_hash).unwrap();
    c1.send(&[0xff, 0xff, 0xff]).unwrap();

    // Neither host nor client 2 may receive any of the spoofed/corrupt packets.
    assert_eq!(host.recv().unwrap(), None);
    assert_eq!(c2.recv().unwrap(), None);

    // Client 1's own genuine frame IS accepted by the host and relayed to c2.
    let genuine = encode(&Message::Frame(CommandFrame {
        tick: 3,
        player: PlayerId(1),
        commands: Vec::new(),
    }));
    c1.send(&genuine).unwrap();
    let at_host = host.recv().unwrap().expect("host receives genuine frame");
    assert_eq!(
        decode(&at_host).unwrap(),
        Message::Frame(CommandFrame {
            tick: 3,
            player: PlayerId(1),
            commands: Vec::new(),
        })
    );
    let at_c2 = c2
        .recv()
        .unwrap()
        .expect("c2 receives relayed genuine frame");
    assert_eq!(at_c2, at_host);

    // Host can still broadcast frames for a locally hosted AI player (PlayerId(3)).
    let ai_frame = encode(&Message::Frame(CommandFrame {
        tick: 3,
        player: PlayerId(3),
        commands: Vec::new(),
    }));
    host.send(&ai_frame).unwrap();
    assert_eq!(c1.recv().unwrap(), Some(ai_frame.clone()));
    assert_eq!(c2.recv().unwrap(), Some(ai_frame));
}
