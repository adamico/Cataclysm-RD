//! Combat stub: the port where harm to creatures crosses out of Fields.
//!
//! [`RecordingCombat`] answers creature queries from a hand-set
//! [`StubCreature`] and **records every sink call, doing nothing else**: no
//! damage is applied, no effect is added, nobody moves or dies. Where BN's
//! sink returns a value Fields branches on, the stub returns a fixed answer
//! (documented on each method and in `STUBS.md`) and makes no draws, so a
//! replay of a case that reaches Creature-side draws will desync there.

use bn_dice::Dice;
use fields::content::Turns;
use fields::ports::{Combat, CreatureId, DamageType, MatGroup, MsgType};
use fields::storage::Tripoint;
use fields::FieldTypeId;
use std::collections::{HashMap, HashSet};

/// What a creature answers. Strings in `tags` cover traits, bionics, worn
/// itypes, worn flags, monster flags, materials (`mat:veggy`, `grp:Flesh`,
/// `liquid`) and species (`species:FUNGUS`).
#[derive(Clone, Debug, Default)]
pub struct StubCreature {
    pub pos: Tripoint,
    pub monster: bool,
    pub avatar: bool,
    pub in_vehicle: bool,
    pub on_ground: bool,
    pub one_arm: bool,
    pub elec_immune: bool,
    pub underwater: bool,
    pub digging: bool,
    pub digs: bool,
    pub flies: bool,
    pub tags: HashSet<String>,
    pub armor: i32,
    pub env_resist: i32,
    pub coverage: i32,
    pub highest_category: String,
}

/// One recorded call into Combat.
#[derive(Clone, Debug, PartialEq)]
pub enum CombatCall {
    DealDamage { c: CreatureId, bp: String, dt: DamageType, amount: i32 },
    ApplyDamage { c: CreatureId, bp: String, amount: i32 },
    Hurtall { c: CreatureId, amount: i32 },
    AddEffect { c: CreatureId, eff: String, dur: Turns, bp: Option<String>, intensity: i32 },
    AddEnvEffect { c: CreatureId, eff: String, vector: String, strength: i32, dur: Turns, bp: Option<String>, intensity: i32 },
    Irradiate { c: CreatureId, rads: f32 },
    ModPain { c: CreatureId, n: i32 },
    ModMoves { c: CreatureId, n: i32 },
    CheckDeadState { c: CreatureId },
    Teleport { c: CreatureId },
    MakeFungus { c: CreatureId },
    RandomBodyPart { c: CreatureId },
    Message { c: Option<CreatureId>, ty: MsgType, player: String, npc: Option<String> },
}

#[derive(Debug, Default)]
pub struct RecordingCombat {
    pub creatures: HashMap<CreatureId, StubCreature>,
    pub calls: Vec<CombatCall>,
}

impl RecordingCombat {
    pub fn add(&mut self, id: u32, c: StubCreature) -> CreatureId {
        let id = CreatureId(id);
        self.creatures.insert(id, c);
        id
    }
    fn c(&self, c: CreatureId) -> &StubCreature {
        &self.creatures[&c]
    }
    fn tag(&self, c: CreatureId, t: &str) -> bool {
        self.c(c).tags.contains(t)
    }
}

const ALL_BODY_PARTS: [&str; 12] =
    ["torso", "head", "eyes", "mouth", "arm_l", "arm_r", "hand_l", "hand_r", "leg_l", "leg_r", "foot_l", "foot_r"];

