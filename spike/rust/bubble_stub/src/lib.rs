//! Reality-bubble stub: just enough per-submap storage and neighbour access
//! for Fields to run (issue #14, session 2).
//!
//! * [`Region`]: loaded submaps (`mapbuffer`), each with its [`SubmapFields`]
//!   plus the non-field members Fields reads (terrain, furniture, radiation,
//!   floor cache, `is_uniform`). Implements [`SubmapStore`].
//! * [`StubMapData`]: a hand-made terrain/furniture catalogue ([`MapData`]).
//! * [`StubItems`]: items per tile ([`Items`]); burning is recorded, not simulated.
//! * [`StubCaches`]: records cache invalidations ([`BubbleCaches`]).
//! * [`StubBubbleMap`]: impassable/permeable sets, vehicles, `rl_dist` ([`BubbleMap`]).
//!
//! None of this is the real Reality bubble; see `spike/rust/STUBS.md`.

use bn_dice::Dice;
use fields::content::Turns;
use fields::ports::*;
use fields::storage::{Local, SmPos, SubTile, SubmapFields, Tripoint, SEEX, SEEY};
use fields::FieldTypeId;
use std::collections::{BTreeMap, HashMap, HashSet};

// ---------------------------------------------------------------- Region

/// One loaded submap: the Fields part plus what Fields asks about.
#[derive(Clone, Debug)]
pub struct SubmapSlot {
    pub fields: SubmapFields,
    pub ter: [[TerId; 12]; 12],
    pub furn: [[FurnId; 12]; 12],
    pub rad: [[i32; 12]; 12],
    pub floor_cache: [[bool; 12]; 12],
    pub is_uniform: bool,
}

impl SubmapSlot {
    pub fn filled(ter: TerId) -> Self {
        Self {
            fields: SubmapFields::default(),
            ter: [[ter; 12]; 12],
            furn: [[FurnId(0); 12]; 12],
            rad: [[0; 12]; 12],
            floor_cache: [[false; 12]; 12],
            is_uniform: true,
        }
    }
}

/// The loaded submaps, keyed by absolute submap position (`mapbuffer`).
#[derive(Clone, Debug, Default)]
pub struct Region {
    pub submaps: BTreeMap<SmPos, SubmapSlot>,
}

impl Region {
    /// A `w × h` block of submaps at `origin`, filled with `ter`.
    pub fn block(origin: SmPos, w: i32, h: i32, ter: TerId) -> Self {
        let mut r = Region::default();
        for y in 0..h {
            for x in 0..w {
                r.submaps.insert(origin.offset(x, y, 0), SubmapSlot::filled(ter));
            }
        }
        r
    }

    /// Place a field the way the harness's layout loader does
    /// (`submap::get_field().add_field` + count + cache).
    pub fn place(&mut self, types: &fields::FieldTypes, p: Tripoint, ty: FieldTypeId, intensity: i32, age: Turns) {
        let (sm, l) = p.split();
        let slot = self.submaps.get_mut(&sm).expect("submap loaded");
        if slot.fields.get_mut(l).add(types, ty, intensity, age) {
            slot.fields.field_count += 1;
            slot.fields.field_cache.push(l);
        }
        slot.is_uniform = false;
    }

    /// Submap positions in the harness order: row-major (y, then x), per z.
    pub fn row_major(&self) -> Vec<SmPos> {
        let mut v: Vec<SmPos> = self.submaps.keys().copied().collect();
        v.sort_by_key(|p| (p.z, p.y, p.x));
        v
    }

    pub fn set_ter_at(&mut self, p: Tripoint, t: TerId) {
        let (sm, l) = p.split();
        self.submaps.get_mut(&sm).unwrap().ter[l.x as usize][l.y as usize] = t;
    }
    pub fn set_furn_at(&mut self, p: Tripoint, f: FurnId) {
        let (sm, l) = p.split();
        self.submaps.get_mut(&sm).unwrap().furn[l.x as usize][l.y as usize] = f;
    }

    /// Every field entry, as (tile, type, intensity, age, alive), in no particular order.
    pub fn entries(&self) -> Vec<(Tripoint, FieldTypeId, i32, Turns, bool)> {
        let mut out = Vec::new();
        for (sm, slot) in &self.submaps {
            for x in 0..SEEX {
                for y in 0..SEEY {
                    let l = Local::new(x, y);
                    for e in slot.fields.get(l).iter() {
                        out.push((Tripoint::from_parts(*sm, l), e.field_type(), e.intensity(), e.age(), e.is_alive()));
                    }
                }
            }
        }
        out
    }
}

