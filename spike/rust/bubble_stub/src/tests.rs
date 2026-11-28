//! Smoke tests: the code compiles, loads BN Content and runs. Not replay tests
//! (session 3). Tests that need BN's JSON look in `$BN_ROOT` and are skipped
//! (with a note on stderr) when it is not set and the default path is missing.

use super::*;
use bn_dice::{BnDice, LoggingDice, SplitMix};
use combat_stub::{CombatCall, RecordingCombat, StubCreature};
use fields::batch::{batch_turns_field, BatchDecay};
use fields::content::{StackingType, MINUTES};
use fields::creatures::creature_in_field;
use fields::heap::StdPriorityQueue;
use fields::map_rules::{propagate_field, MapCtx};
use fields::{process_fields_in_submap, FieldTypes, SubmapCtx};

const DEFAULT_BN: &str = "/tmp/claude-1000/-home-kc00l-Work-Cataclysm-RD/e4e51f43-d7b5-4322-acca-6f43fcb7cfaf/scratchpad/bn";

fn types() -> Option<FieldTypes> {
    let root = std::env::var("BN_ROOT").unwrap_or_else(|_| DEFAULT_BN.to_string());
    let root = std::path::Path::new(&root);
    if !root.join("data/json/field_type.json").exists() {
        eprintln!("skipping: no BN checkout at {} (set BN_ROOT)", root.display());
        return None;
    }
    Some(FieldTypes::load_bn(root).expect("load field types"))
}

const ORIGIN: SmPos = SmPos { x: 200, y: 200, z: 0 };

struct World {
    region: Region,
    md: StubMapData,
    items: StubItems,
    caches: StubCaches,
    turn: i32,
}

impl World {
    fn new(ter: &str) -> Self {
        let md = StubMapData::default();
        let region = Region::block(ORIGIN, 3, 3, md.ter_id(ter));
        Self { region, md, items: StubItems::default(), caches: StubCaches::default(), turn: 3600 }
    }
    fn at(x: i32, y: i32) -> Tripoint {
        Tripoint::new(ORIGIN.x * 12 + x, ORIGIN.y * 12 + y, 0)
    }
    /// One harness turn: row-major submaps, then `calendar::turn += 1`.
    fn turn(&mut self, types: &FieldTypes, dice: &mut dyn bn_dice::Dice) -> bool {
        let mut fire = false;
        self.items.now = self.turn;
        for pos in self.region.row_major() {
            let mut ctx = SubmapCtx {
                types,
                store: &mut self.region,
                map_data: &self.md,
                items: &mut self.items,
                caches: &mut self.caches,
                clock: Clock { turn: self.turn, turns_this_tick: 1 },
                dice,
            };
            fire |= process_fields_in_submap(&mut ctx, pos);
        }
        self.turn += 1;
        fire
    }
    fn count_entries(&self, sm: SmPos) -> i32 {
        let f = &self.region.submaps[&sm].fields;
        (0..12).flat_map(|x| (0..12).map(move |y| Local::new(x, y))).map(|l| f.get(l).count() as i32).sum()
    }
}

#[test]
fn loads_bn_field_types_in_load_order() {
    let Some(t) = types() else { return };
    assert_eq!(t.types.len(), 62);
    assert_eq!(t.fd.null, FieldTypeId(0));
    assert_eq!(t.str_id(t.fd.fire), "fd_fire");
    assert!(t.fd.fire < t.fd.smoke, "std::map order follows int ids");
    let fire = t.get(t.fd.fire);
    assert_eq!(fire.stacking_type, StackingType::Duration);
    assert_eq!(fire.half_life, 30 * MINUTES);
    assert!(fire.has_fire);
    assert_eq!(fire.max_intensity(), 3);
    assert_eq!(t.str_id(t.get(t.fd.gas_vent).wandering_field), "fd_toxic_gas");
    // "repeat last entry" levels inherit the previous level.
    assert_eq!(t.get(t.fd.gas_vent).intensity_levels[2].name, "gas vent");
    // effects fall back to the previous level when absent
    let smoke = t.get(t.fd.smoke);
    assert_eq!(smoke.level(0).field_effects[0].id, "smoke");
}

