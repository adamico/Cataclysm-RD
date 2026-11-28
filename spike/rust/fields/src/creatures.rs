//! Creatures standing in fields: `map::player_in_field`, `map::monster_in_field`,
//! `map::creature_in_field`, `map::burn_body_part` (`map_field.cpp` 115–798).
//!
//! Fields decides and draws; every harm goes out through [`Combat`].
//! Message texts are BN's untranslated strings.

use crate::content::{FieldTypeId, MINUTES, SECONDS};
use crate::map_rules::MapCtx;
use crate::ports::{Combat, CreatureId, DamageType, MatGroup, MsgType};
use crate::storage::{SubTile, Tripoint};

const ITYPE_RM13_ARMOR_ON: &str = "rm13_armor_on";
const BIO_HEATSINK: &str = "bio_heatsink";

impl MapCtx<'_> {
    fn ctile(&self, p: Tripoint) -> Option<SubTile> {
        let (sm, local) = p.split();
        self.store.fields(sm)?;
        Some(SubTile { sm, local })
    }
    fn next_entry(&self, t: SubTile, after: Option<FieldTypeId>) -> Option<FieldTypeId> {
        self.store.fields(t.sm).unwrap().get(t.local).next_type(after)
    }
    fn e_alive(&self, t: SubTile, ty: FieldTypeId) -> bool {
        self.store.fields(t.sm).unwrap().get(t.local).entry(ty).unwrap().is_alive()
    }
    fn e_i(&self, t: SubTile, ty: FieldTypeId) -> i32 {
        self.store.fields(t.sm).unwrap().get(t.local).entry(ty).unwrap().intensity()
    }
    fn e_set_i(&mut self, t: SubTile, ty: FieldTypeId, n: i32) {
        let types = self.types;
        self.store.fields_mut(t.sm).unwrap().get_mut(t.local).entry_mut(ty).unwrap().set_intensity(types, n);
    }
    fn e_entry(&self, t: SubTile, ty: FieldTypeId) -> crate::field::FieldEntry {
        self.store.fields(t.sm).unwrap().get(t.local).entry(ty).unwrap().clone()
    }
}

/// `map::burn_body_part(u, cur, bp_token, scale)`.
fn burn_body_part(ctx: &mut MapCtx<'_>, u: CreatureId, intensity: i32, bp: &str, scale: i32) -> i32 {
    let mut total_damage = 0;
    let damage = ctx.dice.rng(1, (scale + intensity) / 2);
    // A bit ugly, but better than being annoyed by acid when in hazmat
    if ctx.combat.get_armor_type(u, DamageType::Acid, bp) < damage {
        total_damage += ctx.combat.deal_damage(u, bp, DamageType::Acid, damage, ctx.dice);
    }
    // Represents acid seeping in rather than being splashed on
    let dur = ctx.dice.rng(2, 1 + intensity);
    ctx.combat.add_env_effect(u, "corroding", bp, 2 + intensity, dur, Some(bp), 0, ctx.dice);
    total_damage
}

/// The 4-argument `add_env_effect(eff, vector, strength, dur)` overload.
fn env4(ctx: &mut MapCtx<'_>, c: CreatureId, eff: &str, vector: &str, strength: i32, dur: i32) -> bool {
    ctx.combat.add_env_effect(c, eff, vector, strength, dur, None, 1, ctx.dice)
}

