use super::Program;
/// Stage 2: Pairs → FIR lowering.
///
/// Converts the pest `Pairs<Rule>` tree produced by the parser into a typed
/// `Program` FIR node. Not yet implemented — see STATUS.md.
use crate::errors::CompilerError;

pub fn lower(
    _pairs: pest::iterators::Pairs<crate::parser::Rule>,
) -> Result<Program, CompilerError> {
    Err(CompilerError::NotYetImplemented("FIR lowering (Stage 2)"))
}
