//! A region built from a layout and run with the harness conventions of
//! `DUMP_FORMAT.md` (origin `(200, 200)`, z then row-major submap order,
//! one calendar turn per tick, start at `turn_zero + clock`).

use crate::case::{Def, ItemDef, TileState};
use crate::layout::Layout;
use bubble_stub::{Region, StubCaches, StubItem, StubItems, StubMapData, TerDef};
use fields::ports::{Clock, FurnId, MapFlag, SubmapStore, TerId};
use fields::storage::{Local, SmPos, Tripoint, SEEX, SEEY};
use fields::{process_fields_in_submap, FieldTypes, SubmapCtx};

pub const ORIGIN: SmPos = SmPos { x: 200, y: 200, z: 0 };

fn flag(s: &str) -> MapFlag {
    use MapFlag::*;
    match s {
        "FLAMMABLE" => Flammable,
        "FLAMMABLE_ASH" => FlammableAsh,
        "FLAMMABLE_HARD" => FlammableHard,
        "FIRE_CONTAINER" => FireContainer,
        "NO_FLOOR" => NoFloor,
        "SEALED" => Sealed,
        "ALLOW_FIELD_EFFECT" => AllowFieldEffect,
        "SWIMMABLE" => Swimmable,
        "PERMEABLE" => Permeable,
        "FUNGUS" => Fungus,
        _ => panic!("unknown flag {s}"),
    }
}

/// The terrain/furniture catalogue for one case, from BN's own definitions
/// (`ter_def` / `furn_def` marks), so the Rust stub can't drift from BN.
pub fn map_data_from_defs(ter: &[Def], furn: &[Def]) -> StubMapData {
    let conv = |d: &Def| TerDef {
        id: Box::leak(d.id.clone().into_boxed_str()),
        movecost: d.movecost,
        flags: d.flags.iter().map(|f| flag(f)).collect(),
    };
    // t_null / f_null first, so the zero ids mean what they mean in BN.
    let mut t: Vec<TerDef> = ter.iter().map(conv).collect();
    t.sort_by_key(|d| d.id != "t_null");
    let mut f: Vec<TerDef> = furn.iter().map(conv).collect();
    f.sort_by_key(|d| d.id != "f_null");
    StubMapData { ter: t, furn: f }
}

pub struct World<'t> {
    pub types: &'t FieldTypes,
    pub layout: Layout,
    pub region: Region,
    pub md: StubMapData,
    pub items: StubItems,
    pub caches: StubCaches,
    /// `to_turn<int>(calendar::turn)` for the next turn to process.
    pub turn: i32,
    /// Turns processed so far.
    pub done: i32,
}

impl<'t> World<'t> {
    pub fn at(&self, x: i32, y: i32, z: i32) -> Tripoint {
        Tripoint::new(ORIGIN.x * SEEX + x, ORIGIN.y * SEEY + y, z)
    }

    pub fn new(types: &'t FieldTypes, layout: &Layout, ter: &[Def], furn: &[Def], items: &[ItemDef]) -> World<'t> {
        let md = map_data_from_defs(ter, furn);
        let def = md.ter_id(&layout.default_ter);
        let mut region = Region::default();
        for z in layout.zlo..=layout.zhi {
            let r = Region::block(SmPos { z, ..ORIGIN }, layout.w, layout.h, def);
            region.submaps.extend(r.submaps);
        }
        let mut w = World {
            types,
            layout: layout.clone(),
            region,
            md,
            items: StubItems::default(),
            caches: StubCaches::default(),
            turn: layout.clock,
            done: 0,
        };
        for c in &layout.ters {
            let p = w.at(c.x, c.y, c.z);
            let t = w.md.ter_id(&c.id);
            w.region.set_ter_at(p, t);
        }
        for c in &layout.furns {
            let p = w.at(c.x, c.y, c.z);
            let f = w.md.furn_id(&c.id);
            w.region.set_furn_at(p, f);
        }
        for &(x, y, z, rad) in &layout.rads {
            let (sm, l) = w.at(x, y, z).split();
            w.region.set_radiation(sm, l, rad);
        }
        assert_eq!(items.len(), layout.items.len(), "item_def marks vs layout items");
        for (n, d) in items.iter().enumerate() {
            let p = w.at(d.x, d.y, d.z);
            w.items.add(
                p,
                StubItem { id: n as u64 + 1, rock: d.rock, flammable: d.flammable, weight_g: d.weight_g, birthday: layout.clock },
            );
        }
        for f in &layout.fields {
            let ty = types.id(&f.id).unwrap_or_else(|| panic!("unknown field {}", f.id));
            let p = w.at(f.x, f.y, f.z);
            w.region.place(types, p, ty, f.intensity, f.age);
        }
        w
    }

    /// One harness turn: every submap in (z, y, x) order, skipping
    /// `field_count == 0`, then `calendar::turn += 1`.
    pub fn run_turn(&mut self, dice: &mut dyn bn_dice::Dice) {
        self.items.now = self.turn;
        for pos in self.region.row_major() {
            let mut ctx = SubmapCtx {
                types: self.types,
                store: &mut self.region,
                map_data: &self.md,
                items: &mut self.items,
                caches: &mut self.caches,
                clock: Clock { turn: self.turn, turns_this_tick: 1 },
                dice: &mut *dice,
            };
            process_fields_in_submap(&mut ctx, pos);
        }
        self.turn += 1;
        self.done += 1;
    }

    /// Region tile coordinates of an absolute tile.
    pub fn rel(&self, p: Tripoint) -> (i32, i32, i32) {
        (p.x - ORIGIN.x * SEEX, p.y - ORIGIN.y * SEEY, p.z)
    }

    /// Non-field state of every tile that differs from a default tile,
    /// sorted (the `tile` marks of a case).
    pub fn tiles(&self) -> Vec<TileState> {
        let def = self.md.ter_id(&self.layout.default_ter);
        let fnull = self.md.furn_id("f_null");
        let mut out = vec![];
        for (sm, slot) in &self.region.submaps {
            for x in 0..SEEX {
                for y in 0..SEEY {
                    let l = Local::new(x, y);
                    let t: TerId = slot.ter[x as usize][y as usize];
                    let f: FurnId = slot.furn[x as usize][y as usize];
                    let rad = slot.rad[x as usize][y as usize];
                    let at = fields::storage::SubTile { sm: *sm, local: l };
                    let n = self.items.tiles.get(&at).map_or(0, Vec::len);
                    if t == def && f == fnull && rad == 0 && n == 0 {
                        continue;
                    }
                    let (rx, ry, rz) = self.rel(Tripoint::from_parts(*sm, l));
                    out.push(TileState {
                        x: rx,
                        y: ry,
                        z: rz,
                        ter: self.md.ter[t.0 as usize].id.to_string(),
                        furn: self.md.furn[f.0 as usize].id.to_string(),
                        rad,
                        items: n,
                    });
                }
            }
        }
        out.sort();
        out
    }
}