/// `map::player_in_field(u)`.
pub fn player_in_field(ctx: &mut MapCtx<'_>, u: CreatureId) {
    let fd = ctx.types.fd.clone();
    let types = ctx.types;
    let upos = ctx.combat.pos(u);
    let Some(here) = ctx.ctile(upos) else { return };
    let mut inside = false;
    if ctx.combat.in_vehicle(u) {
        if let Some(vp) = ctx.map.veh_at(upos) {
            inside = vp.is_inside;
        }
    }
    let in_vehicle = ctx.combat.in_vehicle(u);

    let mut next = ctx.next_entry(here, None);
    while let Some(ft) = next {
        'entry: {
            if !ctx.e_alive(here, ft) {
                break 'entry;
            }
            if ft == fd.web {
                if !ctx.combat.has_trait(u, "WEB_WALKER") && !in_vehicle {
                    let i = ctx.e_i(here, ft);
                    ctx.combat.add_effect(u, "webbed", 1, None, i);
                    ctx.e_set_i(here, ft, 0);
                    break 'entry;
                } else if in_vehicle {
                    ctx.e_set_i(here, ft, 0);
                    break 'entry;
                }
            }
            if ft == fd.acid {
                // Assume vehicles block acid damage entirely.
                if !in_vehicle && !ctx.combat.has_trait(u, "ACIDPROOF") {
                    let i = ctx.e_i(here, ft);
                    let mut total_damage = 0;
                    total_damage += burn_body_part(ctx, u, i, "foot_l", 2);
                    total_damage += burn_body_part(ctx, u, i, "foot_r", 2);
                    let on_ground = ctx.combat.is_on_ground(u);
                    if on_ground {
                        for bp in ["leg_l", "leg_r", "hand_l", "hand_r", "torso"] {
                            total_damage += burn_body_part(ctx, u, i, bp, 2);
                        }
                        // Less arms = less ability to keep upright
                        if (!ctx.combat.has_two_arms(u) && ctx.dice.one_in(4)) || ctx.dice.one_in(2) {
                            for bp in ["arm_l", "arm_r", "head"] {
                                total_damage += burn_body_part(ctx, u, i, bp, 1);
                            }
                        }
                    }
                    if on_ground && total_damage > 0 {
                        ctx.combat.message(Some(u), MsgType::Bad, "The acid burns your body!", Some("The acid burns <npcname>s body!"));
                    } else if total_damage > 0 {
                        ctx.combat.message(
                            Some(u),
                            MsgType::Bad,
                            "The acid burns your legs and feet!",
                            Some("The acid burns <npcname>s legs and feet!"),
                        );
                    } else if on_ground {
                        ctx.combat.message(Some(u), MsgType::Warning, "You're lying in a pool of acid", None);
                    } else {
                        ctx.combat.message(Some(u), MsgType::Warning, "You're standing in a pool of acid", None);
                    }
                    ctx.combat.check_dead_state(u);
                }
            }
            if ft == fd.sap {
                // Sap does nothing to cars.
                if !in_vehicle {
                    let i = ctx.e_i(here, ft);
                    ctx.e_set_i(here, ft, i - 1);
                }
            }
            if ft == fd.sludge {
                if !in_vehicle {
                    ctx.combat.message(Some(u), MsgType::Bad, "The sludge is thick and sticky.  You struggle to pull free.", None);
                    let i = ctx.e_i(here, ft);
                    ctx.combat.mod_moves(u, -(i * 300));
                    ctx.e_set_i(here, ft, 0);
                }
            }
            if ft == fd.fire {
                // Heatsink or suit prevents ALL fire damage.
                if !ctx.combat.has_active_bionic(u, BIO_HEATSINK) && !ctx.combat.is_wearing(u, ITYPE_RM13_ARMOR_ON) {
                    let mut adjusted_intensity = ctx.e_i(here, ft);
                    if in_vehicle {
                        if inside {
                            adjusted_intensity -= 2;
                        } else {
                            adjusted_intensity -= 1;
                        }
                    }
                    if adjusted_intensity >= 1 {
                        const PLAYER_BURN_MSG: [&str; 4] = [
                            "You burn your legs and feet!",
                            "You're burning up!",
                            "You're set ablaze!",
                            "Your whole body is burning!",
                        ];
                        const NPC_BURN_MSG: [&str; 4] = [
                            "<npcname> burns their legs and feet!",
                            "<npcname> is burning up!",
                            "<npcname> is set ablaze!",
                            "<npcname>s whole body is burning!",
                        ];
                        const PLAYER_WARN_MSG: [&str; 4] = [
                            "You're standing in a fire!",
                            "You're waist-deep in a fire!",
                            "You're surrounded by raging fire!",
                            "You're lying in fire!",
                        ];
                        let burn_min = adjusted_intensity;
                        let burn_max = 3 * adjusted_intensity + 3;
                        let mut parts_burned: Vec<String> = Vec::new();
                        let mut msg_num = (adjusted_intensity - 1) as usize;
                        if !ctx.combat.is_on_ground(u) {
                            // switch with fallthrough 3 → 2 → 1
                            if adjusted_intensity == 3 {
                                parts_burned.extend(["hand_l", "hand_r", "arm_l", "arm_r"].map(String::from));
                            }
                            if (2..=3).contains(&adjusted_intensity) {
                                parts_burned.push("torso".into());
                            }
                            if (1..=3).contains(&adjusted_intensity) {
                                parts_burned.extend(["foot_l", "foot_r", "leg_l", "leg_r"].map(String::from));
                            }
                        } else {
                            // Lying in the fire is BAAAD news, hits every body part.
                            msg_num = 3;
                            parts_burned = ctx.combat.get_all_body_parts(u);
                        }
                        let mut total_damage = 0;
                        for part in &parts_burned {
                            let amount = ctx.dice.rng(burn_min, burn_max);
                            total_damage += ctx.combat.deal_damage(u, part, DamageType::Heat, amount, ctx.dice);
                        }
                        if total_damage > 0 {
                            ctx.combat.message(Some(u), MsgType::Bad, PLAYER_BURN_MSG[msg_num], Some(NPC_BURN_MSG[msg_num]));
                        } else {
                            ctx.combat.message(Some(u), MsgType::Warning, PLAYER_WARN_MSG[msg_num], None);
                        }
                        ctx.combat.check_dead_state(u);
                    }
                }
            }
            if ft == fd.tear_gas {
                // Tear gas will both give you teargas disease and/or blind you.
                let i = ctx.e_i(here, ft);
                if (i > 1 || !ctx.dice.one_in(3)) && (!inside || ctx.dice.one_in(3)) {
                    env4(ctx, u, "teargas", "eyes", 5, 20 * SECONDS);
                }
                let i = ctx.e_i(here, ft);
                if i > 1 && (!inside || ctx.dice.one_in(3)) {
                    env4(ctx, u, "blind", "eyes", i * 2, 10 * SECONDS);
                }
            }
            if ft == fd.fungal_haze {
                if !ctx.combat.has_trait(u, "M_IMMUNE") && (!inside || ctx.dice.one_in(4)) {
                    ctx.combat.add_env_effect(u, "fungus", "mouth", 4, 10 * MINUTES, None, 1, ctx.dice);
                    ctx.combat.add_env_effect(u, "fungus", "eyes", 4, 10 * MINUTES, None, 1, ctx.dice);
                }
            }
            if ft == fd.dazzling {
                if ctx.e_i(here, ft) > 1 || ctx.dice.one_in(5) {
                    env4(ctx, u, "blind", "eyes", 10, 10);
                } else {
                    env4(ctx, u, "blind", "eyes", 2, 2);
                }
            }

            let cur = ctx.e_entry(here, ft);
            if cur.extra_radiation_min(types) > 0 {
                // Get irradiated by the nuclear fallout.
                let (rmin, rmax) = (cur.extra_radiation_min(types), cur.extra_radiation_max(types));
                let rads = ctx.dice.rng(rmin + 1, rmax * (rmax + 1)) as f32;
                let rad_proof = !ctx.combat.irradiate(u, rads, ctx.dice);
                if cur.radiation_hurt_damage_min(types) > 0 && !rad_proof {
                    let msg = types.get(ft).level(cur.intensity() - 1).radiation_hurt_message.clone();
                    ctx.combat.message(Some(u), MsgType::Bad, &msg, None);
                    let d = ctx.dice.rng(cur.radiation_hurt_damage_min(types), cur.radiation_hurt_damage_max(types));
                    ctx.combat.hurtall(u, d);
                }
            }
            if ft == fd.flame_burst {
                // A burst of flame? Only hits the legs and torso.
                if !inside {
                    if !ctx.combat.has_active_bionic(u, BIO_HEATSINK) && !ctx.combat.is_wearing(u, ITYPE_RM13_ARMOR_ON) {
                        ctx.combat.message(Some(u), MsgType::Bad, "You're torched by flames!", Some("<npcname> is torched by flames!"));
                        let a = ctx.dice.rng(2, 6);
                        ctx.combat.deal_damage(u, "leg_l", DamageType::Heat, a, ctx.dice);
                        let b = ctx.dice.rng(2, 6);
                        ctx.combat.deal_damage(u, "leg_r", DamageType::Heat, b, ctx.dice);
                        let c = ctx.dice.rng(4, 9);
                        ctx.combat.deal_damage(u, "torso", DamageType::Heat, c, ctx.dice);
                        ctx.combat.check_dead_state(u);
                    } else {
                        ctx.combat.message(
                            Some(u),
                            MsgType::Neutral,
                            "These flames do not burn you.",
                            Some("Those flames do not burn <npcname>."),
                        );
                    }
                }
            }
            if ft == fd.electricity {
                // Small universal damage based on intensity, only if not electroproofed.
                if !ctx.combat.is_elec_immune(u) {
                    let mut total_damage = 0;
                    for bp in ctx.combat.get_all_body_parts(u) {
                        let i = ctx.e_i(here, ft);
                        let dmg = ctx.dice.rng(1, i);
                        total_damage += ctx.combat.deal_damage(u, &bp, DamageType::Electric, dmg, ctx.dice);
                    }
                    if total_damage > 0 {
                        if ctx.combat.has_trait(u, "ELECTRORECEPTORS") {
                            ctx.combat.message(Some(u), MsgType::Bad, "You're painfully electrocuted!", Some("<npcname> is shocked!"));
                            ctx.combat.mod_pain(u, total_damage / 2);
                        } else {
                            ctx.combat.message(Some(u), MsgType::Bad, "You're shocked!", Some("<npcname> is shocked!"));
                        }
                    } else {
                        ctx.combat.message(
                            Some(u),
                            MsgType::Neutral,
                            "The electric cloud doesn't affect you.",
                            Some("The electric cloud doesn't seem to affect <npcname>."),
                        );
                    }
                }
            }
            if ft == fd.fatigue {
                // Assume the rift is on the ground for now.
                if !in_vehicle {
                    let i = ctx.e_i(here, ft);
                    if ctx.dice.rng(0, 2) < i && ctx.combat.is_avatar(u) {
                        ctx.combat.message(None, MsgType::Bad, "You're violently teleported!", None);
                        ctx.combat.hurtall(u, i);
                        ctx.combat.teleport(u, ctx.dice);
                    }
                }
            }
            // Stepping on a shock vent shuts it down.
            if ft == fd.shock_vent || ft == fd.acid_vent {
                ctx.e_set_i(here, ft, 0);
            }
            if ft == fd.bees {
                // Player is immune to bees while underwater.
                if !ctx.combat.is_underwater(u) {
                    let intensity = ctx.e_i(here, ft);
                    // Bees will try to sting you in random body parts, up to 8 times.
                    // NB: BN re-rolls `rng(1, 7)` on every loop test.
                    let mut i = 0;
                    while i < ctx.dice.rng(1, 7) {
                        let bp = ctx.combat.get_random_body_part(u, ctx.dice);
                        let sum_cover = ctx.combat.worn_coverage(u, &bp);
                        if (ctx.combat.get_armor_cut(u, &bp) <= 1
                            || (sum_cover < 100 && ctx.dice.x_in_y((100 - sum_cover) as f64, 100.0)))
                            && ctx.combat.add_env_effect(u, "stung", &bp, intensity, 9 * MINUTES, None, 1, ctx.dice)
                        {
                            let m = format!("The bees sting you in {bp}!");
                            ctx.combat.message(Some(u), MsgType::Bad, &m, None);
                        }
                        i += 1;
                    }
                }
            }
            if ft == fd.incendiary {
                // Mysterious incendiary substance melts you horribly.
                if ctx.combat.has_trait(u, "M_SKIN2") || ctx.combat.has_trait(u, "M_SKIN3") || ctx.e_i(here, ft) == 1 {
                    ctx.combat.message(Some(u), MsgType::Bad, "The incendiary burns you!", Some("The incendiary burns <npcname>!"));
                    let d = ctx.dice.rng(1, 3);
                    ctx.combat.hurtall(u, d);
                } else {
                    ctx.combat.message(
                        Some(u),
                        MsgType::Bad,
                        "The incendiary melts into your skin!",
                        Some("The incendiary melts into <npcname>s skin!"),
                    );
                    ctx.combat.add_effect(u, "onfire", 8, Some("torso"), 0);
                    let d = ctx.dice.rng(2, 6);
                    ctx.combat.hurtall(u, d);
                }
            }
            // Both gases are unhealthy and become deadly if you cross a related threshold.
            if ft == fd.fungicidal_gas || ft == fd.insecticidal_gas {
                // The gas won't harm you inside a vehicle.
                if !inside {
                    // Full body suits protect you from the effects of the gas.
                    if !(ctx.combat.worn_with_flag(u, "GAS_PROOF")
                        && ctx.combat.get_env_resist(u, "mouth") >= 15
                        && ctx.combat.get_env_resist(u, "eyes") >= 15)
                    {
                        let intensity = ctx.e_i(here, ft);
                        let mut inhaled = env4(ctx, u, "poison", "mouth", 5, intensity * 10 * SECONDS);
                        if ctx.combat.has_trait(u, "THRESH_MYCUS")
                            || ctx.combat.has_trait(u, "THRESH_MARLOSS")
                            || (ft == fd.insecticidal_gas && {
                                let c = ctx.combat.get_highest_category(u);
                                c == "INSECT" || c == "SPIDER"
                            })
                        {
                            inhaled |= env4(ctx, u, "badpoison", "mouth", 5, intensity * 10 * SECONDS);
                            let d = ctx.dice.rng(intensity, intensity * 2);
                            ctx.combat.hurtall(u, d);
                            let m = format!("The {} burns your skin.", ctx.e_entry(here, ft).name(types));
                            ctx.combat.message(Some(u), MsgType::Bad, &m, None);
                        }
                        if inhaled {
                            let m = format!("The {} makes you feel sick.", ctx.e_entry(here, ft).name(types));
                            ctx.combat.message(Some(u), MsgType::Bad, &m, None);
                        }
                    }
                }
            }
        }
        next = ctx.next_entry(here, Some(ft));
    }
}