impl SubmapStore for Region {
    fn fields(&self, sm: SmPos) -> Option<&SubmapFields> {
        self.submaps.get(&sm).map(|s| &s.fields)
    }
    fn fields_mut(&mut self, sm: SmPos) -> Option<&mut SubmapFields> {
        self.submaps.get_mut(&sm).map(|s| &mut s.fields)
    }
    fn ter(&self, sm: SmPos, l: Local) -> TerId {
        self.submaps[&sm].ter[l.x as usize][l.y as usize]
    }
    fn furn(&self, sm: SmPos, l: Local) -> FurnId {
        self.submaps[&sm].furn[l.x as usize][l.y as usize]
    }
    fn set_ter(&mut self, sm: SmPos, l: Local, t: TerId) {
        let s = self.submaps.get_mut(&sm).unwrap();
        s.is_uniform = false;
        s.ter[l.x as usize][l.y as usize] = t;
    }
    fn set_furn(&mut self, sm: SmPos, l: Local, f: FurnId) {
        let s = self.submaps.get_mut(&sm).unwrap();
        s.is_uniform = false;
        s.furn[l.x as usize][l.y as usize] = f;
    }
    fn radiation(&self, sm: SmPos, l: Local) -> i32 {
        self.submaps[&sm].rad[l.x as usize][l.y as usize]
    }
    fn set_radiation(&mut self, sm: SmPos, l: Local, r: i32) {
        let s = self.submaps.get_mut(&sm).unwrap();
        s.is_uniform = false;
        s.rad[l.x as usize][l.y as usize] = r;
    }
    fn floor_cache(&self, sm: SmPos, l: Local) -> bool {
        self.submaps[&sm].floor_cache[l.x as usize][l.y as usize]
    }
    fn mark_not_uniform(&mut self, sm: SmPos) {
        if let Some(s) = self.submaps.get_mut(&sm) {
            s.is_uniform = false;
        }
    }
}

// ---------------------------------------------------------------- terrain / furniture

#[derive(Clone, Debug)]
pub struct TerDef {
    pub id: &'static str,
    pub movecost: i32,
    pub flags: Vec<MapFlag>,
}

/// A hand-made catalogue with the handful of ids the rules name.
/// Real values belong to the terrain context and BN's JSON.
#[derive(Clone, Debug)]
pub struct StubMapData {
    pub ter: Vec<TerDef>,
    pub furn: Vec<TerDef>,
}

impl Default for StubMapData {
    fn default() -> Self {
        use MapFlag::*;
        let t = |id, movecost, flags: &[MapFlag]| TerDef { id, movecost, flags: flags.to_vec() };
        Self {
            ter: vec![
                t("t_null", 2, &[]),
                t("t_dirt", 2, &[]),
                t("t_pit", 10, &[]),
                t("t_grass", 2, &[FlammableAsh]),
                t("t_floor", 2, &[FlammableHard]),
                t("t_wall_wood", 0, &[Flammable]),
                t("t_rock", 0, &[]),
                t("t_water_sh", 5, &[Swimmable]),
                t("t_open_air", 2, &[NoFloor]),
                t("t_fungus", 2, &[Fungus, FlammableAsh]),
                t("t_door_frame_permeable", 0, &[Permeable]),
            ],
            furn: vec![
                t("f_null", 0, &[]),
                t("f_ash", 0, &[]),
                t("f_chair", 1, &[FlammableAsh]),
                t("f_fireplace", 1, &[FireContainer]),
                t("f_fungal_clump", 2, &[Fungus]),
            ],
        }
    }
}

impl StubMapData {
    pub fn ter_id(&self, s: &str) -> TerId {
        TerId(self.ter.iter().position(|d| d.id == s).unwrap_or_else(|| panic!("unknown ter {s}")) as u32)
    }
    pub fn furn_id(&self, s: &str) -> FurnId {
        FurnId(self.furn.iter().position(|d| d.id == s).unwrap_or_else(|| panic!("unknown furn {s}")) as u32)
    }
}

impl MapData for StubMapData {
    fn ter_has_flag(&self, t: TerId, f: MapFlag) -> bool {
        self.ter[t.0 as usize].flags.contains(&f)
    }
    fn furn_has_flag(&self, f: FurnId, flag: MapFlag) -> bool {
        self.furn[f.0 as usize].flags.contains(&flag)
    }
    fn ter_movecost(&self, t: TerId) -> i32 {
        self.ter[t.0 as usize].movecost
    }
    fn furn_movecost(&self, f: FurnId) -> i32 {
        self.furn[f.0 as usize].movecost
    }
    fn t_pit(&self) -> TerId {
        self.ter_id("t_pit")
    }
    fn t_dirt(&self) -> TerId {
        self.ter_id("t_dirt")
    }
    fn f_ash(&self) -> FurnId {
        self.furn_id("f_ash")
    }
    fn f_null(&self) -> FurnId {
        self.furn_id("f_null")
    }
}

// ---------------------------------------------------------------- items

