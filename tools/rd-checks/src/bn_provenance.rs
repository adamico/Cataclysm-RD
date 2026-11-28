use std::collections::HashSet;

use crate::error::Error;
use crate::finding::{Check, Finding};
use crate::source::{self, DocText};
use crate::workspace::Workspace;

const RUST_PATH_ROOTS: [&str; 5] = ["crate", "self", "super", "Self", "core"];

/// The names the Baseline's C++ defines, generated from its sources.
#[derive(Clone, Debug, Default)]
pub struct BnSymbols {
    qualifiers: HashSet<String>,
    names: HashSet<String>,
}

impl BnSymbols {
    /// Reads the symbol list: one `qualifier <name>` or `name <name>` per line.
    /// A qualifier is a class, struct, namespace or enum; a name is any symbol with an underscore.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let mut symbols = Self::default();
        for line in text.lines() {
            match line.split_once(' ') {
                Some(("qualifier", name)) => {
                    symbols.qualifiers.insert(name.to_owned());
                }
                Some(("name", name)) => {
                    symbols.names.insert(name.to_owned());
                }
                _ => {}
            }
        }
        symbols
    }
}

/// Finds doc comments that trace code back to BN: C++ file names, `std::`, `operator`,
/// or a BN symbol in backticks that the workspace does not itself define.
///
/// # Errors
///
/// When a source file can't be read or parsed.
pub fn check_bn_provenance(
    workspace: &Workspace,
    symbols: &BnSymbols,
) -> Result<Vec<Finding>, Error> {
    let files = workspace
        .crates
        .iter()
        .map(|member| source::parse_tree(&member.src()))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    let mut rust_names: HashSet<String> = files
        .iter()
        .flat_map(|file| source::identifiers(&file.syntax))
        .collect();
    rust_names.extend(
        workspace
            .crates
            .iter()
            .map(|member| member.name.replace('-', "_")),
    );
    let mut findings = Vec::new();
    for file in &files {
        for doc in source::doc_texts(&file.syntax) {
            findings.extend(
                provenance_in(&doc, symbols, &rust_names)
                    .into_iter()
                    .map(|message| Finding {
                        path: file.path.clone(),
                        line: doc.line,
                        check: Check::BnProvenance,
                        message,
                    }),
            );
        }
    }
    Ok(findings)
}

fn provenance_in(doc: &DocText, symbols: &BnSymbols, rust_names: &HashSet<String>) -> Vec<String> {
    let mut messages = Vec::new();
    for word in doc
        .text
        .split(|character: char| character.is_whitespace() || character == '`')
    {
        let word = word.trim_end_matches(|character: char| !is_ident_char(character));
        if names_cpp_file(word) {
            messages.push(format!("names the C++ file `{word}`"));
        }
    }
    if doc.text.contains("std::") {
        messages.push("names C++'s `std::`".to_owned());
    }
    for code in doc.text.split('`').skip(1).step_by(2) {
        if code
            .split(|character: char| !is_ident_char(character))
            .any(|word| word == "operator")
        {
            messages.push(format!("names a C++ operator in `{code}`"));
        }
        for path in paths_in(code) {
            if let Some(symbol) = bn_symbol(&path, symbols, rust_names) {
                messages.push(format!("names the BN symbol `{symbol}`"));
            }
        }
    }
    messages
}

fn names_cpp_file(word: &str) -> bool {
    [".cpp", ".h", ".hpp"].iter().any(|extension| {
        word.strip_suffix(extension)
            .is_some_and(|stem| stem.chars().last().is_some_and(is_ident_char))
    })
}

fn bn_symbol(path: &[&str], symbols: &BnSymbols, rust_names: &HashSet<String>) -> Option<String> {
    let first = path.first()?;
    if path.len() > 1 {
        let qualifies = symbols.qualifiers.contains(*first)
            && !rust_names.contains(*first)
            && !RUST_PATH_ROOTS.contains(first);
        return qualifies.then(|| path.join("::"));
    }
    (first.contains('_') && symbols.names.contains(*first) && !rust_names.contains(*first))
        .then(|| (*first).to_owned())
}

fn paths_in(code: &str) -> Vec<Vec<&str>> {
    code.split(|character: char| !is_ident_char(character) && character != ':')
        .filter(|token| !token.is_empty())
        .map(|token| {
            token
                .split("::")
                .filter(|segment| !segment.is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|segments| {
            segments.first().is_some_and(|segment| {
                segment
                    .chars()
                    .next()
                    .is_some_and(|start| start.is_alphabetic() || start == '_')
            })
        })
        .collect()
}

fn is_ident_char(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}
