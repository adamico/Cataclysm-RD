//! Each check, run against a small workspace written for each test.

use std::fs;
use std::path::{Path, PathBuf};

use rd_checks::{
    BnSymbols, Check, Finding, Vocabulary, Workspace, check_bn_provenance, check_quirks,
    check_vocabulary,
};

const SYMBOLS: &str =
    "qualifier map\nqualifier field_entry\nname process_fields_in_submap\nname add_field\n";
const GLOSSARY: &str =
    "# Game\n\n**Fields**:\nGas and fire.\n\n**Reality bubble**:\nThe loaded map.\n";
const ALLOWED: &str = "entry\nid\n";

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(root.join("adr")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nresolver = \"3\"\nmembers = [\"fields\", \"dice\"]\n",
        )
        .unwrap();
        let fixture = Self { root };
        fixture.add_crate("fields", "context");
        fixture.add_crate("dice", "machinery");
        fixture
    }

    fn add_crate(&self, name: &str, layer: &str) {
        let dir = self.root.join(name);
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(
            dir.join("Cargo.toml"),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[package.metadata.layer]\nlayer = \"{layer}\"\n"
            ),
        )
        .unwrap();
        fs::write(dir.join("src/lib.rs"), "").unwrap();
    }

    fn write(&self, relative: &str, text: &str) -> &Self {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
        self
    }

    fn workspace(&self) -> Workspace {
        Workspace::load(&self.root).unwrap()
    }

    fn provenance(&self) -> Vec<Finding> {
        check_bn_provenance(&self.workspace(), &BnSymbols::parse(SYMBOLS)).unwrap()
    }

    fn quirks(&self) -> Vec<Finding> {
        check_quirks(&self.workspace(), &self.root.join("adr")).unwrap()
    }

    fn vocabulary(&self) -> Vec<Finding> {
        let glossaries = [GLOSSARY.to_owned()];
        check_vocabulary(
            &self.workspace(),
            &Vocabulary::from_sources(&glossaries, ALLOWED),
        )
        .unwrap()
    }
}

fn messages(findings: &[Finding]) -> Vec<String> {
    findings
        .iter()
        .map(|finding| finding.message.clone())
        .collect()
}

#[test]
fn workspace_reads_each_crate_layer() {
    let fixture = Fixture::new("layers");
    let workspace = fixture.workspace();
    let context: Vec<&str> = workspace
        .in_layer("context")
        .map(|member| member.name.as_str())
        .collect();
    assert_eq!(context, ["fields"]);
}

#[test]
fn provenance_passes_docs_in_rust_language() {
    let fixture = Fixture::new("provenance_clean");
    fixture.write(
        "fields/src/lib.rs",
        "//! Fields spread and decay.\n\n/// Spreads gas into the `neighbours` of a tile, e.g. smoke.\npub fn spread() {}\n",
    );
    assert_eq!(fixture.provenance(), []);
}

#[test]
fn provenance_flags_cpp_file_names() {
    let fixture = Fixture::new("provenance_file");
    fixture.write(
        "fields/src/lib.rs",
        "/// Translated from map_field.cpp and `rng.h`.\npub fn spread() {}\n",
    );
    assert_eq!(
        messages(&fixture.provenance()),
        [
            "names the C++ file `map_field.cpp`",
            "names the C++ file `rng.h`"
        ]
    );
}

#[test]
fn provenance_flags_std_and_operators() {
    let fixture = Fixture::new("provenance_std");
    fixture.write(
        "fields/src/lib.rs",
        "/// Ordered like std::map.\npub fn order() {}\n/// Same as `operator bool`.\npub fn truthy() {}\n",
    );
    assert_eq!(
        messages(&fixture.provenance()),
        [
            "names C++'s `std::`",
            "names a C++ operator in `operator bool`"
        ]
    );
}

#[test]
fn provenance_flags_qualified_bn_symbols_with_their_line() {
    let fixture = Fixture::new("provenance_qualified");
    fixture.write(
        "fields/src/lib.rs",
        "pub fn first() {}\n\n/// `map::impassable(p)`.\npub fn blocked() {}\n",
    );
    let findings = fixture.provenance();
    assert_eq!(
        messages(&findings),
        ["names the BN symbol `map::impassable`"]
    );
    assert_eq!(findings[0].line, 3);
    assert_eq!(findings[0].check, Check::BnProvenance);
}

