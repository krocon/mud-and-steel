//! A 2-against-2 through the lobby (prompt 037): two humans (scripted, one
//! of them the host) and two computer players hosted by the host, over
//! in-process links. Both peers must stay in sync; the lockstep hash
//! exchange checks every tick.

use ai::{Ai, Difficulty, PlayerView};
use net::LockstepSession;
use net::lobby::{LobbyNode, LobbyState, MemoryLink, MemoryNetwork, SlotKind};
use sim::scenario::{DEMO_SCENARIO, SlotSetup, parse_scenario_for_slots};
use sim::{Command, Map, PlayerId, Sim, Vec2};
use std::path::Path;
use std::sync::Arc;

type Node = LobbyNode<MemoryLink>;

const HOST: u64 = 7_000;
const SEED: u64 = 9;
const TICKS: u64 = 600;

fn twin_meadow() -> Arc<Map> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/maps/twin_meadow");
    let read = |f: &str| std::fs::read(dir.join(f)).unwrap();
    let mut map = Map::load(
        &String::from_utf8(read("map.toml")).unwrap(),
        &read("height.png"),
        &read("terrain.png"),
    )
    .unwrap();
    map.add_objects(&String::from_utf8(read("objects.csv")).unwrap())
        .unwrap();
    Arc::new(map)
}

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

struct Peer {
    sim: Sim,
    session: LockstepSession,
    node: Node,
    ais: Vec<Ai>,
    hashes: Vec<u64>,
}

impl Peer {
    fn poll(&mut self) {
        while self.session.needs_local_input() {
            // The humans drive their units to the middle once.
            let tick = self.session.next_local_tick();
            let local = self.session.local_player();
            let commands = if tick == 40 {
                let units = self
                    .sim
                    .snapshot()
                    .units
                    .iter()
                    .filter(|u| u.owner == local)
                    .map(|u| u.id)
                    .collect();
                vec![Command::MoveTo {
                    units,
                    target: Vec2::from_meters(0, 0),
                }]
            } else {
                Vec::new()
            };
            self.session.submit_local(commands, &mut self.node).unwrap();
            for ai in &mut self.ais {
                let view = PlayerView::new(&self.sim, ai.player());
                let commands = ai.think(&view);
                self.session
                    .submit_hosted(ai.player(), commands, &mut self.node)
                    .unwrap();
            }
        }
        self.session.receive(&mut self.node).unwrap();
        if let Some(commands) = self.session.next_commands() {
            self.sim.step(&commands);
            let hash = self.sim.state_hash();
            self.hashes.push(hash);
            self.session
                .record_hash(self.sim.tick(), hash, &mut self.node)
                .unwrap();
        }
    }
}

#[test]
fn two_humans_and_two_computers_play_two_against_two() {
    let map = twin_meadow();
    let network = MemoryNetwork::new();
    let mut host = LobbyNode::new_host(network.endpoint(HOST), "host", 4);
    host.edit_setup(|s| {
        s.slots[2].kind = SlotKind::Computer("normal".into());
        s.slots[3].kind = SlotKind::Computer("hard".into());
        s.map = "twin_meadow".into();
    });
    let mut link = network.endpoint(HOST + 1);
    link.connect(HOST);
    let mut guest = LobbyNode::new_client(link, HOST, "guest");
    poll_until(&mut [&mut host, &mut guest], |n| {
        n[1].local_player().is_some()
    });
    guest.set_ready(true);
    poll_until(&mut [&mut host, &mut guest], |n| n[0].can_start());

    // The host builds the match from its setup, as the game does.
    let setup = host.setup().clone();
    let slots: Vec<SlotSetup> = setup
        .participants()
        .iter()
        .map(|&p| SlotSetup {
            player: p,
            team: setup.slots[usize::from(p.0)].team,
            faction: None,
        })
        .collect();
    let config = parse_scenario_for_slots(
        DEMO_SCENARIO,
        &slots,
        setup.shared_vision,
        &sim::Catalog::builtin().unwrap(),
        &sim::Scale::builtin().unwrap(),
        &sim::TerrainRules::builtin().unwrap(),
        &map,
    )
    .unwrap();
    let info = host.start(SEED, config, 2, 10).unwrap();
    poll_until(&mut [&mut host, &mut guest], |n| {
        *n[1].state() == LobbyState::Started
    });
    assert_eq!(
        info.players,
        vec![PlayerId(0), PlayerId(1), PlayerId(2), PlayerId(3)]
    );

    let mut peers: Vec<Peer> = [host, guest]
        .into_iter()
        .map(|node| {
            let info = node.start_info().unwrap().clone();
            let local = node.local_player().unwrap();
            let mut session = LockstepSession::new(local, &info.lockstep_config());
            let mut ais = Vec::new();
            if node.is_host() {
                for (player, level) in &info.computers {
                    session.host_player(*player);
                    let level: Difficulty = level.parse().unwrap();
                    ais.push(Ai::new(*player, level, &info.config, &map, info.seed));
                }
            }
            Peer {
                sim: Sim::new(&info.config, map.clone(), info.seed).unwrap(),
                session,
                node,
                ais,
                hashes: Vec::new(),
            }
        })
        .collect();
    let mut rounds = 0;
    while peers.iter().any(|p| p.sim.tick() < TICKS) {
        rounds += 1;
        assert!(rounds < 20 * TICKS, "lockstep stalled");
        for peer in &mut peers {
            if peer.sim.tick() < TICKS {
                peer.poll();
            }
        }
    }
    for _ in 0..10 {
        for peer in &mut peers {
            peer.session.receive(&mut peer.node).unwrap();
        }
    }
    for peer in &peers {
        assert_eq!(peer.session.desync(), None);
        assert_eq!(peer.session.verified_tick(), TICKS);
        assert_eq!(peer.hashes, peers[0].hashes);
    }
    // The computers played: both gave orders, units were lost.
    assert!(peers[0].ais.iter().all(|ai| ai.decisions() > 0));
    let alive: usize = (0..4).map(|p| peers[0].sim.alive_units(PlayerId(p))).sum();
    assert!(alive < peers[0].sim.snapshot().units.len());
}
