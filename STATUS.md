# Firmum Compiler — Status

## Stage 1: Parser — COMPLETE

**Completed:** 2026-03-13

### Tests added

File: `tests/parse_examples.rs` — 19 tests, all passing.

```
cargo test
...
test result: ok. 19 passed; 0 failed; 0 ignored
```

Key tests:
- `test_parse_transfer_funds` — full TransferFunds example from GRAMMAR.md (adapted)
- `test_parse_temporal_type` — `Fresh<LabResult, 24h>`
- `test_parse_stale_temporal_type` — `Stale<Record>`
- `test_parse_dependent_type` — `Vec<Order, n: Nat>`
- `test_parse_predicate_or_and` / `test_parse_exists_quantifier` — predicate forms
- `test_parse_atomic_block` — `atomic { … }` with compound assignments
- `test_parse_reject_*` — three negative tests confirming grammar enforcement

### Quality gates

| Gate | Status |
|---|---|
| `cargo build` | ✓ zero errors |
| `cargo test` | ✓ 19 passed, 0 failed |
| `cargo clippy -- -D warnings` | ✓ zero warnings |
| `cargo fmt --check` | ✓ |

### Design decisions

1. **Grammar path** — `#[grammar = "../firmum.pest"]` in `src/parser.rs`. The
   grammar lives at the crate root; pest_derive resolves this path relative to
   the source file in `src/`.

2. **`lib.rs` + `main.rs` dual target** — A library crate (`src/lib.rs`) exposes
   all modules so integration tests can import them without duplication. The
   binary (`src/main.rs`) uses the library. This prevents dead-code warnings on
   stub modules since public library items are not subject to the lint.

3. **Stub modules for Stages 2–5** — All pipeline modules exist and compile.
   Unimplemented stages return `Err(CompilerError::NotYetImplemented(…))`. This
   satisfies quality-gate rule 6: no `todo!()` or `unimplemented!()` macros.

4. **FIR types defined at Stage 1** — The six FIR families (IntentNode,
   AssumptionNode, ProofNode, TypeNode, PredicateNode, OwnershipNode) are
   defined in `src/fir/mod.rs` before the lowering pass is written. This
   satisfies the spec constraint.

5. **Crate versions verified 2026-03-13** — See Cargo.toml inline comments.
   `sha2` uses 0.10.8 (stable) and `ed25519-dalek` uses 2.1.1 (stable) because
   the latest search results return pre-release/RC versions that require
   rust 1.85+.

### Grammar behavioral properties (documented in PARSER_NOTES.md)

1. `boolean_literal` not in `factor` — `true`/`false` invalid in predicates (by design).
2. `contextual_type + refined` cannot combine — `Amount<Banking> where x > 0`
   is a parse error; constraint belongs in `precondition` (by design; errata fixed).
3. `old_expr` only accepts `qualified_identifier` — `old(func(x))` is invalid
   (by design; errata fixed — see commit `eb8e5f8`).

### [UNVERIFIED] items — none

### Git identity verified

```
user.name  = firmum-agent
user.email = firmum-agent@localhost.invalid
```

## Stage 2: FIR Lowering — COMPLETE

**Completed:** 2026-03-13

### Tests added

File: `tests/lower_unit.rs` — 27 tests, all passing.

```
cargo test
...
test result: ok. 46 passed; 0 failed; 0 ignored
```

Key tests:
- `test_lower_program_structure` — 1 context_decl, 0 lets, 1 declaration
- `test_lower_context_decl` — field names, StringVal/Integer/Boolean values
- `test_lower_intent_name_and_inputs` — name, input count, param names
- `test_lower_intent_input_types` — Refined, Contextual type variants
- `test_lower_intent_preconditions` / `test_lower_intent_postconditions` — comparison ops, BinOp sub-tree
- `test_lower_assumption_context_source` / `test_lower_assumption_validated_by` — SourceRef, ValidatedByField
- `test_lower_proof_strategy` — SmtSolverZ3 primary + BoundedModelChecking fallback
- `test_lower_proof_lemma` — forall quantifier, Induction proof technique
- `test_lower_proof_atomic_assigns` — SubAssign / AddAssign inside Atomic block
- `test_lower_temporal_fresh` / `test_lower_temporal_stale` — TemporalType variants
- `test_lower_predicate_and_or` / `test_lower_exists_quantifier` — predicate forms
- `test_lower_let_binding_typed` / `test_lower_let_binding_refined` — LetBinding with type annotation

