//! Rusted Dawn's own checks on the Engine's code: no BN provenance in docs, every Quirk recorded in an ADR, and public names drawn from the glossaries.

mod bn_provenance;
mod error;
mod finding;
mod quirks;
mod source;
mod vocabulary;
mod words;
mod workspace;

pub use bn_provenance::{BnSymbols, check_bn_provenance};
pub use error::Error;
pub use finding::{Check, Finding};
pub use quirks::check_quirks;
pub use vocabulary::{Vocabulary, check_vocabulary};
pub use workspace::{Crate, Workspace};
