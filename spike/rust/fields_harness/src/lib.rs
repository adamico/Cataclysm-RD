//! The Fields part of the oracle (issue #14, session 3), Rust side.
//!
//! * [`layout`]: the recorder's layout files, read the same way.
//! * [`case`]: `bn-draws/1` case files (draws, call/ret marks, defs, tile states).
//! * [`replay`]: [`replay::ReplayDice`], a [`bn_dice::Dice`] that answers from a
//!   case's `d == 0` draws and fails on any mismatch in function, arguments or count.
//! * [`world`]: a region built from a layout, run turn by turn with the
//!   harness conventions of `DUMP_FORMAT.md`.
//! * [`creature`]: creature cases (`map::creature_in_field`), with a
//!   Combat port answered from the case.
//! * [`dump`]: the Observable dump `bn-fields-dump/1`, byte-compatible with the
//!   recorder's.

pub mod case;
pub mod creature;
pub mod dump;
pub mod layout;
pub mod replay;
pub mod world;

/// BN checkout for `field_type.json` (`$BN_ROOT`, else the session's Baseline clone).
pub const DEFAULT_BN: &str =
    "/tmp/claude-1000/-home-kc00l-Work-Cataclysm-RD/e4e51f43-d7b5-4322-acca-6f43fcb7cfaf/scratchpad/bn";

pub fn bn_root() -> Option<std::path::PathBuf> {
    let root = std::path::PathBuf::from(std::env::var("BN_ROOT").unwrap_or_else(|_| DEFAULT_BN.to_string()));
    root.join("data/json/field_type.json").exists().then_some(root)
}

/// `spike/` (this crate sits at `spike/rust/fields_harness`).
pub fn spike_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}
