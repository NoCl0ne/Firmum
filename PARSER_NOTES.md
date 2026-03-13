# Parser Notes

Known limitations in `firmum.pest` as of Stage 1. These are grammar constraints,
not bugs. They are documented here to inform test fixture authors and future grammar
evolution.

## Limitation 1 — `boolean_literal` not in `factor`

`boolean_literal` (`true` / `false`) is defined and used only in `context_field`
values. It does not appear in the `factor` rule, so `true` and `false` cannot appear
inside predicate expressions.

**Workaround:** Express boolean conditions via comparison, e.g. `flag == 1`.

**Impact:** The type checker (Stage 3) cannot accept `true`/`false` as predicate
terms. Boolean-valued predicates must be encoded as integer comparisons.

## Limitation 2 — `contextual_type` and `refined_type` cannot combine

The grammar allows either `Amount<Banking>` (contextual) or
`Amount where x > 0` (refined) for a parameter type, but not both together.
`Amount<Banking> where amount > 0` is a parse error: the PEG matches
`Amount<Banking>` as `contextual_type` and stops; the remaining `where amount > 0`
is not consumed by any outer rule before `}`.

**Workaround:** Use `Amount<Banking>` as the parameter type and place
`amount > 0` as a separate `precondition` line.

**Grammar example in GRAMMAR.md §Complete Example** uses
`amount : Amount<Banking> where amount > 0`, which is aspirational prose — the
current grammar does not support it. The test fixture in
`tests/parse_examples.rs` uses the corrected form.

## Limitation 3 — `old_expr` accepts only `qualified_identifier`

`old_expr = { "old" ~ "(" ~ qualified_identifier ~ ")" }`

Only `old(x)` or `old(x.y.z)` forms are valid. `old(func(x))` where the
argument is a function call is a parse error.

**Workaround:** Bind the result to a let-variable and reference the variable
inside `old(…)`.

**Impact:** The GRAMMAR.md lemma example `old(sum(accounts.balance))` does not
parse. The test fixture replaces it with `old(acc.balance)`.
