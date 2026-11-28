//! Ports: where Fields stops and another context starts.
//!
//! Each trait is as narrow as the Fields rules need, and is listed in
//! `spike/rust/STUBS.md` with the BN code it stands in for.

use crate::content::{FieldTypeId, Turns};
use crate::storage::{Local, SmPos, SubTile, SubmapFields, Tripoint};
use bn_dice::Dice;

// ---------------------------------------------------------------- Reality bubble

/// Opaque terrain id (`ter_id`), owned by the terrain/furniture context.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct TerId(pub u32);
/// Opaque furniture id (`furn_id`).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct FurnId(pub u32);

/// Per-submap storage the Reality bubble lends to Fields (`submap` + `mapbuffer`).
pub trait SubmapStore {
    /// `mapbuffer::lookup_submap_in_memory(pos)` → its field storage, if loaded.
    fn fields(&self, sm: SmPos) -> Option<&SubmapFields>;
    fn fields_mut(&mut self, sm: SmPos) -> Option<&mut SubmapFields>;
    fn ter(&self, sm: SmPos, l: Local) -> TerId;
    fn furn(&self, sm: SmPos, l: Local) -> FurnId;
    fn set_ter(&mut self, sm: SmPos, l: Local, t: TerId);
    fn set_furn(&mut self, sm: SmPos, l: Local, f: FurnId);
    fn radiation(&self, sm: SmPos, l: Local) -> i32;
    fn set_radiation(&mut self, sm: SmPos, l: Local, r: i32);
    /// `submap::floor_cache[x][y]` (rebuilt by the map; zero-initialised).
    fn floor_cache(&self, sm: SmPos, l: Local) -> bool;
    /// `submap::is_uniform = false`.
    fn mark_not_uniform(&mut self, sm: SmPos);
}

/// The bubble map's caches that field changes invalidate.
pub trait BubbleCaches {
    /// `submap_loader.is_properly_requested(dim, pos)`.
    fn is_properly_requested(&self, sm: SmPos) -> bool;
    /// `map::set_transparency_cache_dirty(abs_to_bub(project_to<ms>(sm)))`.
    fn set_transparency_cache_dirty(&mut self, sm: SmPos);
    /// `map::set_seen_cache_dirty(...)`, same point.
    fn set_seen_cache_dirty(&mut self, sm: SmPos);
    /// `mapbuffer::invalidate_active_field_add_caches` / `_remove_caches`.
    fn field_added(&mut self, p: Tripoint, ty: FieldTypeId);
    fn field_removed(&mut self, p: Tripoint, ty: FieldTypeId);
}

/// Map-level geometry the bubble-map rules need (`map::impassable`, ...).
pub trait BubbleMap {
    /// `map::impassable(p)`.
    fn impassable(&self, p: Tripoint) -> bool;
    /// `map::has_flag(TFLAG_PERMEABLE, p)`.
    fn permeable(&self, p: Tripoint) -> bool;
    /// `map::obstructed_by_vehicle_rotation(from, to)`.
    fn obstructed_by_vehicle_rotation(&self, from: Tripoint, to: Tripoint) -> bool;
    /// `rl_dist(a, b)` (depends on the `trigdist` option).
    fn rl_dist(&self, a: Tripoint, b: Tripoint) -> i32;
    /// `map::veh_at(p)`.
    fn veh_at(&self, p: Tripoint) -> Option<VehTile>;
}

/// What Fields needs from `optional_vpart_position`.
#[derive(Copy, Clone, Debug, Default)]
pub struct VehTile {
    pub is_inside: bool,
    /// `part_with_feature(VPFLAG_NOFIELDS, true)`.
    pub nofields: bool,
}

// ---------------------------------------------------------------- terrain / furniture

/// `ter_bitflags` that Fields reads, plus the string flag `"FUNGUS"`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum MapFlag {
    Flammable,
    FlammableAsh,
    FlammableHard,
    FireContainer,
    NoFloor,
    Sealed,
    AllowFieldEffect,
    Swimmable,
    Permeable,
    /// `has_flag("FUNGUS")` (string flag).
    Fungus,
}

