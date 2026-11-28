//! `process_fields_in_submap` and its helpers (`map_field.cpp` lines 906–1692).
//!
//! Rule order, branch order and the order of every dice call follow BN line by
//! line. BN holds `field_entry&` references (`cur`, `dstfld`, ...); here every
//! access goes back through the store by (submap, local, type), which gives the
//! same result because BN never erases an entry another reference points to.

use crate::content::{FieldTypeId, FieldTypes, Turns, HOURS, MINUTES};
use crate::field::{Field, FieldEntry};
use crate::ports::{BubbleCaches, Clock, FireData, Items, MapData, MapFlag, SubmapStore, TerId};
use crate::storage::{Local, SmPos, SubTile, OVERMAP_DEPTH, OVERMAP_HEIGHT, SEEX, SEEY};
use bn_dice::{random_entry, random_entry_removed, Dice};

/// `eight_dirs_sm`.
pub const EIGHT_DIRS: [(i32, i32); 8] = [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)];

/// Everything one call of `process_fields_in_submap` touches.
pub struct SubmapCtx<'a> {
    pub types: &'a FieldTypes,
    pub store: &'a mut dyn SubmapStore,
    pub map_data: &'a dyn MapData,
    pub items: &'a mut dyn Items,
    pub caches: &'a mut dyn BubbleCaches,
    pub clock: Clock,
    pub dice: &'a mut dyn Dice,
}