/// `map::creature_in_field(critter)`.
pub fn creature_in_field(ctx: &mut MapCtx<'_>, critter: CreatureId) {
    let types = ctx.types;
    let mut in_vehicle = false;
    let mut inside_vehicle = false;
    if ctx.combat.is_monster(critter) {
        monster_in_field(ctx, critter);
    } else if ctx.combat.is_character(critter) {
        in_vehicle = ctx.combat.in_vehicle(critter);
        if in_vehicle {
            if let Some(vp) = ctx.map.veh_at(ctx.combat.pos(critter)) {
                if vp.is_inside {
                    inside_vehicle = true;
                }
                if vp.nofields {
                    // Same as just skipping each time in the loop below
                    return;
                }
            }
        }
        player_in_field(ctx, critter);
    }

    // Position re-read: a teleport above moves the creature.
    let Some(here) = ctx.ctile(ctx.combat.pos(critter)) else { return };
    let mut next = ctx.next_entry(here, None);
    while let Some(ty) = next {
        if ctx.e_alive(here, ty) {
            let cur = ctx.e_entry(here, ty);
            let effects = types.get(ty).level(cur.intensity() - 1).field_effects.clone();
            for fe in &effects {
                if in_vehicle && fe.immune_in_vehicle {
                    continue;
                }
                if inside_vehicle && fe.immune_inside_vehicle {
                    continue;
                }
                if !inside_vehicle && fe.immune_outside_vehicle {
                    continue;
                }
                if in_vehicle && !ctx.dice.one_in(fe.chance_in_vehicle) {
                    continue;
                }
                if inside_vehicle && !ctx.dice.one_in(fe.chance_inside_vehicle) {
                    continue;
                }
                if !inside_vehicle && !ctx.dice.one_in(fe.chance_outside_vehicle) {
                    continue;
                }
                // `fe.get_effect()` rolls its duration even if the creature turns out immune.
                let fx_dur = ctx.dice.rng(fe.min_duration, fe.max_duration);
                if ctx.combat.is_immune_field(critter, ty) || ctx.combat.is_immune_effect(critter, &fe.id) {
                    continue;
                }
                let effect_added = if fe.is_environmental {
                    let vector = fe.bp.as_deref().unwrap_or("");
                    let dur = ctx.dice.rng(fe.min_duration, fe.max_duration);
                    env4(ctx, critter, &fe.id, vector, fe.intensity, dur)
                } else {
                    ctx.combat.add_effect(critter, &fe.id, fx_dur, fe.bp.as_deref(), fe.intensity);
                    true
                };
                if effect_added {
                    let ty = match fe.message_type.as_str() {
                        "good" => MsgType::Good,
                        "bad" => MsgType::Bad,
                        "mixed" => MsgType::Mixed,
                        "warning" => MsgType::Warning,
                        "info" => MsgType::Info,
                        _ => MsgType::Neutral,
                    };
                    ctx.combat.message(Some(critter), ty, &fe.message, Some(&fe.message_npc));
                }
            }
        }
        next = ctx.next_entry(here, Some(ty));
    }
}

