//! Fields, rewritten from BN v0.13.0 (`e7b4976`) rule by rule.
//!
//! Spike for issue #14, session 2. Behavioural spec: `src/map/map_field.cpp`,
//! `field.*`, `field_type.*`, `submap_fields.h`, `src/batch_turns.cpp`.
//! Every rule that draws takes a [`bn_dice::Dice`] at helper level (ADR 0004).
//!
//! | module | BN |
//! |---|---|
//! | [`content`] | `field_type.{h,cpp}`, JSON loading |
//! | [`field`] | `field.{h,cpp}` |
//! | [`storage`] | field members of `submap` (the untangled Tangle) |
//! | [`ports`] | everything Fields asks of other contexts |
//! | [`process`] | `process_fields_in_submap` + anon-namespace helpers |
//! | [`map_rules`] | `map::create_hot_air`, `emit_field`, `propagate_field`, `get_wind_blockers` |
//! | [`creatures`] | `map::player_in_field`, `monster_in_field`, `creature_in_field`, `burn_body_part` |
//! | [`batch`] | `batch_turns_field` (Approximation, opt-in) |
//! | [`heap`] | libstdc++ `priority_queue` order, for `propagate_field` |

pub mod batch;
pub mod content;
pub mod creatures;
pub mod field;
pub mod heap;
pub mod map_rules;
pub mod ports;
pub mod process;
pub mod storage;

pub use content::{FieldTypeId, FieldTypes};
pub use process::{process_fields_in_submap, SubmapCtx};
