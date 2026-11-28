//! The Observable dump `bn-fields-dump/1` (`spike/recorder/DUMP_FORMAT.md`),
//! written byte for byte like `write_dump` in `fields_recorder.cpp`.

use crate::world::World;
use std::fmt::Write;

/// One snapshot: a `dump` line and its `f` lines.
pub fn snapshot(w: &World<'_>, seed: u64, turn: i32) -> String {
    let multi_z = w.layout.nz() > 1;
    let mut rows: Vec<(i32, i32, i32, &str, i32, i32, bool)> = w
        .region
        .entries()
        .into_iter()
        .map(|(p, ty, n, age, alive)| {
            let (x, y, z) = w.rel(p);
            (z, y, x, w.types.str_id(ty), n, age, alive)
        })
        .collect();
    // (z, y, x, t) with t compared as bytes (std::string operator<).
    rows.sort_by(|a, b| (a.0, a.1, a.2, a.3.as_bytes()).cmp(&(b.0, b.1, b.2, b.3.as_bytes())));
    let mut out = String::new();
    writeln!(
        out,
        "{{\"k\":\"dump\",\"format\":\"bn-fields-dump/1\",\"seed\":{seed},\"turn\":{turn},\"region\":[{},{}],\"entries\":{}}}",
        w.layout.w,
        w.layout.h,
        rows.len()
    )
    .unwrap();
    for (z, y, x, t, n, age, alive) in rows {
        let zs = if multi_z { format!(",\"z\":{z}") } else { String::new() };
        writeln!(out, "{{\"k\":\"f\",\"x\":{x},\"y\":{y}{zs},\"t\":\"{t}\",\"n\":{n},\"age\":{age},\"alive\":{alive}}}").unwrap();
    }
    out
}

/// Split a dump file into its snapshots (each a `dump` line + its `f` lines).
pub fn split_snapshots(text: &str) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for line in text.lines() {
        if line.starts_with("{\"k\":\"dump\"") {
            out.push(String::new());
        }
        let cur = out.last_mut().expect("dump file starts with a dump line");
        cur.push_str(line);
        cur.push('\n');
    }
    out
}
