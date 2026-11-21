//! Giving up (prompt 051): a computer player surrenders (`Command::Surrender`)
//! when its situation has been hopeless for a minute, so a lost match ends
//! instead of running out the clock.
//!
//! Hopeless [provisional]: the player cannot produce (no completed barracks
//! or vehicle works), and either none of its armed units can hurt any enemy
//! unit it knows to be alive (rifle squads against tanks), or the strength
//! of its armed units (price times hit point share) is below 15 % of the
//! known enemy's. Known enemies are those it saw and did not see destroyed
//! (`intel.rs`), so it never gives up because of units it could not see.
//! Never in a mission: there the script decides (its `hopeless` warning is
//! for the human player).

use crate::{Ai, PlayerView, Role};
use serde::{Deserialize, Serialize};
use sim::Command;

/// The situation must stay hopeless this long (ticks, 60 s).
const HOPELESS_TICKS: u64 = 1200;
/// Own strength below this share (percent) of the known enemy's.
const HOPELESS_PCT: u64 = 15;
/// With production but without headquarters: below this share.
const CRUSHED_PCT: u64 = 10;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct SurrenderState {
    since: Option<u64>,
    pub(crate) done: bool,
    /// Why it gave up (status line).
    pub(crate) reason: Option<String>,
}

impl Ai {
    /// Surrenders after `HOPELESS_TICKS` of a hopeless situation.
    pub(crate) fn think_surrender(&mut self, view: &PlayerView, out: &mut Vec<Command>) {
        if self.legacy || self.surrender.done || self.in_mission {
            return;
        }
        match self.hopeless(view) {
            Some(reason) => {
                let since = *self.surrender.since.get_or_insert(view.tick);
                if view.tick >= since + HOPELESS_TICKS {
                    out.push(Command::Surrender);
                    self.surrender.done = true;
                    self.surrender.reason = Some(reason);
                }
            }
            None => self.surrender.since = None,
        }
    }

    /// Why the situation is hopeless, `None` if it is not.
    fn hopeless(&self, view: &PlayerView) -> Option<String> {
        let produces = view.structures.iter().any(|s| {
            s.completed() && matches!(self.name_of(s.unit_type), "base_barracks" | "base_factory")
        });
        // With production left the player fights on while its
        // headquarters stands; without it, only while it is not crushed.
        let hq = view
            .structures
            .iter()
            .any(|s| self.name_of(s.unit_type) == "base_hq");
        let crushed = produces && !hq;
        if produces && !crushed {
            return None;
        }
        let armed: Vec<_> = view
            .own
            .iter()
            .filter(|u| u.alive() && u.carrier.is_none() && u.stock.ammo_pct != Some(0))
            .filter(|u| {
                let info = self.type_info(u.unit_type);
                !matches!(info.role, Role::Unarmed | Role::Structure | Role::Transport)
                    && (info.range > sim::Fixed::ZERO || info.indirect.is_some())
            })
            .collect();
        let defeated = |p: sim::PlayerId| view.enemy_players.iter().any(|&(q, d)| q == p && d);
        let known: Vec<_> = self
            .intel
            .known
            .iter()
            .filter(|(_, s)| !defeated(s.owner))
            .collect();
        if known.is_empty() {
            return None;
        }
        let hard_target = |t: u32| matches!(self.type_info(t).role, Role::Armor | Role::Structure);
        let can_hurt = known.iter().any(|(_, s)| {
            armed.iter().any(|u| {
                let info = self.type_info(u.unit_type);
                info.anti_tank || info.indirect.is_some() || !hard_target(s.unit_type)
            })
        });
        if !can_hurt {
            return Some(format!(
                "no unit can hurt the {} enemies known",
                known.len()
            ));
        }
        let own: u64 = armed
            .iter()
            .map(|u| {
                let pct = u64::from(u.hp) * 100 / u64::from(u.max_hp.max(1));
                u64::from(self.value_of(u.unit_type)) * pct / 100
            })
            .sum();
        let enemy = self.known_enemy(view).0;
        let pct = if crushed { CRUSHED_PCT } else { HOPELESS_PCT };
        (own * 100 < enemy * pct)
            .then(|| format!("strength {own} against {enemy} known (below {pct} %)"))
    }
}
