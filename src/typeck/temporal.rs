/// Temporal type state machine (Stage 3).
///
/// Tracks Fresh → Expiring → Stale transitions. Transition points are
/// encoded as symbolic Z3 constants in the SMT-LIB emission layer (Stage 4).
use crate::errors::CompilerError;
use crate::fir::Program;

pub fn check(_program: &Program) -> Result<(), CompilerError> {
    Err(CompilerError::NotYetImplemented(
        "temporal type checker (Stage 3)",
    ))
}
