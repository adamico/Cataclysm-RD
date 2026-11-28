//! Bubble-map Fields rules (`map::` members in `map_field.cpp`) and the
//! `mapbuffer` field operations they go through (`map.cpp` / `mapbuffer.cpp`).
//!
//! These work in absolute map squares (`Tripoint`). Creature rules are in
//! [`crate::creatures`].

use crate::content::{FieldTypeId, FieldTypes, Phase, Turns};
use crate::heap::StdPriorityQueue;
use crate::ports::{BubbleCaches, BubbleMap, Combat, CreatureId, SubmapStore};
use crate::storage::{SubTile, Tripoint};
use bn_dice::{random_entry_removed, Dice};
use std::collections::BTreeSet;

/// `emit_id` → the emit's data (`emit.h`); loading emits is out of scope.
#[derive(Clone, Debug)]
pub struct Emit {
    pub field: FieldTypeId,
    pub intensity: i32,
    pub qty: i32,
    pub chance: i32,
}

pub struct MapCtx<'a> {
    pub types: &'a FieldTypes,
    pub store: &'a mut dyn SubmapStore,
    pub caches: &'a mut dyn BubbleCaches,
    pub map: &'a dyn BubbleMap,
    pub combat: &'a mut dyn Combat,
    pub dice: &'a mut dyn Dice,
    /// `get_player_character()`, for `add_field(..., hit_player = true)`.
    pub player: Option<CreatureId>,
}

impl MapCtx<'_> {
    fn tile(&self, p: Tripoint) -> Option<SubTile> {
        let (sm, local) = p.split();
        self.store.fields(sm)?;
        Some(SubTile { sm, local })
    }

    /// `map::get_field_intensity(p, type)`: 0 if absent (or not loaded).
    pub fn get_field_intensity(&self, p: Tripoint, ty: FieldTypeId) -> i32 {
        let Some(t) = self.tile(p) else { return 0 };
        self.store.fields(t.sm).unwrap().get(t.local).find(ty).map_or(0, |e| e.intensity())
    }

    /// `mapbuffer::add_field` (no `hit_player`).
    fn mb_add_field(&mut self, p: Tripoint, ty: FieldTypeId, intensity: i32, age: Turns) -> bool {
        if !ty.is_some() {
            return false;
        }
        let Some(t) = self.tile(p) else { return false };
        let intensity = intensity.min(self.types.get(ty).max_intensity());
        if intensity <= 0 {
            return false;
        }
        self.store.mark_not_uniform(t.sm);
        let types = self.types;
        let sm = self.store.fields_mut(t.sm).unwrap();
        if sm.get_mut(t.local).add(types, ty, intensity, age) {
            sm.field_count += 1;
            sm.field_cache.push(t.local);
        }
        self.caches.field_added(p, ty);
        true
    }

    /// `map::add_field(p, type, intensity, age, hit_player)`.
    pub fn add_field(&mut self, p: Tripoint, ty: FieldTypeId, intensity: i32, age: Turns, hit_player: bool) -> bool {
        if !self.mb_add_field(p, ty, intensity, age) {
            return false;
        }
        if hit_player {
            if let Some(pc) = self.player {
                if p == self.combat.pos(pc) {
                    // Hit the player with the field if it spawned on top of them.
                    crate::creatures::creature_in_field(self, pc);
                }
            }
        }
        true
    }

    /// `mapbuffer::remove_field`.
    fn mb_remove_field(&mut self, p: Tripoint, ty: FieldTypeId) -> bool {
        let Some(t) = self.tile(p) else { return false };
        let types = self.types;
        let sm = self.store.fields_mut(t.sm).unwrap();
        if !sm.get_mut(t.local).remove(types, ty) {
            return false;
        }
        sm.field_count -= 1;
        self.caches.field_removed(p, ty);
        true
    }

    /// `map::mod_field_intensity` → `mapbuffer::set_field_intensity(isoffset = true)`.
    pub fn mod_field_intensity(&mut self, p: Tripoint, ty: FieldTypeId, offset: i32) -> i32 {
        let Some(t) = self.tile(p) else { return 0 };
        let types = self.types;
        // get_field_entry: nullptr unless the submap's field_count > 0.
        let sm = self.store.fields_mut(t.sm).unwrap();
        let has_any = sm.field_count > 0;
        if has_any {
            if let Some(e) = sm.get_mut(t.local).find_mut(ty) {
                let adjusted = e.intensity() + offset;
                if adjusted > 0 {
                    return e.set_intensity(types, adjusted);
                }
                self.mb_remove_field(p, ty);
                return 0;
            }
        }
        if offset <= 0 {
            return 0;
        }
        if self.mb_add_field(p, ty, offset, 0) {
            offset
        } else {
            0
        }
    }
}