impl Combat for RecordingCombat {
    fn is_monster(&self, c: CreatureId) -> bool {
        self.c(c).monster
    }
    fn is_character(&self, c: CreatureId) -> bool {
        !self.c(c).monster
    }
    fn is_avatar(&self, c: CreatureId) -> bool {
        self.c(c).avatar
    }
    fn pos(&self, c: CreatureId) -> Tripoint {
        self.c(c).pos
    }
    fn in_vehicle(&self, c: CreatureId) -> bool {
        self.c(c).in_vehicle
    }
    fn has_trait(&self, c: CreatureId, t: &str) -> bool {
        self.tag(c, t)
    }
    fn has_active_bionic(&self, c: CreatureId, b: &str) -> bool {
        self.tag(c, b)
    }
    fn is_wearing(&self, c: CreatureId, itype: &str) -> bool {
        self.tag(c, itype)
    }
    fn worn_with_flag(&self, c: CreatureId, flag: &str) -> bool {
        self.tag(c, flag)
    }
    fn is_on_ground(&self, c: CreatureId) -> bool {
        self.c(c).on_ground
    }
    fn has_two_arms(&self, c: CreatureId) -> bool {
        !self.c(c).one_arm
    }
    fn is_elec_immune(&self, c: CreatureId) -> bool {
        self.c(c).elec_immune
    }
    fn is_underwater(&self, c: CreatureId) -> bool {
        self.c(c).underwater
    }
    fn get_armor_type(&self, c: CreatureId, _dt: DamageType, _bp: &str) -> i32 {
        self.c(c).armor
    }
    fn get_armor_cut(&self, c: CreatureId, _bp: &str) -> i32 {
        self.c(c).armor
    }
    fn get_env_resist(&self, c: CreatureId, _bp: &str) -> i32 {
        self.c(c).env_resist
    }
    fn worn_coverage(&self, c: CreatureId, _bp: &str) -> i32 {
        self.c(c).coverage
    }
    fn get_highest_category(&self, c: CreatureId) -> String {
        self.c(c).highest_category.clone()
    }
    fn get_all_body_parts(&self, _c: CreatureId) -> Vec<String> {
        ALL_BODY_PARTS.map(String::from).to_vec()
    }
    /// Fixed answer "torso", no draw (BN's `get_random_body_part` draws).
    fn get_random_body_part(&mut self, c: CreatureId, _dice: &mut dyn Dice) -> String {
        self.calls.push(CombatCall::RandomBodyPart { c });
        "torso".into()
    }
    fn is_immune_field(&self, c: CreatureId, _ty: FieldTypeId) -> bool {
        self.tag(c, "immune_field")
    }
    fn is_immune_effect(&self, c: CreatureId, eff: &str) -> bool {
        self.tag(c, &format!("immune:{eff}"))
    }
    fn digging(&self, c: CreatureId) -> bool {
        self.c(c).digging
    }
    fn digs(&self, c: CreatureId) -> bool {
        self.c(c).digs
    }
    fn flies(&self, c: CreatureId) -> bool {
        self.c(c).flies
    }
    fn has_mon_flag(&self, c: CreatureId, f: &str) -> bool {
        self.tag(c, f)
    }
    fn made_of(&self, c: CreatureId, material: &str) -> bool {
        self.tag(c, &format!("mat:{material}"))
    }
    fn made_of_liquid(&self, c: CreatureId) -> bool {
        self.tag(c, "liquid")
    }
    fn made_of_any(&self, c: CreatureId, g: MatGroup) -> bool {
        self.tag(c, &format!("grp:{g:?}"))
    }
    fn in_species(&self, c: CreatureId, s: &str) -> bool {
        self.tag(c, &format!("species:{s}"))
    }

    /// Recorded; returns `amount` as "dealt" (no armour, no draws).
    fn deal_damage(&mut self, c: CreatureId, bp: &str, dt: DamageType, amount: i32, _dice: &mut dyn Dice) -> i32 {
        self.calls.push(CombatCall::DealDamage { c, bp: bp.into(), dt, amount });
        amount
    }
    fn apply_damage(&mut self, c: CreatureId, bp: &str, amount: i32) {
        self.calls.push(CombatCall::ApplyDamage { c, bp: bp.into(), amount });
    }
    fn hurtall(&mut self, c: CreatureId, amount: i32) {
        self.calls.push(CombatCall::Hurtall { c, amount });
    }
    fn add_effect(&mut self, c: CreatureId, eff: &str, dur: Turns, bp: Option<&str>, intensity: i32) {
        self.calls.push(CombatCall::AddEffect { c, eff: eff.into(), dur, bp: bp.map(String::from), intensity });
    }
    /// Recorded; returns true ("not resisted"), no draws.
    fn add_env_effect(
        &mut self,
        c: CreatureId,
        eff: &str,
        vector: &str,
        strength: i32,
        dur: Turns,
        bp: Option<&str>,
        intensity: i32,
        _dice: &mut dyn Dice,
    ) -> bool {
        self.calls.push(CombatCall::AddEnvEffect {
            c,
            eff: eff.into(),
            vector: vector.into(),
            strength,
            dur,
            bp: bp.map(String::from),
            intensity,
        });
        true
    }
    /// Recorded; returns true (not rad-proof), no draws.
    fn irradiate(&mut self, c: CreatureId, rads: f32, _dice: &mut dyn Dice) -> bool {
        self.calls.push(CombatCall::Irradiate { c, rads });
        true
    }
    fn mod_pain(&mut self, c: CreatureId, n: i32) {
        self.calls.push(CombatCall::ModPain { c, n });
    }
    fn mod_moves(&mut self, c: CreatureId, n: i32) {
        self.calls.push(CombatCall::ModMoves { c, n });
    }
    fn check_dead_state(&mut self, c: CreatureId) {
        self.calls.push(CombatCall::CheckDeadState { c });
    }
    /// Recorded; nobody moves, no draws.
    fn teleport(&mut self, c: CreatureId, _dice: &mut dyn Dice) {
        self.calls.push(CombatCall::Teleport { c });
    }
    /// Recorded; returns false (did not turn).
    fn make_fungus(&mut self, c: CreatureId) -> bool {
        self.calls.push(CombatCall::MakeFungus { c });
        false
    }
    fn message(&mut self, c: Option<CreatureId>, ty: MsgType, player: &str, npc: Option<&str>) {
        self.calls.push(CombatCall::Message { c, ty, player: player.into(), npc: npc.map(String::from) });
    }
}
