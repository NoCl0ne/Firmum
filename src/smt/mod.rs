/// Stage 4: SMT Orchestrator.
///
/// Dispatches verification jobs in declaration order using the SMT-LIB 2 text
/// interface (`std::process::Command` spawning `z3 -in`).  No Rust z3 binding
/// crate is used until compile compatibility with the system Z3 version is
/// verified.
///
/// If the `z3` binary is absent, `orchestrate` returns a clear
/// `CompilerError::SmtError` rather than panicking.
pub mod cache;
pub mod emit;
pub mod run;

use crate::errors::CompilerError;
use crate::fir::Program;

/// Verify every declaration in `program` via Z3.
///
/// For each declaration the orchestrator:
///   1. Emits a self-contained SMT-LIB 2 query (`emit::emit_declaration`).
///   2. Submits it to Z3 (`run::run_z3`).
///   3. Returns `SmtError` if Z3 responds `sat` (counterexample found) or
///      `unknown` (resource limit exceeded).
///
/// Returns `Ok(())` only when every declaration is verified (`unsat`).
pub fn orchestrate(program: &Program) -> Result<(), CompilerError> {
    for decl in &program.declarations {
        let smtlib = emit::emit_declaration(decl);

        let result = run::run_z3(&smtlib)?;

        match result {
            run::SmtResult::Unsat => {
                // Verified: the postconditions are entailed by the preconditions.
            }
            run::SmtResult::Sat => {
                return Err(CompilerError::SmtError(format!(
                    "intent '{}': Z3 found a counterexample (sat); \
                     the postconditions are not entailed by the preconditions",
                    decl.intent.name
                )));
            }
            run::SmtResult::Unknown => {
                return Err(CompilerError::SmtError(format!(
                    "intent '{}': Z3 returned 'unknown'; \
                     increase resource limits or simplify the predicates",
                    decl.intent.name
                )));
            }
        }
    }
    Ok(())
}