/// The terrain/furniture catalogue (`ter_t`, `furn_t`).
pub trait MapData {
    fn ter_has_flag(&self, t: TerId, f: MapFlag) -> bool;
    fn furn_has_flag(&self, f: FurnId, flag: MapFlag) -> bool;
    fn ter_movecost(&self, t: TerId) -> i32;
    fn furn_movecost(&self, f: FurnId) -> i32;
    fn t_pit(&self) -> TerId;
    fn t_dirt(&self) -> TerId;
    fn f_ash(&self) -> FurnId;
    fn f_null(&self) -> FurnId;
}

// ---------------------------------------------------------------- items

/// `fire_data`.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct FireData {
    pub fire_intensity: i32,
    pub smoke_produced: f32,
    pub fuel_produced: f32,
    pub contained: bool,
}

/// A detached item (`detached_ptr<item>`), opaque to Fields.
#[derive(Debug, PartialEq, Eq)]
pub struct DetachedItem(pub u64);

/// Items on a tile (`submap::get_items(local)`), addressed by index.
pub trait Items {
    fn count(&self, at: SubTile) -> usize;
    /// `item::weight(false)` in grams.
    fn weight_g(&self, at: SubTile, i: usize) -> i64;
    /// `item::burn(frd)`: may draw, so it takes the dice. True = destroyed.
    fn burn(&mut self, at: SubTile, i: usize, frd: &mut FireData, dice: &mut dyn Dice) -> bool;
    /// `create_burnproducts(out, fuel, burned_mass)` (needs materials).
    fn burnproducts(&self, at: SubTile, i: usize, burned_g: i64) -> Vec<DetachedItem>;
    /// `fuel->contents.clear_items()` filtered by `!is_irremovable()`.
    fn take_removable_contents(&mut self, at: SubTile, i: usize) -> Vec<DetachedItem>;
    fn erase(&mut self, at: SubTile, i: usize) -> DetachedItem;
    fn push(&mut self, at: SubTile, item: DetachedItem);
    /// `item::flammable()`.
    fn flammable(&self, at: SubTile, i: usize) -> bool;
    /// `it->typeId() == itype_rock`.
    fn is_rock(&self, at: SubTile, i: usize) -> bool;
    /// `item::age()` in turns.
    fn age(&self, at: SubTile, i: usize) -> Turns;
    /// `item::set_age(0_turns)`.
    fn set_age_zero(&mut self, at: SubTile, i: usize);
}

// ---------------------------------------------------------------- creatures (Combat)

/// A creature, as named by the Combat side.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CreatureId(pub u32);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DamageType {
    Acid,
    Heat,
    Electric,
}

/// `Creature::cmat_*` material groups.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MatGroup {
    Flesh,
    Flammable,
    Flameres,
    FleshNVeg,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MsgType {
    Good,
    Bad,
    Mixed,
    Warning,
    Info,
    Neutral,
}

/// Where harm to creatures crosses out of Fields.
///
/// Queries answer the questions BN's `player_in_field` / `monster_in_field`
/// ask a creature; sinks are every call that changes one. Fields decides,
/// draws and calls; the Combat side does the harm.
pub trait Combat {
    // ---- queries
    fn is_monster(&self, c: CreatureId) -> bool;
    /// `as_player()` is non-null (player or NPC).
    fn is_character(&self, c: CreatureId) -> bool;
    /// `is_player()` (the avatar).
    fn is_avatar(&self, c: CreatureId) -> bool;
    fn pos(&self, c: CreatureId) -> Tripoint;
    /// `player::in_vehicle`.
    fn in_vehicle(&self, c: CreatureId) -> bool;
    fn has_trait(&self, c: CreatureId, t: &str) -> bool;
    fn has_active_bionic(&self, c: CreatureId, b: &str) -> bool;
    fn is_wearing(&self, c: CreatureId, itype: &str) -> bool;
    fn worn_with_flag(&self, c: CreatureId, flag: &str) -> bool;
    fn is_on_ground(&self, c: CreatureId) -> bool;
    fn has_two_arms(&self, c: CreatureId) -> bool;
    fn is_elec_immune(&self, c: CreatureId) -> bool;
    fn is_underwater(&self, c: CreatureId) -> bool;
    fn get_armor_type(&self, c: CreatureId, dt: DamageType, bp: &str) -> i32;
    fn get_armor_cut(&self, c: CreatureId, bp: &str) -> i32;
    fn get_env_resist(&self, c: CreatureId, bp: &str) -> i32;
    /// Sum of `get_coverage(bp)` over worn items that cover `bp`.
    fn worn_coverage(&self, c: CreatureId, bp: &str) -> i32;
    fn get_highest_category(&self, c: CreatureId) -> String;
    fn get_all_body_parts(&self, c: CreatureId) -> Vec<String>;
    /// `get_random_body_part()`: draws on the Creature side.
    fn get_random_body_part(&mut self, c: CreatureId, dice: &mut dyn Dice) -> String;
    fn is_immune_field(&self, c: CreatureId, ty: FieldTypeId) -> bool;
    fn is_immune_effect(&self, c: CreatureId, eff: &str) -> bool;
    // monster queries
    fn digging(&self, c: CreatureId) -> bool;
    fn digs(&self, c: CreatureId) -> bool;
    fn flies(&self, c: CreatureId) -> bool;
    fn has_mon_flag(&self, c: CreatureId, f: &str) -> bool;
    fn made_of(&self, c: CreatureId, material: &str) -> bool;
    fn made_of_liquid(&self, c: CreatureId) -> bool;
    fn made_of_any(&self, c: CreatureId, g: MatGroup) -> bool;
    fn in_species(&self, c: CreatureId, s: &str) -> bool;