impl SubmapCtx<'_> {
    // ------------------------------------------------------------ storage access

    fn field(&self, t: SubTile) -> &Field {
        self.store.fields(t.sm).expect("submap loaded").get(t.local)
    }
    fn field_mut(&mut self, t: SubTile) -> &mut Field {
        self.store.fields_mut(t.sm).expect("submap loaded").get_mut(t.local)
    }
    /// The entry an iterator / reference points at (no `displayed` guard).
    fn ent(&self, t: SubTile, ty: FieldTypeId) -> &FieldEntry {
        self.field(t).entry(ty).expect("entry exists")
    }
    fn ent_mut(&mut self, t: SubTile, ty: FieldTypeId) -> &mut FieldEntry {
        self.field_mut(t).entry_mut(ty).expect("entry exists")
    }
    fn i(&self, t: SubTile, ty: FieldTypeId) -> i32 {
        self.ent(t, ty).intensity()
    }
    fn age(&self, t: SubTile, ty: FieldTypeId) -> Turns {
        self.ent(t, ty).age()
    }
    fn set_i(&mut self, t: SubTile, ty: FieldTypeId, n: i32) {
        let types = self.types;
        self.ent_mut(t, ty).set_intensity(types, n);
    }
    fn set_age(&mut self, t: SubTile, ty: FieldTypeId, a: Turns) {
        self.ent_mut(t, ty).set_age(a);
    }
    fn mod_age(&mut self, t: SubTile, ty: FieldTypeId, d: Turns) {
        self.ent_mut(t, ty).mod_age(d);
    }
    /// `field::find_field` (with the `displayed` guard) → does it exist?
    fn has(&self, t: SubTile, ty: FieldTypeId) -> bool {
        self.field(t).find(ty).is_some()
    }
    fn ter(&self, t: SubTile) -> TerId {
        self.store.ter(t.sm, t.local)
    }
    fn ter_furn_has_flag(&self, t: SubTile, f: MapFlag) -> bool {
        self.map_data.ter_has_flag(self.store.ter(t.sm, t.local), f)
            || self.map_data.furn_has_flag(self.store.furn(t.sm, t.local), f)
    }
    fn check_flammable_ter(&self, t: TerId) -> bool {
        let m = self.map_data;
        m.ter_has_flag(t, MapFlag::Flammable) || m.ter_has_flag(t, MapFlag::FlammableAsh) || m.ter_has_flag(t, MapFlag::FlammableHard)
    }
    fn check_flammable_furn(&self, f: crate::ports::FurnId) -> bool {
        let m = self.map_data;
        m.furn_has_flag(f, MapFlag::Flammable) || m.furn_has_flag(f, MapFlag::FlammableAsh) || m.furn_has_flag(f, MapFlag::FlammableHard)
    }
    fn any_flammable_item(&self, t: SubTile) -> bool {
        (0..self.items.count(t)).any(|i| self.items.flammable(t, i))
    }

    // ------------------------------------------------------------ helpers (anon namespace)

    /// `mark_field_cache_dirty`.
    fn mark_field_cache_dirty(&mut self, abs_sm: SmPos, ty: FieldTypeId) {
        if !ty.is_some() {
            return;
        }
        let data = self.types.get(ty);
        if !data.dirty_transparency_cache && data.is_transparent() {
            return;
        }
        if !self.caches.is_properly_requested(abs_sm) {
            return;
        }
        self.caches.set_transparency_cache_dirty(abs_sm);
        self.caches.set_seen_cache_dirty(abs_sm);
    }

    /// `neighbor_tile`: `local + delta` across submap borders; `None` if not loaded.
    pub fn neighbor_tile(&self, base: SmPos, local: Local, d: (i32, i32)) -> Option<SubTile> {
        let nx = local.x + d.0;
        let ny = local.y + d.1;
        let dsx = if nx < 0 { -1 } else if nx >= SEEX { 1 } else { 0 };
        let dsy = if ny < 0 { -1 } else if ny >= SEEY { 1 } else { 0 };
        if dsx == 0 && dsy == 0 {
            return Some(SubTile { sm: base, local: Local::new(nx, ny) });
        }
        let nbr = base.offset(dsx, dsy, 0);
        self.store.fields(nbr)?;
        // C++ `%` on (n + SEEX): n is in [-SEEX, 2*SEEX), so this stays non-negative.
        Some(SubTile { sm: nbr, local: Local::new((nx + SEEX) % SEEX, (ny + SEEY) % SEEY) })
    }

    /// The `add_field` + `++field_count` + `field_cache.push_back` + `is_uniform = false` idiom.
    fn add_raw(&mut self, dst: SubTile, ty: FieldTypeId, intensity: i32, age: Turns, touch_uniform: bool) -> bool {
        let types = self.types;
        let sm = self.store.fields_mut(dst.sm).expect("loaded");
        if sm.get_mut(dst.local).add(types, ty, intensity, age) {
            sm.field_count += 1;
            sm.field_cache.push(dst.local);
            if touch_uniform {
                self.store.mark_not_uniform(dst.sm);
            }
            return true;
        }
        false
    }

    /// `sub_add_field`: returns whether `find_field(type)` is non-null afterwards.
    fn sub_add_field(&mut self, dst: Option<SubTile>, ty: FieldTypeId, intensity: i32, age: Turns) -> bool {
        let Some(dst) = dst else { return false };
        self.add_raw(dst, ty, intensity, age, true);
        self.has(dst, ty)
    }

    /// `sub_passable`.
    fn sub_passable(&self, t: Option<SubTile>) -> bool {
        let Some(t) = t else { return false };
        let ter = self.store.ter(t.sm, t.local);
        let frn = self.store.furn(t.sm, t.local);
        if self.map_data.ter_movecost(ter) == 0 {
            return false;
        }
        if self.map_data.furn_movecost(frn) < 0 {
            return false;
        }
        true
    }

    /// `gas_can_spread_sub`.
    fn gas_can_spread_sub(&self, cur_ty: FieldTypeId, cur_i: i32, dst: Option<SubTile>) -> bool {
        let Some(dst) = dst else { return false };
        if let Some(f) = self.field(dst).find(cur_ty) {
            if f.intensity() >= cur_i {
                return false;
            }
        }
        let ter = self.store.ter(dst.sm, dst.local);
        let frn = self.store.furn(dst.sm, dst.local);
        if self.map_data.ter_movecost(ter) == 0 || self.map_data.furn_movecost(frn) < 0 {
            return self.ter_furn_has_flag(dst, MapFlag::Permeable);
        }
        true
    }

    /// `gas_spread_sub`.
    fn gas_spread_sub(&mut self, cur: SubTile, ty: FieldTypeId, dst: SubTile) {
        let age = self.age(cur, ty);
        let intens = self.i(cur, ty);
        let age_frac = age / intens;
        if self.has(dst, ty) {
            let fi = self.field(dst).find(ty).unwrap().intensity();
            let types = self.types;
            self.field_mut(dst).find_mut(ty).unwrap().set_intensity(types, fi + 1);
            self.set_i(cur, ty, intens - 1);
            self.field_mut(dst).find_mut(ty).unwrap().mod_age(age_frac);
            self.set_age(cur, ty, age - age_frac);
            self.mark_field_cache_dirty(dst.sm, ty);
        } else if self.add_raw(dst, ty, 1, 0, true) {
            if let Some(f) = self.field_mut(dst).find_mut(ty) {
                f.set_age(age_frac);
            }
            self.set_i(cur, ty, intens - 1);
            self.set_age(cur, ty, age - age_frac);
            self.mark_field_cache_dirty(dst.sm, ty);
        }
    }

    fn set_bubble_dirty(&mut self, in_bubble: bool, dirty: bool, pos: SmPos) {
        if in_bubble && dirty {
            self.caches.set_transparency_cache_dirty(pos);
            self.caches.set_seen_cache_dirty(pos);
        }
    }
}

