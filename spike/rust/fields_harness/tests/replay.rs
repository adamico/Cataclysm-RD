//! Replay tests: one per Fields rule (issue #14, session 3, ADR 0004).
//!
//! Each test replays the recorded cases of one layout in `spike/cases/`
//! (8 seeds × 5 turns). The layouts put one field entry (or a few, far apart)
//! with the neighbours a rule reads; turn 1 is the replay unit "one entry, one
//! tile, one turn", later turns replay the state that grew out of it. Per
//! turn the test checks:
//! 1. the draws: function, arguments and count, via `ReplayDice`;
//! 2. the Observable dump, byte for byte against the recorder's snapshot;
//! 3. the non-field tile state (terrain, furniture, radiation, item counts).

use fields::FieldTypes;
use fields_harness::case::Case;
use fields_harness::dump::{snapshot, split_snapshots};
use fields_harness::layout::Layout;
use fields_harness::replay::ReplayDice;
use fields_harness::world::World;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;
use std::sync::OnceLock;

fn types() -> Option<&'static FieldTypes> {
    static T: OnceLock<Option<FieldTypes>> = OnceLock::new();
    T.get_or_init(|| fields_harness::bn_root().map(|r| FieldTypes::load_bn(&r).expect("field types")))
        .as_ref()
}

/// Creature cases: one `map::creature_in_field` call per turn, Combat
/// answered from the case. Checks the draws, every wrapped Combat call and its
/// arguments, the fields on the area (byte-exact dump), the monster's moves,
/// and its hp when no `deal_damage` (armour is Combat's) was involved.
fn replay_creature(types: &FieldTypes, layout: &Layout, case: &Case, snaps: &[String]) {
    use bubble_stub::StubBubbleMap;
    use combat_stub::RecordingCombat;
    use fields::creatures::creature_in_field;
    use fields::map_rules::MapCtx;
    use fields_harness::creature::{stub_from_def, ReplayCombat, SharedDice};
    use std::cell::RefCell;
    use std::rc::Rc;

    // The creature rules read no terrain: a minimal catalogue will do.
    use fields_harness::case::Def;
    let d = |id: &str| Def { id: id.into(), movecost: 2, flags: vec![] };
    let mut w = World::new(types, layout, &[d("t_null"), d("t_dirt"), d("t_pit")], &[d("f_null"), d("f_ash")], &[]);
    let shared = Rc::new(RefCell::new(ReplayDice::new(case)));
    let m = &layout.mons[0];
    let pos = w.at(m.x, m.y, 0);
    let mut combat = ReplayCombat { inner: RecordingCombat::default(), dice: shared.clone(), types, moves: 0, applied: 0, dealt: false, origin: w.at(0, 0, 0) };
    let z = combat.inner.add(1, stub_from_def(case.creature_def.as_ref().unwrap(), pos));
    let map = StubBubbleMap::default();
    // A damaged monster bleeds (fd_blood*, fd_gibs*) through monster code on
    // the Combat side, which the stub does not model: such entries that were
    // not in the layout are left out of BN's snapshot before comparing.
    let initial: Vec<String> = layout.fields.iter().map(|f| f.id.clone()).collect();
    let combat_made = |line: &str| {
        ["\"t\":\"fd_blood", "\"t\":\"fd_gibs"].iter().any(|p| line.contains(p))
            && !initial.iter().any(|id| line.contains(&format!("\"t\":\"{id}\"")))
    };
    let check = |w: &World<'_>, t: i32| {
        let mine = snapshot(w, case.seed, t);
        let lines: Vec<&str> = snaps[t as usize].lines().skip(1).filter(|l| !combat_made(l)).collect();
        let want_body: String = lines.iter().map(|l| format!("{l}\n")).collect();
        let got_body: String = mine.lines().skip(1).map(|l| format!("{l}\n")).collect();
        if got_body != want_body {
            panic!("dump differs after turn {t}\n--- BN\n{}--- Rust\n{mine}", snaps[t as usize]);
        }
    };
    check(&w, 0);
    let hp0 = case.creature[0].0;
    for t in 1..=case.turns {
        shared.borrow_mut().begin_turn(t);
        // The recorder skips the call once the monster is dead.
        if !case.creature[t as usize - 1].2 {
            let mut sd = SharedDice(shared.clone());
            let mut ctx = MapCtx {
                types,
                store: &mut w.region,
                caches: &mut w.caches,
                map: &map,
                combat: &mut combat,
                dice: &mut sd,
                player: None,
            };
            creature_in_field(&mut ctx, z);
        }
        shared.borrow_mut().end_turn(t);
        check(&w, t);
        let (hp, moves, _) = case.creature[t as usize];
        assert_eq!(combat.moves, moves, "monster moves after turn {t} (BN {moves}, Rust {})", combat.moves);
        if !combat.dealt {
            assert_eq!(hp0 - hp, combat.applied, "monster hp lost after turn {t} (BN {}, Rust {})", hp0 - hp, combat.applied);
        }
    }
    assert!(shared.borrow().finished(), "case has events after the last turn");
}

