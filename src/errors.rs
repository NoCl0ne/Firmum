use std::fmt;

/// Compiler-wide error type.
///
/// Each variant corresponds to a pipeline stage. Variants are defined for all
/// stages so that the type is stable across the full pipeline implementation.
#[derive(Debug)]
pub enum CompilerError {
    ParseError(String),
    LoweringError(String),
    TypeCheckError(String),
    AcsError(String),
    SmtError(String),
    CertError(String),
    IoError(std::io::Error),
    /// Stage placeholder — returned by unimplemented pipeline stages.
    /// Satisfies quality-gate rule 6: the caller always handles it via Err(…).
    NotYetImplemented(&'static str),
}

impl fmt::Display for CompilerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseError(msg) => write!(f, "parse error: {msg}"),
            Self::LoweringError(msg) => write!(f, "lowering error: {msg}"),
            Self::TypeCheckError(msg) => write!(f, "type error: {msg}"),
            Self::AcsError(msg) => write!(f, "ACS error: {msg}"),
            Self::SmtError(msg) => write!(f, "SMT error: {msg}"),
            Self::CertError(msg) => write!(f, "certificate error: {msg}"),
            Self::IoError(e) => write!(f, "I/O error: {e}"),
            Self::NotYetImplemented(s) => write!(f, "{s} is not yet implemented"),
        }
    }
}

impl std::error::Error for CompilerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IoError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for CompilerError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}
