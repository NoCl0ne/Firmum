/// Type checker unit tests — Stage 3 exit criterion.
///
/// Asserts:
///   1. zero errors on a valid program (TransferFunds)
///   2. TypeCheckError when intent/assumption names mismatch
///   3. TypeCheckError when intent/proof names mismatch
///   4. TypeCheckError when verify target does not match the intent
///   5. TypeCheckError when old() appears in a precondition
///   6. ACS ≥ 0.70 for the full TransferFunds fixture
///   7. ACS = 0.0 when the assumption block has no edge cases and no strings
///   8. TypeCheckError for cross-context arithmetic (contextual.rs)
///   9. TypeCheckError for Fresh/Stale conflict (temporal.rs)
///  10. decidability::classify returns Ok without panicking
use firmum::parser::parse;
use firmum::typeck;
use firmum::typeck::acs;
use firmum::typeck::decidability;

// ── helpers ───────────────────────────────────────────────────────────────────

fn lower(src: &str) -> firmum::fir::Program {
    let pairs = parse(src).expect("parse failed");
    firmum::fir::lower::lower(pairs).expect("lower failed")
}

// ── fixtures ──────────────────────────────────────────────────────────────────

const TRANSFER_FUNDS: &str = r#"
type Amount in context Banking {
  unit:      "USD"
  precision: 2
  auditable: true
}

intent TransferFunds {
  input:
    sender   : Account where balance >= 0
    receiver : Account where id != sender.id
    amount   : Amount<Banking>
  precondition:
    sender.balance >= amount
    amount > 0
  postcondition:
    sender.balance   == old(sender.balance) - amount
    receiver.balance == old(receiver.balance) + amount
  invariant:
    totalMoneyInSystem == const
  never:
    partial_execution
    silent_failure
}

assumption TransferFunds {
  "amount is the base currency unit, not fractional"
  "sender and receiver are verified accounts in the same system"

  context_source:
    ref#cs-42a9f1b3
    ref#cs-7d8c2e91

  out_of_scope:
    "multi-currency conversion"
    "cross-border regulatory requirements"

  validated_by:
    domain_expert: "Compliance Lead"
    date: 2024-03-15
    confidence: 0.92
    method: document_review
}

proof TransferFunds {
  strategy: smt_solver(z3) with fallback(bounded_model_checking)

  lemma MoneyConservation {
    forall acc: Account =>
      old(acc.balance) == acc.balance
    proof: induction on transaction_log
  }

  verify TransferFunds using MoneyConservation {
    assert sender.balance >= amount
    assert sender.id != receiver.id
    atomic {
      sender.balance   -= amount
      receiver.balance += amount
    }
  }

  certificate: "sha256:<compiler-generated>" verified_at: compile_time
}
"#;

/// Minimal valid declaration — empty intent/assumption/proof all named Foo.
const MINIMAL_VALID: &str = r#"
intent Foo {}
assumption Foo {}
proof Foo {
  strategy: smt_solver(z3)
  verify Foo {}
}
"#;

/// Names do NOT match: intent=Foo, assumption=Bar.
const MISMATCH_ASSUMPTION: &str = r#"
intent Foo {}
assumption Bar {}
proof Foo {
  strategy: smt_solver(z3)
  verify Foo {}
}
"#;

/// Names do NOT match: intent=Foo, proof=Baz.
const MISMATCH_PROOF: &str = r#"
intent Foo {}
assumption Foo {}
proof Baz {
  strategy: smt_solver(z3)
  verify Foo {}
}
"#;

/// verify target does not match the intent name.
const WRONG_VERIFY_TARGET: &str = r#"
intent Foo {}
assumption Foo {}
proof Foo {
  strategy: smt_solver(z3)
  verify NotFoo {}
}
"#;

/// old() inside a precondition — type error.
const OLD_IN_PRECONDITION: &str = r#"
intent Foo {
  input:
    x : Int
  precondition:
    old(x) == x
}
assumption Foo {}
proof Foo {
  strategy: smt_solver(z3)
  verify Foo {}
}
"#;

/// Minimal assumption with no strings and no edge cases.
const NO_ASSUMPTIONS: &str = r#"
intent Foo {}
assumption Foo {}
proof Foo {
  strategy: smt_solver(z3)
  verify Foo {}
}
"#;

/// Two contextual params with the SAME context — no conflict.
const SAME_CONTEXT_PARAMS: &str = r#"
intent Foo {
  input:
    a : Amount<Banking>
    b : Amount<Banking>
  postcondition:
    a == b
}
assumption Foo {}
proof Foo {
  strategy: smt_solver(z3)
  verify Foo {}
}
"#;