/// Replay one recorded seed; returns the first mismatch.
fn replay_seed(types: &FieldTypes, layout: &Layout, case_path: &Path) -> Result<(), String> {
    let case = Case::load(case_path);
    let dump_path = case_path.to_string_lossy().replace(".case.jsonl", ".dump.jsonl");
    let snaps = split_snapshots(&std::fs::read_to_string(&dump_path).unwrap());
    assert_eq!(snaps.len() as i32, case.turns + 1, "{dump_path}: one snapshot per turn (--dump-every)");
    assert_eq!(case.bypass_steps, 0, "case has engine bypasses: not replayable");
    let run = catch_unwind(AssertUnwindSafe(|| {
        if case.is_creature_case() {
            return replay_creature(types, layout, &case, &snaps);
        }
        let mut w = World::new(types, layout, &case.ter_defs, &case.furn_defs, &case.item_defs);
        let mut dice = ReplayDice::new(&case);
        let check = |w: &World<'_>, t: i32| {
            let mine = snapshot(w, case.seed, t);
            if mine != snaps[t as usize] {
                panic!("dump differs after turn {t}\n--- BN\n{}--- Rust\n{mine}", snaps[t as usize]);
            }
            let mut want = case.tiles[t as usize].clone();
            want.sort();
            let got = w.tiles();
            if got != want {
                panic!("tile state differs after turn {t}\n BN:   {want:?}\n Rust: {got:?}");
            }
        };
        check(&w, 0);
        for t in 1..=case.turns {
            dice.begin_turn(t);
            w.run_turn(&mut dice);
            dice.end_turn(t);
            check(&w, t);
        }
        assert!(dice.finished(), "case has events after the last turn");
    }));
    run.map_err(|e| {
        let msg = e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()));
        format!("{}: {}", case_path.file_name().unwrap().to_string_lossy(), msg.unwrap_or_default())
    })
}

fn replay_rule(name: &str) {
    let Some(types) = types() else {
        eprintln!("skipping: no BN checkout (set BN_ROOT)");
        return;
    };
    let dir = fields_harness::spike_dir().join("cases");
    let layout = Layout::load(&dir.join("layouts").join(format!("{name}.layout")));
    // $REPLAY_CASES points at another recorded set (e.g. the 64-seed run).
    let root = std::env::var("REPLAY_CASES").map(std::path::PathBuf::from).unwrap_or_else(|_| dir.join("recorded"));
    let mut cases: Vec<_> = std::fs::read_dir(root.join(name))
        .unwrap_or_else(|e| panic!("no recorded cases for {name}: {e}"))
        .map(|e| e.unwrap().path())
        .filter(|p| p.to_string_lossy().ends_with(".case.jsonl"))
        .collect();
    cases.sort();
    assert!(!cases.is_empty());
    let failures: Vec<String> = cases.iter().filter_map(|c| replay_seed(types, &layout, c).err()).collect();
    if !failures.is_empty() {
        panic!("{name}: {}/{} seeds fail\n{}", failures.len(), cases.len(), failures.join("\n\n"));
    }
}

macro_rules! replay_tests {
    ($($name:ident),* $(,)?) => { $( #[test] fn $name() { replay_rule(stringify!($name)); } )* };
}

replay_tests!(
    smoke,
    fire_grass,
    fire_wall,
    fire_furn,
    fire_items,
    fire_container,
    fire_pit,
    fire_water,
    fire_growth,
    fire_web,
    fire_zrise,
    fire_zfall,
    fire_edge,
    fire_ash_ter,
    fire_items_spread,
    gas_spread,
    gas_fall,
    gas_edge,
    fungicidal,
    electricity,
    vents,
    incendiary,
    gas_remover,
    push_items,
    bees,
    radiation,
    wandering,
    upgrade,
    decay,
    // creature cases: map::creature_in_field -> monster_in_field (monsters only)
    creature_fire,
    creature_fire3,
    creature_fire1_veggy,
    creature_acid,
    creature_gases,
    creature_gases_veggy,
    creature_elec,
    creature_sticky,
    creature_misc,
    creature_bug,
    creature_gases_breather,
    creature_gases_low,
    creature_fire3_walker,
    creature_incendiary,
    creature_incendiary3,
    creature_fungal,
    creature_spider,
);

/// Deliverable 3: the Rust Observable dump is byte-identical to the
/// recorder's on the smoke case (session 1's `samples/smoke-seed-42.dump.jsonl`,
/// turns 0 and 20), with the draws given by the case.
#[test]
fn smoke_dump_is_byte_exact() {
    let Some(types) = types() else { return };
    let spike = fields_harness::spike_dir();
    let layout = Layout::load(&spike.join("soak/layouts/smoke.layout"));
    let case = Case::load(&spike.join("cases/recorded/smoke/seed-42.case.jsonl"));
    let mut w = World::new(types, &layout, &case.ter_defs, &case.furn_defs, &case.item_defs);
    let mut out = snapshot(&w, 42, 0);
    let mut dice = ReplayDice::new(&case);
    for t in 1..=20 {
        dice.begin_turn(t);
        w.run_turn(&mut dice);
        dice.end_turn(t);
    }
    out += &snapshot(&w, 42, 20);
    let want = std::fs::read_to_string(spike.join("samples/smoke-seed-42.dump.jsonl")).unwrap();
    assert!(out == want, "Rust dump differs from the recorder's\n--- BN\n{want}--- Rust\n{out}");
}

/// Whole-run replay of a soak case (100 turns, ~250k draws), when one is
/// given: `FIELDS_SOAK_CASE=<dir>/seed-1.case.jsonl FIELDS_SOAK_LAYOUT=<layout>`.
/// The dump must hold every turn (`run_soak.sh ... "--dump-every --record"`).
/// Too big to commit, so skipped by default.
#[test]
fn soak_case_replays_whole_run() {
    let (Ok(case), Ok(layout)) = (std::env::var("FIELDS_SOAK_CASE"), std::env::var("FIELDS_SOAK_LAYOUT")) else {
        eprintln!("skipping: FIELDS_SOAK_CASE / FIELDS_SOAK_LAYOUT not set");
        return;
    };
    let Some(types) = types() else { return };
    let layout = Layout::load(Path::new(&layout));
    replay_seed(types, &layout, Path::new(&case)).unwrap_or_else(|e| panic!("{e}"));
}
