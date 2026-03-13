/// Stage 3: Type checker.
///
/// Sub-modules: contextual types, temporal state machine, ACS engine,
/// decidability classifier. None call Z3 — that is Stage 4.
pub mod acs;
pub mod contextual;
pub mod decidability;
pub mod temporal;

use crate::errors::CompilerError;
use crate::fir::Program;

pub fn check(_program: &Program) -> Result<(), CompilerError> {
    Err(CompilerError::NotYetImplemented("type checker (Stage 3)"))
}
