use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use syn::visit::{self, Visit};

use crate::error::Error;

pub(crate) struct SourceFile {
    pub(crate) path: PathBuf,
    pub(crate) syntax: syn::File,
    pub(crate) module_path: Vec<String>,
}

pub(crate) struct DocText {
    pub(crate) text: String,
    pub(crate) line: usize,
}

pub(crate) fn parse_tree(src: &Path) -> Result<Vec<SourceFile>, Error> {
    rust_files(src)?
        .into_iter()
        .map(|path| {
            let text =
                fs::read_to_string(&path).map_err(|error| Error::Read(path.clone(), error))?;
            let syntax =
                syn::parse_file(&text).map_err(|error| Error::Parse(path.clone(), error))?;
            let module_path = module_path(src, &path);
            Ok(SourceFile {
                path,
                syntax,
                module_path,
            })
        })
        .collect()
}

pub(crate) fn line_of(span: proc_macro2::Span) -> usize {
    span.start().line
}

pub(crate) fn doc_texts(file: &syn::File) -> Vec<DocText> {
    let mut collector = DocCollector::default();
    collector.visit_file(file);
    collector.docs
}

pub(crate) fn identifiers(file: &syn::File) -> HashSet<String> {
    let mut collector = IdentCollector::default();
    collector.visit_file(file);
    collector.idents
}

fn rust_files(dir: &Path) -> Result<Vec<PathBuf>, Error> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        let entries =
            fs::read_dir(&current).map_err(|error| Error::Read(current.clone(), error))?;
        for entry in entries {
            let path = entry
                .map_err(|error| Error::Read(current.clone(), error))?
                .path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                found.push(path);
            }
        }
    }
    found.sort();
    Ok(found)
}

fn module_path(src: &Path, file: &Path) -> Vec<String> {
    let relative = file.strip_prefix(src).unwrap_or(file).with_extension("");
    relative
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .filter(|name| !matches!(*name, "mod" | "lib" | "main"))
        .map(str::to_owned)
        .collect()
}

#[derive(Default)]
struct DocCollector {
    docs: Vec<DocText>,
}

impl<'ast> Visit<'ast> for DocCollector {
    fn visit_attribute(&mut self, attribute: &'ast syn::Attribute) {
        if let syn::Meta::NameValue(pair) = &attribute.meta
            && pair.path.is_ident("doc")
            && let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(text),
                ..
            }) = &pair.value
        {
            self.docs.push(DocText {
                text: text.value(),
                line: line_of(text.span()),
            });
        }
        visit::visit_attribute(self, attribute);
    }
}

#[derive(Default)]
struct IdentCollector {
    idents: HashSet<String>,
}

impl<'ast> Visit<'ast> for IdentCollector {
    fn visit_ident(&mut self, ident: &'ast proc_macro2::Ident) {
        self.idents.insert(ident.to_string());
    }
}
