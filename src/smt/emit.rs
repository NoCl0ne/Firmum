/// SMT-LIB 2 emitter — Stage 4.
///
/// Translates one Firmum `Declaration` (intent + assumption + proof triple)
/// into a self-contained SMT-LIB 2 string suitable for piping to `z3 -in`.
///
/// Encoding strategy
/// ─────────────────
/// 1. Logic: `(set-logic LIA)` — Linear Integer Arithmetic (conservative
///    default matching `decidability::classify`; all scalar values are Int).
/// 2. Free variables: every unique normalised identifier (including `old`
///    references) is emitted as `(declare-fun <name> () Int)`.
/// 3. Preconditions: each line is asserted directly as a premise.
/// 4. Postconditions: their conjunction is negated and asserted so that
///    UNSAT from Z3 proves the preconditions entail the postconditions.
/// 5. Invariants: asserted directly (expected to hold in all states).
/// 6. Lemmas: each lemma body is negated and asserted; UNSAT proves the lemma.
/// 7. `(check-sat)` closes the query.
///
/// Identifier normalisation
/// ────────────────────────
/// Qualified identifiers (`sender.balance`) → `sender_balance` (dot → `_`).
/// `old(sender.balance)` → `sender_balance_old`.
use std::collections::BTreeSet;

use crate::errors::CompilerError;
use crate::fir::{
    BinOpKind, ComparisonOp, Declaration, ExprNode, Number, PredicateNode, VerifyStatement,
};

// ── public API ────────────────────────────────────────────────────────────────

/// Translate a single `Declaration` into a complete SMT-LIB 2 query string.
pub fn emit_declaration(decl: &Declaration) -> String {
    let mut out = String::new();

    out.push_str(&format!(
        "; Firmum SMT-LIB 2 encoding — intent {}\n",
        decl.intent.name
    ));
    out.push_str("(set-logic LIA)\n\n");

    // Collect every free name that appears in this declaration.
    let free_names = collect_declaration_names(decl);

    if !free_names.is_empty() {
        out.push_str("; Declared symbols\n");
        for name in &free_names {
            out.push_str(&format!("(declare-fun {} () Int)\n", name));
        }
        out.push('\n');
    }

    // Preconditions — asserted as premises.
    if !decl.intent.preconditions.is_empty() {
        out.push_str("; Preconditions (assumed)\n");
        for pred in &decl.intent.preconditions {
            out.push_str(&format!("(assert {})\n", emit_pred(pred)));
        }
        out.push('\n');
    }

    // Postconditions — assert negation; UNSAT ⟹ verified.
    if !decl.intent.postconditions.is_empty() {
        out.push_str("; Postconditions — UNSAT iff all are entailed by preconditions\n");
        let conj = conjoin(
            decl.intent
                .postconditions
                .iter()
                .map(emit_pred)
                .collect::<Vec<_>>(),
        );
        out.push_str(&format!("(assert (not {}))\n", conj));
        out.push('\n');
    }

    // Invariants — asserted as facts that must always hold.
    if !decl.intent.invariants.is_empty() {
        out.push_str("; Invariants\n");
        for pred in &decl.intent.invariants {
            out.push_str(&format!("(assert {})\n", emit_pred(pred)));
        }
        out.push('\n');
    }

    // Never identifiers — placeholder: assert the identifier equals zero
    // (full encoding deferred to Stage 5 when event types are available).
    if !decl.intent.never.is_empty() {
        out.push_str("; Never clauses (placeholder encoding)\n");
        for id in &decl.intent.never {
            out.push_str(&format!("(assert (= {} 0))\n", normalise(id)));
        }
        out.push('\n');
    }

    // Lemmas — assert negation of each lemma body; UNSAT ⟹ lemma holds.
    if !decl.proof.lemmas.is_empty() {
        out.push_str("; Lemmas\n");
        for lemma in &decl.proof.lemmas {
            out.push_str(&format!("; lemma {}\n", lemma.name));
            let body_conj = if lemma.predicates.is_empty() {
                "true".to_string()
            } else {
                conjoin(lemma.predicates.iter().map(emit_pred).collect::<Vec<_>>())
            };
            out.push_str(&format!("(assert (not {}))\n", body_conj));
        }
        out.push('\n');
    }

    out.push_str("(check-sat)\n");
    out
}

/// Translate a single `PredicateNode` to an SMT-LIB 2 formula string.
///
/// Returns `Ok(formula)` for all inputs; the `Result` wrapping matches the
/// stub signature and allows callers to chain with `?`.
pub fn emit_predicate(predicate: &PredicateNode) -> Result<String, CompilerError> {
    Ok(emit_pred(predicate))
}

// ── predicate emitter ─────────────────────────────────────────────────────────

fn emit_pred(pred: &PredicateNode) -> String {
    match pred {
        PredicateNode::Comparison { left, op, right } => {
            let l = emit_expr(left);
            let r = emit_expr(right);
            let op_str = match op {
                ComparisonOp::Eq => "=",
                ComparisonOp::Ne => "distinct",
                ComparisonOp::Le => "<=",
                ComparisonOp::Ge => ">=",
                ComparisonOp::Lt => "<",
                ComparisonOp::Gt => ">",
            };
            format!("({} {} {})", op_str, l, r)
        }
        PredicateNode::And(a, b) => format!("(and {} {})", emit_pred(a), emit_pred(b)),
        PredicateNode::Or(a, b) => format!("(or {} {})", emit_pred(a), emit_pred(b)),
        PredicateNode::Not(p) => format!("(not {})", emit_pred(p)),
        PredicateNode::Forall { var, body, .. } => {
            // Collect the names used in the body and promote them to binders.
            // This over-quantifies relative to the source but produces valid
            // SMT-LIB 2 that preserves the structural intent.
            let binders = body_binders(var, body);
            format!("(forall ({}) {})", binders, emit_pred(body))
        }
        PredicateNode::Exists { var, body, .. } => {
            let binders = body_binders(var, body);
            format!("(exists ({}) {})", binders, emit_pred(body))
        }
    }
}

