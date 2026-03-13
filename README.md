# Firmum

> A programming language where proof is part of the syntax.  
> Code is not correct until it is formally verified.

---

## Overview

Firmum is a formal verification meta-language with a mandatory three-block structure —
`intent`, `assumption`, `proof` — enforced at the grammar level. Every declaration
must supply all three; partial declarations are parse errors, not warnings. The
compiler translates Firmum source through a typed intermediate representation (FIR),
emits SMT-LIB 2 for Z3 [8], and produces a cryptographically signed module
certificate. Generated TypeScript is guaranteed to pass `tsc --strict`.

**File extension:** `.frm`  
**Parser:** [pest](https://pest.rs/) (PEG, v2.8+)  
**Implementation language:** Rust (stable 1.78+)  
**License:** AGPL-3.0

---

## Motivation

Most production bugs are not logic errors. They are implicit assumptions that were
never written down in a machine-readable form — what currency unit is in use, what
authentication guarantees exist upstream, what the valid range of an input really
means in the domain. These assumptions live in tickets, emails, and conversations.
When they drift from the code, damage follows.

This gap is growing. AI systems generate a large and increasing fraction of production
code. The problem is not syntactic correctness; modern models produce code that
compiles and passes tests. The problem is that there is no mechanism to verify that
the output reflects what the human intended [5, 6].

Firmum makes undocumented assumptions a compile-time error. An AI generating Firmum
cannot produce structurally incorrect output silently — either the proof passes and the
module is formally verified, or the build fails.

---

## Grammar Design

### Three mandatory blocks

Every Firmum declaration requires three blocks sharing the same identifier. The grammar
enforces the order `intent → assumption → proof`; any other sequence is a parse error.

| Block | Purpose |
|---|---|
| `intent` | Formal contract: typed inputs/outputs, pre/postconditions, invariants, forbidden behaviors |
| `assumption` | Human context: domain rules, structured source references, expert validation |
| `proof` | Formal verification: Z3-backed, compile-time certified |

### Type system

**Refined types** attach predicates to base types inline:

```text
x: Int where x > 0 AND x < 1000
```

Refined types apply only to base types (identifiers). Constraints on compound types
belong in the `precondition` section.

**Dependent types** bind a value parameter at the type site:

```text
items: Vec<Order, n: Nat>
```

A single-parameter form such as `Vec<T>` matches `contextual_type`, not
`dependent_type`. A dependent type without a value parameter has no semantic meaning.

**Contextual types** make cross-domain confusion a compile-time type error:

```text
let a: Amount<Banking> = 100
let b: Amount<Crypto>  = 100
let c = a + b   // TYPE ERROR: context mismatch
```

Cross-context operations are rejected at the point of use. No manual cast exists.

**Temporal types** track data expiry across three states enforced by the type checker:

```text
Fresh<T, d>     — data valid within duration d
Expiring<T, d>  — data approaching expiry; requires explicit handling
Stale<T>        — data past expiry; rejected where Fresh or Expiring is required
```

Transitions `Fresh → Expiring → Stale` are encoded as symbolic constants in Z3.
`Stale<T>` where `Fresh<T, d>` is required is a compile-time type error.
`Expiring<T, d>` cannot be silently passed as `Fresh<T, d>`.

### Assumption Coverage Score

The compiler computes an Assumption Coverage Score (ACS) from 0.0 to 1.0:

```
ACS = Σᵢ [ W(aᵢ) · |map(aᵢ) ∩ E| ] / |E|
```

where `E` is the set of edge cases extracted statically from the intent block and
`W(aᵢ)` is the weight of the validation source:

| Method | Weight |
|---|---|
| `formal_audit` | 1.00 |
| `document_review` | 0.80 |
| `peer_review` | 0.60 |
| `interview` | 0.35 |
| Unlinked string | 0.10 (fixed) |

A valid `context_source` reference applies a 1.15× traceability bonus. Anti-Goodhart
controls prevent padding: semantic deduplication (cosine similarity > 0.90 → single
assumption) and novelty decay (n-th string covering the same edge case contributes
`W(aₙ)/n`).

**Build threshold policy:**

| ACS range | Effect |
|---|---|
| ≥ 0.70 | Pass |
| [0.50, 0.70) | Build error |
| < 0.50 | Hard error + mandatory human review gate |

### context_source and GDPR

`context_source` fields in `.frm` source reference opaque registry IDs
(`ref#<id>`), never raw source paths. The mapping from IDs to real artefacts lives
in a separate **ContextRegistry** — GDPR-scoped, access-controlled, with a
`retention_policy` and `personal_data` flag per entry. The compiler consumes only
content hashes; it never reads referenced document text.

### Certificate and PKI

The proof certificate is an Ed25519-signed `ModuleCertificate` containing:
`proof_hash`, `acs_score`, `conservative_warning` flag, `compiler_version`,
`verified_at`, and `z3_version`. Three signing levels:

1. **Root CA** — Ed25519, air-gapped; signs compiler and organisation keys.
2. **Compiler signing key** — per-release, HSM-backed; signs every emitted certificate.
3. **Org signing key** — team-controlled; required for production (dual-signature).

The `conservative_warning` flag is embedded in the signed payload. It cannot be
stripped without invalidating the signature. A production module loader rejects any
module carrying this flag unless an explicit human-approval override is recorded.

### Gradual formalization

Adoption does not require a full rewrite. Four integration levels:

| Level | Description |
|---|---|
| 0 | Pure TypeScript — no Firmum tooling |
| 1 | `@firmum/annotate` decorators on existing TypeScript functions; ACS check in CI, no build block |
| 2 | `.frm` sidecar file alongside existing `.ts` — verifies without modifying (`binds: "file.ts#functionName"`) |
| 3 | Native `.frm` with TypeScript codegen |

---

## Usage

```bash
# Type check, ACS computation — no Z3
firmum check <file.frm>

# Full build: type check + Z3 verification + certificate emission
firmum build <file.frm>

# Print proof certificate for a module
firmum proof <module_id>

# Print ACS score and coverage gap report
firmum acs <file.frm>
```

### Build configuration (`firmum.toml`)

```toml
[verification]
acs_threshold          = 0.70     # minimum ACS for build success
z3_timeout_secs        = 30       # per-module timeout
allow_conservative     = false    # block production on conservative_warning

[cache]
backend                = "sled"   # embedded; use "redis" for team deployments

[verification_policy]
production_policy      = "block_on_conservative_warning"
require_dual_signature = true     # compiler key + org key both required
```

---

## Installation

**Prerequisites:**

- Rust stable 1.78+  
- `libz3.so` 4.12.x or 4.13.x; set `Z3_SYS_Z3_HEADER`  
- libtss2 3.x ESAPI (optional — Hardware Trust Protocol)  
- LLVM 17+ (optional — LLVM IR codegen target)  
- Intel SGX SDK 2.20+ (optional — SGX enclave compiler measurement)

```bash
git clone https://github.com/NoCl0ne/firmum
cd firmum
cargo build --release
```

Pin the Z3 version. The Firmum Z3 bridge is tested against 4.12.x and 4.13.x only.

---

## Examples

### Finance — money conservation and contextual type safety

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
    forall t: Transaction =>
      old(totalMoneyInSystem) == totalMoneyInSystem
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

Cross-context arithmetic is rejected statically:

```text
let a: Amount<Banking> = 100
let b: Amount<Crypto>  = 100
let c = a + b   // TYPE ERROR: context mismatch
```

### Medicine — temporal type enforcement

```text
intent PrescribeMedication {
  input:
    result : Fresh<LabResult, 24h>
    // Stale<LabResult> is a compile-time type error.
    // Expiring<LabResult, 24h> requires explicit handling.
}
```

### Security — injection prevention and forbidden cryptography

```text
type UserInput in context Untrusted { sanitized: false }
type UserInput in context Sanitized { sanitized: true }

intent SaveToDatabase {
  input: data: UserInput<Sanitized>
  // Passing UserInput<Untrusted> is a TYPE ERROR.
}

intent StorePassword {
  never:
    plaintext_storage
    md5_hashing
    sha1_hashing
  postcondition:
    stored_value == bcrypt(password, cost >= 12)
}
```

---

## Planned Domain Libraries

| Library | Status |
|---|---|
| `stdlib/finance` | In progress — TransferFunds, BatchPayment, AuditLog, FXConversion, Reconciliation |
| `stdlib/medical` | Planned — PrescribeMedication, DrugInteractionCheck, DosageVerify |
| `stdlib/security` | Planned — SanitizeInput, VerifyJWT, HashPassword, CheckCORS |

---

## Current Status

> Early development — grammar specification complete, implementation in progress.

- [x] Grammar specification (`GRAMMAR.md`, `firmum.pest`)
- [ ] Parser — pest.rs
- [ ] FIR lowering — Firmum Intermediate Representation
- [ ] Type checker — contextual types, temporal state transitions, `old()` scoping, decidability classifier
- [ ] SMT Orchestrator — parallel Z3 dispatch, ProofCache (content-addressed), 30s timeout, fallback protocol
- [ ] Z3 bridge — SMT-LIB 2 translation layer (LIA, LRA, BV)
- [ ] ACS engine — edge case extraction, weighted coverage, anti-Goodhart controls
- [ ] Certificate PKI — Ed25519 signing hierarchy, production module loader
- [ ] TypeScript codegen (`tsc --strict` compatible)
- [ ] CLI (`firmum check`, `firmum build`, `firmum proof`, `firmum acs`)
- [ ] `strategy: ai_assisted` — CEGIS+LLM healing loop (K=5, human escalation)
- [ ] Hardware Trust Protocol — Semantic PCR / TPM PCR slot 14 (SGX, TrustZone, RISC-V Keystone)
- [ ] `stdlib/finance`, `stdlib/medical`, `stdlib/security`

---

## Contributing

The grammar is the source of truth. Any change to `firmum.pest` must be reflected in
`GRAMMAR.md` and in affected test fixtures under `tests/`. Grammar changes that expand
the keyword set must be justified by a concrete type-checker or codegen requirement.
Reserved keywords that cannot be tested against a working compiler phase are
provisional and subject to removal.

No benchmarks, performance claims, or claims about Z3 decidability classes may appear
in documentation without a corresponding test fixture or citation.

---

## License

AGPL-3.0 — see [LICENSE](LICENSE)

---

*Created by NoCl0ne — March 2026*

---

### References

[1] Leino, K. R. M. (2010). Dafny: An Automatic Program Verifier for Functional
    Correctness. LPAR-16. Springer.

[5] Endres, M., Fakhoury, S., Chakraborty, S., and Lahiri, S. K. (2023). Can Large
    Language Models Transform Natural Language Intent into Formal Method Postconditions?
    arXiv:2310.01831.

[6] Lahiri, S. K. (2024). Evaluating LLM-driven User-Intent Formalization for
    Verification-Aware Languages. arXiv:2406.09757.

[8] de Moura, L. and Bjorner, N. (2008). Z3: An Efficient SMT Solver. TACAS. LNCS 4963.