#[test]
fn fire_and_smoke_run_and_keep_field_count() {
    let Some(t) = types() else { return };
    let mut w = World::new("t_grass");
    w.region.place(&t, World::at(12, 12), t.fd.fire, 2, 1);
    w.region.place(&t, World::at(13, 12), t.fd.fire, 1, 1);
    w.region.place(&t, World::at(16, 16), t.fd.smoke, 3, 1);
    w.region.place(&t, World::at(5, 5), t.fd.toxic_gas, 3, 1);
    w.region.set_furn_at(World::at(14, 13), w.md.furn_id("f_chair"));
    w.items.add(World::at(12, 12), StubItem { id: 1, rock: false, flammable: true, weight_g: 100, birthday: 0 });
    let mut eng = BnDice::new(SplitMix(42));
    let mut saw_fire = false;
    for _ in 0..50 {
        saw_fire |= w.turn(&t, &mut eng);
        for sm in w.region.row_major() {
            // BN's hand-kept counter matches the entries (no gas_remover here).
            assert_eq!(w.region.submaps[&sm].fields.field_count, w.count_entries(sm), "{sm:?}");
        }
    }
    assert!(saw_fire);
    assert!(w.items.calls.iter().any(|c| matches!(c, ItemCall::Burn { .. })), "items on a fire tile get burned");
    // Spread happened somewhere: more than the 4 seeded tiles saw a field.
    let tiles: HashSet<Tripoint> = w.region.entries().into_iter().map(|e| e.0).collect();
    eprintln!("tiles with fields after 50 turns: {}", tiles.len());
}

#[test]
fn draw_sequence_is_deterministic_and_helper_level() {
    let Some(t) = types() else { return };
    let run = || {
        let mut w = World::new("t_grass");
        w.region.place(&t, World::at(12, 12), t.fd.fire, 2, 1);
        w.region.place(&t, World::at(18, 14), t.fd.smoke, 3, 1);
        let mut eng = BnDice::new(SplitMix(7));
        let mut log = LoggingDice::new(&mut eng);
        for _ in 0..10 {
            w.turn(&t, &mut log);
        }
        log.log
    };
    let a = run();
    assert!(!a.is_empty());
    assert_eq!(a, run());
    assert!(a.iter().any(|l| l.starts_with("dice(2,")), "half-life decay uses dice(2, age)");
    assert!(a.iter().any(|l| l.starts_with("one_in(")));
}

#[test]
fn newborn_fields_only_age() {
    let Some(t) = types() else { return };
    let mut w = World::new("t_dirt");
    w.region.place(&t, World::at(12, 12), t.fd.fire, 3, 0);
    let mut eng = BnDice::new(SplitMix(1));
    let mut log = LoggingDice::new(&mut eng);
    w.turn(&t, &mut log);
    // Age 0 → effects skipped; aging makes age 1, then dice(2, 1) vs half-life.
    assert_eq!(log.log, vec!["dice(2,1)=2".to_string()]);
    let e = w.region.entries();
    assert_eq!(e.len(), 1);
    assert_eq!((e[0].2, e[0].3), (3, 1));
}

#[test]
fn bees_spread_to_every_free_neighbour() {
    // BN quirk kept: the loop goes on after cur.set_field_intensity(0).
    let Some(t) = types() else { return };
    let mut w = World::new("t_dirt");
    w.region.place(&t, World::at(18, 18), t.fd.bees, 3, 5);
    let mut eng = BnDice::new(SplitMix(3));
    w.turn(&t, &mut eng);
    let bees: Vec<_> = w.region.entries().into_iter().filter(|e| e.1 == t.fd.bees).collect();
    // The spent source is removed at the end of its own pass.
    assert_eq!(bees.len(), 8, "one swarm on each of the 8 neighbours");
    assert!(bees.iter().all(|e| e.0 != World::at(18, 18)));
    // First neighbour gets intensity 3; the rest get the clamped 1.
    let mut is: Vec<i32> = bees.iter().map(|e| e.2).collect();
    is.sort();
    assert_eq!(is, [1, 1, 1, 1, 1, 1, 1, 3]);
}

#[test]
fn shock_vent_bolts_cross_submaps() {
    let Some(t) = types() else { return };
    let mut w = World::new("t_dirt");
    w.region.place(&t, World::at(18, 18), t.fd.shock_vent, 1, 5);
    let mut eng = BnDice::new(SplitMix(11));
    w.turn(&t, &mut eng);
    let elec = w.region.entries().into_iter().filter(|e| e.1 == t.fd.electricity).count();
    assert!(elec > 0);
    for sm in w.region.row_major() {
        assert_eq!(w.region.submaps[&sm].fields.field_count, w.count_entries(sm));
        let cache = &w.region.submaps[&sm].fields.field_cache;
        let uniq: HashSet<_> = cache.iter().collect();
        assert_eq!(uniq.len(), cache.len(), "cache deduplicated after its own pass");
    }
}

