//! Field types loaded from BN's JSON Content (`data/json/field_type.json`).
//!
//! Translation of `field_type.{h,cpp}`. Int ids are assigned in load order,
//! like `generic_factory`: this matters, because a tile's entries live in a
//! `std::map<field_type_id, field_entry>` ordered by int id, and that order is
//! the order rules run (and draw) in.

use serde_json::Value;
use std::collections::HashMap;

/// `time_duration`, in turns (1 turn = 1 second in BN).
pub type Turns = i32;
pub const SECONDS: Turns = 1;
pub const MINUTES: Turns = 60;
pub const HOURS: Turns = 3600;
pub const DAYS: Turns = 86400;

/// `field_type_id` (an `int_id`). `FieldTypeId(0)` is `fd_null`, which is falsy in BN.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct FieldTypeId(pub u16);

impl FieldTypeId {
    pub const NULL: FieldTypeId = FieldTypeId(0);
    /// `operator bool` of `int_id`: anything but id 0.
    pub fn is_some(self) -> bool {
        self.0 != 0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Phase {
    Null,
    Solid,
    Liquid,
    Gas,
    Plasma,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum StackingType {
    Intensity,
    Duration,
}

#[derive(Clone, Debug, Default)]
pub struct FieldEffect {
    pub id: String,
    pub min_duration: Turns,
    pub max_duration: Turns,
    pub intensity: i32,
    /// `bodypart_str_id::NULL_ID()` is `None`.
    pub bp: Option<String>,
    pub is_environmental: bool,
    pub immune_in_vehicle: bool,
    pub immune_inside_vehicle: bool,
    pub immune_outside_vehicle: bool,
    pub chance_in_vehicle: i32,
    pub chance_inside_vehicle: i32,
    pub chance_outside_vehicle: i32,
    pub message_type: String,
    pub message: String,
    pub message_npc: String,
}

/// `field_intensity_level`, gameplay members only (no colour, symbol, light colour).
#[derive(Clone, Debug)]
pub struct IntensityLevel {
    pub name: String,
    pub dangerous: bool,
    pub transparent: bool,
    pub move_cost: i32,
    pub extra_radiation_min: i32,
    pub extra_radiation_max: i32,
    pub radiation_hurt_damage_min: i32,
    pub radiation_hurt_damage_max: i32,
    pub radiation_hurt_message: String,
    pub intensity_upgrade_chance: i32,
    pub intensity_upgrade_duration: Turns,
    pub monster_spawn_chance: i32,
    pub monster_spawn_count: i32,
    pub monster_spawn_radius: i32,
    pub monster_spawn_group: String,
    pub light_emitted: f32,
    pub local_light_override: f32,
    pub translucency: f32,
    pub convection_temperature_mod: i32,
    pub scent_neutralization: i32,
    pub field_effects: Vec<FieldEffect>,
}

impl Default for IntensityLevel {
    fn default() -> Self {
        Self {
            name: String::new(),
            dangerous: false,
            transparent: true,
            move_cost: 0,
            extra_radiation_min: 0,
            extra_radiation_max: 0,
            radiation_hurt_damage_min: 0,
            radiation_hurt_damage_max: 0,
            radiation_hurt_message: String::new(),
            intensity_upgrade_chance: 0,
            intensity_upgrade_duration: 0,
            monster_spawn_chance: 0,
            monster_spawn_count: 0,
            monster_spawn_radius: 0,
            monster_spawn_group: String::new(),
            light_emitted: 0.0,
            local_light_override: -1.0,
            translucency: 0.0,
            convection_temperature_mod: 0,
            scent_neutralization: 0,
            field_effects: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FieldType {
    pub id: String,
    pub legacy_enum_id: i32,
    pub intensity_levels: Vec<IntensityLevel>,
    pub underwater_age_speedup: Turns,
    pub outdoor_age_speedup: Turns,
    pub decay_amount_factor: i32,
    pub percent_spread: i32,
    pub apply_slime_factor: i32,
    pub gas_absorption_factor: i32,
    pub is_splattering: bool,
    pub dirty_transparency_cache: bool,
    pub has_fire: bool,
    pub has_acid: bool,
    pub has_elec: bool,
    pub has_fume: bool,
    pub priority: i32,
    pub half_life: Turns,
    pub phase: Phase,
    pub accelerated_decay: bool,
    pub display_items: bool,
    pub display_field: bool,
    pub wandering_field: FieldTypeId,
    pub stacking_type: StackingType,
    pub immunity_traits: Vec<String>,
    pub immunity_body_part_env_resistance: Vec<(String, i32)>,
    pub immune_mtypes: Vec<String>,
}

impl FieldType {
    /// `get_intensity_level(level)`: out of range → last level (BN also debugmsgs).
    pub fn level(&self, level: i32) -> &IntensityLevel {
        if level < 0 || level as usize >= self.intensity_levels.len() {
            return self.intensity_levels.last().expect("no intensity levels");
        }
        &self.intensity_levels[level as usize]
    }
    pub fn is_dangerous(&self) -> bool {
        self.intensity_levels.iter().any(|l| l.dangerous)
    }
    pub fn is_transparent(&self) -> bool {
        self.intensity_levels.iter().all(|l| l.transparent)
    }
    pub fn get_transparent(&self, level: i32) -> bool {
        self.level(level).transparent
    }
    pub fn max_intensity(&self) -> i32 {
        self.intensity_levels.len() as i32
    }
}

/// The well-known `fd_*` globals of `field_type.cpp` (`set_field_type_ids`).
#[derive(Clone, Debug, Default)]
#[allow(non_snake_case)]
pub struct Fd {
    pub null: FieldTypeId,
    pub web: FieldTypeId,
    pub acid: FieldTypeId,
    pub sap: FieldTypeId,
    pub sludge: FieldTypeId,
    pub fire: FieldTypeId,
    pub smoke: FieldTypeId,
    pub toxic_gas: FieldTypeId,
    pub tear_gas: FieldTypeId,
    pub nuke_gas: FieldTypeId,
    pub gas_vent: FieldTypeId,
    pub fire_vent: FieldTypeId,
    pub flame_burst: FieldTypeId,
    pub electricity: FieldTypeId,
    pub fatigue: FieldTypeId,
    pub push_items: FieldTypeId,
    pub shock_vent: FieldTypeId,
    pub acid_vent: FieldTypeId,
    pub plasma: FieldTypeId,
    pub laser: FieldTypeId,
    pub dazzling: FieldTypeId,
    pub bees: FieldTypeId,
    pub incendiary: FieldTypeId,
    pub relax_gas: FieldTypeId,
    pub fungal_haze: FieldTypeId,
    pub hot_air1: FieldTypeId,
    pub hot_air2: FieldTypeId,
    pub hot_air3: FieldTypeId,
    pub hot_air4: FieldTypeId,
    pub fungicidal_gas: FieldTypeId,
    pub insecticidal_gas: FieldTypeId,
    pub smoke_vent: FieldTypeId,
    pub gas_remover: FieldTypeId,
}

/// All loaded field types (`generic_factory<field_type>`), indexed by int id.
#[derive(Clone, Debug)]
pub struct FieldTypes {
    pub types: Vec<FieldType>,
    by_str: HashMap<String, FieldTypeId>,
    pub fd: Fd,
}

#[derive(Debug)]
pub struct LoadError(pub String);

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for LoadError {}

fn err<T>(m: impl Into<String>) -> Result<T, LoadError> {
    Err(LoadError(m.into()))
}

/// `time_duration` JSON reader: an int is turns; a string is `"<n> <unit>"` pairs.
pub fn parse_duration(v: &Value) -> Result<Turns, LoadError> {
    if let Some(i) = v.as_i64() {
        return Ok(i as Turns);
    }
    let Some(s) = v.as_str() else { return err(format!("bad duration {v}")) };
    let toks: Vec<&str> = s.split_whitespace().collect();
    let mut total: i64 = 0;
    let mut i = 0;
    while i < toks.len() {
        let (num, unit) = match toks[i].find(|c: char| c.is_alphabetic()) {
            Some(p) if p > 0 => (&toks[i][..p], &toks[i][p..]),
            _ => {
                if i + 1 >= toks.len() {
                    return err(format!("bad duration '{s}'"));
                }
                i += 1;
                (toks[i - 1], toks[i])
            }
        };
        let n: i64 = num.parse().map_err(|_| LoadError(format!("bad duration '{s}'")))?;
        let mult = match unit {
            "turns" | "turn" | "t" | "seconds" | "second" | "s" => 1,
            "minutes" | "minute" | "m" => MINUTES as i64,
            "hours" | "hour" | "h" => HOURS as i64,
            "days" | "day" | "d" => DAYS as i64,
            _ => return err(format!("bad duration unit '{unit}' in '{s}'")),
        };
        total += n * mult;
        i += 1;
    }
    Ok(total as Turns)
}

fn translation(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Object(o) => o.get("str").and_then(Value::as_str).unwrap_or("").to_string(),
        _ => String::new(),
    }
}

fn get_bool(o: &Value, k: &str, d: bool) -> bool {
    o.get(k).and_then(Value::as_bool).unwrap_or(d)
}
fn get_i32(o: &Value, k: &str, d: i32) -> i32 {
    o.get(k).and_then(Value::as_f64).map(|f| f as i32).unwrap_or(d)
}
fn get_f32(o: &Value, k: &str, d: f32) -> f32 {
    o.get(k).and_then(Value::as_f64).map(|f| f as f32).unwrap_or(d)
}
fn get_dur(o: &Value, k: &str, d: Turns) -> Result<Turns, LoadError> {
    match o.get(k) {
        Some(v) => parse_duration(v),
        None => Ok(d),
    }
}

struct Pending {
    ty: FieldType,
    wandering_field_id: String,
}

fn load_one(jo: &Value) -> Result<Pending, LoadError> {
    let id = jo.get("id").and_then(Value::as_str).ok_or(LoadError("field_type without id".into()))?;
    let mut levels: Vec<IntensityLevel> = Vec::new();
    for jao in jo.get("intensity_levels").and_then(Value::as_array).into_iter().flatten() {
        // Every member falls back to the previous level (field_type::load).
        let fb = levels.last().cloned().unwrap_or_default();
        let mut l = IntensityLevel {
            name: jao.get("name").map(translation).unwrap_or(fb.name.clone()),
            transparent: get_bool(jao, "transparent", fb.transparent),
            dangerous: get_bool(jao, "dangerous", fb.dangerous),
            move_cost: get_i32(jao, "move_cost", fb.move_cost),
            extra_radiation_min: get_i32(jao, "extra_radiation_min", fb.extra_radiation_min),
            extra_radiation_max: get_i32(jao, "extra_radiation_max", fb.extra_radiation_max),
            radiation_hurt_damage_min: get_i32(jao, "radiation_hurt_damage_min", fb.radiation_hurt_damage_min),
            radiation_hurt_damage_max: get_i32(jao, "radiation_hurt_damage_max", fb.radiation_hurt_damage_max),
            radiation_hurt_message: jao
                .get("radiation_hurt_message")
                .map(translation)
                .unwrap_or(fb.radiation_hurt_message.clone()),
            intensity_upgrade_chance: get_i32(jao, "intensity_upgrade_chance", fb.intensity_upgrade_chance),
            intensity_upgrade_duration: get_dur(jao, "intensity_upgrade_duration", fb.intensity_upgrade_duration)?,
            monster_spawn_chance: get_i32(jao, "monster_spawn_chance", fb.monster_spawn_chance),
            monster_spawn_count: get_i32(jao, "monster_spawn_count", fb.monster_spawn_count),
            monster_spawn_radius: get_i32(jao, "monster_spawn_radius", fb.monster_spawn_radius),
            monster_spawn_group: jao
                .get("monster_spawn_group")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or(fb.monster_spawn_group.clone()),
            light_emitted: get_f32(jao, "light_emitted", fb.light_emitted),
            local_light_override: get_f32(jao, "light_override", fb.local_light_override),
            translucency: get_f32(jao, "translucency", fb.translucency),
            convection_temperature_mod: get_i32(jao, "convection_temperature_mod", fb.convection_temperature_mod),
            scent_neutralization: get_i32(jao, "scent_neutralization", fb.scent_neutralization),
            field_effects: Vec::new(),
        };
        if let Some(effs) = jao.get("effects").and_then(Value::as_array) {
            for joe in effs {
                l.field_effects.push(FieldEffect {
                    id: joe
                        .get("effect_id")
                        .and_then(Value::as_str)
                        .ok_or(LoadError(format!("{id}: effect without effect_id")))?
                        .to_string(),
                    min_duration: get_dur(joe, "min_duration", 0)?,
                    max_duration: get_dur(joe, "max_duration", 0)?,
                    intensity: get_i32(joe, "intensity", 0),
                    bp: joe.get("body_part").and_then(Value::as_str).map(str::to_string),
                    // BN: optional() without a default assigns bool() = false when absent;
                    // the struct initializer (true) never applies (session 3 replay finding).
                    is_environmental: get_bool(joe, "is_environmental", false),
                    immune_in_vehicle: get_bool(joe, "immune_in_vehicle", false),
                    immune_inside_vehicle: get_bool(joe, "immune_inside_vehicle", false),
                    immune_outside_vehicle: get_bool(joe, "immune_outside_vehicle", false),
                    chance_in_vehicle: get_i32(joe, "chance_in_vehicle", 0),
                    chance_inside_vehicle: get_i32(joe, "chance_inside_vehicle", 0),
                    chance_outside_vehicle: get_i32(joe, "chance_outside_vehicle", 0),
                    message_type: joe
                        .get("message_type")
                        .and_then(Value::as_str)
                        .unwrap_or("neutral")
                        .to_string(),
                    message: joe.get("message").map(translation).unwrap_or_default(),
                    message_npc: joe.get("message_npc").map(translation).unwrap_or_default(),
                });
            }
        } else {
            // "Use effects from previous intensity level"
            l.field_effects = fb.field_effects.clone();
        }
        levels.push(l);
    }
    if levels.is_empty() {
        return err(format!("{id}: No intensity levels defined for field type"));
    }
    let imm = jo.get("immunity_data");
    let phase = match jo.get("phase").and_then(Value::as_str) {
        Some("solid") => Phase::Solid,
        Some("liquid") => Phase::Liquid,
        Some("gas") => Phase::Gas,
        Some("plasma") => Phase::Plasma,
        _ => Phase::Null,
    };
    let stacking_type = match jo.get("stacking_type").and_then(Value::as_str) {
        None | Some("intensity") => StackingType::Intensity,
        Some("duration") => StackingType::Duration,
        Some(o) => return err(format!("{id}: bad stacking_type {o}")),
    };
    Ok(Pending {
        ty: FieldType {
            id: id.to_string(),
            legacy_enum_id: get_i32(jo, "legacy_enum_id", -1),
            intensity_levels: levels,
            underwater_age_speedup: get_dur(jo, "underwater_age_speedup", 0)?,
            outdoor_age_speedup: get_dur(jo, "outdoor_age_speedup", 0)?,
            decay_amount_factor: get_i32(jo, "decay_amount_factor", 0),
            percent_spread: get_i32(jo, "percent_spread", 0),
            apply_slime_factor: get_i32(jo, "apply_slime_factor", 0),
            gas_absorption_factor: get_i32(jo, "gas_absorption_factor", 0),
            is_splattering: get_bool(jo, "is_splattering", false),
            dirty_transparency_cache: get_bool(jo, "dirty_transparency_cache", false),
            has_fire: get_bool(jo, "has_fire", false),
            has_acid: get_bool(jo, "has_acid", false),
            has_elec: get_bool(jo, "has_elec", false),
            has_fume: get_bool(jo, "has_fume", false),
            priority: get_i32(jo, "priority", 0),
            half_life: get_dur(jo, "half_life", 0)?,
            phase,
            accelerated_decay: get_bool(jo, "accelerated_decay", false),
            display_items: get_bool(jo, "display_items", true),
            display_field: get_bool(jo, "display_field", false),
            wandering_field: FieldTypeId::NULL,
            stacking_type,
            immunity_traits: imm
                .and_then(|i| i.get("traits"))
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect(),
            immunity_body_part_env_resistance: imm
                .and_then(|i| i.get("body_part_env_resistance"))
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|p| Some((p.get(0)?.as_str()?.to_string(), p.get(1)?.as_i64()? as i32)))
                .collect(),
            immune_mtypes: jo
                .get("immune_mtypes")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect(),
        },
        wandering_field_id: jo.get("wandering_field").and_then(Value::as_str).unwrap_or("fd_null").to_string(),
    })
}

impl FieldTypes {
    /// Load every `"type": "field_type"` object from the given JSON documents, in order.
    /// Then `finalize_all`: resolve the well-known ids and `wandering_field`.
    pub fn load_json(docs: &[&str]) -> Result<Self, LoadError> {
        let mut pending: Vec<Pending> = Vec::new();
        let mut by_str: HashMap<String, FieldTypeId> = HashMap::new();
        for doc in docs {
            let v: Value = serde_json::from_str(doc).map_err(|e| LoadError(e.to_string()))?;
            let arr = match v {
                Value::Array(a) => a,
                o => vec![o],
            };
            for jo in arr {
                if jo.get("type").and_then(Value::as_str) != Some("field_type") {
                    continue;
                }
                let p = load_one(&jo)?;
                if by_str.contains_key(&p.ty.id) {
                    // generic_factory would override in place; not needed for core `bn`.
                    return err(format!("duplicate field type {}", p.ty.id));
                }
                by_str.insert(p.ty.id.clone(), FieldTypeId(pending.len() as u16));
                pending.push(p);
            }
        }
        if by_str.get("fd_null") != Some(&FieldTypeId(0)) {
            return err("fd_null must be the first field type (int id 0)");
        }
        let resolve = |s: &str| -> Result<FieldTypeId, LoadError> {
            by_str.get(s).copied().ok_or(LoadError(format!("unknown field type {s}")))
        };
        let mut types = Vec::with_capacity(pending.len());
        for p in pending {
            let mut ty = p.ty;
            ty.wandering_field = resolve(&p.wandering_field_id)?;
            types.push(ty);
        }
        let fd = Fd {
            null: resolve("fd_null")?,
            web: resolve("fd_web")?,
            acid: resolve("fd_acid")?,
            sap: resolve("fd_sap")?,
            sludge: resolve("fd_sludge")?,
            fire: resolve("fd_fire")?,
            smoke: resolve("fd_smoke")?,
            toxic_gas: resolve("fd_toxic_gas")?,
            tear_gas: resolve("fd_tear_gas")?,
            nuke_gas: resolve("fd_nuke_gas")?,
            gas_vent: resolve("fd_gas_vent")?,
            fire_vent: resolve("fd_fire_vent")?,
            flame_burst: resolve("fd_flame_burst")?,
            electricity: resolve("fd_electricity")?,
            fatigue: resolve("fd_fatigue")?,
            push_items: resolve("fd_push_items")?,
            shock_vent: resolve("fd_shock_vent")?,
            acid_vent: resolve("fd_acid_vent")?,
            plasma: resolve("fd_plasma")?,
            laser: resolve("fd_laser")?,
            dazzling: resolve("fd_dazzling")?,
            bees: resolve("fd_bees")?,
            incendiary: resolve("fd_incendiary")?,
            relax_gas: resolve("fd_relax_gas")?,
            fungal_haze: resolve("fd_fungal_haze")?,
            hot_air1: resolve("fd_hot_air1")?,
            hot_air2: resolve("fd_hot_air2")?,
            hot_air3: resolve("fd_hot_air3")?,
            hot_air4: resolve("fd_hot_air4")?,
            fungicidal_gas: resolve("fd_fungicidal_gas")?,
            insecticidal_gas: resolve("fd_insecticidal_gas")?,
            smoke_vent: resolve("fd_smoke_vent")?,
            gas_remover: resolve("fd_gas_remover")?,
        };
        Ok(Self { types, by_str, fd })
    }

    /// Load `data/json/field_type.json` from a BN checkout.
    pub fn load_bn(bn_root: &std::path::Path) -> Result<Self, LoadError> {
        let p = bn_root.join("data/json/field_type.json");
        let s = std::fs::read_to_string(&p).map_err(|e| LoadError(format!("{}: {e}", p.display())))?;
        Self::load_json(&[&s])
    }

    pub fn get(&self, id: FieldTypeId) -> &FieldType {
        &self.types[id.0 as usize]
    }
    pub fn id(&self, s: &str) -> Option<FieldTypeId> {
        self.by_str.get(s).copied()
    }
    pub fn str_id(&self, id: FieldTypeId) -> &str {
        &self.get(id).id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn durations() {
        assert_eq!(parse_duration(&json!("30 minutes")).unwrap(), 1800);
        assert_eq!(parse_duration(&json!("2 turns")).unwrap(), 2);
        assert_eq!(parse_duration(&json!("1 days")).unwrap(), 86400);
        assert_eq!(parse_duration(&json!("1h 30m")).unwrap(), 5400);
        assert_eq!(parse_duration(&json!(7)).unwrap(), 7);
    }
}