#[derive(Clone, Debug)]
pub struct StubItem {
    pub id: u64,
    pub rock: bool,
    pub flammable: bool,
    pub weight_g: i64,
    pub birthday: Turns,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ItemCall {
    Burn { at: SubTile, index: usize, frd: FireData },
    Pushed { to: SubTile, id: u64 },
}

/// Items per tile. `burn` is recorded and never destroys or produces fuel.
#[derive(Debug, Default)]
pub struct StubItems {
    pub tiles: HashMap<SubTile, Vec<StubItem>>,
    pub now: Turns,
    pub calls: Vec<ItemCall>,
    pending: HashMap<u64, StubItem>,
}

impl StubItems {
    pub fn add(&mut self, p: Tripoint, item: StubItem) {
        let (sm, local) = p.split();
        self.tiles.entry(SubTile { sm, local }).or_default().push(item);
    }
    fn list(&self, at: SubTile) -> &[StubItem] {
        self.tiles.get(&at).map(Vec::as_slice).unwrap_or(&[])
    }
}

impl Items for StubItems {
    fn count(&self, at: SubTile) -> usize {
        self.list(at).len()
    }
    fn weight_g(&self, at: SubTile, i: usize) -> i64 {
        self.list(at)[i].weight_g
    }
    fn burn(&mut self, at: SubTile, i: usize, frd: &mut FireData, _dice: &mut dyn Dice) -> bool {
        self.calls.push(ItemCall::Burn { at, index: i, frd: *frd });
        false
    }
    fn burnproducts(&self, _at: SubTile, _i: usize, _burned_g: i64) -> Vec<DetachedItem> {
        Vec::new()
    }
    fn take_removable_contents(&mut self, _at: SubTile, _i: usize) -> Vec<DetachedItem> {
        Vec::new()
    }
    fn erase(&mut self, at: SubTile, i: usize) -> DetachedItem {
        let it = self.tiles.get_mut(&at).unwrap().remove(i);
        let id = it.id;
        self.pending.insert(id, it);
        DetachedItem(id)
    }
    fn push(&mut self, at: SubTile, item: DetachedItem) {
        if let Some(it) = self.pending.remove(&item.0) {
            self.calls.push(ItemCall::Pushed { to: at, id: it.id });
            self.tiles.entry(at).or_default().push(it);
        }
    }
    fn flammable(&self, at: SubTile, i: usize) -> bool {
        self.list(at)[i].flammable
    }
    fn is_rock(&self, at: SubTile, i: usize) -> bool {
        self.list(at)[i].rock
    }
    fn age(&self, at: SubTile, i: usize) -> Turns {
        self.now - self.list(at)[i].birthday
    }
    fn set_age_zero(&mut self, at: SubTile, i: usize) {
        let now = self.now;
        self.tiles.get_mut(&at).unwrap()[i].birthday = now;
    }
}

// ---------------------------------------------------------------- caches

#[derive(Clone, Debug, PartialEq)]
pub enum CacheCall {
    TransparencyDirty(SmPos),
    SeenDirty(SmPos),
    FieldAdded(Tripoint, FieldTypeId),
    FieldRemoved(Tripoint, FieldTypeId),
}

/// Records invalidations. `requested` holds submaps "properly requested" by
/// the bubble; the harness region is outside the bubble, so it starts empty.
#[derive(Debug, Default)]
pub struct StubCaches {
    pub requested: HashSet<SmPos>,
    pub calls: Vec<CacheCall>,
}

impl BubbleCaches for StubCaches {
    fn is_properly_requested(&self, sm: SmPos) -> bool {
        self.requested.contains(&sm)
    }
    fn set_transparency_cache_dirty(&mut self, sm: SmPos) {
        self.calls.push(CacheCall::TransparencyDirty(sm));
    }
    fn set_seen_cache_dirty(&mut self, sm: SmPos) {
        self.calls.push(CacheCall::SeenDirty(sm));
    }
    fn field_added(&mut self, p: Tripoint, ty: FieldTypeId) {
        self.calls.push(CacheCall::FieldAdded(p, ty));
    }
    fn field_removed(&mut self, p: Tripoint, ty: FieldTypeId) {
        self.calls.push(CacheCall::FieldRemoved(p, ty));
    }
}

// ---------------------------------------------------------------- bubble map

/// Geometry for the bubble-map rules, set up by hand.
#[derive(Debug, Default)]
pub struct StubBubbleMap {
    pub impassable: HashSet<Tripoint>,
    pub permeable: HashSet<Tripoint>,
    pub vehicles: HashMap<Tripoint, VehTile>,
    /// BN option `trigdist` (default false: square distance).
    pub trigdist: bool,
}

impl BubbleMap for StubBubbleMap {
    fn impassable(&self, p: Tripoint) -> bool {
        self.impassable.contains(&p)
    }
    fn permeable(&self, p: Tripoint) -> bool {
        self.permeable.contains(&p)
    }
    fn obstructed_by_vehicle_rotation(&self, _from: Tripoint, _to: Tripoint) -> bool {
        false
    }
    fn rl_dist(&self, a: Tripoint, b: Tripoint) -> i32 {
        let dx = (a.x - b.x).abs();
        let dy = (a.y - b.y).abs();
        let dz = (a.z - b.z).abs();
        if self.trigdist {
            // trig_dist: rounded Euclidean (BN: static_cast<int>(round(sqrt(...)))).
            (((dx * dx + dy * dy + dz * dz) as f64).sqrt()).round() as i32
        } else {
            dx.max(dy).max(dz)
        }
    }
    fn veh_at(&self, p: Tripoint) -> Option<VehTile> {
        self.vehicles.get(&p).copied()
    }
}

#[cfg(test)]
mod tests;