/// `map::create_hot_air(p, intensity)`.
pub fn create_hot_air(ctx: &mut MapCtx<'_>, p: Tripoint, intensity: i32) {
    let fd = &ctx.types.fd;
    let hot_air = match intensity {
        1 => fd.hot_air1,
        2 => fd.hot_air2,
        3 => fd.hot_air3,
        4 => fd.hot_air4,
        _ => return, // debugmsg("Tried to spread hot air with intensity %d")
    };
    for _ in 0..5 {
        // `point(rng(-1, 1), rng(-1, 1))`: argument order is unspecified in C++;
        // the Reference build is clang, which evaluates left to right.
        let dx = ctx.dice.rng(-1, 1);
        let dy = ctx.dice.rng(-1, 1);
        ctx.add_field(p.offset(dx, dy), hot_air, 1, 0, true);
    }
}

/// `map::get_wind_blockers(winddirection, pos)` → the three tiles (as points).
pub fn get_wind_blockers(winddirection: i32, pos: Tripoint) -> (Tripoint, Tripoint, Tripoint) {
    const E: (i32, i32) = (1, 0);
    const NE: (i32, i32) = (1, -1);
    const SE: (i32, i32) = (1, 1);
    const S: (i32, i32) = (0, 1);
    const SW: (i32, i32) = (-1, 1);
    const W: (i32, i32) = (-1, 0);
    const NW: (i32, i32) = (-1, -1);
    const N: (i32, i32) = (0, -1);
    let outputs: [(i32, [(i32, i32); 3]); 9] = [
        (330, [E, NE, SE]),
        (301, [SE, E, S]),
        (240, [S, SW, SE]),
        (211, [SW, W, S]),
        (150, [W, NW, SW]),
        (121, [NW, N, W]),
        (60, [N, NW, NE]),
        (31, [NE, E, N]),
        (0, [E, NE, SE]),
    ];
    // Default-constructed tripoints if no row matches (negative direction).
    let mut r = (Tripoint::default(), Tripoint::default(), Tripoint::default());
    for (th, pts) in outputs {
        if winddirection >= th {
            r = (pos.offset(pts[0].0, pts[0].1), pos.offset(pts[1].0, pts[1].1), pos.offset(pts[2].0, pts[2].1));
            break;
        }
    }
    r
}

/// `map::emit_field(pos, src, mul)`.
pub fn emit_field(ctx: &mut MapCtx<'_>, pos: Tripoint, src: Option<&Emit>, mul: f32) {
    let Some(src) = src else { return };
    let chance = src.chance as f32 * mul;
    if ctx.dice.x_in_y(chance as f64, 100.0) {
        let qty = if chance > 100.0 {
            ctx.dice.roll_remainder((src.qty as f32 * chance / 100.0f32) as f64)
        } else {
            src.qty
        };
        propagate_field(ctx, pos, src.field, qty, src.intensity);
    }
}

type GasBlast = (f32, Tripoint);

/// `map::propagate_field(center, type, amount, max_intensity)`.
pub fn propagate_field(ctx: &mut MapCtx<'_>, center: Tripoint, ty: FieldTypeId, mut amount: i32, max_intensity: i32) {
    let mut open = StdPriorityQueue::new(|a: &GasBlast, b: &GasBlast| a.0 > b.0);
    let mut closed: BTreeSet<Tripoint> = BTreeSet::new();
    open.push((0.0, center));

    let not_gas = ctx.types.get(ty).phase != Phase::Gas;

    while amount > 0 && !open.is_empty() {
        if closed.contains(&open.top().1) {
            open.pop();
            continue;
        }

        // All points with equal gas intensity should propagate at the same time.
        let mut gas_front: Vec<GasBlast> = vec![*open.top()];
        let cur_intensity = ctx.get_field_intensity(open.top().1, ty);
        open.pop();
        while !open.is_empty() && ctx.get_field_intensity(open.top().1, ty) == cur_intensity {
            if !closed.contains(&open.top().1) {
                gas_front.push(*open.top());
            }
            open.pop();
        }

        let increment = 1.max(amount / gas_front.len() as i32);

        while !gas_front.is_empty() {
            let gp = random_entry_removed(ctx.dice, &mut gas_front);
            closed.insert(gp.1);
            let cur_intensity = ctx.get_field_intensity(gp.1, ty);
            if cur_intensity < max_intensity {
                let bonus = (max_intensity - cur_intensity).min(increment);
                ctx.mod_field_intensity(gp.1, ty, bonus);
                amount -= bonus;
            } else {
                amount -= 1;
            }

            if amount <= 0 {
                return;
            }

            const X_OFFSET: [i32; 8] = [-1, 1, 0, 0, 1, -1, -1, 1];
            const Y_OFFSET: [i32; 8] = [0, 0, -1, 1, -1, 1, -1, 1];
            for i in 0..8 {
                let pt = gp.1.offset(X_OFFSET[i], Y_OFFSET[i]);
                if closed.contains(&pt) {
                    continue;
                }
                if ctx.map.impassable(pt) && (not_gas || !ctx.map.permeable(pt)) {
                    closed.insert(pt);
                    continue;
                }
                if !ctx.map.obstructed_by_vehicle_rotation(gp.1, pt) {
                    open.push((ctx.map.rl_dist(center, pt) as f32, pt));
                }
            }
        }
    }
}
