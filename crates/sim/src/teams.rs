//! Teams (prompt 037): which players are allies, and whether allies share
//! their vision.
//!
//! Part of [`SimConfig`](crate::SimConfig), so every peer and every replay
//! has the same teams. A player without an entry is on a team of their
//! own, so the default (no entries) is every player against every other,
//! as before teams existed.
//!
//! Allies do not pick each other as targets, do not fire through each
//! other, cannot order an attack on each other, see each other's units
//! and do not contest each other's supply points. With `shared_vision`
//! every player also sees what their allies see. Nobody can command an
//! ally's units. A team loses when all its players are defeated
//! (`crate::outcome`).

use crate::command::PlayerId;
use crate::hash::StableHasher;
use serde::{Deserialize, Serialize};

/// Team of every player of a match.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Teams {
    /// `(player, team)`, sorted by player, at most one entry per player.
    /// Players without an entry are on a team of their own.
    pub members: Vec<(PlayerId, u8)>,
    /// Allies see what the other players of their team see.
    pub shared_vision: bool,
}

impl Teams {
    /// Teams from `(player, team)` pairs (any order; a later entry for the
    /// same player wins). If no two players share a team, the result is a
    /// free-for-all without entries (the same match as without teams).
    pub fn new(members: &[(PlayerId, u8)], shared_vision: bool) -> Self {
        let mut sorted: Vec<(PlayerId, u8)> = Vec::with_capacity(members.len());
        for &(player, team) in members {
            match sorted.iter_mut().find(|(p, _)| *p == player) {
                Some(entry) => entry.1 = team,
                None => sorted.push((player, team)),
            }
        }
        sorted.sort_unstable_by_key(|&(p, _)| p);
        let mut teams: Vec<u8> = sorted.iter().map(|&(_, t)| t).collect();
        teams.sort_unstable();
        teams.dedup();
        if teams.len() == sorted.len() {
            return Self::default();
        }
        Self {
            members: sorted,
            shared_vision,
        }
    }

    /// No teams: every player fights every other.
    pub const fn is_free_for_all(&self) -> bool {
        self.members.is_empty()
    }

    /// The team of `player`: its entry, else a team of its own (numbers
    /// above 255 that no entry can name).
    pub fn team_of(&self, player: PlayerId) -> u16 {
        self.members
            .binary_search_by_key(&player, |&(p, _)| p)
            .map_or(256 + u16::from(player.0), |i| u16::from(self.members[i].1))
    }

    /// Whether `a` and `b` are on the same side (a player is its own ally).
    pub fn allied(&self, a: PlayerId, b: PlayerId) -> bool {
        a == b || self.team_of(a) == self.team_of(b)
    }

    /// Whether `a` and `b` are enemies.
    pub fn hostile(&self, a: PlayerId, b: PlayerId) -> bool {
        !self.allied(a, b)
    }

    /// Feeds the teams into the state hash. Nothing for a free-for-all, so
    /// matches without teams hash as before teams existed.
    pub(crate) fn hash_into(&self, h: &mut StableHasher) {
        if self.is_free_for_all() {
            return;
        }
        h.write_u8(0x7E);
        h.write_u64(self.members.len() as u64);
        for &(player, team) in &self.members {
            h.write_u8(player.0);
            h.write_u8(team);
        }
        h.write_u8(u8::from(self.shared_vision));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn players_without_entry_are_alone() {
        let teams = Teams::new(
            &[(PlayerId(2), 0), (PlayerId(0), 0), (PlayerId(1), 1)],
            true,
        );
        assert_eq!(teams.members[0].0, PlayerId(0));
        assert!(teams.allied(PlayerId(0), PlayerId(2)));
        assert!(teams.hostile(PlayerId(0), PlayerId(1)));
        assert!(teams.hostile(PlayerId(3), PlayerId(4)));
        assert!(teams.allied(PlayerId(3), PlayerId(3)));
        let ffa = Teams::default();
        assert!(ffa.is_free_for_all());
        assert_eq!(Teams::new(&[(PlayerId(0), 3), (PlayerId(1), 5)], true), ffa);
        assert!(ffa.hostile(PlayerId(0), PlayerId(1)));
    }
}
