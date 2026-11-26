//! `SimHost` functions for destroyed map objects (prompt 020): the stage
//! events of every tick, the state of changed objects and where an object
//! is drawn. Godot swaps model variants and plays effects from this.

use crate::helpers::collect_tick_events;
use crate::{SimHost, fixed_to_f32, objects};
use godot::prelude::*;
use sim::{Map, ObjectCause, ObjectEvent, ObjectStage, Remains, Sim};

/// Events kept for Godot at most (if it stops taking them).
const MAX_PENDING_EVENTS: usize = 4096;

/// Appends the object events of the latest tick.
pub(crate) fn collect_events(pending: &mut Vec<(u64, ObjectEvent)>, sim: &Sim) {
    collect_tick_events(pending, sim.tick(), sim.object_events(), MAX_PENDING_EVENTS);
}

/// Model variant that shows object `index` in `stage`: the damaged or ruin
/// variant of the rules, else the variant the map placed.
fn variant_for(sim: &Sim, map: &Map, index: u32, stage: ObjectStage) -> String {
    let object = &map.objects()[index as usize];
    let kind = &map.object_kinds()[usize::from(object.kind)];
    let rules = sim.destruction_rules();
    let class = rules.class_of(&kind.id).map(|c| &rules.classes[c]);
    let wanted = class.and_then(|c| match stage {
        ObjectStage::Intact => None,
        ObjectStage::Damaged => c.damaged_variant.clone(),
        ObjectStage::Destroyed => c.ruin_variant.clone(),
    });
    wanted.unwrap_or_else(|| kind.variant.clone())
}

/// Dictionary of an object's state for Godot.
fn state_dict(sim: &Sim, map: &Map, index: u32) -> VarDictionary {
    let mut d = VarDictionary::new();
    let Some(state) = sim.object_state(index) else {
        return d;
    };
    let object = &map.objects()[index as usize];
    let kind = &map.object_kinds()[usize::from(object.kind)];
    d.set("object", i64::from(index));
    d.set("id", kind.id.as_str());
    d.set("stage", state.stage.name());
    d.set(
        "remains",
        sim.object_remains(index).map_or("", Remains::name),
    );
    d.set("fall_deg", i64::from(state.fall_deg));
    d.set("hp", i64::from(state.hp));
    d.set(
        "max_hp",
        i64::from(sim.object_hp(index).map_or(0, |(_, max)| max)),
    );
    d.set(
        "variant",
        variant_for(sim, map, index, state.stage).as_str(),
    );
    // Prompt 041: burned out objects are drawn charred.
    d.set("burnt", sim.object_burnt(index));
    d
}

#[godot_api(secondary)]
impl SimHost {
    /// Objects that changed their stage since the last call, in order:
    /// `{tick, object, id, stage ("intact" / "damaged" / "destroyed"),
    /// remains ("ruin" / "fallen" / "gone" / "flat"), fall_deg, hp, max_hp,
    /// variant (model variant to show), position (Vector3, the hit or the
    /// crushing vehicle), cause ("hit" / "crushed" / "fire"), unit, burnt
    /// (the fire burned out, prompt 041)}`.
    #[func]
    fn take_object_events(&mut self) -> Array<VarDictionary> {
        let events = std::mem::take(&mut self.object_events);
        let Some(game) = self.game.as_ref() else {
            return Array::new();
        };
        let mut out = Array::new();
        for (tick, e) in events {
            let mut d = state_dict(&game.sim, game.map(), e.object);
            // The event's own stage (the object may have changed again).
            d.set("stage", e.stage.name());
            d.set(
                "variant",
                variant_for(&game.sim, game.map(), e.object, e.stage).as_str(),
            );
            d.set("fall_deg", i64::from(e.fall_deg));
            d.set("tick", i64::try_from(tick).unwrap_or(i64::MAX));
            d.set(
                "position",
                Vector3::new(
                    fixed_to_f32(e.at.x),
                    fixed_to_f32(e.at.z),
                    fixed_to_f32(e.at.y),
                ),
            );
            let (cause, unit) = match e.cause {
                ObjectCause::Hit(u) => ("hit", u),
                ObjectCause::Crushed(u) => ("crushed", u),
                ObjectCause::Fire(u) => ("fire", u),
            };
            d.set("cause", cause);
            d.set("unit", i64::from(unit.0));
            out.push(&d);
        }
        out
    }

    /// Every object whose state differs from its start (ascending index),
    /// as in `take_object_events` without tick, position and cause. For
    /// building the presentation of a running match.
    #[func]
    fn changed_objects(&self) -> Array<VarDictionary> {
        let Some(game) = self.game.as_ref() else {
            return Array::new();
        };
        game.sim
            .changed_objects()
            .map(|(i, _)| state_dict(&game.sim, game.map(), i))
            .collect()
    }

    /// State of object `index` (empty for an unknown index).
    #[func]
    fn object_state(&self, index: i64) -> VarDictionary {
        let Some(game) = self.game.as_ref() else {
            return VarDictionary::new();
        };
        u32::try_from(index)
            .ok()
            .filter(|&i| (i as usize) < game.map().objects().len())
            .map_or_else(VarDictionary::new, |i| state_dict(&game.sim, game.map(), i))
    }

    /// Where `object_groups(chunk_m)` draws object `index`:
    /// `{kind, chunk: Vector2i, slot}` (slot = instance in the buffer);
    /// empty for an unknown index.
    #[func]
    fn object_instance(&self, index: i64, chunk_m: f32) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(game) = self.game.as_ref() else {
            return d;
        };
        let Some((kind, chunk, slot)) = usize::try_from(index)
            .ok()
            .and_then(|i| objects::instance_of(game.map(), i, chunk_m.max(1.0)))
        else {
            return d;
        };
        d.set("kind", i64::from(kind));
        d.set("chunk", Vector2i::new(chunk.0, chunk.1));
        d.set("slot", i64::from(slot));
        d
    }

    /// Indices of objects of the catalog `id` within `radius` meters of
    /// `(x, z)` that can still be destroyed, nearest first (self-test,
    /// tools).
    #[func]
    fn destructible_objects_near(
        &self,
        id: GString,
        x: f32,
        z: f32,
        radius: f32,
    ) -> PackedInt64Array {
        let Some(game) = self.game.as_ref() else {
            return PackedInt64Array::new();
        };
        let id = id.to_string();
        let kinds = game.map().object_kinds();
        let mut found: Vec<(f32, i64)> = (0u32..)
            .zip(game.map().objects())
            .filter(|(_, o)| kinds[usize::from(o.kind)].id == id)
            .filter(|&(i, _)| {
                game.sim
                    .object_stage(i)
                    .is_some_and(|s| s != ObjectStage::Destroyed)
                    && game.sim.object_hp(i).is_some_and(|(_, max)| max > 0)
            })
            .map(|(i, o)| {
                let dx = fixed_to_f32(o.position.x) - x;
                let dz = fixed_to_f32(o.position.y) - z;
                ((dx * dx + dz * dz).sqrt(), i64::from(i))
            })
            .filter(|&(d, _)| d <= radius)
            .collect();
        found.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        found.into_iter().map(|(_, i)| i).collect()
    }
}