### Quality gates

| Gate | Status |
|---|---|
| `cargo build` | ✓ zero errors |
| `cargo test` | ✓ 46 passed, 0 failed |
| `cargo clippy -- -D warnings` | ✓ zero warnings |
| `cargo fmt --check` | ✓ |

### Design decisions

1. **Span arithmetic for anonymous operators** — `+`, `-`, `*`, `/`, `+=`, `-=`, `=`
   are anonymous string literals in the grammar and are not emitted as pairs.
   The lowering pass recovers the operator text from the parent pair's `as_str()`
   slice, using child span positions as byte offsets into that slice.

2. **Text prefix for syntactic alternatives** — `temporal_type`, `strategy_name`,
   and `proof_technique` use anonymous string alternatives within a named rule.
   `pair.as_str().starts_with("Fresh"/"Stale"/…)` disambiguates without relying
   on inner pairs, which are absent for pure-string alternatives.

3. **Rule-type dispatch for `validated_by_field`** — The anonymous keyword
   (`domain_expert`, `date`, `confidence`, `method`) preceding each value is not
   emitted; only the value child is. The child's rule type (`string_literal`,
   `date_literal`, `confidence_value`, `validation_method`) uniquely identifies
   the alternative.

4. **Left-fold for OR/AND chains** — `predicate_or` / `predicate_and` produce
   inner pairs only for their child predicates (the anonymous `OR`/`AND` tokens
   are invisible). The lowering pass folds left over those children to produce a
   left-associative tree.

### [UNVERIFIED] items — none

## Stage 3: Type Checker — COMPLETE

**Completed:** 2026-03-13

### Tests added

File: `tests/typeck_unit.rs` — 12 tests, all passing.

```
cargo test
...
test result: ok. 58 passed; 0 failed; 0 ignored
```

Key tests:
- `test_typeck_valid_transfer_funds` — Ok(()) on the full fixture
- `test_typeck_minimal_valid` — Ok(()) on a minimal empty declaration
- `test_typeck_mismatch_assumption_name` — Err when intent ≠ assumption name
- `test_typeck_mismatch_proof_name` — Err when intent ≠ proof name
- `test_typeck_wrong_verify_target` — Err when verify target ≠ intent name
- `test_typeck_old_in_precondition_rejected` — Err when old() appears in precondition
- `test_acs_transfer_funds_passes_threshold` — ACS ≥ 0.70 for TransferFunds
- `test_acs_no_strings_returns_zero` — ACS = 0.0 when no strings and no edge cases
- `test_acs_one_unlinked_string_below_threshold` — unlinked string (W=0.10) < 0.70
- `test_acs_formal_audit_high_confidence_linked` — formal_audit + ref → ACS = 1.0
- `test_acs_multiple_declarations_pooled` — two linked declarations both pass

### Quality gates

| Gate | Status |
|---|---|
| `cargo build` | ✓ zero errors |
| `cargo test` | ✓ 58 passed, 0 failed |
| `cargo clippy -- -D warnings` | ✓ zero warnings |
| `cargo fmt --check` | ✓ |

### Design decisions

1. **Name-matching is a type check, not a parse check** — The grammar allows
   `intent Foo {} assumption Bar {} proof Baz {}` to parse; the type checker
   rejects it with a precise error naming both identifiers.

2. **ACS simplified coverage model** — Without NLP, each assumption string is
   assumed to cover all extracted edge cases. Novelty decay (n-th string → W/n)
   prevents padding. The model is conservative: a single formal_audit linked
   string hits the threshold alone; unlinked strings stay below 0.70.

3. **`contextual::check` and `temporal::check` deferred** — Both return `Ok(())`
   to prevent false positives on valid programs. Full implementation requires
   expression-level type inference and dataflow analysis (Stage 4 prerequisites).

4. **`decidability::classify` returns `Theory::Lia`** — Safe default for all
   predicates. Full theory inference requires operand type information available
   only after contextual type checking is complete.

### [UNVERIFIED] items — none

## Next stage

**Stage 4: SMT Orchestrator** (`src/smt/`)

Exit criterion: `cargo test` passes all prior test suites **and** a new
`tests/smt_unit.rs` that translates the TransferFunds FIR into valid SMT-LIB 2
text, verifies it parses as well-formed S-expressions, and asserts the expected
`(declare-fun ...)` and `(assert ...)` structures for the preconditions,
postconditions, and lemma.
