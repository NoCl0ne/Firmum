/// Parser integration tests.
///
/// Exit criterion for Stage 1: the full TransferFunds example parses without
/// error. Known grammar limitations (documented in PARSER_NOTES.md):
///
///   1. `boolean_literal` is not in `factor` — true/false cannot appear in
///      predicate expressions.
///   2. `contextual_type` cannot be combined with a `where` predicate —
///      `Amount<Banking> where x > 0` is a parse error. The predicate must
///      be moved to `precondition`.
///   3. `old_expr` only accepts `qualified_identifier` — `old(func(x))` is
///      invalid; use `old(var.field)` instead.
use firmum::parser::parse;

/// TransferFunds adapted for the grammar as written in firmum.pest.
///
/// Differences from the GRAMMAR.md §Complete Example:
///   - `amount : Amount<Banking> where amount > 0`
///     → `amount : Amount<Banking>` (grammar limitation 2)
///     + `amount > 0` added as a separate precondition
///   - `old(sum(accounts.balance))`
///     → `old(acc.balance)` (grammar limitation 3)
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

#[test]
fn test_parse_transfer_funds() {
    let result = parse(TRANSFER_FUNDS);
    assert!(
        result.is_ok(),
        "TransferFunds example must parse without error: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_minimal_declaration() {
    // Minimal syntactically valid declaration — all sections optional.
    let src = r#"
intent Foo {}
assumption Foo {}
proof Foo {
  strategy: smt_solver(z3)
  verify Foo {}
}
"#;
    let result = parse(src);
    assert!(
        result.is_ok(),
        "minimal declaration must parse: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_temporal_type() {
    let src = r#"
intent PrescribeMedication {
  input:
    result : Fresh<LabResult, 24h>
}
assumption PrescribeMedication {}
proof PrescribeMedication {
  strategy: induction
  verify PrescribeMedication {}
}
"#;
    let result = parse(src);
    assert!(
        result.is_ok(),
        "Fresh<T, d> temporal type must parse: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_stale_temporal_type() {
    let src = r#"
intent UseStale {
  input:
    data : Stale<Record>
}
assumption UseStale {}
proof UseStale {
  strategy: smt_solver(z3)
  verify UseStale {}
}
"#;
    let result = parse(src);
    assert!(
        result.is_ok(),
        "Stale<T> temporal type must parse: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_context_decl() {
    let src = r#"
type Amount in context Banking {
  unit: "USD"
  precision: 2
  auditable: true
}
intent Foo {}
assumption Foo {}
proof Foo {
  strategy: smt_solver(z3)
  verify Foo {}
}
"#;
    let result = parse(src);
    assert!(
        result.is_ok(),
        "context_decl must parse: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_let_bindings() {
    let src = r#"
let x = 42
let y : Amount = 100
intent Foo {}
assumption Foo {}
proof Foo {
  strategy: smt_solver(z3)
  verify Foo {}
}
"#;
    let result = parse(src);
    assert!(
        result.is_ok(),
        "let bindings must parse: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_refined_type_let() {
    let src = r#"
let z : Int where z > 0 = 5
intent Foo {}
assumption Foo {}
proof Foo {
  strategy: smt_solver(z3)
  verify Foo {}
}
"#;
    let result = parse(src);
    assert!(
        result.is_ok(),
        "refined type in let binding must parse: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_source_refs() {
    let src = r#"
intent Foo {}
assumption Foo {
  context_source:
    ref#abc-123
    slack#payments-team/2024-03-14
    github#org/repo/issues/42
}
proof Foo {
  strategy: smt_solver(z3)
  verify Foo {}
}
"#;
    let result = parse(src);
    assert!(
        result.is_ok(),
        "source_ref variants must parse: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_all_validation_methods() {
    let src = r#"
intent Foo {}
assumption Foo {
  validated_by:
    method: formal_audit
    confidence: 1.0
}
proof Foo {
  strategy: smt_solver(z3)
  verify Foo {}
}
"#;
    let result = parse(src);
    assert!(
        result.is_ok(),
        "formal_audit method must parse: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_ai_assisted_strategy() {
    let src = r#"
intent Foo {}
assumption Foo {}
proof Foo {
  strategy: ai_assisted
  verify Foo {}
}
"#;
    let result = parse(src);
    assert!(
        result.is_ok(),
        "ai_assisted strategy must parse: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_atomic_block() {
    let src = r#"
intent Foo {}
assumption Foo {}
proof Foo {
  strategy: smt_solver(z3)
  verify Foo {
    atomic {
      x.balance -= amount
      y.balance += amount
    }
  }
}
"#;
    let result = parse(src);
    assert!(
        result.is_ok(),
        "atomic block must parse: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_reject_assumption_before_intent() {
    // Grammar enforces intent → assumption → proof order.
    let src = r#"
assumption Foo {}
intent Foo {}
proof Foo {
  strategy: smt_solver(z3)
  verify Foo {}
}
"#;
    assert!(
        parse(src).is_err(),
        "assumption before intent must be a parse error"
    );
}

#[test]
fn test_parse_reject_missing_proof() {
    let src = r#"
intent Foo {}
assumption Foo {}
"#;
    assert!(
        parse(src).is_err(),
        "declaration without proof block must be a parse error"
    );
}

#[test]
fn test_parse_reject_missing_assumption() {
    let src = r#"
intent Foo {}
proof Foo {
  strategy: smt_solver(z3)
  verify Foo {}
}
"#;
    assert!(
        parse(src).is_err(),
        "declaration without assumption block must be a parse error"
    );
}

#[test]
fn test_parse_lemma_direct_proof() {
    let src = r#"
intent Foo {}
assumption Foo {}
proof Foo {
  strategy: smt_solver(z3)
  lemma Bar {
    x > 0
    proof: direct
  }
  verify Foo {}
}
"#;
    let result = parse(src);
    assert!(
        result.is_ok(),
        "lemma with direct proof must parse: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_lemma_contradiction_proof() {
    let src = r#"
intent Foo {}
assumption Foo {}
proof Foo {
  strategy: bounded_model_checking
  lemma Baz {
    x != 0
    proof: contradiction
  }
  verify Foo {}
}
"#;
    let result = parse(src);
    assert!(
        result.is_ok(),
        "lemma with contradiction proof must parse: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_dependent_type() {
    let src = r#"
intent Foo {
  input:
    items : Vec<Order, n: Nat>
}
assumption Foo {}
proof Foo {
  strategy: smt_solver(z3)
  verify Foo {}
}
"#;
    let result = parse(src);
    assert!(
        result.is_ok(),
        "dependent type Vec<T, n: Nat> must parse: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_predicate_or_and() {
    let src = r#"
intent Foo {
  precondition:
    x > 0 AND y > 0
    a == b OR c == d
}
assumption Foo {}
proof Foo {
  strategy: smt_solver(z3)
  verify Foo {}
}
"#;
    let result = parse(src);
    assert!(
        result.is_ok(),
        "AND/OR predicates must parse: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_exists_quantifier() {
    let src = r#"
intent Foo {
  postcondition:
    exists v: Value => v > 0
}
assumption Foo {}
proof Foo {
  strategy: smt_solver(z3)
  verify Foo {}
}
"#;
    let result = parse(src);
    assert!(
        result.is_ok(),
        "exists quantifier must parse: {:?}",
        result.err()
    );
}
