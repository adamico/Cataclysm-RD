//! Fields-only soak run, Rust side (issue #14, session 3).
//!
//!   cargo run --release -p fields_harness --bin soak -- <layout> <defs-case> <seeds-file> <turns> <out-dir> [--dump-every]
//!
//! Same layout and run conventions as the recorder (`DUMP_FORMAT.md`), but the
//! dice are live: `BnDice` over SplitMix64 seeded from the soak seed, not a
//! replay and not BN's mt19937 stream. Writes `<out-dir>/seed-<s>.dump.jsonl`
//! in `bn-fields-dump/1`, so `spike/soak/summarise_dumps.py` and
//! `spike/soak/compare_soak.py` read both sides the same way.
//! `<defs-case>` is any recorded case of the same layout: the terrain,
//! furniture and item definitions come from BN through it.

use bn_dice::{BnDice, SplitMix};
use fields::FieldTypes;
use fields_harness::case::Case;
use fields_harness::dump::snapshot;
use fields_harness::layout::Layout;
use fields_harness::world::World;
use std::path::Path;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    if a.len() < 5 {
        eprintln!("usage: soak <layout> <defs-case> <seeds-file> <turns> <out-dir> [--dump-every]");
        std::process::exit(1);
    }
    let types = FieldTypes::load_bn(&fields_harness::bn_root().expect("BN_ROOT")).unwrap();
    let layout = Layout::load(Path::new(&a[0]));
    let defs = Case::load(Path::new(&a[1]));
    let seeds: Vec<u64> = std::fs::read_to_string(&a[2])
        .unwrap()
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| l.trim().parse().unwrap())
        .collect();
    let turns: i32 = a[3].parse().unwrap();
    let out = Path::new(&a[4]);
    let every = a.iter().any(|s| s == "--dump-every");
    std::fs::create_dir_all(out).unwrap();
    for seed in seeds {
        let mut w = World::new(&types, &layout, &defs.ter_defs, &defs.furn_defs, &defs.item_defs);
        // Seed mixing so nearby soak seeds give unrelated streams.
        let mut dice = BnDice::new(SplitMix(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03));
        let mut text = snapshot(&w, seed, 0);
        for t in 1..=turns {
            w.run_turn(&mut dice);
            if t == turns || every {
                text += &snapshot(&w, seed, t);
            }
        }
        std::fs::write(out.join(format!("seed-{seed}.dump.jsonl")), text).unwrap();
    }
    println!("rust soak: {} turns -> {}", turns, out.display());
}
