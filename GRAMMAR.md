# Firmum Grammar Specification

**File extension:** `.frm`  
**Encoding:** UTF-8  
**Parser:** [pest](https://pest.rs/) (PEG, v2.8+)

This document is the authoritative prose specification for Firmum grammar.
The reference implementation is `firmum.pest`. Where the two conflict,
`firmum.pest` governs.

---

## Top-Level Structure

A Firmum source file is a sequence of `context_decl`, `let_stmt`, and `declaration`
entries. At least one `declaration` is required for a compilable program — a file
containing only `let_stmt` or `context_decl` is syntactically valid but the type
checker requires at least one declaration.

Each declaration is a matched triple — `intent`, `assumption`, `proof` — sharing the
same identifier. All three are mandatory. **Order is enforced by the grammar.** Any
other sequence is a parse error.

```ebnf
program     = (context_decl | let_stmt | declaration)+ EOF

declaration = intent_block
              assumption_block
              proof_block

identifier       = !(keyword boundary) ALPHA (ALPHA | DIGIT | "_")*
string_literal   = '"' (non-quote-non-backslash | "\" ANY)* '"'
integer          = DIGIT+
decimal          = DIGIT+ "." DIGIT+
number           = decimal | integer
boolean_literal  = ("true" | "false") boundary
date_literal     = DIGIT{4} "-" DIGIT{2} "-" DIGIT{2}
```

`string_literal` supports escape sequences. A backslash followed by any character is
consumed as a single escaped character, including `\"` for embedded quotes. Unescaped
quotes terminate the string.

`boolean_literal` requires a word boundary after `true` or `false` to prevent partial
matches on identifiers such as `trueness`. It is a value, not a type — it does not
appear in `type_expr` and is used only in `context_field`.

---

## INTENT Block

Sections within `intent_block` have a fixed order enforced by the grammar.

```ebnf
intent_block = "intent" identifier "{" intent_body "}"

intent_body  = input_section?
               output_section?
               precondition_section?
               postcondition_section?
               invariant_section*
               never_section?

input_section  = "input"  ":" param+
output_section = "output" ":" param+

param = identifier ":" type_expr

precondition_section  = "precondition"  ":" predicate_line+
postcondition_section = "postcondition" ":" predicate_line+
invariant_section     = "invariant"     ":" predicate_line+

predicate_line = !(section_keyword boundary | "}") predicate_or

never_section = "never" ":" never_id+
never_id      = !"}" identifier
```

`predicate_line` uses a negative lookahead on section keywords to prevent greedy
`predicate_or+` rules from consuming the keyword that opens the next section.
`never_id` uses `!"}"` as its only lookahead because all block and section keywords
are reserved — `identifier` rejects them already, making a boundary check unnecessary.

The `never` block accepts free identifiers. The registry of valid forbidden behaviors
is the responsibility of domain libraries (`stdlib/finance`, `stdlib/medical`,
`stdlib/security`), not the core grammar. This preserves extensibility without
grammar churn when new domain behaviors are defined.

---

## Type Expressions

```ebnf
type_expr = temporal_type
          | contextual_type
          | dependent_type
          | refined_type
          | base_type

base_type = identifier

refined_type = identifier "where" predicate_or

dependent_type = identifier "<" type_expr "," identifier ":" base_type ">"

contextual_type = identifier "<" identifier ">"

temporal_type = "Fresh"    "<" type_expr "," duration ">"
              | "Expiring" "<" type_expr "," duration ">"
              | "Stale"    "<" type_expr ">"

duration  = number time_unit
time_unit = "ms" | "min" | "s" | "h" | "d"
```

`refined_type` applies only to base types. Constraints on compound types such as
`Fresh<T, d>` belong in the `precondition` section of the intent block. This is a
deliberate design decision: it keeps the grammar unambiguous, avoids nested predicate
parsing inside type expressions, and concentrates all constraints in one visible
location.

`dependent_type` requires exactly two parameters: a type and a named value parameter
with a colon (`Vec<T, n: Nat>`). A single-parameter form matches `contextual_type`.
The ordered PEG choice must attempt `dependent_type` before `contextual_type` because
both begin with `identifier "<"`.

`time_unit` tokens (`ms`, `min`, `s`, `h`, `d`) are not reserved keywords — they
appear only after `number` in the `duration` context and are disambiguated
structurally. `ms` must precede `s` and `min` must precede any future `m`-prefixed
unit in the ordered choice to avoid partial consumption.

**Temporal state transitions** are tracked by the type checker, not the grammar.
The type checker transitions `Fresh<T, d>` to `Expiring<T, d>` as expiry approaches
and to `Stale<T>` once the duration elapses. Transition points are encoded as
**symbolic constants in the SMT-LIB emission layer**, allowing Z3 to reason about
expiry conditions within proofs. Using `Stale<T>` where `Fresh<T, d>` is required is
a compile-time type error. `Expiring<T, d>` requires explicit handling — it cannot be
passed where `Fresh<T, d>` is required without acknowledgment.

---

## Predicates and Expressions

Predicates are **stratified by precedence** to eliminate PEG ambiguity and
left-recursion. Precedence from lowest to highest: OR < AND < atoms.

```ebnf
predicate_or  = predicate_and ("OR"  predicate_and)*
predicate_and = predicate_atom ("AND" predicate_atom)*

predicate_atom = "!" predicate_atom
               | "forall" identifier ":" type_expr "=>" predicate_atom
               | "exists" identifier ":" type_expr "=>" predicate_atom
               | "(" predicate_or ")"
               | comparison

comparison    = expr comparison_op expr
comparison_op = "==" | "!=" | "<=" | ">=" | "<" | ">"

expr   = term   (("+" | "-") term)*
term   = factor (("*" | "/") factor)*
factor = number
       | string_literal
       | old_expr
       | function_call
       | qualified_identifier
       | "(" expr ")"

old_expr             = "old" "(" qualified_identifier ")"
qualified_identifier = identifier ("." identifier)*
function_call        = identifier "(" (expr ("," expr)*)? ")"
```

`old(x)` references the value of `x` at the start of execution. It is valid only
inside `postcondition` and `verify` blocks — enforced by the type checker, not the
grammar. Semantic rejection at the correct call site produces a better diagnostic
than grammar contortion.

`function_call` must be attempted before `qualified_identifier` in the ordered PEG
choice because both begin with an identifier. If `qualified_identifier` were tried
first, a function call would be partially consumed and the open parenthesis would
become an error.

`<=` must precede `<`, and `>=` must precede `>` in the `comparison_op` ordered
choice; otherwise `<=` parses as `<` followed by a stray `=`.

---

## ASSUMPTION Block

Sections within `assumption_block` are in **free order**. The grammar does not enforce
any ordering among the four section types. Free order is intentional: a fixed section
order would cause silent data loss when a developer writes sections out of the expected
sequence — `out_of_scope` before `context_source` would parse successfully but silently
lower the ACS score without a diagnostic. Duplicate sections are syntactically valid
and flagged as semantic errors by the type checker.

```ebnf
assumption_block = "assumption" identifier "{" assumption_section* "}"

assumption_section = assumption_string
                   | context_source_section
                   | out_of_scope_section
                   | validated_by_section

assumption_string      = string_literal
context_source_section = "context_source" ":" source_ref+
out_of_scope_section   = "out_of_scope"   ":" string_literal+
```

### context_source and GDPR Indirection

`source_ref` accepts a structured form: `source_type "#" source_path`.

```ebnf
source_ref  = source_type "#" source_path
source_type = "ref" | "slack" | "email" | "github" | "jira" | "doc"
source_path = (ALNUM | "-" | "_" | "/" | "." | "@")+
```

**`ref` is the required `source_type` in all committed `.frm` files.** The form
`ref#<id>` is an opaque identifier — a content-addressed registry key. The mapping
from that key to the real source artefact (a Slack thread, an email, a document) lives
in the **ContextRegistry**: a separate, GDPR-scoped, access-controlled store with a
`retention_policy` and a `personal_data` flag per entry. The compiler consumes only
the SHA-256 content hash from the registry; it never reads the text of referenced
documents.

Structured source types (`slack`, `email`, `github`, `jira`, `doc`) are accepted by
the grammar for local development and migration tooling. They must not appear in source
files entering a build pipeline because they create machine-readable links to artefacts
that may contain personal data.

`source_type` tokens are not reserved keywords — they appear only before `#` in
`source_ref` and are disambiguated structurally. Identifiers such as `email_address`,
`doc_id`, `github_user`, and `ref_id` remain valid.

`"@"` in `source_path` enables email-style references
(`email#user@company.com/thread-id`). Per-type path validation is a type-checker or
CLI responsibility, not a grammar responsibility.

### validated_by

```ebnf
validated_by_section = "validated_by" ":" validated_by_field*

validated_by_field = "domain_expert" ":" string_literal
                   | "date"          ":" date_literal
                   | "confidence"    ":" confidence_value
                   | "method"        ":" validation_method

confidence_value  = ("0" | "1") ("." DIGIT*)?
validation_method = "interview" | "document_review"
                  | "formal_audit" | "peer_review"
```

`validated_by:` with no fields is syntactically valid. The compiler must emit an
explicit warning in this case because the ACS contribution is zero and the developer
may believe validation has been satisfied.

`confidence_value` is syntactically constrained to values beginning with `0` or `1`,
producing an immediate parse error for inputs such as `2.5` or `-0.1`. The range
constraint `[0.0, 1.0]` is enforced by the type checker. `1.999` is syntactically
valid; the type checker rejects it with a precise, correctly-located diagnostic.

`method` determines the base weight `W(aᵢ)` in the ACS formula. See the next section.

---

## Assumption Coverage Score

The ACS quantifies how well the declared assumptions cover the edge cases implied by
the intent block. The compiler computes it at build time using:

```
ACS = Σᵢ [ W(aᵢ) · |map(aᵢ) ∩ E| ] / |E|
```

where `E` is the set of edge cases extracted statically from the intent block, and
`map(aᵢ)` is the set of edge cases covered by assumption string `aᵢ`.

**Edge case extraction rules applied to the intent block:**

1. **Boundary rule** — for each refined type predicate `x OP k`, extract `x = k`
   and `x = k ± ε`.
2. **Forbidden behavior rule** — each identifier in the `never` block generates one
   edge case.
3. **Old-value delta rule** — for each postcondition `x == old(x) ± expr`, extract
   `expr = 0`, `expr = MAX_TYPE`, `old(x) = 0`.
4. **Context disjointness rule** — for each pair of contextual parameters sharing the
   same base type, one cross-context edge case.

**Effective weight `W(aᵢ)`:**

| Source | Base weight | Notes |
|---|---|---|
| `formal_audit` | 1.00 | × `confidence` |
| `document_review` | 0.80 | × `confidence` |
| `peer_review` | 0.60 | × `confidence` |
| `interview` | 0.35 | × `confidence` |
| Unlinked string | 0.10 | Fixed; `confidence` ignored |

A valid `context_source` reference (`ref#…`) applies a **1.15× traceability bonus**
to the effective weight.

**Anti-Goodhart controls:**

- **Semantic deduplication** — two assumption strings with cosine similarity > 0.90
  are treated as a single assumption for scoring purposes.
- **Novelty decay** — the n-th string covering the same edge case contributes
  `W(aₙ) / n`.

**Build threshold policy:**

| ACS range | Effect |
|---|---|
| ≥ 0.70 | Pass |
| [0.50, 0.70) | Build error |
| < 0.50 | Hard error + mandatory human review gate |

---

## PROOF Block

```ebnf
proof_block = "proof" identifier "{" proof_body "}"

proof_body  = strategy_decl
              lemma_decl*
              verify_decl+
              certificate_decl?

strategy_decl = "strategy" ":" strategy_expr
strategy_expr = strategy_name ("with" "fallback" "(" strategy_name ")")?
strategy_name = "smt_solver" "(" "z3" ")"
              | "bounded_model_checking"
              | "induction"
              | "ai_assisted"
```

`smt_solver(z3)` is the only SMT solver supported. `"z3"` is a literal string in the
grammar, not an identifier — `smt_solver(z4)` is a parse error. Future solver support
requires an explicit grammar extension.

`ai_assisted` implements a CEGIS+LLM healing loop (planned): Z3 produces a
counterexample, which is passed to an LLM synthesizer that proposes a reformulated
proof block. The reformulated block is re-submitted to Z3. This cycle repeats up to
K = 5 iterations. If the limit is reached without a closed proof, the module escalates
to the human fallback protocol. Modules verified only by `ai_assisted` cannot receive
an `org_signature` without explicit human approval.

```ebnf
lemma_decl    = "lemma" identifier "{" predicate_or+ proof_method? "}"
proof_method  = "proof" ":" proof_technique
proof_technique = "induction" "on" identifier
                | "contradiction"
                | "direct"
```

`predicate_or+` in `lemma_decl` terminates correctly before `proof_method` because
`"proof"` is reserved — `identifier` rejects it, so `predicate_or` cannot consume it.

`"contradiction"` and `"direct"` are string literals, not symbol references. A lemma
named `contradiction` or `direct` is syntactically valid; the type checker should warn
on names coinciding with `proof_technique` terms.

```ebnf
verify_decl = "verify" identifier ("using" identifier)?
              "{" verify_statement* "}"

verify_statement = assert_stmt
                 | atomic_stmt
                 | assign_stmt

assert_stmt = "assert" predicate_or
atomic_stmt = "atomic" "{" verify_statement+ "}"
assign_stmt = qualified_identifier ("+=" | "-=" | "=") expr

certificate_decl = "certificate" ":" string_literal
                   "verified_at" ":" "compile_time"
```

`atomic {}` blocks carry transactional semantics. The FIR lowering phase encodes
atomicity as a linearisability assertion emitted to Z3: all assignments within the
block are either all applied or none are applied.

### Certificate and PKI

The `certificate` value in source is a placeholder. The compiler emits a
cryptographically signed `ModuleCertificate` containing:

| Field | Description |
|---|---|
| `module_id` | Stable module identifier |
| `proof_hash` | SHA-256 over canonical FIR proof |
| `acs_score` | ACS at build time |
| `verification_strategy` | Strategy used |
| `conservative_warning` | Present when verification is incomplete |
| `compiler_version` | Semver of the Firmum compiler |
| `verified_at` | Build timestamp |
| `z3_version` | Semver of Z3 used |
| `compiler_signature` | Ed25519, compiler signing key |
| `org_signature` | Ed25519, required for production |

**Three-level PKI hierarchy:**

1. **Root CA** — Ed25519, air-gapped; signs compiler and organisation keys.
2. **Compiler signing key** — per-release, HSM-backed; signs every emitted certificate.
3. **Org signing key** — team-controlled; required for production deployment
   (dual-signature enforcement).

`conservative_warning` is a compiler output status, not a syntactic construct. It is
emitted when neither Z3 nor `ai_assisted` can complete verification. It is embedded in
the signed payload — it cannot be stripped without invalidating the signature. A
production module loader rejects any module carrying this flag unless an explicit
human-approval override is recorded in the audit trail.

---

## Context Declarations

```ebnf
context_decl  = "type" identifier "in" "context" identifier
                "{" context_field* "}"
context_field = identifier ":" (string_literal | number | boolean_literal)
```

Context declarations define the named contexts used by contextual types. The type
checker rejects cross-context operations at the point of use; no manual cast exists.

---

## Let Bindings

```ebnf
let_stmt = "let" identifier (":" type_expr)? "=" expr
```

`let_stmt` has no explicit terminator. Termination is safe because `identifier`
rejects all reserved keywords — `expr` stops before `intent`, `type`, `let`, `proof`,
and any other keyword. This is an emergent property of keyword reservation, not a
special lookahead rule.

---

## Lexical Rules

```ebnf
ALPHA      = 'a'..'z' | 'A'..'Z' | "_"
DIGIT      = '0'..'9'
WHITESPACE = " " | "\t" | "\n" | "\r"
COMMENT    = "//" (!"\n" ANY)* ("\n" | EOF)
           | "/*" (!"*/" ANY)* "*/"
EOF        = !.
```

Whitespace and comments are ignored between tokens.

---

## Reserved Keywords

The following identifiers are reserved. Using any of them as a variable, parameter,
or lemma name is a parse error.

```
intent        assumption    proof         input         output
where         never         context       Fresh         Stale
Expiring      old           forall        exists        assert
atomic        strategy      verify        lemma         induction
certificate   AND           OR            true          false
let           in            using         with          fallback
type          interview     document_review  formal_audit  peer_review
smt_solver    bounded_model_checking       ai_assisted
compile_time  validated_by  context_source  out_of_scope
domain_expert precondition  postcondition  invariant
```

The following are **not** reserved keywords because they are disambiguated by
structural context and reservation would block common identifier names:

- **Time units** — `ms`, `min`, `s`, `h`, `d`: always follow a `number` in `duration`.
- **Source types** — `ref`, `slack`, `email`, `github`, `jira`, `doc`: always precede
  `#` in `source_ref`.

---

## Type Checker Responsibilities

The following are enforced by the type checker, not the grammar:

- The three blocks of a declaration share the same identifier.
- `verify <id>` refers to an intent declared in the same file.
- `old()` is valid only in `postcondition` and `verify` blocks.
- Duplicate fields in `validated_by` and `assumption_section`.
- `confidence` range `[0.0, 1.0]`.
- `Fresh → Expiring → Stale` temporal state transitions; transition points are
  symbolic Z3 constants in the SMT-LIB emission layer.
- At least one `declaration` exists in the program.
- Warning when `validated_by:` has no fields (ACS contribution = 0).
- Warning when a lemma name coincides with a `proof_technique` term.
- Rejection of non-`ref#` `source_ref` entries in production-mode builds.
- ACS score computation and threshold enforcement.
- `conservative_warning` propagation and production loader rejection.

---

## Complete Example

```text
type Amount in context Banking {
  unit:      "USD"
  precision: 2
  auditable: true
}

intent TransferFunds {
  input:
    sender   : Account where balance >= 0
    receiver : Account where id != sender.id
    amount   : Amount<Banking> where amount > 0
  precondition:
    sender.balance >= amount
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
    forall t: Transaction =>
      old(sum(accounts.balance)) == sum(accounts.balance)
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
```