fn fire_mat_bonus(c: &dyn Combat, z: CreatureId, flammable: i32, flameres: i32) -> i32 {
    let mut dam = 0;
    if c.made_of_any(z, MatGroup::Flesh) {
        dam += 3;
    }
    if c.made_of(z, "veggy") {
        dam += 12;
    }
    if c.made_of_liquid(z) || c.made_of_any(z, MatGroup::Flammable) {
        dam += flammable;
    }
    if c.made_of_any(z, MatGroup::Flameres) {
        dam += flameres;
    }
    dam
}

/// `map::monster_in_field(z)`.
pub fn monster_in_field(ctx: &mut MapCtx<'_>, z: CreatureId) {
    let fd = ctx.types.fd.clone();
    if ctx.combat.digging(z) {
        // Digging monsters are immune to fields
        return;
    }
    let zpos = ctx.combat.pos(z);
    if ctx.map.veh_at(zpos).is_some() {
        // FIXME (BN): Immune when in a vehicle for now.
        return;
    }
    let Some(here) = ctx.ctile(zpos) else { return };

    let mut dam = 0;
    let mut next = ctx.next_entry(here, None);
    while let Some(ft) = next {
        if !ctx.e_alive(here, ft) {
            next = ctx.next_entry(here, Some(ft));
            continue;
        }
        if ft == fd.web && !ctx.combat.has_mon_flag(z, "WEBWALK") {
            let i = ctx.e_i(here, ft);
            ctx.combat.add_effect(z, "webbed", 1, None, i);
            ctx.e_set_i(here, ft, 0);
        }
        if ft == fd.acid && !ctx.combat.flies(z) {
            let i = ctx.e_i(here, ft);
            let d = ctx.dice.rng(i, i * 3);
            ctx.combat.deal_damage(z, "torso", DamageType::Acid, d, ctx.dice);
            ctx.combat.check_dead_state(z);
            if d > 0 {
                let dur = ctx.dice.rng(d / 2, d * 2);
                ctx.combat.add_effect(z, "corroding", dur, None, 0);
            }
        }
        if ft == fd.sap {
            let i = ctx.e_i(here, ft);
            ctx.combat.mod_moves(z, -(i * 5));
            ctx.e_set_i(here, ft, i - 1);
        }
        if ft == fd.sludge && !ctx.combat.digs(z) && !ctx.combat.flies(z) && !ctx.combat.has_mon_flag(z, "SLUDGEPROOF") {
            let i = ctx.e_i(here, ft);
            ctx.combat.mod_moves(z, -(i * 300));
            ctx.e_set_i(here, ft, 0);
        }
        if ft == fd.fire {
            // TODO (BN): MATERIALS Use fire resistance
            if ctx.combat.has_mon_flag(z, "FIREPROOF") || ctx.combat.has_mon_flag(z, "FIREY") {
                return;
            }
            dam += fire_mat_bonus(&*ctx.combat, z, 20, -20);
            if ctx.combat.flies(z) {
                dam -= 15;
            }
            dam -= ctx.combat.get_armor_type(z, DamageType::Heat, "torso");
            let i = ctx.e_i(here, ft);
            if i == 1 {
                dam += ctx.dice.rng(2, 6);
            } else if i == 2 {
                dam += ctx.dice.rng(6, 12);
                if !ctx.combat.flies(z) {
                    ctx.combat.mod_moves(z, -20);
                    if dam > 0 {
                        let d = ctx.dice.rng(dam / 2, dam * 2);
                        ctx.combat.add_effect(z, "onfire", d, None, 0);
                    }
                }
            } else if i == 3 {
                dam += ctx.dice.rng(10, 20);
                if !ctx.combat.flies(z) || ctx.dice.one_in(3) {
                    ctx.combat.mod_moves(z, -40);
                    if dam > 0 {
                        let d = ctx.dice.rng(dam / 2, dam * 2);
                        ctx.combat.add_effect(z, "onfire", d, None, 0);
                    }
                }
            }
        }
        if ft == fd.smoke && !ctx.combat.has_mon_flag(z, "NO_BREATHE") {
            let i = ctx.e_i(here, ft);
            if i == 3 {
                let m = ctx.dice.rng(10, 20);
                ctx.combat.mod_moves(z, -m);
            }
            // Plants suffer from smoke even worse
            if ctx.combat.made_of(z, "veggy") {
                let m = ctx.dice.rng(1, i * 12);
                ctx.combat.mod_moves(z, -m);
            }
        }
        if ft == fd.tear_gas && ctx.combat.made_of_any(z, MatGroup::FleshNVeg) && !ctx.combat.has_mon_flag(z, "NO_BREATHE") {
            let i = ctx.e_i(here, ft);
            if i == 3 {
                let d = ctx.dice.rng(MINUTES, 2 * MINUTES);
                ctx.combat.add_effect(z, "stunned", d, None, 0);
                dam += ctx.dice.rng(4, 10);
            } else if i == 2 {
                let d = ctx.dice.rng(5, 10);
                ctx.combat.add_effect(z, "stunned", d, None, 0);
                dam += ctx.dice.rng(2, 5);
            } else {
                let d = ctx.dice.rng(1, 5);
                ctx.combat.add_effect(z, "stunned", d, None, 0);
            }
            if ctx.combat.made_of(z, "veggy") {
                let m = ctx.dice.rng(i * 5, i * 12);
                ctx.combat.mod_moves(z, -m);
                dam += i * ctx.dice.rng(8, 14);
            }
            if ctx.combat.has_mon_flag(z, "SEES") {
                ctx.combat.add_effect(z, "blind", i * 8, None, 0);
            }
        }
        if ft == fd.relax_gas && ctx.combat.made_of_any(z, MatGroup::FleshNVeg) && !ctx.combat.has_mon_flag(z, "NO_BREATHE") {
            let i = ctx.e_i(here, ft);
            let d = ctx.dice.rng(i * 4, i * 8);
            ctx.combat.add_effect(z, "stunned", d, None, 0);
        }
        if ft == fd.dazzling && ctx.combat.has_mon_flag(z, "SEES") && !ctx.combat.has_mon_flag(z, "ELECTRONIC") {
            let i = ctx.e_i(here, ft);
            ctx.combat.add_effect(z, "blind", i * 12, None, 0);
            let d = i * ctx.dice.rng(5, 12);
            ctx.combat.add_effect(z, "stunned", d, None, 0);
        }
        if ft == fd.toxic_gas && !ctx.combat.has_mon_flag(z, "NO_BREATHE") {
            let i = ctx.e_i(here, ft);
            dam += i;
            ctx.combat.mod_moves(z, -i);
        }
        if ft == fd.nuke_gas && !ctx.combat.has_mon_flag(z, "NO_BREATHE") {
            let i = ctx.e_i(here, ft);
            if i == 3 {
                let m = ctx.dice.rng(60, 120);
                ctx.combat.mod_moves(z, -m);
                dam += ctx.dice.rng(30, 50);
            } else if i == 2 {
                let m = ctx.dice.rng(20, 50);
                ctx.combat.mod_moves(z, -m);
                dam += ctx.dice.rng(10, 25);
            } else {
                let m = ctx.dice.rng(0, 15);
                ctx.combat.mod_moves(z, -m);
                dam += ctx.dice.rng(0, 12);
            }
            if ctx.combat.made_of(z, "veggy") {
                let m = ctx.dice.rng(i * 5, i * 12);
                ctx.combat.mod_moves(z, -m);
                dam *= i;
            }
        }
        if ft == fd.flame_burst {
            if ctx.combat.has_mon_flag(z, "FIREPROOF") || ctx.combat.has_mon_flag(z, "FIREY") {
                return;
            }
            dam += fire_mat_bonus(&*ctx.combat, z, 50, -25);
            dam += ctx.dice.rng(0, 8);
            ctx.combat.mod_moves(z, -20);
        }
        if ft == fd.electricity {
            // A separate hit so that it can apply effects
            let i = ctx.e_i(here, ft);
            let d = ctx.dice.rng(1, i * 3);
            ctx.combat.deal_damage(z, "torso", DamageType::Electric, d, ctx.dice);
        }
        if ft == fd.fatigue {
            let i = ctx.e_i(here, ft);
            if ctx.dice.rng(0, 2) < i {
                dam += i;
                ctx.combat.teleport(z, ctx.dice);
            }
        }
        if ft == fd.incendiary {
            if ctx.combat.has_mon_flag(z, "FIREPROOF") || ctx.combat.has_mon_flag(z, "FIREY") {
                return;
            }
            dam += fire_mat_bonus(&*ctx.combat, z, 20, -5);
            let i = ctx.e_i(here, ft);
            let no_onfire = ctx.combat.made_of_liquid(z) || ctx.combat.made_of_any(z, MatGroup::Flameres);
            if i == 1 {
                dam += ctx.dice.rng(2, 6);
            } else if i == 2 {
                dam += ctx.dice.rng(6, 12);
                ctx.combat.mod_moves(z, -20);
                if !no_onfire {
                    let d = ctx.dice.rng(8, 12);
                    ctx.combat.add_effect(z, "onfire", d, None, 0);
                }
            } else if i == 3 {
                dam += ctx.dice.rng(10, 20);
                ctx.combat.mod_moves(z, -40);
                if !no_onfire {
                    let d = ctx.dice.rng(12, 16);
                    ctx.combat.add_effect(z, "onfire", d, None, 0);
                }
            }
        }
        if ft == fd.fungal_haze
            && !ctx.combat.in_species(z, "FUNGUS")
            && !ctx.combat.has_mon_flag(z, "NO_BREATHE")
            && !ctx.combat.make_fungus(z)
        {
            // Don't insta-kill jabberwocks, that's silly
            let i = ctx.e_i(here, ft);
            let m = ctx.dice.rng(10 * i, 30 * i);
            ctx.combat.mod_moves(z, -m);
            dam += ctx.dice.rng(0, 10 * i);
        }
        if ft == fd.fungicidal_gas && ctx.combat.in_species(z, "FUNGUS") {
            let i = ctx.e_i(here, ft);
            let m = ctx.dice.rng(10 * i, 30 * i);
            ctx.combat.mod_moves(z, -m);
            dam += ctx.dice.rng(4, 7 * i);
        }
        if ft == fd.insecticidal_gas && (ctx.combat.in_species(z, "INSECT") || ctx.combat.in_species(z, "SPIDER")) {
            let i = ctx.e_i(here, ft);
            let m = ctx.dice.rng(10 * i, 30 * i);
            ctx.combat.mod_moves(z, -m);
            dam += ctx.dice.rng(4, 7 * i);
        }
        next = ctx.next_entry(here, Some(ft));
    }

    if dam > 0 {
        ctx.combat.apply_damage(z, "torso", dam);
        ctx.combat.check_dead_state(z);
    }
}
