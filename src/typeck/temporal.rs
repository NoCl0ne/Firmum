/// Temporal type state machine — Stage 3.
///
/// Rejects `Stale<T>` used where `Fresh<T, d>` is required.  Two parameters
/// with the same inner base type but different temporal states (Fresh vs Stale)
/// cannot appear together in the same predicate comparison or verify-block
/// expression, because the stale value invalidates the freshness guarantee.
///
/// State model: Fresh → Expiring → Stale
///   - `Expiring` is not a hard error by itself but should not mix with `Fresh`.
///   - Full transition tracking (dataflow across assignment boundaries) is
///     deferred to Stage 4 when the SMT-LIB emission layer is available.
///
/// Limitation: only top-level parameter names are resolved.  Qualified
/// identifiers are resolved by their first segment.
use std::collections::HashMap;

use crate::errors::CompilerError;
use crate::fir::{ExprNode, PredicateNode, Program, TemporalType, TypeNode, VerifyStatement};

#[derive(Clone, Debug)]
enum TKind {
    Fresh,
    Expiring,
    Stale,
}

/// Maps a parameter name to its (inner_base_type, temporal_kind) pair.
type TempEnv = HashMap<String, (String, TKind)>;

pub fn check(program: &Program) -> Result<(), CompilerError> {
    for decl in &program.declarations {
        let mut env: TempEnv = HashMap::new();
        for param in decl.intent.inputs.iter().chain(decl.intent.outputs.iter()) {
            let entry = match &param.ty {
                TypeNode::Temporal(TemporalType::Fresh { inner, .. }) => {
                    Some((type_base_name(inner), TKind::Fresh))
                }
                TypeNode::Temporal(TemporalType::Expiring { inner, .. }) => {
                    Some((type_base_name(inner), TKind::Expiring))
                }
                TypeNode::Temporal(TemporalType::Stale(inner)) => {
                    Some((type_base_name(inner), TKind::Stale))
                }
                _ => None,
            };
            if let Some((base, kind)) = entry {
                env.insert(param.name.clone(), (base, kind));
            }
        }
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

// ── helpers ───────────────────────────────────────────────────────────────────

/// Extract the base type name from a TypeNode for use as the canonical key.
fn type_base_name(ty: &TypeNode) -> String {
    match ty {
        TypeNode::Base(s) => s.clone(),
        TypeNode::Contextual { base, .. } | TypeNode::Refined { base, .. } => base.clone(),
        TypeNode::Dependent { base, .. } => base.clone(),
        // Nested temporal types are not supported here; use an empty key so
        // they do not trigger spurious conflicts.
        TypeNode::Temporal(_) => String::new(),
    }
}

// ── predicate / statement walkers ─────────────────────────────────────────────

fn check_predicate(pred: &PredicateNode, env: &TempEnv) -> Result<(), CompilerError> {
    match pred {
        PredicateNode::Comparison { left, right, .. } => {
            let mut pairs: Vec<(String, TKind)> = Vec::new();
            collect_temporal(left, env, &mut pairs);
            collect_temporal(right, env, &mut pairs);
            check_temporal_conflicts(&pairs)
        }
        PredicateNode::And(a, b) | PredicateNode::Or(a, b) => {
            check_predicate(a, env)?;
            check_predicate(b, env)
        }
        PredicateNode::Not(p) => check_predicate(p, env),
        PredicateNode::Forall { body, .. } | PredicateNode::Exists { body, .. } => {
            check_predicate(body, env)
        }
    }
}

fn check_verify_stmt(stmt: &VerifyStatement, env: &TempEnv) -> Result<(), CompilerError> {
    match stmt {
        VerifyStatement::Assert(pred) => check_predicate(pred, env),
        VerifyStatement::Atomic(stmts) => {
            for s in stmts {
                check_verify_stmt(s, env)?;
            }
            Ok(())
        }
        VerifyStatement::Assign { expr, .. } => {
            let mut pairs: Vec<(String, TKind)> = Vec::new();
            collect_temporal(expr, env, &mut pairs);
            check_temporal_conflicts(&pairs)
        }
    }
}

// ── expression walker ─────────────────────────────────────────────────────────

fn collect_temporal(expr: &ExprNode, env: &TempEnv, out: &mut Vec<(String, TKind)>) {
    match expr {
        ExprNode::Identifier(name) | ExprNode::OldValue(name) => {
            let key = name.split('.').next().unwrap_or(name.as_str());
            if let Some((base, kind)) = env.get(key) {
                out.push((base.clone(), kind.clone()));
            }
        }
        ExprNode::BinOp { left, right, .. } => {
            collect_temporal(left, env, out);
            collect_temporal(right, env, out);
        }
        ExprNode::FunctionCall { args, .. } => {
            for a in args {
                collect_temporal(a, env, out);
            }
        }
        ExprNode::Number(_) | ExprNode::StringLit(_) => {}
    }
}

// ── conflict detector ─────────────────────────────────────────────────────────

fn check_temporal_conflicts(pairs: &[(String, TKind)]) -> Result<(), CompilerError> {
    for i in 0..pairs.len() {
        for j in (i + 1)..pairs.len() {
            // Two entries with the same (non-empty) base type are in conflict
            // if one is Fresh and the other is Stale or Expiring.
            if pairs[i].0.is_empty() || pairs[i].0 != pairs[j].0 {
                continue;
            }
            let conflict = matches!(
                (&pairs[i].1, &pairs[j].1),
                (TKind::Fresh, TKind::Stale)
                    | (TKind::Stale, TKind::Fresh)
                    | (TKind::Fresh, TKind::Expiring)
                    | (TKind::Expiring, TKind::Fresh)
            );
            if conflict {
                return Err(CompilerError::TypeCheckError(format!(
                    "temporal type conflict: 'Stale<{}>' (or 'Expiring') \
                     used where 'Fresh<{}>' is required in the same expression",
                    pairs[i].0, pairs[j].0,
                )));
            }
        }
    }
    Ok(())
}
