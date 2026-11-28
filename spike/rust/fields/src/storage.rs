//! Per-submap field storage, owned by Fields.
//!
//! **The untangled Tangle.** In BN, `submap` holds `fld[SEEX][SEEY]`,
//! `field_count` and `field_cache` next to terrain, furniture, items,
//! radiation and the floor cache, and Fields code reaches into all of them
//! through `submap&` and `mapbuffer&`. Here Fields owns [`SubmapFields`]
//! (exactly the three field members). The Reality bubble keeps one per loaded
//! submap and lends it out through [`crate::ports::SubmapStore`], which also
//! answers the non-field per-tile questions Fields asks (terrain, furniture,
//! radiation, floor cache).

use crate::field::Field;

pub const SEEX: i32 = 12;
pub const SEEY: i32 = 12;
pub const OVERMAP_DEPTH: i32 = 10;
pub const OVERMAP_HEIGHT: i32 = 10;

/// `tripoint_abs_sm`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct SmPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl SmPos {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
    pub fn offset(self, dx: i32, dy: i32, dz: i32) -> Self {
        Self::new(self.x + dx, self.y + dy, self.z + dz)
    }
}

/// `point_sm_ms`: a tile inside a submap, `0..SEEX` × `0..SEEY`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Local {
    pub x: i32,
    pub y: i32,
}

impl Local {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// Absolute map-square position (`tripoint_abs_ms`). The map-level rules
/// (`map::propagate_field`, `map::creature_in_field`, ...) work in these.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Tripoint {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Tripoint {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
    pub fn offset(self, dx: i32, dy: i32) -> Self {
        Self::new(self.x + dx, self.y + dy, self.z)
    }
    /// Split into (submap, local).
    pub fn split(self) -> (SmPos, Local) {
        (
            SmPos::new(self.x.div_euclid(SEEX), self.y.div_euclid(SEEY), self.z),
            Local::new(self.x.rem_euclid(SEEX), self.y.rem_euclid(SEEY)),
        )
    }
    pub fn from_parts(sm: SmPos, l: Local) -> Self {
        Self::new(sm.x * SEEX + l.x, sm.y * SEEY + l.y, sm.z)
    }
}

/// One tile of one submap (`SubTile` in `map_field.cpp`, minus the pointer).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SubTile {
    pub sm: SmPos,
    pub local: Local,
}

/// The field part of a `submap`: grid, `field_count`, `field_cache`.
#[derive(Clone, Debug)]
pub struct SubmapFields {
    fld: Vec<Field>,
    /// Number of entries (BN keeps this by hand; rules adjust it, and some BN
    /// paths do not, e.g. `fd_gas_remover`).
    pub field_count: i32,
    /// Positions of tiles with fields; may hold duplicates and dead tiles
    /// between compactions, exactly like BN.
    pub field_cache: Vec<Local>,
}

impl Default for SubmapFields {
    fn default() -> Self {
        Self { fld: vec![Field::default(); (SEEX * SEEY) as usize], field_count: 0, field_cache: Vec::new() }
    }
}

impl SubmapFields {
    fn idx(l: Local) -> usize {
        debug_assert!((0..SEEX).contains(&l.x) && (0..SEEY).contains(&l.y), "{l:?}");
        (l.x * SEEY + l.y) as usize
    }
    pub fn get(&self, l: Local) -> &Field {
        &self.fld[Self::idx(l)]
    }
    pub fn get_mut(&mut self, l: Local) -> &mut Field {
        &mut self.fld[Self::idx(l)]
    }
}
