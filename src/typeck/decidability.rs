/// Decidability classifier — Stage 3.
///
/// Classifies each predicate by SMT theory before dispatch to Z3:
///   LIA — Linear Integer Arithmetic
///   LRA — Linear Real Arithmetic
///   BV  — fixed-width Bit Vectors
///
/// Formulas in undecidable fragments produce a hard compiler error before
/// Z3 is invoked, preventing unbounded execution.
///
/// Full implementation deferred: requires expression-level type inference
/// to determine whether operands are integer, real, or bitvector-typed.
use crate::errors::CompilerError;
use crate::fir::PredicateNode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Theory {
    Lia,
    Lra,
    Bv,
}

pub fn classify(_predicate: &PredicateNode) -> Result<Theory, CompilerError> {
    // Default to LIA — the most common theory in Firmum predicates.
    // Full classification deferred to Stage 4 when Z3 dispatch is implemented.
    Ok(Theory::Lia)
}
