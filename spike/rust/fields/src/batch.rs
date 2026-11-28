//! `batch_turns_field` (`src/batch_turns.cpp`).
//!
//! **Approximation** (ADR 0004 / issue #14): BN replaces the per-turn
//! `dice(2, age) > half_life` decay with a deterministic linear drop when a
//! submap comes back after N unsimulated turns. It is not Gameplay-equivalent
//! to running N turns, and it is not checked by replay. It runs only when the
//! caller opts in with [`BatchDecay::Approximate`]; the default skips it.

use crate::content::FieldTypes;
use crate::ports::SubmapStore;
use crate::process::compact_field_cache;
use crate::storage::SmPos;

/// Opt-in switch for the Approximation.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum BatchDecay {
    /// Do nothing (default): the caller must simulate turns or accept the gap.
    #[default]
    Off,
    /// BN's linear decay (`batch_turns_field`).
    Approximate,
}

/// `compute_field_decay`: returns (drops, remaining_age).
pub fn compute_field_decay(current_intensity: i32, half_life_turns: i32, current_age_turns: i32, elapsed_turns: i32) -> (i32, i32) {
    if half_life_turns <= 0 || current_intensity <= 0 {
        return (0, current_age_turns + elapsed_turns);
    }
    let total_age = current_age_turns + elapsed_turns;
    let drops = total_age / half_life_turns;
    let remaining = total_age % half_life_turns;
    (drops.min(current_intensity), remaining)
}

/// `batch_turns_field(sm, n)`, gated by `mode`. Draws nothing.
pub fn batch_turns_field(mode: BatchDecay, types: &FieldTypes, store: &mut dyn SubmapStore, pos: SmPos, n: i32) {
    if mode == BatchDecay::Off {
        return;
    }
    let Some(sm) = store.fields_mut(pos) else { return };
    if n <= 0 || sm.field_count == 0 {
        return;
    }
    // BN iterates the live field_cache (no snapshot); nothing is added here.
    let positions = sm.field_cache.clone();
    for local in positions {
        let sm = store.fields_mut(pos).unwrap();
        if !sm.get(local).displayed_field_type().is_some() {
            continue;
        }
        let mut next = sm.get(local).next_type(None);
        while let Some(ty) = next {
            let sm = store.fields_mut(pos).unwrap();
            let fld = sm.get_mut(local);
            let cur = fld.entry_mut(ty).unwrap();
            // Dead entries clean up on next normal tick; leave them alone.
            if cur.is_alive() {
                let fdata = types.get(ty);
                // Fire fields are never batch-decayed.
                if !fdata.has_fire {
                    if fdata.half_life > 0 {
                        let hl = fdata.half_life;
                        let current_age = cur.age();
                        let intensity = cur.intensity();
                        let (drops, remaining_age) = compute_field_decay(intensity, hl, current_age, n);
                        if drops > 0 {
                            cur.set_intensity(types, intensity - drops);
                            cur.set_age(remaining_age);
                            if !cur.is_alive() {
                                sm.field_count -= 1;
                                let fld = sm.get_mut(local);
                                fld.remove(types, ty);
                                next = fld.next_type(Some(ty));
                                continue;
                            }
                        } else {
                            // Just age the field without decaying.
                            cur.mod_age(n);
                        }
                    } else {
                        // No half-life: simply age the field.
                        cur.mod_age(n);
                    }
                }
            }
            next = store.fields(pos).unwrap().get(local).next_type(Some(ty));
        }
    }
    compact_field_cache(store, pos);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decay_math() {
        assert_eq!(compute_field_decay(3, 10, 5, 20), (2, 5));
        assert_eq!(compute_field_decay(1, 10, 5, 100), (1, 5));
        assert_eq!(compute_field_decay(3, 0, 5, 100), (0, 105));
    }
}