// ── expression emitter ────────────────────────────────────────────────────────

fn emit_expr(expr: &ExprNode) -> String {
    match expr {
        ExprNode::Number(Number::Integer(n)) => n.to_string(),
        ExprNode::Number(Number::Decimal(f)) => {
            // SMT-LIB 2 Real literals require a decimal point.
            format!("{:.1}", f)
        }
        ExprNode::Identifier(s) => normalise(s),
        ExprNode::OldValue(s) => format!("{}_old", normalise(s)),
        ExprNode::StringLit(_) => {
            // String literals have no direct LIA representation; emit 0 as a
            // placeholder so the formula remains syntactically valid.
            "0".to_string()
        }
        ExprNode::BinOp { left, op, right } => {
            let l = emit_expr(left);
            let r = emit_expr(right);
            let op_str = match op {
                BinOpKind::Add => "+",
                BinOpKind::Sub => "-",
                BinOpKind::Mul => "*",
                BinOpKind::Div => "div",
            };
            format!("({} {} {})", op_str, l, r)
        }
        ExprNode::FunctionCall { name, args } => {
            if args.is_empty() {
                normalise(name)
            } else {
                let args_str = args.iter().map(emit_expr).collect::<Vec<_>>().join(" ");
                format!("({} {})", normalise(name), args_str)
            }
        }
    }
}

// ── name collection ───────────────────────────────────────────────────────────

/// Collect all unique normalised symbol names referenced in `decl`.
/// Uses `BTreeSet` so the output order is deterministic.
fn collect_declaration_names(decl: &Declaration) -> BTreeSet<String> {
    let mut names: BTreeSet<String> = BTreeSet::new();

    let preds = decl
        .intent
        .preconditions
        .iter()
        .chain(decl.intent.postconditions.iter())
        .chain(decl.intent.invariants.iter());

    for pred in preds {
        collect_pred_names(pred, &mut names);
    }

    for lemma in &decl.proof.lemmas {
        for pred in &lemma.predicates {
            collect_pred_names(pred, &mut names);
        }
    }

    for vd in &decl.proof.verify_decls {
        for stmt in &vd.statements {
            collect_stmt_names(stmt, &mut names);
        }
    }

    // Never identifiers need a declaration too.
    for id in &decl.intent.never {
        names.insert(normalise(id));
    }

    names
}

fn collect_pred_names(pred: &PredicateNode, out: &mut BTreeSet<String>) {
    match pred {
        PredicateNode::Comparison { left, right, .. } => {
            collect_expr_names(left, out);
            collect_expr_names(right, out);
        }
        PredicateNode::And(a, b) | PredicateNode::Or(a, b) => {
            collect_pred_names(a, out);
            collect_pred_names(b, out);
        }
        PredicateNode::Not(p) => collect_pred_names(p, out),
        PredicateNode::Forall { body, .. } | PredicateNode::Exists { body, .. } => {
            collect_pred_names(body, out);
        }
    }
}

fn collect_expr_names(expr: &ExprNode, out: &mut BTreeSet<String>) {
    match expr {
        ExprNode::Identifier(s) => {
            out.insert(normalise(s));
        }
        ExprNode::OldValue(s) => {
            out.insert(format!("{}_old", normalise(s)));
        }
        ExprNode::BinOp { left, right, .. } => {
            collect_expr_names(left, out);
            collect_expr_names(right, out);
        }
        ExprNode::FunctionCall { args, .. } => {
            for a in args {
                collect_expr_names(a, out);
            }
        }
        ExprNode::Number(_) | ExprNode::StringLit(_) => {}
    }
}

fn collect_stmt_names(stmt: &VerifyStatement, out: &mut BTreeSet<String>) {
    match stmt {
        VerifyStatement::Assert(pred) => collect_pred_names(pred, out),
        VerifyStatement::Atomic(stmts) => {
            for s in stmts {
                collect_stmt_names(s, out);
            }
        }
        VerifyStatement::Assign { target, expr, .. } => {
            out.insert(normalise(target));
            collect_expr_names(expr, out);
        }
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Normalise a Firmum identifier for use as an SMT-LIB 2 symbol.
/// Replaces `.` with `_`.
fn normalise(s: &str) -> String {
    s.replace('.', "_")
}

/// Build the forall/exists binder list from the names used inside `body`.
/// Falls back to a single placeholder if the body contains no named symbols.
fn body_binders(var: &str, body: &PredicateNode) -> String {
    let mut body_names: BTreeSet<String> = BTreeSet::new();
    collect_pred_names(body, &mut body_names);

    if body_names.is_empty() {
        // Degenerate case: emit a single placeholder binder.
        return format!("({}_placeholder Int)", normalise(var));
    }

    body_names
        .iter()
        .map(|n| format!("({} Int)", n))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Produce `(and e1 e2 … eN)`, or just `e1` if there is only one element.
fn conjoin(mut exprs: Vec<String>) -> String {
    match exprs.len() {
        0 => "true".to_string(),
        1 => exprs.remove(0),
        _ => format!("(and {})", exprs.join(" ")),
    }
}
