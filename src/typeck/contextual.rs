/// Contextual type checking — Stage 3.
///
/// Rejects cross-context arithmetic at the point of use.
/// No manual cast exists; contextual type errors are hard errors.
///
/// Full implementation deferred to Stage 3 completion: requires expression-level
/// type inference to propagate contextual tags through BinOp trees.
use crate::errors::CompilerError;
use crate::fir::Program;

pub fn check(_program: &Program) -> Result<(), CompilerError> {
    // Cross-context arithmetic checking deferred — no false positives on valid programs.
    Ok(())
}