#[test]
fn player_in_fire_crosses_to_combat() {
    let Some(t) = types() else { return };
    let mut w = World::new("t_dirt");
    let p = World::at(15, 15);
    w.region.place(&t, p, t.fd.fire, 2, 10);
    let mut combat = RecordingCombat::default();
    let u = combat.add(1, StubCreature { pos: p, avatar: true, ..Default::default() });
    let map = StubBubbleMap::default();
    let mut eng = BnDice::new(SplitMix(5));
    let mut ctx = MapCtx {
        types: &t,
        store: &mut w.region,
        caches: &mut w.caches,
        map: &map,
        combat: &mut combat,
        dice: &mut eng,
        player: Some(u),
    };
    creature_in_field(&mut ctx, u);
    let dealt: Vec<_> = combat
        .calls
        .iter()
        .filter_map(|c| match c {
            CombatCall::DealDamage { bp, dt: DamageType::Heat, .. } => Some(bp.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(dealt, ["torso", "foot_l", "foot_r", "leg_l", "leg_r"]);
}

#[test]
fn monster_in_toxic_gas_takes_one_hit() {
    let Some(t) = types() else { return };
    let mut w = World::new("t_dirt");
    let p = World::at(15, 15);
    w.region.place(&t, p, t.fd.toxic_gas, 3, 10);
    let mut combat = RecordingCombat::default();
    let z = combat.add(2, StubCreature { pos: p, monster: true, ..Default::default() });
    let map = StubBubbleMap::default();
    let mut eng = BnDice::new(SplitMix(5));
    let mut ctx =
        MapCtx { types: &t, store: &mut w.region, caches: &mut w.caches, map: &map, combat: &mut combat, dice: &mut eng, player: None };
    creature_in_field(&mut ctx, z);
    assert!(combat.calls.contains(&CombatCall::ApplyDamage { c: z, bp: "torso".into(), amount: 3 }));
    assert!(combat.calls.contains(&CombatCall::ModMoves { c: z, n: -3 }));
}

#[test]
fn propagate_field_spends_amount() {
    let Some(t) = types() else { return };
    let mut w = World::new("t_dirt");
    let mut combat = RecordingCombat::default();
    let map = StubBubbleMap::default();
    let mut eng = BnDice::new(SplitMix(9));
    let center = World::at(18, 18);
    let mut ctx =
        MapCtx { types: &t, store: &mut w.region, caches: &mut w.caches, map: &map, combat: &mut combat, dice: &mut eng, player: None };
    propagate_field(&mut ctx, center, t.fd.smoke, 15, 3);
    let total: i32 = w.region.entries().iter().filter(|e| e.1 == t.fd.smoke).map(|e| e.2).sum();
    assert_eq!(total, 15);
    assert_eq!(w.region.entries().iter().find(|e| e.0 == center).unwrap().2, 3);
}

#[test]
fn batch_decay_is_opt_in() {
    let Some(t) = types() else { return };
    let mut w = World::new("t_dirt");
    let p = World::at(15, 15);
    w.region.place(&t, p, t.fd.smoke, 3, 0);
    let (sm, _) = p.split();
    let hl = t.get(t.fd.smoke).half_life;
    batch_turns_field(BatchDecay::Off, &t, &mut w.region, sm, hl * 2);
    assert_eq!(w.region.entries()[0].2, 3);
    batch_turns_field(BatchDecay::Approximate, &t, &mut w.region, sm, hl * 2 + 1);
    let e = w.region.entries();
    assert_eq!((e[0].2, e[0].3), (1, 1));
    batch_turns_field(BatchDecay::Approximate, &t, &mut w.region, sm, hl);
    assert!(w.region.entries().is_empty());
    assert_eq!(w.region.submaps[&sm].fields.field_count, 0);
}

#[test]
fn heap_tie_order_matches_libstdcxx() {
    // Expected order printed by clang++/libstdc++ for the same pushes and pops
    // (std::priority_queue<pair<float,int>, vector, a.first > b.first>).
    let d = [0., 1., 1., 1., 1., 1., 1., 1., 1., 2., 2., 2., 1., 2., 2., 2., 3., 3., 2., 1., 0., 2., 2.];
    let mut q = StdPriorityQueue::new(|a: &(f32, i32), b: &(f32, i32)| a.0 > b.0);
    let mut out = Vec::new();
    for (i, v) in d.iter().enumerate() {
        q.push((*v, i as i32));
        if i % 5 == 4 {
            out.push(q.top().1);
            q.pop();
        }
    }
    while !q.is_empty() {
        out.push(q.top().1);
        q.pop();
    }
    assert_eq!(out, [0, 2, 4, 7, 20, 19, 6, 5, 12, 8, 1, 3, 9, 22, 14, 15, 11, 13, 21, 18, 10, 16, 17]);
}
