/// Decidability classifier (Stage 3).
///
/// Classifies each predicate by SMT theory before dispatch:
///   LIA  — Linear Integer Arithmetic
///   LRA  — Linear Real Arithmetic
///   BV   — fixed-width Bit Vectors
///
/// Formulas in undecidable fragments produce a hard compiler error before
/// Z3 is invoked, preventing unbounded execution.
use crate::errors::CompilerError;
use crate::fir::PredicateNode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Theory {
    Lia,
    Lra,
    Bv,
}

pub fn classify(_predicate: &PredicateNode) -> Result<Theory, CompilerError> {
    Err(CompilerError::NotYetImplemented(
        "decidability classifier (Stage 3)",
    ))
}
