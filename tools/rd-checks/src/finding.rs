use std::fmt;
use std::path::PathBuf;

/// Which rule a finding breaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Check {
    /// A doc comment traces the code back to BN.
    BnProvenance,
    /// A Quirk is not named in any ADR.
    Quirk,
    /// A public name uses a word outside the glossaries and the vocabulary.
    Vocabulary,
}

impl fmt::Display for Check {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::BnProvenance => "bn-provenance",
            Self::Quirk => "quirk",
            Self::Vocabulary => "vocabulary",
        })
    }
}

/// One place in the code that breaks a rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    /// The source file.
    pub path: PathBuf,
    /// The 1-based line in that file.
    pub line: usize,
    /// The rule broken.
    pub check: Check,
    /// What is wrong, in a sentence.
    pub message: String,
}

impl fmt::Display for Finding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}:{}: [{}] {}",
            self.path.display(),
            self.line,
            self.check,
            self.message
        )
    }
}