#[test]
fn provenance_flags_bn_names_the_workspace_does_not_define() {
    let fixture = Fixture::new("provenance_name");
    fixture.write(
        "fields/src/lib.rs",
        "/// Like `process_fields_in_submap`, then `add_field`.\npub fn process() {}\npub fn add_field() {}\n",
    );
    assert_eq!(
        messages(&fixture.provenance()),
        ["names the BN symbol `process_fields_in_submap`"]
    );
}

#[test]
fn provenance_allows_rust_paths_through_rust_names() {
    let fixture = Fixture::new("provenance_rust_path");
    fixture.write(
        "fields/src/lib.rs",
        "/// See `crate::spread` and `dice::roll`.\npub fn spread() {}\n",
    );
    assert_eq!(fixture.provenance(), []);
}

#[test]
fn provenance_checks_inner_docs_in_every_crate() {
    let fixture = Fixture::new("provenance_inner");
    fixture.write("dice/src/lib.rs", "//! Port of rng.cpp.\n");
    let findings = fixture.provenance();
    assert_eq!(messages(&findings), ["names the C++ file `rng.cpp`"]);
    assert!(findings[0].path.ends_with("dice/src/lib.rs"));
}

#[test]
fn quirks_pass_when_an_adr_names_them() {
    let fixture = Fixture::new("quirks_recorded");
    fixture
        .write("fields/src/lib.rs", "mod quirks;\n")
        .write(
            "fields/src/quirks.rs",
            "pub struct LibstdcxxHeapOrder;\nstruct Private;\n",
        )
        .write(
            "adr/0001-fields.md",
            "Fields keeps `LibstdcxxHeapOrder` because ...\n",
        );
    assert_eq!(fixture.quirks(), []);
}

#[test]
fn quirks_fail_when_no_adr_names_them() {
    let fixture = Fixture::new("quirks_missing");
    fixture
        .write("fields/src/lib.rs", "mod quirks;\n")
        .write(
            "fields/src/quirks.rs",
            "pub struct LibstdcxxHeapOrder;\npub fn clang_argument_order() {}\n",
        )
        .write(
            "adr/0001-fields.md",
            "Fields keeps `LibstdcxxHeapOrderOld`.\n",
        );
    let findings = fixture.quirks();
    assert_eq!(findings.len(), 2);
    assert!(
        findings[0]
            .message
            .starts_with("Quirk `LibstdcxxHeapOrder` is not named")
    );
    assert_eq!(findings[1].line, 2);
}

#[test]
fn quirks_cover_inline_and_nested_modules() {
    let fixture = Fixture::new("quirks_nested");
    fixture
        .write(
            "dice/src/lib.rs",
            "mod quirks {\n    pub fn small_seed() {}\n}\npub fn roll() {}\n",
        )
        .write("fields/src/quirks/heap.rs", "pub struct TieOrder;\n");
    let names: Vec<String> = fixture
        .quirks()
        .iter()
        .map(|finding| finding.message.clone())
        .collect();
    assert_eq!(names.len(), 2);
    assert!(names.iter().any(|message| message.contains("`small_seed`")));
    assert!(names.iter().any(|message| message.contains("`TieOrder`")));
}

#[test]
fn vocabulary_passes_glossary_words_and_plurals() {
    let fixture = Fixture::new("vocabulary_clean");
    fixture.write(
        "fields/src/lib.rs",
        "pub struct FieldEntry;\npub trait RealityBubble {}\npub enum FieldId {}\npub type Entries = ();\n",
    );
    assert_eq!(fixture.vocabulary(), []);
}

#[test]
fn vocabulary_flags_unknown_words_in_public_names() {
    let fixture = Fixture::new("vocabulary_unknown");
    fixture.write(
        "fields/src/lib.rs",
        "pub struct SmPos;\npub struct FieldHTTPEntry;\n",
    );
    assert_eq!(
        messages(&fixture.vocabulary()),
        [
            "`SmPos` uses Sm, Pos outside the glossaries and the vocabulary",
            "`FieldHTTPEntry` uses HTTP outside the glossaries and the vocabulary",
        ]
    );
}

#[test]
fn vocabulary_ignores_private_names_and_other_layers() {
    let fixture = Fixture::new("vocabulary_scope");
    fixture
        .write(
            "fields/src/lib.rs",
            "struct SmPos;\npub(crate) struct TerId;\n",
        )
        .write("dice/src/lib.rs", "pub struct MinstdRand0;\n");
    assert_eq!(fixture.vocabulary(), []);
}
