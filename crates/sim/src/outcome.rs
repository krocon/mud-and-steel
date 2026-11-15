//! Victory conditions (prompt 027): who is defeated, and when the match is
//! decided.
//!
//! A player is defeated when they surrender or have no living units left.
//! The match is decided as soon as at most one player is undefeated: that
//! player wins, or it is a draw if the last players fall in the same tick.
//! Only matches that start with at least two players can be decided. There
//! are no teams yet: every player fights every other.

use crate::command::PlayerId;
use crate::hash::StableHasher;
use serde::{Deserialize, Serialize};

/// Why a player lost.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DefeatReason {
    /// The player sent [`Command::Surrender`](crate::Command::Surrender).
    Surrendered,
    /// The player had no living units left.
    Eliminated,
}

/// A player of the match and whether (and when) they lost.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerStanding {
    pub player: PlayerId,
    /// Reason and tick (number of ticks simulated when it was decided).
    pub defeat: Option<(DefeatReason, u64)>,
}

/// The result of a decided match.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// `None` for a draw.
    pub winner: Option<PlayerId>,
    /// Number of ticks simulated when the match was decided.
    pub tick: u64,
}

/// Standings of all players, part of the simulation state.
#[derive(Clone, Debug)]
pub(crate) struct Standings {
    /// Ascending by player id.
    players: Vec<PlayerStanding>,
    outcome: Option<Outcome>,
}

impl Standings {
    /// `players` must be sorted and deduplicated.
    pub(crate) fn new(players: &[PlayerId]) -> Self {
        Self {
            players: players
                .iter()
                .map(|&player| PlayerStanding {
                    player,
                    defeat: None,
                })
                .collect(),
            outcome: None,
        }
    }

    pub(crate) fn players(&self) -> &[PlayerStanding] {
        &self.players
    }

    pub(crate) const fn outcome(&self) -> Option<Outcome> {
        self.outcome
    }

    /// Marks `player` as surrendered. Ignored for unknown or already defeated
    /// players and once the match is decided.
    pub(crate) fn surrender(&mut self, player: PlayerId, tick: u64) {
        if self.outcome.is_some() {
            return;
        }
        if let Some(standing) = self.players.iter_mut().find(|s| s.player == player)
            && standing.defeat.is_none()
        {
            standing.defeat = Some((DefeatReason::Surrendered, tick));
        }
    }

    /// End-of-tick check: players without living units are eliminated, then
    /// the match is decided if at most one player is left.
    pub(crate) fn update(&mut self, tick: u64, living_units: impl Fn(PlayerId) -> usize) {
        if self.outcome.is_some() || self.players.len() < 2 {
            return;
        }
        for standing in &mut self.players {
            if standing.defeat.is_none() && living_units(standing.player) == 0 {
                standing.defeat = Some((DefeatReason::Eliminated, tick));
            }
        }
        let mut undefeated = self.players.iter().filter(|s| s.defeat.is_none());
        match (undefeated.next(), undefeated.next()) {
            (Some(_), Some(_)) => {}
            (winner, _) => {
                self.outcome = Some(Outcome {
                    winner: winner.map(|s| s.player),
                    tick,
                });
            }
        }
    }

    /// Feeds the standings into the state hash, but only once a player has
    /// been defeated: before that they carry no information beyond the
    /// player list (which the units already determine), so the hashes of
    /// undecided matches stay as they were before victory conditions existed.
    pub(crate) fn hash_into(&self, h: &mut StableHasher) {
        if self.players.iter().all(|s| s.defeat.is_none()) {
            return;
        }
        h.write_u8(0xD5);
        h.write_u64(self.players.len() as u64);
        for standing in &self.players {
            h.write_u8(standing.player.0);
            match standing.defeat {
                None => h.write_u8(0),
                Some((reason, tick)) => {
                    h.write_u8(match reason {
                        DefeatReason::Surrendered => 1,
                        DefeatReason::Eliminated => 2,
                    });
                    h.write_u64(tick);
                }
            }
        }
        match self.outcome {
            None => h.write_u8(0),
            Some(outcome) => {
                match outcome.winner {
                    Some(player) => {
                        h.write_u8(1);
                        h.write_u8(player.0);
                    }
                    None => h.write_u8(2),
                }
                h.write_u64(outcome.tick);
            }
        }
    }
}
