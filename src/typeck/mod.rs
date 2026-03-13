/// Stage 3: type checker.
///
/// Checks enforced here (grammar cannot enforce these):
///   1. Every declaration: intent/assumption/proof share the same identifier.
///   2. Every verify_decl target references the intent in the same triple.
///   3. old() does not appear in preconditions, invariants, or refined-type
///      parameter predicates.
///
/// Sub-modules handle ACS, contextual types, temporal state, and decidability.
/// None invoke Z3 — that is Stage 4.
pub mod acs;
pub mod contextual;
pub mod decidability;
pub mod temporal;

use crate::errors::CompilerError;
use crate::fir::{ExprNode, PredicateNode, Program, VerifyStatement};

pub fn check(program: &Program) -> Result<(), CompilerError> {
    for decl in &program.declarations {
        // Rule 1: all three blocks must share the same identifier.
        if decl.intent.name != decl.assumption.name {
            return Err(CompilerError::TypeCheckError(format!(
                "declaration name mismatch: intent '{}' vs assumption '{}'",
                decl.intent.name, decl.assumption.name
            )));
        }
        if decl.intent.name != decl.proof.name {
            return Err(CompilerError::TypeCheckError(format!(
                "declaration name mismatch: intent '{}' vs proof '{}'",
                decl.intent.name, decl.proof.name
            )));
        }

        // Rule 2: every verify_decl target must reference the intent in this triple.
        for vd in &decl.proof.verify_decls {
            if vd.target != decl.intent.name {
                return Err(CompilerError::TypeCheckError(format!(
                    "verify '{}' does not reference intent '{}'",
                    vd.target, decl.intent.name
                )));
            }
        }

        // Rule 3: old() is not valid in preconditions.
        for pred in &decl.intent.preconditions {
            if predicate_contains_old(pred) {
                return Err(CompilerError::TypeCheckError(
                    "old() is not valid inside a precondition".to_string(),
                ));
            }
        }

        // Rule 3: old() is not valid in invariants.
        for pred in &decl.intent.invariants {
            if predicate_contains_old(pred) {
                return Err(CompilerError::TypeCheckError(
                    "old() is not valid inside an invariant".to_string(),
                ));
            }
        }

        // Rule 3: old() is not valid in refined-type parameter predicates.
        for param in decl.intent.inputs.iter().chain(decl.intent.outputs.iter()) {
            if let crate::fir::TypeNode::Refined { predicate, .. } = &param.ty {
                if predicate_contains_old(predicate) {
                    return Err(CompilerError::TypeCheckError(format!(
                        "old() is not valid in the refined-type predicate of parameter '{}'",
                        param.name
                    )));
                }
            }
        }
    }

    contextual::check(program)?;
    temporal::check(program)?;
    Ok(())
}

// ── AST walk helpers (pub(crate) for use in acs.rs) ──────────────────────────

pub(crate) fn predicate_contains_old(pred: &PredicateNode) -> bool {
    match pred {
        PredicateNode::Comparison { left, right, .. } => {
            expr_contains_old(left) || expr_contains_old(right)
        }
        PredicateNode::Or(a, b) | PredicateNode::And(a, b) => {
            predicate_contains_old(a) || predicate_contains_old(b)
        }
        PredicateNode::Not(p) => predicate_contains_old(p),
        PredicateNode::Forall { body, .. } | PredicateNode::Exists { body, .. } => {
            predicate_contains_old(body)
        }
    }
}

pub(crate) fn expr_contains_old(expr: &ExprNode) -> bool {
    match expr {
        ExprNode::OldValue(_) => true,
        ExprNode::BinOp { left, right, .. } => expr_contains_old(left) || expr_contains_old(right),
        ExprNode::FunctionCall { args, .. } => args.iter().any(expr_contains_old),
        _ => false,
    }
}

#[allow(dead_code)]
pub(crate) fn verify_statement_contains_old(stmt: &VerifyStatement) -> bool {
    match stmt {
        VerifyStatement::Assert(pred) => predicate_contains_old(pred),
        VerifyStatement::Atomic(stmts) => stmts.iter().any(verify_statement_contains_old),
        VerifyStatement::Assign { expr, .. } => expr_contains_old(expr),
    }
}
