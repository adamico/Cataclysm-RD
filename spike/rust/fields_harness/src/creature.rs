//! Creature cases: `map::creature_in_field` → `monster_in_field` replayed
//! against the bubble-map recordings (`monster` layouts, session 3).
//!
//! [`ReplayCombat`] is the Combat port answered from the case: queries come
//! from BN's `creature_def` mark, and every sink BN reaches through a wrapped
//! call (`Creature::add_effect`, `check_dead_state`, `add_env_effect`,
//! `teleport::teleport`, `monster::make_fungus`) must be the next recorded
//! call with the same arguments. `deal_damage` is virtual in BN and cannot be
//! wrapped at link time: its draws are consumed as the run of non-Fields draws
//! that follows (see `ReplayDice::skip_foreign_draws`).

use crate::replay::ReplayDice;
use bn_dice::{Dice, ForeignCall};
use combat_stub::{RecordingCombat, StubCreature};
use fields::content::Turns;
use fields::ports::{Combat, CreatureId, DamageType, MatGroup, MsgType};
use fields::storage::Tripoint;
use fields::{FieldTypeId, FieldTypes};
use serde_json::Value;
use std::cell::RefCell;
use std::rc::Rc;

pub type Shared<'c> = Rc<RefCell<ReplayDice<'c>>>;

/// A `Dice` over a shared `ReplayDice`, so the Combat stub can read the same case.
pub struct SharedDice<'c>(pub Shared<'c>);

impl Dice for SharedDice<'_> {
    fn rng(&mut self, lo: i32, hi: i32) -> i32 {
        self.0.borrow_mut().rng(lo, hi)
    }
    fn rng_float(&mut self, lo: f64, hi: f64) -> f64 {
        self.0.borrow_mut().rng_float(lo, hi)
    }
    fn one_in(&mut self, chance: i32) -> bool {
        self.0.borrow_mut().one_in(chance)
    }
    fn x_in_y(&mut self, x: f64, y: f64) -> bool {
        self.0.borrow_mut().x_in_y(x, y)
    }
    fn dice(&mut self, number: i32, sides: i32) -> i32 {
        self.0.borrow_mut().dice(number, sides)
    }
    fn roll_remainder(&mut self, value: f64) -> i32 {
        self.0.borrow_mut().roll_remainder(value)
    }
    fn foreign(&mut self, name: &str) -> Option<ForeignCall> {
        self.0.borrow_mut().foreign(name)
    }
}

/// The `creature_def` mark of a creature case.
pub fn stub_from_def(def: &Value, pos: Tripoint) -> StubCreature {
    let b = |k: &str| def[k].as_bool().unwrap_or(false);
    StubCreature {
        pos,
        monster: b("monster"),
        flies: b("flies"),
        digs: b("digs"),
        digging: b("digging"),
        armor: def["armor_heat_torso"].as_i64().unwrap_or(0) as i32,
        tags: def["tags"].as_array().unwrap().iter().map(|t| t.as_str().unwrap().to_string()).collect(),
        ..Default::default()
    }
}

pub struct ReplayCombat<'c, 't> {
    pub inner: RecordingCombat,
    pub dice: Shared<'c>,
    pub types: &'t FieldTypes,
    /// Sum of `mod_moves` (BN: `z.moves -= ...` directly).
    pub moves: i32,
    /// Sum of `apply_damage`.
    pub applied: i32,
    /// `deal_damage` or `add_effect` was called: hp then also moves on the
    /// Combat side (armour; effects such as `onfire` that act when added).
    pub dealt: bool,
    /// Absolute tile of the layout's (0, 0), to place a teleported creature.
    pub origin: Tripoint,
}

impl<'c, 't> ReplayCombat<'c, 't> {
    fn call(&self, name: &str) -> ForeignCall {
        self.dice.borrow_mut().foreign(name).expect("replay source")
    }
    fn check(&self, what: &str, got: String, want: String) {
        if got != want {
            panic!("replay mismatch: Rust {what} {got}, BN {want}");
        }
    }
}

