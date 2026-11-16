//! Lobby slots (prompt 037): computer players, teams, factions, closed
//! slots, over in-process links.

use net::lobby::{LobbyNode, LobbyState, MemoryLink, MemoryNetwork, SlotKind};
use sim::PlayerId;
use sim::scenario::demo_config;

type Node = LobbyNode<MemoryLink>;

const HOST: u64 = 76_561_000_000_000_000;
const SEED: u64 = 42;

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

#[test]
fn slots_hold_computers_teams_and_factions() {
    let network = MemoryNetwork::new();
    let mut host = LobbyNode::new_host(network.endpoint(HOST), "host", 4);
    // Slot 1 for a human, slots 2 and 3 for computers.
    host.edit_setup(|s| {
        s.slots[2].kind = SlotKind::Computer("normal".into());
        s.slots[3].kind = SlotKind::Computer("hard".into());
        s.mode = "base".into();
        s.map = "twin_meadow".into();
    });
    let mut guest = client(&network, 1, "guest");
    poll_until(&mut [&mut host, &mut guest], |n| {
        n[1].setup().slots.len() == 4 && n[1].local_player().is_some()
    });
    assert_eq!(guest.local_player(), Some(PlayerId(1)));
    assert_eq!(guest.setup().slots[1].kind, SlotKind::Human);
    assert_eq!(guest.setup().mode, "base");
    // The game is full: every slot is taken.
    let mut late = client(&network, 2, "late");
    poll_until(&mut [&mut host, &mut guest, &mut late], |n| {
        matches!(n[2].state(), LobbyState::Closed(_))
    });
    assert!(matches!(late.state(), LobbyState::Closed(r) if r.contains("full")));

    // The guest picks its own team and faction; the host cannot take its
    // slot away.
    guest.choose(0, "us");
    host.edit_setup(|s| s.slots[1].kind = SlotKind::Closed);
    poll_until(&mut [&mut host, &mut guest], |n| {
        n[1].setup().slots[1].faction == "us"
    });
    assert_eq!(host.setup().slots[1].kind, SlotKind::Human);
    assert_eq!(host.setup().slots[1].team, 0);
    // Everybody on one team: no match.
    host.edit_setup(|s| {
        for slot in &mut s.slots {
            slot.team = 0;
        }
    });
    guest.set_ready(true);
    poll_until(&mut [&mut host, &mut guest], |n| {
        n[0].players().iter().all(|p| p.ready)
    });
    assert!(!host.can_start(), "one team only");
    host.edit_setup(|s| {
        s.slots[1].team = 1;
        s.slots[3].team = 1;
    });
    assert!(host.can_start());
    let info = host.start(SEED, demo_config(), 3, 10).unwrap();
    assert_eq!(info.players.len(), 4);
    assert_eq!(
        info.computers,
        vec![(PlayerId(2), "normal".into()), (PlayerId(3), "hard".into())]
    );
    poll_until(&mut [&mut host, &mut guest], |n| {
        *n[1].state() == LobbyState::Started
    });
    assert_eq!(guest.start_info(), Some(&info));
}

#[test]
fn a_leaving_human_frees_the_slot_and_closed_slots_take_nobody() {
    let network = MemoryNetwork::new();
    let mut host = LobbyNode::new_host(network.endpoint(HOST), "host", 3);
    host.edit_setup(|s| s.slots[1].kind = SlotKind::Closed);
    assert_eq!(host.max_players(), Some(2));
    let mut guest = client(&network, 1, "guest");
    poll_until(&mut [&mut host, &mut guest], |n| {
        n[1].local_player().is_some()
    });
    assert_eq!(guest.local_player(), Some(PlayerId(2)));
    guest.leave();
    host.poll();
    assert_eq!(host.setup().slots[2].kind, SlotKind::Open);
    assert_eq!(host.setup().participants(), vec![PlayerId(0)]);
}
