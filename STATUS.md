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

### Known grammar limitations (documented in PARSER_NOTES.md)

1. `boolean_literal` not in `factor` — `true`/`false` invalid in predicates.
2. `contextual_type + refined` cannot combine — `Amount<Banking> where x > 0`
   is a parse error; use `Amount<Banking>` + separate precondition.
3. `old_expr` only accepts `qualified_identifier` — `old(func(x))` is invalid.

The GRAMMAR.md §Complete Example uses forms that violate limitations 2 and 3.
Test fixtures use corrected equivalents.

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

## Next stage

**Stage 3: Type Checker** (`src/typeck/`)

Exit criterion: `cargo test` passes `tests/parse_examples.rs`, `tests/lower_unit.rs`,
**and** a new `tests/typeck_unit.rs` that type-checks the lowered TransferFunds
program and asserts: zero errors on a valid program; at least one error when
intent/assumption/proof names mismatch; ACS score ≥ 0.70 for the full fixture.
