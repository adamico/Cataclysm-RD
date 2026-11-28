//! `field_entry` and `field` (one tile's entries). Translation of `field.{h,cpp}`.

use crate::content::{FieldTypeId, FieldTypes, StackingType, Turns};
use std::collections::BTreeMap;
use std::ops::Bound;

/// An active or passive effect on a tile (`field_entry`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldEntry {
    ty: FieldTypeId,
    intensity: i32,
    age: Turns,
    is_alive: bool,
}

impl Default for FieldEntry {
    fn default() -> Self {
        Self { ty: FieldTypeId::NULL, intensity: 1, age: 0, is_alive: false }
    }
}

impl FieldEntry {
    /// The constructor does **not** clamp intensity and always sets `is_alive`.
    pub fn new(ty: FieldTypeId, intensity: i32, age: Turns) -> Self {
        Self { ty, intensity, age, is_alive: true }
    }
    pub fn field_type(&self) -> FieldTypeId {
        self.ty
    }
    pub fn intensity(&self) -> i32 {
        self.intensity
    }
    pub fn age(&self) -> Turns {
        self.age
    }
    pub fn is_alive(&self) -> bool {
        self.is_alive
    }
    /// `set_field_intensity`: alive iff `new > 0`; stored value clamped to `[1, max]`.
    pub fn set_intensity(&mut self, types: &FieldTypes, new_intensity: i32) -> i32 {
        self.is_alive = new_intensity > 0;
        self.intensity = new_intensity.min(types.get(self.ty).max_intensity()).max(1);
        self.intensity
    }
    pub fn set_age(&mut self, age: Turns) -> Turns {
        self.age = age;
        age
    }
    pub fn mod_age(&mut self, delta: Turns) -> Turns {
        self.set_age(self.age + delta)
    }

    // Per-intensity accessors (`type.obj().get_*(intensity - 1)`).
    pub fn extra_radiation_min(&self, t: &FieldTypes) -> i32 {
        t.get(self.ty).level(self.intensity - 1).extra_radiation_min
    }
    pub fn extra_radiation_max(&self, t: &FieldTypes) -> i32 {
        t.get(self.ty).level(self.intensity - 1).extra_radiation_max
    }
    pub fn radiation_hurt_damage_min(&self, t: &FieldTypes) -> i32 {
        t.get(self.ty).level(self.intensity - 1).radiation_hurt_damage_min
    }
    pub fn radiation_hurt_damage_max(&self, t: &FieldTypes) -> i32 {
        t.get(self.ty).level(self.intensity - 1).radiation_hurt_damage_max
    }
    pub fn intensity_upgrade_chance(&self, t: &FieldTypes) -> i32 {
        t.get(self.ty).level(self.intensity - 1).intensity_upgrade_chance
    }
    pub fn intensity_upgrade_duration(&self, t: &FieldTypes) -> Turns {
        t.get(self.ty).level(self.intensity - 1).intensity_upgrade_duration
    }
    pub fn name<'t>(&self, t: &'t FieldTypes) -> &'t str {
        &t.get(self.ty).level(self.intensity - 1).name
    }
    pub fn move_cost(&self, t: &FieldTypes) -> i32 {
        t.get(self.ty).level(self.intensity - 1).move_cost
    }
    /// `gas_can_spread`: alive, gas phase, `percent_spread > 0`.
    pub fn gas_can_spread(&self, t: &FieldTypes) -> bool {
        let ft = t.get(self.ty);
        self.is_alive && ft.phase == crate::content::Phase::Gas && ft.percent_spread > 0
    }
}

/// One tile's entries (`field`): at most one entry per type, ordered by int id
/// like BN's `std::map<field_type_id, field_entry>`.
#[derive(Clone, Debug, Default)]
pub struct Field {
    entries: BTreeMap<FieldTypeId, FieldEntry>,
    displayed: FieldTypeId,
}

impl Field {
    /// `find_field`: `nullptr` when nothing is displayed, even if entries exist.
    pub fn find(&self, ty: FieldTypeId) -> Option<&FieldEntry> {
        if !self.displayed.is_some() {
            return None;
        }
        self.entries.get(&ty)
    }
    pub fn find_mut(&mut self, ty: FieldTypeId) -> Option<&mut FieldEntry> {
        if !self.displayed.is_some() {
            return None;
        }
        self.entries.get_mut(&ty)
    }

    /// Direct access without the `displayed` guard (the iterator path in BN).
    pub fn entry_mut(&mut self, ty: FieldTypeId) -> Option<&mut FieldEntry> {
        self.entries.get_mut(&ty)
    }
    pub fn entry(&self, ty: FieldTypeId) -> Option<&FieldEntry> {
        self.entries.get(&ty)
    }

    /// `add_field`. Returns true only if a new entry was inserted. An existing
    /// entry stacks intensity (clamped) or, for `duration` stacking, adds
    /// `new_age - half_life` to its age when `new_age < half_life`.
    pub fn add(&mut self, types: &FieldTypes, ty: FieldTypeId, intensity: i32, age: Turns) -> bool {
        if !ty.is_some() {
            // BN: debugmsg("Tried to add null field")
            return false;
        }
        if let Some(e) = self.entries.get_mut(&ty) {
            let ft = types.get(ty);
            if ft.stacking_type == StackingType::Intensity {
                let n = e.intensity() + intensity;
                e.set_intensity(types, n);
            } else {
                let half_life = ft.half_life;
                if age < half_life {
                    e.mod_age(age - half_life);
                }
            }
            return false;
        }
        if !self.displayed.is_some() || types.get(ty).priority >= types.get(self.displayed).priority {
            self.displayed = ty;
        }
        self.entries.insert(ty, FieldEntry::new(ty, intensity, age));
        true
    }

    /// `remove_field(type)`; recomputes the displayed type (last max priority wins).
    pub fn remove(&mut self, types: &FieldTypes, ty: FieldTypeId) -> bool {
        if self.entries.remove(&ty).is_none() {
            return false;
        }
        self.displayed = FieldTypeId::NULL;
        for k in self.entries.keys() {
            if !self.displayed.is_some() || types.get(*k).priority >= types.get(self.displayed).priority {
                self.displayed = *k;
            }
        }
        true
    }

    pub fn count(&self) -> usize {
        self.entries.len()
    }
    pub fn displayed_field_type(&self) -> FieldTypeId {
        self.displayed
    }
    /// The next key after `after` (or the first), so a loop can walk the map the
    /// way a `std::map` iterator does while entries are inserted or erased.
    pub fn next_type(&self, after: Option<FieldTypeId>) -> Option<FieldTypeId> {
        match after {
            None => self.entries.keys().next().copied(),
            Some(k) => self.entries.range((Bound::Excluded(k), Bound::Unbounded)).next().map(|(k, _)| *k),
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = &FieldEntry> {
        self.entries.values()
    }
    pub fn types(&self) -> Vec<FieldTypeId> {
        self.entries.keys().copied().collect()
    }
    /// `total_move_cost`.
    pub fn total_move_cost(&self, types: &FieldTypes) -> i32 {
        self.entries.values().map(|e| e.move_cost(types)).sum()
    }
}
