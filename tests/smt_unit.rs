/// SMT Orchestrator unit tests — Stage 4 exit criterion.
///
/// Emission tests (no Z3 required):
///   1. Emitted text contains `declare-fun` entries for referenced identifiers.
///   2. Precondition assertions match the expected SMT-LIB 2 form.
///   3. Postcondition old-value constants appear in the output.
///   4. The lemma quantifier (`forall`) is present.
///   5. `(check-sat)` closes the query.
///   6. The output is well-formed: open and close parentheses are balanced.
///
/// Z3 invocation test (requires Z3 binary on PATH):
///   7. `test_z3_trivial_unsat` — #[ignore] because Z3 is not installed.
///      Once Z3 is available, remove the #[ignore] annotation.
use firmum::parser::parse;
use firmum::smt;

// ── helpers ───────────────────────────────────────────────────────────────────

fn lower(src: &str) -> firmum::fir::Program {
    let pairs = parse(src).expect("parse failed");
    firmum::fir::lower::lower(pairs).expect("lower failed")
}

// ── fixture ───────────────────────────────────────────────────────────────────

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

// ── emission tests (no Z3 required) ──────────────────────────────────────────

#[test]
fn test_emit_transfer_funds_has_declarations() {
    let prog = lower(TRANSFER_FUNDS);
    let decl = &prog.declarations[0];
    let text = smt::emit::emit_declaration(decl);
    assert!(
        text.contains("declare-fun"),
        "output must contain (declare-fun ...) entries; got:\n{text}"
    );
    assert!(
        text.contains("sender_balance"),
        "sender.balance must appear as sender_balance; got:\n{text}"
    );
    assert!(
        text.contains("amount"),
        "amount must appear in declarations; got:\n{text}"
    );
}

#[test]
fn test_emit_transfer_funds_preconditions() {
    let prog = lower(TRANSFER_FUNDS);
    let decl = &prog.declarations[0];
    let text = smt::emit::emit_declaration(decl);
    assert!(
        text.contains("(assert (>= sender_balance amount))"),
        "first precondition must be (assert (>= sender_balance amount)); got:\n{text}"
    );
    assert!(
        text.contains("(assert (> amount 0))"),
        "second precondition must be (assert (> amount 0)); got:\n{text}"
    );
}

#[test]
fn test_emit_transfer_funds_postconditions() {
    let prog = lower(TRANSFER_FUNDS);
    let decl = &prog.declarations[0];
    let text = smt::emit::emit_declaration(decl);
    assert!(
        text.contains("sender_balance_old"),
        "old(sender.balance) must appear as sender_balance_old; got:\n{text}"
    );
    assert!(
        text.contains("receiver_balance_old"),
        "old(receiver.balance) must appear as receiver_balance_old; got:\n{text}"
    );
    // The postcondition block should be wrapped in (assert (not ...))
    assert!(
        text.contains("(assert (not"),
        "postconditions must be wrapped in (assert (not ...)); got:\n{text}"
    );
}

#[test]
fn test_emit_transfer_funds_lemma() {
    let prog = lower(TRANSFER_FUNDS);
    let decl = &prog.declarations[0];
    let text = smt::emit::emit_declaration(decl);
    assert!(
        text.contains("forall"),
        "MoneyConservation lemma must emit a forall quantifier; got:\n{text}"
    );
    assert!(
        text.contains("acc_balance"),
        "forall body must reference acc.balance as acc_balance; got:\n{text}"
    );
}

#[test]
fn test_emit_check_sat_present() {
    let prog = lower(TRANSFER_FUNDS);
    let decl = &prog.declarations[0];
    let text = smt::emit::emit_declaration(decl);
    assert!(
        text.contains("(check-sat)"),
        "output must end with (check-sat); got:\n{text}"
    );
}

#[test]
fn test_emit_well_formed_s_expressions() {
    let prog = lower(TRANSFER_FUNDS);
    let decl = &prog.declarations[0];
    let text = smt::emit::emit_declaration(decl);
    let open = text.chars().filter(|&c| c == '(').count();
    let close = text.chars().filter(|&c| c == ')').count();
    assert_eq!(
        open, close,
        "SMT-LIB 2 output must have balanced parentheses; \
         found {open} '(' and {close} ')'"
    );
}

// ── Z3 invocation test ────────────────────────────────────────────────────────

/// Submit a trivially unsatisfiable formula and assert Z3 returns "unsat".
///
/// BLOCKED: Z3 is not installed on this system (`which z3` → not found).
/// Remove the `#[ignore]` annotation once Z3 is available on PATH.
#[test]
#[ignore = "Z3 binary not found on PATH; install Z3 to enable live SMT tests"]
fn test_z3_trivial_unsat() {
    // (assert false) is trivially unsatisfiable: no model can satisfy false.
    let smtlib = "(set-logic QF_LIA)\n(assert false)\n(check-sat)\n";
    let result =
        smt::run::run_z3(smtlib).expect("z3 invocation should succeed when Z3 is installed");
    assert_eq!(
        result,
        smt::run::SmtResult::Unsat,
        "(assert false) must produce unsat"
    );
}
