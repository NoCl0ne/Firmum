/// Contextual type checking — Stage 3.
///
/// Rejects cross-context arithmetic: two parameters with the same base type
/// but different context tags cannot appear together in the same predicate
/// comparison or verify-block expression.  No implicit cast exists between
/// contexts; the error is a hard type error.
///
/// Scope: checks preconditions, postconditions, invariants, and verify-block
/// statements for each declaration.
///
/// Limitation: only top-level parameter names are resolved.  Qualified
/// identifiers (`sender.balance`) are resolved by their first segment
/// (`sender`).  Full expression-level type inference is deferred to Stage 4.
use std::collections::HashMap;

use crate::errors::CompilerError;
use crate::fir::{ExprNode, PredicateNode, Program, TypeNode, VerifyStatement};

/// Maps a parameter name to its contextual (base_type, context_name) pair.
type CtxEnv = HashMap<String, (String, String)>;

pub fn check(program: &Program) -> Result<(), CompilerError> {
    for decl in &program.declarations {
        let mut env: CtxEnv = HashMap::new();
        for param in decl.intent.inputs.iter().chain(decl.intent.outputs.iter()) {
            if let TypeNode::Contextual { base, context } = &param.ty {
                env.insert(param.name.clone(), (base.clone(), context.clone()));
            }
        }
        // Nothing to check if no contextual parameters are present.
        if env.is_empty() {
            continue;
        }

        for pred in decl
            .intent
            .preconditions
            .iter()
            .chain(decl.intent.postconditions.iter())
            .chain(decl.intent.invariants.iter())
        {
            check_predicate(pred, &env)?;
        }

        for vd in &decl.proof.verify_decls {
            for stmt in &vd.statements {
                check_verify_stmt(stmt, &env)?;
            }
        }
    }
    Ok(())
}

// ── predicate / statement walkers ─────────────────────────────────────────────

fn check_predicate(pred: &PredicateNode, env: &CtxEnv) -> Result<(), CompilerError> {
    match pred {
        PredicateNode::Comparison { left, right, .. } => {
            // Collect contextual (base, context) from both sides of the comparison.
            let mut pairs: Vec<(String, String)> = Vec::new();
            collect_ctx(left, env, &mut pairs);
            collect_ctx(right, env, &mut pairs);
            check_ctx_conflicts(&pairs)
        }
        PredicateNode::And(a, b) | PredicateNode::Or(a, b) => {
            // Each branch is checked independently; `a > 0 AND b > 0` is fine.
            check_predicate(a, env)?;
            check_predicate(b, env)
        }
        PredicateNode::Not(p) => check_predicate(p, env),
        PredicateNode::Forall { body, .. } | PredicateNode::Exists { body, .. } => {
            check_predicate(body, env)
        }
    }
}

fn check_verify_stmt(stmt: &VerifyStatement, env: &CtxEnv) -> Result<(), CompilerError> {
    match stmt {
        VerifyStatement::Assert(pred) => check_predicate(pred, env),
        VerifyStatement::Atomic(stmts) => {
            for s in stmts {
                check_verify_stmt(s, env)?;
            }
            Ok(())
        }
        VerifyStatement::Assign { expr, .. } => {
            let mut pairs: Vec<(String, String)> = Vec::new();
            collect_ctx(expr, env, &mut pairs);
            check_ctx_conflicts(&pairs)
        }
    }
}

// ── expression walker ─────────────────────────────────────────────────────────

fn collect_ctx(expr: &ExprNode, env: &CtxEnv, out: &mut Vec<(String, String)>) {
    match expr {
        ExprNode::Identifier(name) | ExprNode::OldValue(name) => {
            // Resolve by first segment: "sender.balance" → look up "sender".
            let key = name.split('.').next().unwrap_or(name.as_str());
            if let Some((base, ctx)) = env.get(key) {
                out.push((base.clone(), ctx.clone()));
            }
        }
        ExprNode::BinOp { left, right, .. } => {
            collect_ctx(left, env, out);
            collect_ctx(right, env, out);
        }
        ExprNode::FunctionCall { args, .. } => {
            for a in args {
                collect_ctx(a, env, out);
            }
        }
        ExprNode::Number(_) | ExprNode::StringLit(_) => {}
    }
}

// ── conflict detector ─────────────────────────────────────────────────────────

fn check_ctx_conflicts(pairs: &[(String, String)]) -> Result<(), CompilerError> {
    for i in 0..pairs.len() {
        for j in (i + 1)..pairs.len() {
            if pairs[i].0 == pairs[j].0 && pairs[i].1 != pairs[j].1 {
                return Err(CompilerError::TypeCheckError(format!(
                    "cross-context arithmetic: type '{}' is used in context '{}' \
                     and context '{}' within the same expression",
                    pairs[i].0, pairs[i].1, pairs[j].1,
                )));
            }
        }
    }
    Ok(())
}
