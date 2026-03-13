# Parser Notes

Behavioural properties of `firmum.pest` that test authors and future contributors
should be aware of. All three are **intentional design decisions**, not open issues.

---

## 1 — `boolean_literal` is used only in `context_field`

`boolean_literal` (`true` / `false`) appears exclusively in `context_field` values.
It does not appear in `factor` and therefore cannot be used inside predicate
expressions. This is correct by design.

From GRAMMAR.md:
> `boolean_literal` is a value, not a type — it does not appear in `type_expr`
> and is used only in `context_field`.

---

## 2 — `refined_type` applies only to base types

`refined_type = { identifier ~ "where" ~ predicate_or }` requires a bare identifier
before `where`. A `contextual_type` (`Amount<Banking>`) or any other compound type
cannot appear in refined position. This is correct by design.

From GRAMMAR.md:
> `refined_type` applies only to base types (identifiers). Constraints on compound
> types such as `Fresh<T, d>` or `Amount<Banking>` belong in the `precondition`
> section of the intent block. This keeps the grammar unambiguous and concentrates
> all constraints in one visible location.

**Resolved spec erratum (fixed 2026-03-13):**
The Complete Example previously used `amount : Amount<Banking> where amount > 0`.
This does not parse: `refined_type` requires a bare identifier before `where`.
The fix moves the constraint to the `precondition` section and updates the example
to `amount : Amount<Banking>` with `amount > 0` as a separate predicate line.
Applied in GRAMMAR.md, README.md, and firmum_arXiv_paper.txt.

---

## 3 — `old_expr` accepts only `qualified_identifier`

`old_expr = { "old" ~ "(" ~ qualified_identifier ~ ")" }` intentionally restricts
the argument to field references (`x`, `x.y`, `x.y.z`). Arbitrary expressions
such as `old(func(x))` are not accepted. This is correct by design.

**Resolved spec erratum (fixed 2026-03-13):**
The Complete Example previously used `old(sum(accounts.balance)) == sum(accounts.balance)`
inside `lemma MoneyConservation`. This does not parse: `old_expr` accepts only
`qualified_identifier`, and `sum(accounts.balance)` is a `function_call`.
The fix replaces the predicate with `old(totalMoneyInSystem) == totalMoneyInSystem`,
which preserves the semantic intent (total money in the system is unchanged) using
only a valid qualified identifier inside `old()`.
Applied in GRAMMAR.md, README.md, and firmum_arXiv_paper.txt.
