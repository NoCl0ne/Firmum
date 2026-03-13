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

**Note on the Complete Example in GRAMMAR.md line 521:**
`amount : Amount<Banking> where amount > 0` is aspirational prose that illustrates
the intended domain model; it does not parse with the current grammar. The
precondition `amount > 0` must be written as a separate `precondition:` line
(as shown in the test fixture `tests/parse_examples.rs`). This is a known
discrepancy between the illustrative example and the current grammar rule, not
a grammar bug.

---

## 3 — `old_expr` accepts only `qualified_identifier`

`old_expr = { "old" ~ "(" ~ qualified_identifier ~ ")" }` intentionally restricts
the argument to field references (`x`, `x.y`, `x.y.z`). Arbitrary expressions
such as `old(func(x))` are not accepted. This is correct by design.

**Note on the Complete Example in GRAMMAR.md line 558:**
`old(sum(accounts.balance))` does not parse because `sum(accounts.balance)` is a
`function_call`, not a `qualified_identifier`. This is a known discrepancy between
the illustrative example and the current grammar rule. Postconditions using `old()`
on field references (`old(sender.balance)`, lines 525–526 of the same example) are
valid and parse correctly.
