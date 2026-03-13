/// Contextual type checking (Stage 3).
///
/// Rejects cross-context arithmetic at the point of use.
/// No manual cast exists; contextual type errors are hard errors.
use crate::errors::CompilerError;
use crate::fir::Program;

pub fn check(_program: &Program) -> Result<(), CompilerError> {
    Err(CompilerError::NotYetImplemented(
        "contextual type checker (Stage 3)",
    ))
}
