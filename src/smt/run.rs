/// Z3 process runner — Stage 4.
///
/// Invokes the Z3 binary via `std::process::Command` using the SMT-LIB 2 text
/// interface (`z3 -in` reads from stdin).  No Rust z3 binding crate is used
/// until compile compatibility with the installed Z3 version is verified.
///
/// If the `z3` binary is not found on PATH, the function returns a clear
/// `CompilerError::SmtError` rather than panicking.
use std::io::Write as IoWrite;
use std::process::{Command, Stdio};

use crate::errors::CompilerError;

/// The result of a `(check-sat)` query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmtResult {
    /// The formula is satisfiable (a model exists).
    Sat,
    /// The formula is unsatisfiable (no model exists — proof valid).
    Unsat,
    /// Z3 could not determine satisfiability within resource limits.
    Unknown,
}

/// Invoke `z3 -in`, write `smtlib_input` to its stdin, and parse the first
/// output line as `sat`, `unsat`, or `unknown`.
///
/// Returns `Err(CompilerError::SmtError)` if:
/// - The `z3` binary is not found on PATH.
/// - Spawning or communicating with the process fails.
pub fn run_z3(smtlib_input: &str) -> Result<SmtResult, CompilerError> {
    let mut child = Command::new("z3")
        .arg("-in")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                CompilerError::SmtError(
                    "z3 binary not found on PATH; \
                     install Z3 (https://github.com/Z3Prover/z3) to enable SMT verification"
                        .to_string(),
                )
            } else {
                CompilerError::SmtError(format!("failed to spawn z3: {e}"))
            }
        })?;

    // Write input to z3's stdin, then drop the handle to signal EOF.
    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| CompilerError::SmtError("could not open z3 stdin pipe".to_string()))?;
        stdin
            .write_all(smtlib_input.as_bytes())
            .map_err(|e| CompilerError::SmtError(format!("failed to write to z3 stdin: {e}")))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| CompilerError::SmtError(format!("failed to wait for z3 process: {e}")))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next().unwrap_or("").trim();

    match first_line {
        "unsat" => Ok(SmtResult::Unsat),
        "sat" => Ok(SmtResult::Sat),
        _ => Ok(SmtResult::Unknown),
    }
}
