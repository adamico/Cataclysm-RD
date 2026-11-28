use std::fmt;
use std::io;
use std::path::PathBuf;

/// Why a check could not run to the end.
#[derive(Debug)]
pub enum Error {
    /// A file or directory could not be read.
    Read(PathBuf, io::Error),
    /// A Rust source file is not valid Rust.
    Parse(PathBuf, syn::Error),
    /// `cargo metadata` failed on the workspace.
    Metadata(cargo_metadata::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(path, error) => write!(formatter, "{}: {error}", path.display()),
            Self::Parse(path, error) => write!(formatter, "{}: {error}", path.display()),
            Self::Metadata(error) => write!(formatter, "cargo metadata: {error}"),
        }
    }
}

impl std::error::Error for Error {}
