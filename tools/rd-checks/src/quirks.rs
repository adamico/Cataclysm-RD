use std::fs;
use std::path::Path;

use syn::visit::{self, Visit};

use crate::error::Error;
use crate::finding::{Check, Finding};
use crate::source::{self, line_of};
use crate::workspace::Workspace;

const QUIRKS_MODULE: &str = "quirks";

/// Finds public items of a `quirks` module, in any crate, that no ADR in `adr_dir` names in backticks.
///
/// # Errors
///
/// When a source file or an ADR can't be read, or a source file can't be parsed.
pub fn check_quirks(workspace: &Workspace, adr_dir: &Path) -> Result<Vec<Finding>, Error> {
    let records = adr_text(adr_dir)?;
    let mut findings = Vec::new();
    for member in &workspace.crates {
        for file in source::parse_tree(&member.src())? {
            let mut collector = QuirkCollector {
                inside: file
                    .module_path
                    .iter()
                    .any(|module| module == QUIRKS_MODULE),
                quirks: Vec::new(),
            };
            collector.visit_file(&file.syntax);
            findings.extend(
                collector
                    .quirks
                    .into_iter()
                    .filter(|(name, _)| !is_recorded(&records, name))
                    .map(|(name, line)| Finding {
                        path: file.path.clone(),
                        line,
                        check: Check::Quirk,
                        message: format!(
                            "Quirk `{name}` is not named in any ADR under {}",
                            adr_dir.display()
                        ),
                    }),
            );
        }
    }
    Ok(findings)
}

fn adr_text(adr_dir: &Path) -> Result<String, Error> {
    let mut text = String::new();
    let entries =
        fs::read_dir(adr_dir).map_err(|error| Error::Read(adr_dir.to_path_buf(), error))?;
    for entry in entries {
        let path = entry
            .map_err(|error| Error::Read(adr_dir.to_path_buf(), error))?
            .path();
        if path.extension().is_some_and(|extension| extension == "md") {
            text.push_str(
                &fs::read_to_string(&path).map_err(|error| Error::Read(path.clone(), error))?,
            );
        }
    }
    Ok(text)
}

fn is_recorded(records: &str, name: &str) -> bool {
    records
        .match_indices(&format!("`{name}"))
        .any(|(start, matched)| {
            records[start + matched.len()..]
                .chars()
                .next()
                .is_some_and(|after| !(after.is_alphanumeric() || after == '_'))
        })
}

struct QuirkCollector {
    inside: bool,
    quirks: Vec<(String, usize)>,
}

impl QuirkCollector {
    fn record(&mut self, visibility: &syn::Visibility, ident: &syn::Ident) {
        if self.inside && matches!(visibility, syn::Visibility::Public(_)) {
            self.quirks.push((ident.to_string(), line_of(ident.span())));
        }
    }
}

impl<'ast> Visit<'ast> for QuirkCollector {
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        let outer = self.inside;
        self.inside = outer || item.ident == QUIRKS_MODULE;
        visit::visit_item_mod(self, item);
        self.inside = outer;
    }

    fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) {
        self.record(&item.vis, &item.ident);
    }

    fn visit_item_enum(&mut self, item: &'ast syn::ItemEnum) {
        self.record(&item.vis, &item.ident);
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        self.record(&item.vis, &item.sig.ident);
    }

    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        self.record(&item.vis, &item.ident);
    }

    fn visit_item_type(&mut self, item: &'ast syn::ItemType) {
        self.record(&item.vis, &item.ident);
    }

    fn visit_item_const(&mut self, item: &'ast syn::ItemConst) {
        self.record(&item.vis, &item.ident);
    }

    fn visit_item_static(&mut self, item: &'ast syn::ItemStatic) {
        self.record(&item.vis, &item.ident);
    }
}