/// Two contextual params with DIFFERENT contexts for the same base type.
/// The postcondition compares them directly → cross-context arithmetic error.
const CROSS_CONTEXT_PARAMS: &str = r#"
intent Foo {
  input:
    a : Amount<Banking>
    b : Amount<Crypto>
  postcondition:
    a == b
}
assumption Foo {}
proof Foo {
  strategy: smt_solver(z3)
  verify Foo {}
}
"#;

/// Two temporal params with the SAME state (both Fresh) — no conflict.
const SAME_TEMPORAL_STATE: &str = r#"
intent Foo {
  input:
    x : Fresh<LabResult, 24h>
    y : Fresh<LabResult, 48h>
  postcondition:
    x == y
}
assumption Foo {}
proof Foo {
  strategy: smt_solver(z3)
  verify Foo {}
}
"#;

/// Fresh and Stale params of the same base type in one comparison → error.
const STALE_WHERE_FRESH_REQUIRED: &str = r#"
intent Foo {
  input:
    fresh_data : Fresh<LabResult, 24h>
    stale_data : Stale<LabResult>
  postcondition:
    fresh_data == stale_data
}
assumption Foo {}
proof Foo {
  strategy: smt_solver(z3)
  verify Foo {}
}
"#;

/// One unlinked string assumption (no context_source, no validated_by).
const ONE_UNLINKED_STRING: &str = r#"
intent Foo {
  never:
    forbidden_action
}
assumption Foo {
  "the system does not perform forbidden_action"
}
proof Foo {
  strategy: smt_solver(z3)
  verify Foo {}
}
"#;

// ── name-matching tests ───────────────────────────────────────────────────────

#[test]
fn test_typeck_valid_transfer_funds() {
    let prog = lower(TRANSFER_FUNDS);
    assert!(
        typeck::check(&prog).is_ok(),
        "TransferFunds should pass the type checker"
    );
}

#[test]
fn test_typeck_minimal_valid() {
    let prog = lower(MINIMAL_VALID);
    assert!(typeck::check(&prog).is_ok(), "minimal valid should pass");
}

#[test]
fn test_typeck_mismatch_assumption_name() {
    let prog = lower(MISMATCH_ASSUMPTION);
    let err = typeck::check(&prog).expect_err("expected TypeCheckError");
    let msg = err.to_string();
    assert!(
        msg.contains("Foo") && msg.contains("Bar"),
        "error should mention both names; got: {msg}"
    );
}

#[test]
fn test_typeck_mismatch_proof_name() {
    let prog = lower(MISMATCH_PROOF);
    let err = typeck::check(&prog).expect_err("expected TypeCheckError");
    let msg = err.to_string();
    assert!(
        msg.contains("Foo") && msg.contains("Baz"),
        "error should mention both names; got: {msg}"
    );
}

#[test]
fn test_typeck_wrong_verify_target() {
    let prog = lower(WRONG_VERIFY_TARGET);
    let err = typeck::check(&prog).expect_err("expected TypeCheckError for wrong verify target");
    let msg = err.to_string();
    assert!(
        msg.contains("NotFoo"),
        "error should mention the mismatched target; got: {msg}"
    );
}

#[test]
fn test_typeck_old_in_precondition_rejected() {
    let prog = lower(OLD_IN_PRECONDITION);
    let err = typeck::check(&prog).expect_err("old() in precondition should be a type error");
    assert!(
        err.to_string().contains("precondition"),
        "error should mention precondition; got: {err}"
    );
}

// ── ACS tests ─────────────────────────────────────────────────────────────────

#[test]
fn test_acs_transfer_funds_passes_threshold() {
    let prog = lower(TRANSFER_FUNDS);
    let score = acs::compute(&prog).expect("ACS computation failed");
    assert!(
        score >= acs::THRESHOLD_PASS,
        "TransferFunds ACS {score:.4} is below the 0.70 threshold"
    );
}

#[test]
fn test_acs_transfer_funds_score_in_range() {
    let prog = lower(TRANSFER_FUNDS);
    let score = acs::compute(&prog).expect("ACS computation failed");
    assert!(
        (0.0..=1.0).contains(&score),
        "ACS must be in [0, 1]; got {score}"
    );
}

#[test]
fn test_acs_no_strings_returns_zero() {
    let prog = lower(NO_ASSUMPTIONS);
    let score = acs::compute(&prog).expect("ACS computation failed");
    assert_eq!(score, 0.0, "no strings and no edge cases → ACS = 0");
}

