/// FIR lowering unit tests — Stage 2 exit criterion.
///
/// Each test lowers a source fixture and asserts field values on the
/// resulting `Program` FIR node. The TransferFunds fixture mirrors
/// `tests/parse_examples.rs` exactly so the same grammar constraints apply.
use firmum::fir::{
    AssignOp, AssumptionSection, BinOpKind, ComparisonOp, ExprNode, PredicateNode, ProofTechnique,
    SourceType, StrategyName, TemporalType, TimeUnit, TypeNode, ValidatedByField, ValidationMethod,
    VerifyStatement,
};
use firmum::fir::{ContextFieldValue, Number};
use firmum::parser::parse;

// ── helpers ───────────────────────────────────────────────────────────────────

fn lower(src: &str) -> firmum::fir::Program {
    let pairs = parse(src).expect("parse failed");
    firmum::fir::lower::lower(pairs).expect("lower failed")
}

// ── TransferFunds ─────────────────────────────────────────────────────────────

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
fn test_lower_program_structure() {
    let prog = lower(TRANSFER_FUNDS);
    assert_eq!(prog.contexts.len(), 1, "one context_decl");
    assert_eq!(prog.lets.len(), 0, "no let_stmt");
    assert_eq!(prog.declarations.len(), 1, "one declaration");
}

#[test]
fn test_lower_context_decl() {
    let prog = lower(TRANSFER_FUNDS);
    let ctx = &prog.contexts[0];
    assert_eq!(ctx.type_name, "Amount");
    assert_eq!(ctx.context_name, "Banking");
    assert_eq!(ctx.fields.len(), 3);

    assert_eq!(ctx.fields[0].name, "unit");
    assert!(matches!(&ctx.fields[0].value, ContextFieldValue::StringVal(s) if s == "USD"));

    assert_eq!(ctx.fields[1].name, "precision");
    assert!(matches!(
        &ctx.fields[1].value,
        ContextFieldValue::Integer(2)
    ));

    assert_eq!(ctx.fields[2].name, "auditable");
    assert!(matches!(
        &ctx.fields[2].value,
        ContextFieldValue::Boolean(true)
    ));
}

#[test]
fn test_lower_intent_name_and_inputs() {
    let prog = lower(TRANSFER_FUNDS);
    let intent = &prog.declarations[0].intent;
    assert_eq!(intent.name, "TransferFunds");
    assert_eq!(intent.inputs.len(), 3);
    assert_eq!(intent.inputs[0].name, "sender");
    assert_eq!(intent.inputs[1].name, "receiver");
    assert_eq!(intent.inputs[2].name, "amount");
}

#[test]
fn test_lower_intent_input_types() {
    let prog = lower(TRANSFER_FUNDS);
    let intent = &prog.declarations[0].intent;

    // sender : Account where balance >= 0  →  Refined
    assert!(matches!(&intent.inputs[0].ty, TypeNode::Refined { base, .. } if base == "Account"));

    // amount : Amount<Banking>  →  Contextual
    assert!(
        matches!(&intent.inputs[2].ty, TypeNode::Contextual { base, context } if base == "Amount" && context == "Banking")
    );
}

#[test]
fn test_lower_intent_preconditions() {
    let prog = lower(TRANSFER_FUNDS);
    let intent = &prog.declarations[0].intent;
    assert_eq!(intent.preconditions.len(), 2);

    // sender.balance >= amount
    let pre0 = &intent.preconditions[0];
    assert!(
        matches!(
            pre0,
            PredicateNode::Comparison {
                op: ComparisonOp::Ge,
                ..
            }
        ),
        "first precondition is >="
    );

    // amount > 0
    let pre1 = &intent.preconditions[1];
    assert!(
        matches!(
            pre1,
            PredicateNode::Comparison {
                op: ComparisonOp::Gt,
                ..
            }
        ),
        "second precondition is >"
    );
}

