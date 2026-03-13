/// Certificate PKI — ModuleCertificate and Ed25519 signing (Stage 4).
///
/// Three-level signing hierarchy:
///   Root CA → Compiler signing key (per-release, HSM-backed)
///           → Org signing key (team-controlled)
///
/// ModuleCertificate fields: module_id, proof_hash, acs_score,
/// verification_strategy, conservative_warning (optional), compiler_version,
/// verified_at, z3_version, compiler_signature, org_signature.
///
/// conservative_warning is part of the signed payload; stripping it
/// invalidates the signature.
use crate::errors::CompilerError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleCertificate {
    pub module_id: String,
    pub proof_hash: String,
    pub acs_score: f64,
    pub verification_strategy: String,
    pub conservative_warning: Option<String>,
    pub compiler_version: String,
    pub verified_at: String,
    pub z3_version: String,
    /// Ed25519 signature by the compiler signing key (hex-encoded).
    pub compiler_signature: String,
    /// Ed25519 signature by the org signing key (hex-encoded). Required for production.
    pub org_signature: Option<String>,
}

pub fn sign(_cert: &ModuleCertificate) -> Result<ModuleCertificate, CompilerError> {
    Err(CompilerError::NotYetImplemented(
        "certificate signing (Stage 4)",
    ))
}
