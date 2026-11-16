//! `SimHost` functions for prompt 049 (wounds, buddy aid, medics, crews):
//! the wound state and what every soldier does for a comrade, wounded
//! counts for the HUD, bailed-out crews. The simulation decides
//! everything; `scenes/world/infantry_view.gd` and `soldier_motion.gd`
//! only show it.

use crate::{SimHost, ground_to_godot};
use godot::prelude::*;
use sim::{AidView, SoldierSnapshot, UnitId, WoundView};

/// Codes of `soldier_wounds`.
fn wound_code(w: WoundView) -> u8 {
    match w {
        WoundView::Fit => 0,
        WoundView::Bandaging => 1,
        WoundView::Down { stable: false } => 2,
        WoundView::Down { stable: true } => 3,
        WoundView::Limping => 4,
        WoundView::Bandaged => 5,
        WoundView::Gone => 6,
    }
}

/// Code of `soldier_aid_kinds` and the comrade involved (unit, index).
fn aid_code(s: &SoldierSnapshot) -> (u8, Option<(UnitId, u16)>) {
    match s.aid {
        AidView::None => (0, None),
        AidView::Drag {
            soldier,
            pulling: false,
        } => (1, Some((s.unit, soldier))),
        AidView::Drag {
            soldier,
            pulling: true,
        } => (2, Some((s.unit, soldier))),
        AidView::Dragged => (3, None),
        AidView::HelpUp { soldier } => (4, Some((s.unit, soldier))),
        AidView::Treat { unit, soldier } => (5, Some((unit, soldier))),
        AidView::Stretcher {
            unit,
            soldier,
            front: true,
        } => (6, Some((unit, soldier))),
        AidView::Stretcher {
            unit,
            soldier,
            front: false,
        } => (7, Some((unit, soldier))),
        AidView::OnStretcher => (8, None),
    }
}

#[godot_api(secondary)]
impl SimHost {
    /// Wound state of every soldier in the order of `soldier_units`:
    /// 0 fit, 1 lightly wounded (dressing the wound), 2 heavily wounded
    /// (down, bleeding), 3 down with the bleeding stopped, 4 treated and
    /// limping, 5 treated (bandage, recovered), 6 not there (a crew member
    /// who did not get out).
    #[func]
    fn soldier_wounds(&self) -> PackedByteArray {
        self.current
            .soldiers
            .iter()
            .map(|s| wound_code(s.wound))
            .collect()
    }

    /// What every soldier does for a comrade, or what is done with him
    /// (order of `soldier_units`): 0 nothing, 1 going to a wounded comrade,
    /// 2 dragging him, 3 being dragged, 4 pulling up a knocked-down
    /// comrade, 5 treating a casualty (medic), 6 / 7 front / rear stretcher
    /// bearer, 8 lying on a stretcher.
    #[func]
    fn soldier_aid_kinds(&self) -> PackedByteArray {
        self.current
            .soldiers
            .iter()
            .map(|s| aid_code(s).0)
            .collect()
    }

    /// Position of the comrade every soldier's aid concerns (order of
    /// `soldier_units`; his own position where there is none).
    #[func]
    fn soldier_aid_targets(&self) -> PackedVector3Array {
        let soldiers = &self.current.soldiers;
        soldiers
            .iter()
            .map(|s| {
                let other = aid_code(s).1.and_then(|(unit, index)| {
                    soldiers
                        .binary_search_by_key(&(unit, index), |o| (o.unit, o.index))
                        .ok()
                        .map(|k| &soldiers[k])
                });
                let o = other.unwrap_or(s);
                ground_to_godot(o.position, o.height)
            })
            .collect()
    }

    /// Lightly and heavily wounded (untreated) men per unit, only units
    /// that have some: `{unit id: Vector2i(light, heavy)}`.
    #[func]
    fn unit_wounded_counts(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let mut current: Option<(UnitId, i32, i32)> = None;
        let flush = |c: Option<(UnitId, i32, i32)>, d: &mut VarDictionary| {
            if let Some((id, light, heavy)) = c
                && light + heavy > 0
            {
                d.set(i64::from(id.0), Vector2i::new(light, heavy));
            }
        };
        for s in &self.current.soldiers {
            if current.is_none_or(|c| c.0 != s.unit) {
                flush(current, &mut d);
                current = Some((s.unit, 0, 0));
            }
            if s.hp == 0 {
                continue;
            }
            if let Some(c) = current.as_mut() {
                match s.wound {
                    WoundView::Bandaging => c.1 += 1,
                    WoundView::Down { .. } => c.2 += 1,
                    _ => {}
                }
            }
        }
        flush(current, &mut d);
        d
    }

    /// Whether unit `id` is a crew that bailed out of a destroyed vehicle.
    #[func]
    fn unit_is_crew(&self, id: i64) -> bool {
        let (Some(game), Ok(id)) = (self.game.as_ref(), u32::try_from(id)) else {
            return false;
        };
        game.sim.is_bailed_crew(UnitId(id))
    }

    /// Debug and self-test: wounds soldier `soldier` of unit `id` (heavily
    /// or lightly). Only in matches without other human players (it does
    /// not go over the network).
    #[func]
    fn debug_wound_soldier(&mut self, id: i64, soldier: i64, heavy: bool) {
        let (Some(game), Ok(id), Ok(soldier)) = (
            self.game.as_mut(),
            u32::try_from(id),
            u16::try_from(soldier),
        ) else {
            return;
        };
        if game.networked {
            godot_warn!("debug wounds are not possible in network matches");
            return;
        }
        game.sim.wound_soldier(UnitId(id), soldier, heavy);
    }
}