    // ---- sinks
    /// `deal_damage(nullptr, bp, damage_instance(dt, amount))` → dealt total.
    fn deal_damage(&mut self, c: CreatureId, bp: &str, dt: DamageType, amount: i32, dice: &mut dyn Dice) -> i32;
    /// `apply_damage(nullptr, bp, amount, true)`.
    fn apply_damage(&mut self, c: CreatureId, bp: &str, amount: i32);
    fn hurtall(&mut self, c: CreatureId, amount: i32);
    /// `add_effect(eff, dur, bp, intensity)`.
    fn add_effect(&mut self, c: CreatureId, eff: &str, dur: Turns, bp: Option<&str>, intensity: i32);
    /// `add_env_effect(eff, vector, strength, dur, bp, intensity)`: resist roll on the Creature side.
    #[allow(clippy::too_many_arguments)]
    fn add_env_effect(
        &mut self,
        c: CreatureId,
        eff: &str,
        vector: &str,
        strength: i32,
        dur: Turns,
        bp: Option<&str>,
        intensity: i32,
        dice: &mut dyn Dice,
    ) -> bool;
    /// `irradiate(rads)` → false if rad-proof.
    fn irradiate(&mut self, c: CreatureId, rads: f32, dice: &mut dyn Dice) -> bool;
    fn mod_pain(&mut self, c: CreatureId, n: i32);
    fn mod_moves(&mut self, c: CreatureId, n: i32);
    fn check_dead_state(&mut self, c: CreatureId);
    /// `teleport::teleport(c)`.
    fn teleport(&mut self, c: CreatureId, dice: &mut dyn Dice);
    /// `monster::make_fungus()` (mutates; true if it turned).
    fn make_fungus(&mut self, c: CreatureId) -> bool;
    /// `add_msg_player_or_npc` / `add_msg_if_player` (npc = None) / global `add_msg` (c = None).
    fn message(&mut self, c: Option<CreatureId>, ty: MsgType, player: &str, npc: Option<&str>);
}

// ---------------------------------------------------------------- clock

/// `calendar::turn` plus `action_time_scale`'s per-tick turn count.
#[derive(Copy, Clone, Debug)]
pub struct Clock {
    /// `to_turn<int>(calendar::turn)`, counted from `calendar::turn_zero`.
    pub turn: i32,
    /// `calendar_turns_this_tick()` (>= 1).
    pub turns_this_tick: i32,
}

impl Clock {
    pub fn duration_this_tick(&self) -> Turns {
        self.turns_this_tick
    }
    /// `action_time_scale::once_every_this_tick(freq)`.
    pub fn once_every_this_tick(&self, freq: Turns) -> bool {
        if freq <= 0 {
            return false;
        }
        let to = self.turn;
        let from = to - self.duration_this_tick();
        // calendar::ticks_between: C++ int division (truncates toward zero).
        (to / freq) - (from / freq) > 0
    }
}