#[test]
fn test_acs_one_unlinked_string_below_threshold() {
    let prog = lower(ONE_UNLINKED_STRING);
    let score = acs::compute(&prog).expect("ACS computation failed");
    // one never id → 1 edge case; one unlinked string → W=0.10; ACS = 0.10
    assert!(
        score < acs::THRESHOLD_PASS,
        "one unlinked string should be below 0.70; got {score:.4}"
    );
    assert!(
        score > 0.0,
        "one unlinked string covering a never id → ACS > 0"
    );
}

#[test]
fn test_acs_formal_audit_high_confidence_linked() {
    let src = r#"
intent Foo {
  never:
    forbidden_op
}
assumption Foo {
  "formal audit of all forbidden operations"

  context_source:
    ref#audit-001

  validated_by:
    method: formal_audit
    confidence: 0.99
}
proof Foo {
  strategy: smt_solver(z3)
  verify Foo {}
}
"#;
    let prog = lower(src);
    let score = acs::compute(&prog).expect("ACS computation failed");
    // W = 1.00 × 0.99 × 1.15 ≈ 1.139 → capped at 1.0; 1 edge case → ACS = 1.0
    assert!(
        score >= acs::THRESHOLD_PASS,
        "formal_audit linked string should exceed threshold; got {score:.4}"
    );
}

#[test]
fn test_acs_multiple_declarations_pooled() {
    let src = r#"
intent A {
  never:
    bad_a
}
assumption A {
  "assumption about A"
  context_source:
    ref#a-001
  validated_by:
    method: document_review
    confidence: 0.90
}
proof A {
  strategy: smt_solver(z3)
  verify A {}
}

intent B {
  never:
    bad_b
}
assumption B {
  "assumption about B"
  context_source:
    ref#b-001
  validated_by:
    method: document_review
    confidence: 0.90
}
proof B {
  strategy: smt_solver(z3)
  verify B {}
}
"#;
    let prog = lower(src);
    assert!(typeck::check(&prog).is_ok());
    let score = acs::compute(&prog).expect("ACS failed");
    assert!(
        score >= acs::THRESHOLD_PASS,
        "two covered declarations → ACS ≥ 0.70; got {score:.4}"
    );
}

// ── contextual type tests ──────────────────────────────────────────────────────

#[test]
fn test_contextual_same_context_ok() {
    let prog = lower(SAME_CONTEXT_PARAMS);
    assert!(
        typeck::check(&prog).is_ok(),
        "two params with the same context should pass contextual check"
    );
}

#[test]
fn test_contextual_cross_context_rejected() {
    let prog = lower(CROSS_CONTEXT_PARAMS);
    let err = typeck::check(&prog).expect_err("cross-context arithmetic must be a type error");
    let msg = err.to_string();
    assert!(
        msg.contains("Banking") && msg.contains("Crypto"),
        "error should name both conflicting contexts; got: {msg}"
    );
}

// ── temporal type tests ────────────────────────────────────────────────────────

#[test]
fn test_temporal_same_state_ok() {
    let prog = lower(SAME_TEMPORAL_STATE);
    assert!(
        typeck::check(&prog).is_ok(),
        "two Fresh params with the same base should pass temporal check"
    );
}

#[test]
fn test_temporal_stale_where_fresh_rejected() {
    let prog = lower(STALE_WHERE_FRESH_REQUIRED);
    let err = typeck::check(&prog).expect_err("Stale where Fresh is required must be a type error");
    let msg = err.to_string();
    assert!(
        msg.contains("Stale") || msg.contains("temporal"),
        "error should mention temporal conflict; got: {msg}"
    );
}

// ── decidability tests ─────────────────────────────────────────────────────────

#[test]
fn test_decidability_classify_returns_value() {
    // classify is a conservative stub that returns Theory::Lia for all predicates.
    // This test verifies the function is callable and does not panic.
    let prog = lower(MINIMAL_VALID);
    // Use a comparison predicate from a lowered program if available,
    // or construct one directly via the public FIR types.
    use firmum::fir::{ComparisonOp, ExprNode, Number, PredicateNode};
    let pred = PredicateNode::Comparison {
        left: ExprNode::Number(firmum::fir::Number::Integer(0)),
        op: ComparisonOp::Ge,
        right: ExprNode::Number(Number::Integer(0)),
    };
    let result = decidability::classify(&pred);
    assert!(
        result.is_ok(),
        "classify should return Ok for any predicate; got: {result:?}"
    );
    drop(prog);
}
