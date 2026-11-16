//! Victory conditions (prompt 027): who is defeated, and when the match is
//! decided.
//!
//! A player is defeated when they surrender or have no living units left.
//! The match is decided as soon as at most one player is undefeated: that
//! player wins, or it is a draw if the last players fall in the same tick.
//! Only matches that start with at least two players can be decided.
//! With teams (prompt 037) the match is decided once at most one team has
//! an undefeated player; the lowest undefeated player of that team is the
//! `winner`. A surrender hands nothing to the allies.

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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerStanding {
    pub player: PlayerId,
    /// Reason and tick (number of ticks simulated when it was decided).
    pub defeat: Option<(DefeatReason, u64)>,
}

/// The result of a decided match.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Outcome {
    /// `None` for a draw.
    pub winner: Option<PlayerId>,
    /// Number of ticks simulated when the match was decided.
    pub tick: u64,
}

/// Standings of all players, part of the simulation state.
#[derive(Clone, Debug, Serialize, Deserialize)]
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
    /// the match is decided if the undefeated players are all `allied`
    /// (at most one team, or one player without teams, is left).
    pub(crate) fn update(
        &mut self,
        tick: u64,
        living_units: impl Fn(PlayerId) -> usize,
        allied: impl Fn(PlayerId, PlayerId) -> bool,
    ) {
        if self.outcome.is_some() || self.players.len() < 2 {
            return;
        }
        for standing in &mut self.players {
            if standing.defeat.is_none() && living_units(standing.player) == 0 {
                standing.defeat = Some((DefeatReason::Eliminated, tick));
            }
        }
        let mut undefeated = self.players.iter().filter(|s| s.defeat.is_none());
        let first = undefeated.next().map(|s| s.player);
        if let Some(first) = first
            && undefeated.any(|s| !allied(first, s.player))
        {
            return;
        }
        self.outcome = Some(Outcome {
            winner: first,
            tick,
        });
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

#[cfg(test)]
mod tests {
    use super::*;

    const P: [PlayerId; 3] = [PlayerId(0), PlayerId(1), PlayerId(2)];

    fn ffa(a: PlayerId, b: PlayerId) -> bool {
        a == b
    }

    #[test]
    fn last_players_falling_together_is_a_draw() {
        let mut standings = Standings::new(&P[..2]);
        standings.update(5, |_| 3, ffa);
        assert_eq!(standings.outcome(), None);
        standings.update(6, |_| 0, ffa);
        assert_eq!(
            standings.outcome(),
            Some(Outcome {
                winner: None,
                tick: 6
            })
        );
        assert!(
            standings
                .players()
                .iter()
                .all(|s| s.defeat == Some((DefeatReason::Eliminated, 6)))
        );
    }

    #[test]
    fn three_players_decide_when_one_is_left() {
        let mut standings = Standings::new(&P);
        standings.update(1, |p| usize::from(p != P[2]), ffa);
        assert_eq!(standings.outcome(), None);
        standings.surrender(P[0], 2);
        standings.update(2, |p| usize::from(p != P[2]), ffa);
        assert_eq!(standings.outcome().map(|o| o.winner), Some(Some(P[1])));
        // Final: later changes do not matter.
        standings.update(3, |_| 0, ffa);
        assert_eq!(
            standings.outcome().map(|o| (o.winner, o.tick)),
            Some((Some(P[1]), 2))
        );
    }

    #[test]
    fn a_team_wins_when_every_enemy_is_defeated() {
        let p: [PlayerId; 4] = [PlayerId(0), PlayerId(1), PlayerId(2), PlayerId(3)];
        let mut standings = Standings::new(&p);
        // Teams {0, 2} and {1, 3}.
        let allied = |a: PlayerId, b: PlayerId| a.0 % 2 == b.0 % 2;
        standings.surrender(p[0], 1);
        standings.update(1, |_| 4, allied);
        assert_eq!(standings.outcome(), None, "player 2 still fights");
        standings.update(2, |q| usize::from(q != p[1]), allied);
        assert_eq!(standings.outcome(), None, "player 3 still fights");
        standings.update(3, |q| usize::from(q == p[2]), allied);
        assert_eq!(
            standings.outcome(),
            Some(Outcome {
                winner: Some(p[2]),
                tick: 3
            })
        );
    }

    #[test]
    fn single_player_is_never_decided() {
        let mut standings = Standings::new(&P[..1]);
        standings.update(1, |_| 0, ffa);
        assert_eq!(standings.outcome(), None);
    }
}
