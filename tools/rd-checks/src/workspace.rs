use std::path::{Path, PathBuf};

use cargo_metadata::MetadataCommand;

use crate::error::Error;

/// One crate of the workspace, with the layer it declares.
#[derive(Clone, Debug)]
pub struct Crate {
    /// The package name.
    pub name: String,
    /// `[package.metadata.layer] layer`, if declared.
    pub layer: Option<String>,
    /// The directory holding the crate's `src/`.
    pub root: PathBuf,
}

impl Crate {
    /// The crate's `src/` directory.
    #[must_use]
    pub fn src(&self) -> PathBuf {
        self.root.join("src")
    }
}

/// The crates of one Cargo workspace.
#[derive(Clone, Debug)]
pub struct Workspace {
    /// Its member crates.
    pub crates: Vec<Crate>,
}

impl Workspace {
    /// Reads the workspace whose `Cargo.toml` is in `manifest_dir`.
    ///
    /// # Errors
    ///
    /// When `cargo metadata` fails on it.
    pub fn load(manifest_dir: &Path) -> Result<Self, Error> {
        let metadata = MetadataCommand::new()
            .manifest_path(manifest_dir.join("Cargo.toml"))
            .no_deps()
            .exec()
            .map_err(Error::Metadata)?;
        let crates = metadata
            .workspace_packages()
            .into_iter()
            .map(|package| Crate {
                name: package.name.to_string(),
                layer: package
                    .metadata
                    .get("layer")
                    .and_then(|layer| layer.get("layer")?.as_str())
                    .map(str::to_owned),
                root: package
                    .manifest_path
                    .parent()
                    .map_or_else(PathBuf::new, |dir| dir.as_std_path().to_path_buf()),
            })
            .collect();
        Ok(Self { crates })
    }

    /// The crates that declare `layer`.
    pub fn in_layer<'a>(&'a self, layer: &'a str) -> impl Iterator<Item = &'a Crate> {
        self.crates
            .iter()
            .filter(move |member| member.layer.as_deref() == Some(layer))
    }
}
