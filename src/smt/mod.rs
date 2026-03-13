/// Stage 4: SMT Orchestrator.
///
/// Dispatches verification jobs in topological dependency order.
/// Uses SMT-LIB 2 text interface (std::process::Command invoking the z3
/// binary) as the safe default; no Rust z3 binding crate is added until
/// compile compatibility with the system Z3 version is verified.
pub mod cache;
pub mod emit;

use crate::errors::CompilerError;
use crate::fir::Program;

pub fn orchestrate(_program: &Program) -> Result<(), CompilerError> {
    Err(CompilerError::NotYetImplemented(
        "SMT orchestrator (Stage 4)",
    ))
}
