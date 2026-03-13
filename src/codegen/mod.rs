/// TypeScript codegen (Stage 5).
///
/// Generated TypeScript is guaranteed to pass `tsc --strict`.
/// This is Level 3 gradual formalization (native .frm with TypeScript codegen).
use crate::errors::CompilerError;
use crate::fir::Program;

pub fn emit_typescript(_program: &Program) -> Result<String, CompilerError> {
    Err(CompilerError::NotYetImplemented(
        "TypeScript codegen (Stage 5)",
    ))
}