#[test]
fn test_lower_intent_postconditions() {
    let prog = lower(TRANSFER_FUNDS);
    let intent = &prog.declarations[0].intent;
    assert_eq!(intent.postconditions.len(), 2);

    // sender.balance == old(sender.balance) - amount
    let post0 = &intent.postconditions[0];
    assert!(
        matches!(
            post0,
            PredicateNode::Comparison {
                op: ComparisonOp::Eq,
                right: ExprNode::BinOp {
                    op: BinOpKind::Sub,
                    ..
                },
                ..
            }
        ),
        "first postcondition: == (old - amount)"
    );
}

#[test]
fn test_lower_intent_never() {
    let prog = lower(TRANSFER_FUNDS);
    let intent = &prog.declarations[0].intent;
    assert_eq!(intent.never, vec!["partial_execution", "silent_failure"]);
}

#[test]
fn test_lower_assumption_name_and_strings() {
    let prog = lower(TRANSFER_FUNDS);
    let assum = &prog.declarations[0].assumption;
    assert_eq!(assum.name, "TransferFunds");

    let strings: Vec<_> = assum
        .sections
        .iter()
        .filter_map(|s| {
            if let AssumptionSection::StringAssumption(t) = s {
                Some(t.as_str())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(strings.len(), 2);
    assert!(strings[0].contains("base currency unit"));
    assert!(strings[1].contains("verified accounts"));
}

#[test]
fn test_lower_assumption_context_source() {
    let prog = lower(TRANSFER_FUNDS);
    let assum = &prog.declarations[0].assumption;

    let refs = assum
        .sections
        .iter()
        .find_map(|s| {
            if let AssumptionSection::ContextSource(r) = s {
                Some(r)
            } else {
                None
            }
        })
        .expect("context_source section");

    assert_eq!(refs.len(), 2);
    assert!(matches!(refs[0].source_type, SourceType::Ref));
    assert_eq!(refs[0].path, "cs-42a9f1b3");
    assert!(matches!(refs[1].source_type, SourceType::Ref));
    assert_eq!(refs[1].path, "cs-7d8c2e91");
}

#[test]
fn test_lower_assumption_out_of_scope() {
    let prog = lower(TRANSFER_FUNDS);
    let assum = &prog.declarations[0].assumption;

    let oos = assum
        .sections
        .iter()
        .find_map(|s| {
            if let AssumptionSection::OutOfScope(items) = s {
                Some(items)
            } else {
                None
            }
        })
        .expect("out_of_scope section");

    assert_eq!(oos.len(), 2);
    assert!(oos[0].contains("multi-currency"));
    assert!(oos[1].contains("cross-border"));
}

#[test]
fn test_lower_assumption_validated_by() {
    let prog = lower(TRANSFER_FUNDS);
    let assum = &prog.declarations[0].assumption;

    let vb = assum
        .sections
        .iter()
        .find_map(|s| {
            if let AssumptionSection::ValidatedBy(v) = s {
                Some(v)
            } else {
                None
            }
        })
        .expect("validated_by section");

    assert_eq!(vb.fields.len(), 4);

    assert!(matches!(&vb.fields[0], ValidatedByField::DomainExpert(s) if s == "Compliance Lead"));
    assert!(matches!(&vb.fields[1], ValidatedByField::Date(d) if d == "2024-03-15"));
    assert!(matches!(&vb.fields[2], ValidatedByField::Confidence(c) if (*c - 0.92).abs() < 1e-9));
    assert!(matches!(
        &vb.fields[3],
        ValidatedByField::Method(ValidationMethod::DocumentReview)
    ));
}

#[test]
fn test_lower_proof_strategy() {
    let prog = lower(TRANSFER_FUNDS);
    let proof = &prog.declarations[0].proof;
    assert_eq!(proof.name, "TransferFunds");
    assert!(matches!(proof.strategy.primary, StrategyName::SmtSolverZ3));
    assert!(matches!(
        proof.strategy.fallback,
        Some(StrategyName::BoundedModelChecking)
    ));
}

#[test]
fn test_lower_proof_lemma() {
    let prog = lower(TRANSFER_FUNDS);
    let proof = &prog.declarations[0].proof;
    assert_eq!(proof.lemmas.len(), 1);

    let lemma = &proof.lemmas[0];
    assert_eq!(lemma.name, "MoneyConservation");
    assert_eq!(lemma.predicates.len(), 1);

    // forall acc: Account => old(acc.balance) == acc.balance
    assert!(
        matches!(&lemma.predicates[0], PredicateNode::Forall { var, .. } if var == "acc"),
        "lemma predicate is forall"
    );

    let method = lemma.proof_method.as_ref().expect("proof_method");
    assert!(
        matches!(method.technique, ProofTechnique::Induction { ref on } if on == "transaction_log")
    );
}

#[test]
fn test_lower_proof_verify_decl() {
    let prog = lower(TRANSFER_FUNDS);
    let proof = &prog.declarations[0].proof;
    assert_eq!(proof.verify_decls.len(), 1);

    let vd = &proof.verify_decls[0];
    assert_eq!(vd.target, "TransferFunds");
    assert_eq!(vd.using.as_deref(), Some("MoneyConservation"));
    // assert, assert, atomic
    assert_eq!(vd.statements.len(), 3);
    assert!(matches!(vd.statements[0], VerifyStatement::Assert(_)));
    assert!(matches!(vd.statements[1], VerifyStatement::Assert(_)));
    assert!(matches!(vd.statements[2], VerifyStatement::Atomic(_)));
}

#[test]
fn test_lower_proof_atomic_assigns() {
    let prog = lower(TRANSFER_FUNDS);
    let proof = &prog.declarations[0].proof;
    let vd = &proof.verify_decls[0];

    if let VerifyStatement::Atomic(stmts) = &vd.statements[2] {
        assert_eq!(stmts.len(), 2);

        // sender.balance -= amount
        assert!(
            matches!(&stmts[0], VerifyStatement::Assign { target, op: AssignOp::SubAssign, .. } if target == "sender.balance"),
            "first atomic assign is -= on sender.balance"
        );

        // receiver.balance += amount
        assert!(
            matches!(&stmts[1], VerifyStatement::Assign { target, op: AssignOp::AddAssign, .. } if target == "receiver.balance"),
            "second atomic assign is += on receiver.balance"
        );
    } else {
        panic!("expected Atomic statement at index 2");
    }
}

#[test]
fn test_lower_proof_certificate() {
    let prog = lower(TRANSFER_FUNDS);
    let proof = &prog.declarations[0].proof;
    let cert = proof.certificate.as_ref().expect("certificate");
    assert!(cert.value.contains("sha256"));
}

// ── isolated feature tests ────────────────────────────────────────────────────

#[test]
fn test_lower_temporal_fresh() {
    let src = r#"
intent Foo {
  input:
    result : Fresh<LabResult, 24h>
}
assumption Foo {}
proof Foo {
  strategy: induction
  verify Foo {}
}
"#;
    let prog = lower(src);
    let ty = &prog.declarations[0].intent.inputs[0].ty;
    assert!(
        matches!(ty, TypeNode::Temporal(TemporalType::Fresh { duration, .. }) if matches!(duration.unit, TimeUnit::Hour) && matches!(duration.value, Number::Integer(24))),
        "Fresh<LabResult, 24h> lowers to Temporal::Fresh with 24h"
    );
}

#[test]
fn test_lower_temporal_stale() {
    let src = r#"
intent Foo {
  input:
    data : Stale<Record>
}
assumption Foo {}
proof Foo {
  strategy: smt_solver(z3)
  verify Foo {}
}
"#;
    let prog = lower(src);
    let ty = &prog.declarations[0].intent.inputs[0].ty;
    assert!(matches!(ty, TypeNode::Temporal(TemporalType::Stale(_))));
}

#[test]
fn test_lower_dependent_type() {
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
    let prog = lower(src);
    let ty = &prog.declarations[0].intent.inputs[0].ty;
    assert!(
        matches!(ty, TypeNode::Dependent { base, value_param_name, value_param_type, .. }
            if base == "Vec" && value_param_name == "n" && value_param_type == "Nat"),
        "Vec<Order, n: Nat> lowers to Dependent"
    );
}

#[test]
fn test_lower_let_binding_typed() {
    let src = r#"
let y : Amount = 100
intent Foo {}
assumption Foo {}
proof Foo {
  strategy: smt_solver(z3)
  verify Foo {}
}
"#;
    let prog = lower(src);
    assert_eq!(prog.lets.len(), 1);
    let lb = &prog.lets[0];
    assert_eq!(lb.name, "y");
    assert!(lb.ty.is_some());
    assert!(matches!(lb.expr, ExprNode::Number(Number::Integer(100))));
}

#[test]
fn test_lower_let_binding_refined() {
    let src = r#"
let z : Int where z > 0 = 5
intent Foo {}
assumption Foo {}
proof Foo {
  strategy: smt_solver(z3)
  verify Foo {}
}
"#;
    let prog = lower(src);
    assert_eq!(prog.lets.len(), 1);
    let lb = &prog.lets[0];
    assert_eq!(lb.name, "z");
    assert!(matches!(&lb.ty, Some(TypeNode::Refined { base, .. }) if base == "Int"));
}

#[test]
fn test_lower_predicate_and_or() {
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
    let prog = lower(src);
    let preds = &prog.declarations[0].intent.preconditions;
    assert_eq!(preds.len(), 2);
    assert!(matches!(preds[0], PredicateNode::And(_, _)));
    assert!(matches!(preds[1], PredicateNode::Or(_, _)));
}

#[test]
fn test_lower_exists_quantifier() {
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
    let prog = lower(src);
    let post = &prog.declarations[0].intent.postconditions[0];
    assert!(
        matches!(post, PredicateNode::Exists { var, .. } if var == "v"),
        "exists quantifier lowered"
    );
}

#[test]
fn test_lower_source_refs_multiple_types() {
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
    let prog = lower(src);
    let assum = &prog.declarations[0].assumption;
    let refs = assum
        .sections
        .iter()
        .find_map(|s| {
            if let AssumptionSection::ContextSource(r) = s {
                Some(r)
            } else {
                None
            }
        })
        .expect("context_source");

    assert_eq!(refs.len(), 3);
    assert!(matches!(refs[0].source_type, SourceType::Ref));
    assert!(matches!(refs[1].source_type, SourceType::Slack));
    assert!(matches!(refs[2].source_type, SourceType::Github));
}

#[test]
fn test_lower_ai_assisted_strategy() {
    let src = r#"
intent Foo {}
assumption Foo {}
proof Foo {
  strategy: ai_assisted
  verify Foo {}
}
"#;
    let prog = lower(src);
    assert!(matches!(
        prog.declarations[0].proof.strategy.primary,
        StrategyName::AiAssisted
    ));
    assert!(prog.declarations[0].proof.strategy.fallback.is_none());
}

#[test]
fn test_lower_lemma_direct_proof() {
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
    let prog = lower(src);
    let lemma = &prog.declarations[0].proof.lemmas[0];
    assert_eq!(lemma.name, "Bar");
    assert!(matches!(
        lemma.proof_method.as_ref().unwrap().technique,
        ProofTechnique::Direct
    ));
}

#[test]
fn test_lower_lemma_contradiction_proof() {
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
    let prog = lower(src);
    let lemma = &prog.declarations[0].proof.lemmas[0];
    assert!(matches!(
        lemma.proof_method.as_ref().unwrap().technique,
        ProofTechnique::Contradiction
    ));
}