/// `process_fields_in_submap(dim, sm, pos, mb)`. Returns true if any fire
/// field was alive when processed.
pub fn process_fields_in_submap(ctx: &mut SubmapCtx<'_>, pos: SmPos) -> bool {
    let Some(sm) = ctx.store.fields(pos) else { return false };
    if sm.field_count == 0 {
        return false;
    }
    let in_bubble = ctx.caches.is_properly_requested(pos);
    let types = ctx.types;
    let fd = &types.fd;

    let mut has_fire = false;
    // Snapshot before iterating (wandering fields push to this submap's cache).
    let field_positions: Vec<Local> = sm.field_cache.clone();
    for local in field_positions {
        let here = SubTile { sm: pos, local };
        let mut dirty_transparency_cache = false;

        if !ctx.field(here).displayed_field_type().is_some() {
            continue;
        }

        let mut next = ctx.field(here).next_type(None);
        while let Some(ty) = next {
            // `it` is at `ty`; the next key is looked up after this entry is done,
            // so inserts behind or ahead of it behave like std::map iteration.
            let cur_entry = ctx.ent(here, ty).clone();

            // Dead entries — clean up.
            if !cur_entry.is_alive() {
                ctx.store.fields_mut(pos).unwrap().field_count -= 1;
                let cur_fd_type = types.get(ty);
                if cur_fd_type.dirty_transparency_cache
                    || !cur_fd_type.is_transparent()
                    || !cur_fd_type.get_transparent(cur_entry.intensity() - 1)
                {
                    dirty_transparency_cache = true;
                }
                ctx.field_mut(here).remove(types, ty);
                ctx.set_bubble_dirty(in_bubble, dirty_transparency_cache, pos);
                next = ctx.field(here).next_type(Some(ty));
                continue;
            }

            dirty_transparency_cache |= types.get(ty).dirty_transparency_cache;

            // Track fire before the newborn suppression below.
            if types.get(ty).has_fire {
                has_fire = true;
            }

            // Newborn fields skip effects this tick.
            let is_newborn = cur_entry.age() == 0;
            let cur_fd_type_id = if is_newborn { fd.null } else { ty };
            let cur_fd_type = types.get(cur_fd_type_id);

            // Intensity upgrade.
            if !is_newborn {
                let chance = ctx.ent(here, ty).intensity_upgrade_chance(types);
                if chance > 0 && ctx.dice.one_in(chance) {
                    let dur = ctx.ent(here, ty).intensity_upgrade_duration(types);
                    if dur > 0 && ctx.clock.once_every_this_tick(dur) {
                        let n = ctx.i(here, ty) + 1;
                        ctx.set_i(here, ty, n);
                    }
                }
            }

            // `ter` / `frn` are bound once: later set_ter/set_furn do not change them.
            let ter = ctx.store.ter(pos, local);
            let frn = ctx.store.furn(pos, local);
            let md = ctx.map_data;
            let tf = |f: MapFlag| md.ter_has_flag(ter, f) || md.furn_has_flag(frn, f);

            // Dissipate faster in water.
            if md.ter_has_flag(ter, MapFlag::Swimmable) {
                let s = types.get(ty).underwater_age_speedup;
                ctx.mod_age(here, ty, s);
            }

            // ---- fd_fire ------------------------------------------------
            if cur_fd_type_id == fd.fire {
                let a = ctx.age(here, ty).max(-24 * HOURS);
                ctx.set_age(here, ty, a);

                let can_spread = !tf(MapFlag::FireContainer);
                let no_floor = md.ter_has_flag(ter, MapFlag::NoFloor);
                let can_burn =
                    !no_floor && can_spread && (ctx.check_flammable_ter(ter) || ctx.check_flammable_furn(frn));
                let is_sealed = tf(MapFlag::Sealed) && !tf(MapFlag::AllowFieldEffect);

                let tick_turns = ctx.clock.turns_this_tick;
                let tick_duration = ctx.clock.duration_this_tick();
                let mut time_added: Turns = 0;

                // --- Item burning ---
                if !is_sealed && ctx.items.count(here) > 0 {
                    let mut new_content = Vec::new();
                    // NOTE (BN): item detonation skipped.
                    let mut frd = FireData {
                        fire_intensity: ctx.i(here, ty),
                        contained: !can_spread,
                        ..FireData::default()
                    };
                    let max_c = ctx.i(here, ty) * 2;
                    let mut consumed = 0;
                    let mut idx = 0;
                    while idx < ctx.items.count(here) && consumed < max_c {
                        let old_weight = ctx.items.weight_g(here, idx);
                        let destroyed = ctx.items.burn(here, idx, &mut frd, ctx.dice);
                        let new_weight = if destroyed { 0 } else { ctx.items.weight_g(here, idx) };
                        if old_weight != new_weight {
                            new_content.extend(ctx.items.burnproducts(here, idx, old_weight - new_weight));
                        }
                        if destroyed {
                            new_content.extend(ctx.items.take_removable_contents(here, idx));
                            ctx.items.erase(here, idx);
                            consumed += 1;
                        } else {
                            idx += 1;
                        }
                    }
                    for prod in new_content {
                        ctx.items.push(here, prod);
                    }
                    // float * int → float, passed as double.
                    time_added = ctx.dice.roll_remainder((frd.fuel_produced * tick_turns as f32) as f64);
                }

                // --- Vehicle fire damage: TODO in BN too (not translated, nothing to translate) ---

                // --- Terrain fuel consumption ---
                if can_burn {
                    if md.ter_has_flag(ter, MapFlag::Swimmable) {
                        ctx.mod_age(here, ty, 4 * MINUTES * tick_turns);
                    }
                    let ci = ctx.i(here, ty);
                    if tf(MapFlag::Flammable) {
                        time_added += tick_duration * (5 - ci);
                        if ci > 1 && ctx.dice.one_in(200 - ci * 50) {
                            ctx.store.set_ter(pos, local, md.t_dirt());
                        }
                    } else if tf(MapFlag::FlammableHard) && ctx.dice.one_in(3) {
                        time_added += tick_duration * (4 - ci);
                        if ci > 1 && ctx.dice.one_in(200 - ci * 50) {
                            ctx.store.set_ter(pos, local, md.t_dirt());
                        }
                    } else if md.ter_has_flag(ter, MapFlag::FlammableAsh) {
                        time_added += tick_duration * (5 - ci);
                        if ci > 1 && ctx.dice.one_in(200 - ci * 50) {
                            ctx.store.set_ter(pos, local, md.t_dirt());
                        }
                    } else if md.furn_has_flag(frn, MapFlag::FlammableAsh) {
                        time_added += tick_duration * (5 - ci);
                        if ci > 1 && ctx.dice.one_in(200 - ci * 50) {
                            ctx.store.set_furn(pos, local, md.f_ash());
                        }
                    }
                }

                if time_added != 0 {
                    let a = ctx.age(here, ty);
                    ctx.set_age(here, ty, a - time_added);
                } else if can_burn {
                    let ci = ctx.i(here, ty);
                    ctx.mod_age(here, ty, 10 * ci * tick_turns);
                }

                // --- Z-rise: level-3 fire spreads upward ---
                if can_spread && pos.z < OVERMAP_HEIGHT && ctx.i(here, ty) == 3 {
                    let above_pos = pos.offset(0, 0, 1);
                    if ctx.store.fields(above_pos).is_some() {
                        let above = SubTile { sm: above_pos, local };
                        let above_ter = ctx.store.ter(above_pos, local);
                        if md.ter_has_flag(above_ter, MapFlag::NoFloor)
                            || md.ter_has_flag(above_ter, MapFlag::Flammable)
                            || md.ter_has_flag(above_ter, MapFlag::FlammableAsh)
                            || md.ter_has_flag(above_ter, MapFlag::FlammableHard)
                        {
                            if ctx.has(above, fd.fire) {
                                ctx.field_mut(above).find_mut(fd.fire).unwrap().mod_age(-2);
                            } else {
                                ctx.add_raw(above, fd.fire, 1, 0, true);
                            }
                        }
                    }
                }

                // --- Neighbor scan for flashpoint / intensity growth / spreading ---
                let t_pit = md.t_pit();
                let in_pit = can_spread && ter == t_pit;
                let mut adjacent_fires = 0;

                if can_spread && ctx.i(here, ty) > 1 && ctx.dice.one_in(3) {
                    // Flashpoint: fuel adjacent fires from our excess age.
                    let end_it = ctx.dice.rng(0, 7) as usize;
                    for c in 0..8usize {
                        if ctx.age(here, ty) >= 0 {
                            continue;
                        }
                        let i = (end_it + 1 + c) % 8;
                        let Some(dst) = ctx.neighbor_tile(pos, local, EIGHT_DIRS[i]) else { continue };
                        let dstfld = ctx.field(dst).find(fd.fire).cloned();
                        if let Some(df) = &dstfld {
                            if (df.intensity() <= ctx.i(here, ty) || df.age() > ctx.age(here, ty))
                                && (in_pit == (ctx.ter(dst) == t_pit))
                            {
                                if df.intensity() < 2 {
                                    ctx.field_mut(dst).find_mut(fd.fire).unwrap().set_intensity(types, df.intensity() + 1);
                                }
                                ctx.field_mut(dst).find_mut(fd.fire).unwrap().mod_age(-5 * MINUTES);
                                ctx.mod_age(here, ty, 5 * MINUTES);
                            }
                            adjacent_fires += 1;
                        }
                    }
                } else if ctx.age(here, ty) < 0 && ctx.i(here, ty) < 3 {
                    // Intensity growth from neighbours.
                    let mut maximum_intensity;
                    if ctx.age(here, ty) < -500 * MINUTES {
                        maximum_intensity = 3;
                    } else {
                        for d in EIGHT_DIRS {
                            if let Some(dst) = ctx.neighbor_tile(pos, local, d) {
                                if ctx.has(dst, fd.fire) {
                                    adjacent_fires += 1;
                                }
                            }
                        }
                        maximum_intensity = 1 + (adjacent_fires >= 3) as i32 + (adjacent_fires >= 7) as i32;
                        if maximum_intensity < 2 && ctx.age(here, ty) < -50 * MINUTES {
                            maximum_intensity = 2;
                        }
                    }
                    if ctx.i(here, ty) < maximum_intensity {
                        let n = ctx.i(here, ty) + 1;
                        ctx.set_i(here, ty, n);
                        let n = ctx.i(here, ty);
                        ctx.mod_age(here, ty, 10 * MINUTES * n);
                    }
                }
                let _ = adjacent_fires;

                // Fire spreading to adjacent tiles.
                if can_spread {
                    let end_i = ctx.dice.rng(0, 7) as usize;
                    for c in 0..8usize {
                        let ci = ctx.i(here, ty);
                        if ctx.dice.one_in(ci * 2) {
                            continue;
                        }
                        let Some(dst) = ctx.neighbor_tile(pos, local, EIGHT_DIRS[(end_i + 1 + c) % 8]) else {
                            continue;
                        };
                        if ctx.has(dst, fd.fire) {
                            continue;
                        }
                        let dter = ctx.store.ter(dst.sm, dst.local);
                        let dfur = ctx.store.furn(dst.sm, dst.local);
                        if in_pit != (dter == t_pit) {
                            continue;
                        }
                        let nearweb = ctx.has(dst, fd.web);
                        let mut spread_chance = 25 * (ci - 1);
                        if nearweb {
                            spread_chance = 50 + spread_chance / 2;
                        }
                        let dst_has_flammable_items = ctx.any_flammable_item(dst);
                        let power = ci + if ctx.dice.one_in(5) { 1 } else { 0 };
                        let dflag = |f: MapFlag| md.ter_has_flag(dter, f) || md.furn_has_flag(dfur, f);
                        let can_ignite = ctx.dice.rng(1, 100) < spread_chance
                            && (ctx.check_flammable_ter(dter) || ctx.check_flammable_furn(dfur) || nearweb)
                            && ((power >= 3 && ctx.age(here, ty) < 0 && ctx.dice.one_in(20))
                                || (power >= 2 && dflag(MapFlag::Flammable) && ctx.dice.one_in(2))
                                || (power >= 2 && dflag(MapFlag::FlammableAsh) && ctx.dice.one_in(2))
                                || (power >= 3 && dflag(MapFlag::FlammableHard) && ctx.dice.one_in(5))
                                || nearweb
                                || (dst_has_flammable_items && ctx.dice.one_in(5)));
                        if can_ignite {
                            if ctx.sub_add_field(Some(dst), fd.fire, 1, 0) {
                                ctx.field_mut(dst).find_mut(fd.fire).unwrap().set_age(2 * MINUTES);
                                ctx.mod_age(here, ty, MINUTES);
                            }
                            if nearweb {
                                // `nearwebfld->set_field_intensity(0)` (pointer taken before the add).
                                ctx.field_mut(dst).entry_mut(fd.web).unwrap().set_intensity(types, 0);
                            }
                        }
                    }
                }

                // --- Z-fall: fire on open-air tile falls to z-level below ---
                if no_floor && pos.z > -OVERMAP_DEPTH {
                    let below_pos = pos.offset(0, 0, -1);
                    if ctx.store.fields(below_pos).is_some() {
                        let below = SubTile { sm: below_pos, local };
                        if !ctx.has(below, fd.fire) {
                            ctx.add_raw(below, fd.fire, 1, 0, true);
                            let n = ctx.i(here, ty) - 1;
                            ctx.set_i(here, ty, n);
                        } else {
                            let bi = ctx.field(below).find(fd.fire).unwrap().intensity();
                            let ci = ctx.i(here, ty);
                            let mut new_i = ci.max(bi);
                            if new_i < 3 && ci == bi {
                                new_i += 1;
                            }
                            if bi < 3 || ctx.dice.one_in(10) {
                                ctx.set_i(here, ty, ci - 1);
                            }
                            ctx.field_mut(below).find_mut(fd.fire).unwrap().set_intensity(types, new_i);
                        }
                    }
                }
            } // end fd_fire

            // ---- Gas spreading (simplified — no wind) --------------------
            if !is_newborn && ctx.ent(here, ty).gas_can_spread(types) {
                let gas_pct = cur_fd_type.percent_spread;
                if gas_pct > 0
                    && ctx.i(here, ty) > 1
                    && !ctx.store.floor_cache(pos, local)
                    && ctx.dice.rng(1, 100) <= gas_pct
                {
                    // Try to fall first.
                    let mut spread_done = false;
                    if pos.z > -OVERMAP_DEPTH {
                        let below_pos = pos.offset(0, 0, -1);
                        if ctx.store.fields(below_pos).is_some() {
                            let dst = SubTile { sm: below_pos, local };
                            if ctx.gas_can_spread_sub(ty, ctx.i(here, ty), Some(dst)) {
                                ctx.gas_spread_sub(here, ty, dst);
                                spread_done = true;
                            }
                        }
                    }
                    if !spread_done {
                        let start = ctx.dice.rng(0, 7) as usize;
                        for c in 0..8usize {
                            if spread_done {
                                continue;
                            }
                            let dst = ctx.neighbor_tile(pos, local, EIGHT_DIRS[(start + c) % 8]);
                            if ctx.gas_can_spread_sub(ty, ctx.i(here, ty), dst) {
                                ctx.gas_spread_sub(here, ty, dst.unwrap());
                                spread_done = true;
                            }
                        }
                    }
                    // Outdoor age speedup (simplified — skip wind/shelter check).
                    let outdoor_speedup = cur_fd_type.outdoor_age_speedup;
                    if outdoor_speedup > 0 {
                        ctx.mod_age(here, ty, outdoor_speedup);
                    }
                }
            }

            // ---- fd_fungicidal_gas ----------------------------------------
            if !is_newborn && cur_fd_type_id == fd.fungicidal_gas {
                let intensity = ctx.i(here, ty);
                if md.ter_has_flag(ter, MapFlag::Fungus) && ctx.dice.one_in(10 / intensity) {
                    ctx.store.set_ter(pos, local, md.t_dirt());
                }
                if md.furn_has_flag(frn, MapFlag::Fungus) && ctx.dice.one_in(10 / intensity) {
                    ctx.store.set_furn(pos, local, md.f_null());
                }
            }

            // ---- fd_electricity ------------------------------------------
            if !is_newborn && cur_fd_type_id == fd.electricity && !ctx.dice.one_in(5) {
                let selfp = Some(here);
                if !ctx.sub_passable(selfp) && ctx.i(here, ty) > 1 {
                    let mut tries = 0;
                    while tries < 10 && ctx.age(here, ty) < 5 * MINUTES && ctx.i(here, ty) > 1 {
                        let dx = ctx.dice.rng(-1, 1);
                        let dy = ctx.dice.rng(-1, 1);
                        if dx == 0 && dy == 0 {
                            tries += 1;
                            continue;
                        }
                        let dst = ctx.neighbor_tile(pos, local, (dx, dy));
                        if ctx.sub_passable(dst) {
                            let a = ctx.age(here, ty) + 1;
                            ctx.sub_add_field(dst, fd.electricity, 1, a);
                            let n = ctx.i(here, ty) - 1;
                            ctx.set_i(here, ty, n);
                            tries = 0;
                        } else {
                            tries += 1;
                        }
                    }
                } else {
                    let mut grounded: Vec<(i32, i32)> = Vec::new();
                    for d in EIGHT_DIRS {
                        let dst = ctx.neighbor_tile(pos, local, d);
                        if dst.is_some() && !ctx.sub_passable(dst) {
                            grounded.push(d);
                        }
                    }
                    if grounded.is_empty() {
                        let dx = ctx.dice.rng(-1, 1);
                        let dy = ctx.dice.rng(-1, 1);
                        let dst = ctx.neighbor_tile(pos, local, (dx, dy));
                        // May be this very tile, so `elec` may be `cur`.
                        let elec_i = dst.and_then(|d| ctx.field(d).find(fd.electricity).map(|e| e.intensity()));
                        if ctx.sub_passable(dst) && elec_i.is_some_and(|i| i < 3) {
                            let d = dst.unwrap();
                            let ei = ctx.field(d).find(fd.electricity).unwrap().intensity();
                            ctx.field_mut(d).find_mut(fd.electricity).unwrap().set_intensity(types, ei + 1);
                            let n = ctx.i(here, ty) - 1;
                            ctx.set_i(here, ty, n);
                        } else if ctx.sub_passable(dst) {
                            let a = ctx.age(here, ty) + 1;
                            ctx.sub_add_field(dst, fd.electricity, 1, a);
                        }
                        let n = ctx.i(here, ty) - 1;
                        ctx.set_i(here, ty, n);
                    }
                    while !grounded.is_empty() && ctx.i(here, ty) > 1 {
                        let d = random_entry_removed(ctx.dice, &mut grounded);
                        let dst = ctx.neighbor_tile(pos, local, d);
                        let a = ctx.age(here, ty) + 1;
                        ctx.sub_add_field(dst, fd.electricity, 1, a);
                        let n = ctx.i(here, ty) - 1;
                        ctx.set_i(here, ty, n);
                    }
                }
            }

            // ---- fd_fire_vent -------------------------------------------
            if !is_newborn && cur_fd_type_id == fd.fire_vent {
                if ctx.i(here, ty) > 1 {
                    if ctx.dice.one_in(3) {
                        let n = ctx.i(here, ty) - 1;
                        ctx.set_i(here, ty, n);
                    }
                    // create_hot_air() skipped in BN — render/audio effect only.
                } else {
                    let a = ctx.age(here, ty);
                    ctx.sub_add_field(Some(here), fd.flame_burst, 1, a);
                    ctx.set_i(here, ty, 0);
                }
            }

            // ---- fd_flame_burst -----------------------------------------
            if !is_newborn && cur_fd_type_id == fd.flame_burst {
                if ctx.i(here, ty) > 1 {
                    let n = ctx.i(here, ty) - 1;
                    ctx.set_i(here, ty, n);
                } else {
                    ctx.set_i(here, ty, 0);
                }
            }

            // ---- fd_incendiary ------------------------------------------
            if !is_newborn && cur_fd_type_id == fd.incendiary {
                let dx = ctx.dice.rng(-1, 1);
                let dy = ctx.dice.rng(-1, 1);
                if let Some(dst) = ctx.neighbor_tile(pos, local, (dx, dy)) {
                    if ctx.ter_furn_has_flag(dst, MapFlag::Flammable)
                        || ctx.ter_furn_has_flag(dst, MapFlag::FlammableAsh)
                        || ctx.ter_furn_has_flag(dst, MapFlag::FlammableHard)
                    {
                        ctx.sub_add_field(Some(dst), fd.fire, 1, 0);
                    }
                    if ctx.any_flammable_item(dst) {
                        ctx.sub_add_field(Some(dst), fd.fire, 1, 0);
                    }
                }
                // create_hot_air() skipped in BN.
            }

            // ---- fd_gas_remover ------------------------------------------
            if !is_newborn && cur_fd_type_id == fd.gas_remover {
                let dx = ctx.dice.rng(-1, 1);
                let dy = ctx.dice.rng(-1, 1);
                if let Some(dst) = ctx.neighbor_tile(pos, local, (dx, dy)) {
                    // BN erases inside a range-for over the same map (UB on the
                    // erased node). Translated as the intended "remove each match";
                    // field_count is NOT decremented, as in BN. See STUBS.md.
                    for fty in ctx.field(dst).types() {
                        let t = types.get(fty);
                        if t.is_dangerous() && t.phase == crate::content::Phase::Gas {
                            ctx.field_mut(dst).remove(types, fty);
                        }
                    }
                }
            }

            // ---- fd_shock_vent ------------------------------------------
            if !is_newborn && cur_fd_type_id == fd.shock_vent {
                if ctx.i(here, ty) > 1 {
                    if ctx.dice.one_in(5) {
                        let n = ctx.i(here, ty) - 1;
                        ctx.set_i(here, ty, n);
                    }
                } else {
                    ctx.set_i(here, ty, 3);
                    let num_bolts = ctx.dice.rng(3, 6);
                    for _b in 0..num_bolts {
                        let mut xdir = 0;
                        let mut ydir = 0;
                        while xdir == 0 && ydir == 0 {
                            xdir = ctx.dice.rng(-1, 1);
                            ydir = ctx.dice.rng(-1, 1);
                        }
                        let dist = ctx.dice.rng(4, 12);
                        let mut cx = local.x;
                        let mut cy = local.y;
                        for _n in 0..dist {
                            cx += xdir;
                            cy += ydir;
                            let delta = (cx - local.x, cy - local.y);
                            let bolt_dst = ctx.neighbor_tile(pos, local, delta);
                            let bi = ctx.dice.rng(2, 3);
                            ctx.sub_add_field(bolt_dst, fd.electricity, bi, 0);
                            if ctx.dice.one_in(4) {
                                xdir = if xdir == 0 { ctx.dice.rng(0, 1) * 2 - 1 } else { 0 };
                            }
                            if ctx.dice.one_in(4) {
                                ydir = if ydir == 0 { ctx.dice.rng(0, 1) * 2 - 1 } else { 0 };
                            }
                        }
                    }
                }
            }

            // ---- fd_acid_vent ------------------------------------------
            if !is_newborn && cur_fd_type_id == fd.acid_vent {
                if ctx.i(here, ty) > 1 {
                    if ctx.age(here, ty) >= MINUTES {
                        let n = ctx.i(here, ty) - 1;
                        ctx.set_i(here, ty, n);
                        ctx.set_age(here, ty, 0);
                    }
                } else {
                    ctx.set_i(here, ty, 3);
                    for dx in -5..=5i32 {
                        for dy in -5..=5i32 {
                            if dx.abs() + dy.abs() > 5 {
                                continue;
                            }
                            let Some(dst) = ctx.neighbor_tile(pos, local, (dx, dy)) else { continue };
                            // Dead in BN: a stored intensity is never 0 (clamped to >= 1).
                            let acid0 = ctx.field(dst).find(fd.acid).is_some_and(|a| a.intensity() == 0);
                            if acid0 {
                                let mut new_intensity = 3 - (dx.abs() + dy.abs()) / 2 + if ctx.dice.one_in(3) { 1 } else { 0 };
                                new_intensity = new_intensity.clamp(0, 3);
                                if new_intensity > 0 {
                                    ctx.sub_add_field(Some(dst), fd.acid, new_intensity, 0);
                                }
                            }
                        }
                    }
                }
            }

            // ---- fd_push_items ------------------------------------------
            if !is_newborn && cur_fd_type_id == fd.push_items {
                let mut idx = 0;
                while idx < ctx.items.count(here) {
                    if !ctx.items.is_rock(here, idx) || ctx.items.age(here, idx) < 1 {
                        idx += 1;
                        continue;
                    }
                    ctx.items.set_age_zero(here, idx);
                    let mut valid_dirs: Vec<(i32, i32)> = Vec::new();
                    for d in EIGHT_DIRS {
                        if let Some(dst) = ctx.neighbor_tile(pos, local, d) {
                            if ctx.has(dst, fd.push_items) {
                                valid_dirs.push(d);
                            }
                        }
                    }
                    if !valid_dirs.is_empty() {
                        let target_d = random_entry(ctx.dice, &valid_dirs);
                        if let Some(dst) = ctx.neighbor_tile(pos, local, target_d) {
                            let detached = ctx.items.erase(here, idx);
                            ctx.items.push(dst, detached);
                            // Creature interactions skipped in BN (no creatures outside the bubble).
                            continue;
                        }
                    }
                    idx += 1;
                }
            }

            // ---- fd_bees ------------------------------------------------
            if !is_newborn && cur_fd_type_id == fd.bees {
                let bee_killers = [
                    fd.web, fd.fire, fd.smoke, fd.toxic_gas, fd.tear_gas, fd.relax_gas, fd.nuke_gas, fd.gas_vent,
                    fd.smoke_vent, fd.fungicidal_gas, fd.insecticidal_gas, fd.fire_vent, fd.flame_burst,
                    fd.electricity, fd.fatigue, fd.shock_vent, fd.plasma, fd.laser,
                ];
                let killed = bee_killers.iter().any(|k| ctx.has(here, *k));
                if killed {
                    ctx.set_i(here, ty, 0);
                } else {
                    // Wander randomly (player-chasing skipped in BN).
                    let start = ctx.dice.rng(0, 7) as usize;
                    for c in 0..8usize {
                        let dst = ctx.neighbor_tile(pos, local, EIGHT_DIRS[(start + c) % 8]);
                        if let Some(dst) = dst {
                            if !ctx.has(dst, fd.bees) {
                                // BN keeps looping after cur is spent: later neighbours
                                // get bees at the clamped intensity 1.
                                let (ci, ca) = (ctx.i(here, ty), ctx.age(here, ty));
                                ctx.add_raw(dst, fd.bees, ci, ca, true);
                                ctx.set_i(here, ty, 0);
                            }
                        }
                    }
                }
            }

            // ---- Radiation ----------------------------------------------
            if !is_newborn && ctx.ent(here, ty).extra_radiation_max(types) > 0 {
                let e = ctx.ent(here, ty);
                let (lo, hi) = (e.extra_radiation_min(types), e.extra_radiation_max(types));
                let extra = ctx.dice.rng(lo, hi);
                let r = ctx.store.radiation(pos, local);
                ctx.store.set_radiation(pos, local, r + extra);
            }

            // ---- Wandering fields (in-submap only) ----------------------
            if !is_newborn && cur_fd_type.wandering_field.is_some() {
                let wtype = cur_fd_type.wandering_field;
                let wintens = ctx.i(here, ty);
                let wradius = wintens - 1;
                for wx in -wradius..=wradius {
                    for wy in -wradius..=wradius {
                        let wl = Local::new(local.x + wx, local.y + wy);
                        if wl.x < 0 || wl.x >= SEEX || wl.y < 0 || wl.y >= SEEY {
                            continue;
                        }
                        let wt = SubTile { sm: pos, local: wl };
                        if let Some(wi) = ctx.field(wt).find(wtype).map(|f| f.intensity()) {
                            if wi < wintens {
                                ctx.field_mut(wt).find_mut(wtype).unwrap().set_intensity(types, wi + 1);
                            }
                        } else {
                            // BN does not clear is_uniform on this path.
                            ctx.add_raw(wt, wtype, wintens, 0, false);
                        }
                    }
                }
            }

            // ---- Aging + half-life decay --------------------------------
            let d = ctx.clock.duration_this_tick();
            let a = ctx.age(here, ty) + d;
            ctx.set_age(here, ty, a);
            let fdata = types.get(ty);
            if fdata.half_life > 0 && ctx.age(here, ty) > 0 {
                let age = ctx.age(here, ty);
                if ctx.dice.dice(2, age) > fdata.half_life {
                    ctx.set_age(here, ty, 0);
                    let n = ctx.i(here, ty) - 1;
                    ctx.set_i(here, ty, n);
                }
            }

            if !ctx.ent(here, ty).is_alive() {
                ctx.store.fields_mut(pos).unwrap().field_count -= 1;
                let removed = types.get(ty);
                dirty_transparency_cache =
                    dirty_transparency_cache || removed.dirty_transparency_cache || !removed.is_transparent();
                ctx.field_mut(here).remove(types, ty);
            }
            next = ctx.field(here).next_type(Some(ty));

            ctx.set_bubble_dirty(in_bubble, dirty_transparency_cache, pos);
        } // end field-entry loop
    }

    compact_field_cache(ctx.store, pos);
    has_fire
}

/// "Compact + deduplicate the field_cache in one O(n) pass" (shared with batch_turns).
pub fn compact_field_cache(store: &mut dyn SubmapStore, pos: SmPos) {
    let sm = store.fields_mut(pos).expect("loaded");
    let mut seen = [false; (SEEX * SEEY) as usize];
    let cache = std::mem::take(&mut sm.field_cache);
    let kept: Vec<Local> = cache
        .into_iter()
        .filter(|l| {
            if !sm.get(*l).displayed_field_type().is_some() {
                return false;
            }
            let idx = (l.x + l.y * SEEX) as usize;
            if seen[idx] {
                return false;
            }
            seen[idx] = true;
            true
        })
        .collect();
    sm.field_cache = kept;
}