impl Combat for ReplayCombat<'_, '_> {
    fn is_monster(&self, c: CreatureId) -> bool {
        self.inner.is_monster(c)
    }
    fn is_character(&self, c: CreatureId) -> bool {
        self.inner.is_character(c)
    }
    fn is_avatar(&self, c: CreatureId) -> bool {
        self.inner.is_avatar(c)
    }
    fn pos(&self, c: CreatureId) -> Tripoint {
        self.inner.pos(c)
    }
    fn in_vehicle(&self, c: CreatureId) -> bool {
        self.inner.in_vehicle(c)
    }
    fn has_trait(&self, c: CreatureId, t: &str) -> bool {
        self.inner.has_trait(c, t)
    }
    fn has_active_bionic(&self, c: CreatureId, b: &str) -> bool {
        self.inner.has_active_bionic(c, b)
    }
    fn is_wearing(&self, c: CreatureId, itype: &str) -> bool {
        self.inner.is_wearing(c, itype)
    }
    fn worn_with_flag(&self, c: CreatureId, flag: &str) -> bool {
        self.inner.worn_with_flag(c, flag)
    }
    fn is_on_ground(&self, c: CreatureId) -> bool {
        self.inner.is_on_ground(c)
    }
    fn has_two_arms(&self, c: CreatureId) -> bool {
        self.inner.has_two_arms(c)
    }
    fn is_elec_immune(&self, c: CreatureId) -> bool {
        self.inner.is_elec_immune(c)
    }
    fn is_underwater(&self, c: CreatureId) -> bool {
        self.inner.is_underwater(c)
    }
    fn get_armor_type(&self, c: CreatureId, dt: DamageType, bp: &str) -> i32 {
        assert!(dt == DamageType::Heat && bp == "torso", "creature_def records heat/torso armour only");
        self.inner.get_armor_type(c, dt, bp)
    }
    fn get_armor_cut(&self, c: CreatureId, bp: &str) -> i32 {
        self.inner.get_armor_cut(c, bp)
    }
    fn get_env_resist(&self, c: CreatureId, bp: &str) -> i32 {
        self.inner.get_env_resist(c, bp)
    }
    fn worn_coverage(&self, c: CreatureId, bp: &str) -> i32 {
        self.inner.worn_coverage(c, bp)
    }
    fn get_highest_category(&self, c: CreatureId) -> String {
        self.inner.get_highest_category(c)
    }
    fn get_all_body_parts(&self, c: CreatureId) -> Vec<String> {
        self.inner.get_all_body_parts(c)
    }
    fn get_random_body_part(&mut self, _c: CreatureId, _dice: &mut dyn Dice) -> String {
        panic!("get_random_body_part: player cases are not recorded (see REPLAY_FINDINGS.md)")
    }
    fn is_immune_field(&self, c: CreatureId, ty: FieldTypeId) -> bool {
        self.inner.creatures[&c].tags.contains(&format!("immune_field:{}", self.types.str_id(ty)))
    }
    fn is_immune_effect(&self, c: CreatureId, eff: &str) -> bool {
        self.inner.is_immune_effect(c, eff)
    }
    fn digging(&self, c: CreatureId) -> bool {
        self.inner.digging(c)
    }
    fn digs(&self, c: CreatureId) -> bool {
        self.inner.digs(c)
    }
    fn flies(&self, c: CreatureId) -> bool {
        self.inner.flies(c)
    }
    fn has_mon_flag(&self, c: CreatureId, f: &str) -> bool {
        self.inner.has_mon_flag(c, f)
    }
    fn made_of(&self, c: CreatureId, material: &str) -> bool {
        assert_eq!(material, "veggy", "creature_def records mat:veggy only");
        self.inner.made_of(c, material)
    }
    fn made_of_liquid(&self, c: CreatureId) -> bool {
        self.inner.made_of_liquid(c)
    }
    fn made_of_any(&self, c: CreatureId, g: MatGroup) -> bool {
        self.inner.made_of_any(c, g)
    }
    fn in_species(&self, c: CreatureId, s: &str) -> bool {
        self.inner.in_species(c, s)
    }

    fn deal_damage(&mut self, c: CreatureId, bp: &str, dt: DamageType, amount: i32, dice: &mut dyn Dice) -> i32 {
        self.dealt = true;
        // Virtual in BN: not bracketed in the case. Take the draws it made.
        let _ = dice;
        self.dice.borrow_mut().skip_foreign_draws();
        self.inner.deal_damage(c, bp, dt, amount, &mut bn_dice::BnDice::new(bn_dice::SplitMix(0)))
    }
    fn apply_damage(&mut self, c: CreatureId, bp: &str, amount: i32) {
        self.applied += amount;
        self.inner.apply_damage(c, bp, amount);
    }
    fn hurtall(&mut self, c: CreatureId, amount: i32) {
        self.inner.hurtall(c, amount);
    }
    fn add_effect(&mut self, c: CreatureId, eff: &str, dur: Turns, bp: Option<&str>, intensity: i32) {
        self.dealt = true;
        self.inner.add_effect(c, eff, dur, bp, intensity);
        let fc = self.call("Creature::add_effect");
        self.check(
            "add_effect",
            format!("({eff}, {dur}, {intensity})"),
            format!("({}, {}, {})", fc.sarg("eff"), fc.arg("dur"), fc.arg("intensity")),
        );
    }
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
    ) -> bool {
        self.inner.add_env_effect(c, eff, vector, strength, dur, bp, intensity, dice);
        let fc = self.call("Creature::add_env_effect");
        self.check(
            "add_env_effect",
            format!("({eff}, {vector}, {strength}, {dur})"),
            format!("({}, {}, {}, {})", fc.sarg("eff"), fc.sarg("vector"), fc.arg("strength"), fc.arg("dur")),
        );
        fc.ret("r") != 0.0
    }
    fn irradiate(&mut self, _c: CreatureId, _rads: f32, _dice: &mut dyn Dice) -> bool {
        panic!("irradiate: player cases are not recorded")
    }
    fn mod_pain(&mut self, c: CreatureId, n: i32) {
        self.inner.mod_pain(c, n);
    }
    fn mod_moves(&mut self, c: CreatureId, n: i32) {
        self.moves += n;
        self.inner.mod_moves(c, n);
    }
    fn check_dead_state(&mut self, c: CreatureId) {
        self.inner.check_dead_state(c);
        self.call("Creature::check_dead_state");
    }
    fn teleport(&mut self, c: CreatureId, dice: &mut dyn Dice) {
        self.inner.teleport(c, dice);
        // BN moved the creature (Combat side): follow it.
        let fc = self.call("teleport::teleport");
        let p = self.origin.offset(fc.ret("x") as i32, fc.ret("y") as i32);
        self.inner.creatures.get_mut(&c).unwrap().pos = p;
    }
    fn make_fungus(&mut self, c: CreatureId) -> bool {
        self.inner.make_fungus(c);
        self.call("monster::make_fungus").ret("r") != 0.0
    }
    fn message(&mut self, c: Option<CreatureId>, ty: MsgType, player: &str, npc: Option<&str>) {
        self.inner.message(c, ty, player, npc);
    }
}
