/// ProofCache — content-addressed proof result store (Stage 4).
///
/// Cache key = SHA-256(canonical_fir || compiler_version || z3_version).
/// A cache hit returns the stored certificate without re-invoking Z3.
/// The ACS check is NOT bypassed on a cache hit.
use crate::errors::CompilerError;

pub fn lookup(_key: &[u8]) -> Result<Option<Vec<u8>>, CompilerError> {
    Err(CompilerError::NotYetImplemented("ProofCache (Stage 4)"))
}

pub fn store(_key: &[u8], _value: &[u8]) -> Result<(), CompilerError> {
    Err(CompilerError::NotYetImplemented("ProofCache (Stage 4)"))
}
