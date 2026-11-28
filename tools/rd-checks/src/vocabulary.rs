use std::collections::HashSet;

use syn::visit::{self, Visit};

use crate::error::Error;
use crate::finding::{Check, Finding};
use crate::source::{self, line_of};
use crate::words::{split_camel_case, word_forms};
use crate::workspace::Workspace;

const CONTEXT_LAYER: &str = "context";

/// The words a public name in a context crate may use: every word of every glossary term, plus an allow-list.
#[derive(Clone, Debug, Default)]
pub struct Vocabulary {
    words: HashSet<String>,
}

impl Vocabulary {
    /// Builds it from glossary files (terms written `**Term**:`) and an allow-list of one word per line.
    #[must_use]
    pub fn from_sources(glossaries: &[String], allowed: &str) -> Self {
        let terms = glossaries
            .iter()
            .flat_map(|glossary| glossary.lines())
            .filter_map(term_of);
        let words = terms
            .flat_map(|term| {
                term.split(|character: char| !character.is_alphanumeric())
                    .filter(|word| !word.is_empty())
                    .map(str::to_lowercase)
                    .collect::<Vec<_>>()
            })
            .chain(
                allowed
                    .lines()
                    .map(str::trim)
                    .filter(|word| !word.is_empty())
                    .map(str::to_lowercase),
            )
            .collect();
        Self { words }
    }

    fn knows(&self, word: &str) -> bool {
        word_forms(word)
            .iter()
            .any(|form| self.words.contains(form))
    }
}

/// Finds public types, enums, traits and type aliases in context crates whose names use a word the vocabulary doesn't know.
///
/// # Errors
///
/// When a source file can't be read or parsed.
pub fn check_vocabulary(
    workspace: &Workspace,
    vocabulary: &Vocabulary,
) -> Result<Vec<Finding>, Error> {
    let mut findings = Vec::new();
    for member in workspace.in_layer(CONTEXT_LAYER) {
        for file in source::parse_tree(&member.src())? {
            let mut collector = NameCollector::default();
            collector.visit_file(&file.syntax);
            for (name, line) in collector.names {
                let unknown: Vec<String> = split_camel_case(&name)
                    .into_iter()
                    .filter(|word| !vocabulary.knows(word))
                    .collect();
                if !unknown.is_empty() {
                    findings.push(Finding {
                        path: file.path.clone(),
                        line,
                        check: Check::Vocabulary,
                        message: format!(
                            "`{name}` uses {} outside the glossaries and the vocabulary",
                            unknown.join(", ")
                        ),
                    });
                }
            }
        }
    }
    Ok(findings)
}

fn term_of(line: &str) -> Option<&str> {
    line.strip_prefix("**")?
        .split_once("**:")
        .map(|(term, _)| term)
}

#[derive(Default)]
struct NameCollector {
    names: Vec<(String, usize)>,
}

impl NameCollector {
    fn record(&mut self, visibility: &syn::Visibility, ident: &syn::Ident) {
        if matches!(visibility, syn::Visibility::Public(_)) {
            self.names.push((ident.to_string(), line_of(ident.span())));
        }
    }
}

impl<'ast> Visit<'ast> for NameCollector {
    fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) {
        self.record(&item.vis, &item.ident);
        visit::visit_item_struct(self, item);
    }

    fn visit_item_enum(&mut self, item: &'ast syn::ItemEnum) {
        self.record(&item.vis, &item.ident);
        visit::visit_item_enum(self, item);
    }

    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        self.record(&item.vis, &item.ident);
        visit::visit_item_trait(self, item);
    }

    fn visit_item_type(&mut self, item: &'ast syn::ItemType) {
        self.record(&item.vis, &item.ident);
        visit::visit_item_type(self, item);
    }
}
