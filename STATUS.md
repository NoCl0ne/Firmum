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

## Next stage

**Stage 2: FIR Lowering** (`src/fir/lower.rs`)

Exit criterion: `cargo test` passes `tests/parse_examples.rs` **and** a new
`tests/lower_unit.rs` that lowers the TransferFunds fixture to a `Program` FIR
node and asserts correct field values (intent name, input count, assumption
section types, proof strategy).

Work to do:
- Implement `fir::lower::lower(pairs)` — walk the pest Pairs tree and produce
  `Program { contexts, lets, declarations }`.
- Handle all grammar rules: `context_decl`, `let_stmt`, `declaration`,
  and all sub-rules recursively.
- Unit test the lowered AST field by field.
