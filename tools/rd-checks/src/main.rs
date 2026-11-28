//! Runs Rusted Dawn's own checks on a workspace and prints each finding.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use rd_checks::{
    BnSymbols, Error, Finding, Vocabulary, Workspace, check_bn_provenance, check_quirks,
    check_vocabulary,
};

const USAGE: &str = "usage: rd-checks [--workspace DIR] [--repo DIR]";

struct Paths {
    workspace: PathBuf,
    repo: PathBuf,
}

impl Paths {
    fn from_args(mut args: impl Iterator<Item = String>) -> Option<Self> {
        let mut paths = Self {
            workspace: PathBuf::from("."),
            repo: PathBuf::from("."),
        };
        while let Some(flag) = args.next() {
            let value = PathBuf::from(args.next()?);
            match flag.as_str() {
                "--workspace" => paths.workspace = value,
                "--repo" => paths.repo = value,
                _ => return None,
            }
        }
        Some(paths)
    }
}

fn main() -> ExitCode {
    let Some(paths) = Paths::from_args(std::env::args().skip(1)) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    match run(&paths) {
        Ok(findings) if findings.is_empty() => ExitCode::SUCCESS,
        Ok(findings) => {
            for finding in &findings {
                println!("{finding}");
            }
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("rd-checks: {error}");
            ExitCode::from(2)
        }
    }
}

fn run(paths: &Paths) -> Result<Vec<Finding>, Error> {
    let workspace = Workspace::load(&paths.workspace)?;
    let repo = &paths.repo;
    let symbols = BnSymbols::parse(&read(&repo.join("tools/rd-checks/bn-symbols.txt"))?);
    let glossaries = [
        read(&repo.join("GLOSSARY.md"))?,
        read(&repo.join("docs/game/GLOSSARY.md"))?,
    ];
    let vocabulary =
        Vocabulary::from_sources(&glossaries, &read(&repo.join("tools/vocabulary.txt"))?);
    let mut findings = check_bn_provenance(&workspace, &symbols)?;
    findings.extend(check_quirks(&workspace, &repo.join("docs/adr"))?);
    findings.extend(check_vocabulary(&workspace, &vocabulary)?);
    Ok(findings)
}

fn read(path: &Path) -> Result<String, Error> {
    fs::read_to_string(path).map_err(|error| Error::Read(path.to_path_buf(), error))
}
