/// Assumption Coverage Score (ACS) engine — Stage 3.
///
/// Formula: ACS = Σᵢ [ W(aᵢ) · |map(aᵢ) ∩ E| ] / |E|
///
/// Edge case extraction rules (GRAMMAR.md §ACS):
///   1. Boundary rule        — refined-type params and precondition comparisons → 2 each
///   2. Forbidden behavior   — each never identifier → 1
///   3. Old-value delta      — each postcondition referencing old() → 3
///   4. Context disjointness — each pair of contextual input params with the
///      same base type → 1
///
/// Coverage model (simplified — no NLP):
///   Each assumption string is assumed to cover all edge cases. Novelty decay
///   (n-th string covering the same cases contributes W/n) prevents inflation.
///
/// Effective weight W(aᵢ):
///   linked string (assumption block has context_source):
///     W = method_weight × confidence × 1.15  (traceability bonus)
///   unlinked string (no context_source):
///     W = 0.10  (fixed; confidence and method ignored)
///
/// Method weights: formal_audit=1.00, document_review=0.80,
///                 peer_review=0.60, interview=0.35
///
/// Build threshold policy:
///   ACS ≥ 0.70          → pass
///   ACS ∈ [0.50, 0.70)  → build error
///   ACS < 0.50          → hard error + mandatory human review gate
use crate::errors::CompilerError;
use crate::fir::Program;
use crate::fir::{AssumptionNode, AssumptionSection, TypeNode, ValidatedByField, ValidationMethod};

pub const THRESHOLD_PASS: f64 = 0.70;
pub const THRESHOLD_WARN: f64 = 0.50;

pub fn compute(program: &Program) -> Result<f64, CompilerError> {
    let e = edge_case_count(program);
    if e == 0 {
        return Ok(0.0);
    }

    let mut numerator = 0.0f64;
    for decl in &program.declarations {
        numerator += assumption_contribution(&decl.assumption, e);
    }

    // Clamp to [0.0, 1.0].  The raw numerator can exceed 1.0 when the
    // traceability bonus (×1.15) combined with a high method weight and
    // confidence pushes the first string above 1.0, e.g.:
    //   document_review (0.80) × confidence 0.92 × 1.15 ≈ 0.847 per string,
    //   plus a second string contributing 0.847/2 ≈ 0.424 → raw sum ≈ 1.27.
    // Capping at 1.0 is intentional: no program should achieve an ACS above
    // the maximum possible score regardless of how many strings are supplied.
    Ok(numerator.min(1.0))
}

// ── edge case extraction ──────────────────────────────────────────────────────

fn edge_case_count(program: &Program) -> usize {
    let mut count = 0;
    for decl in &program.declarations {
        let intent = &decl.intent;

        // Rule 1a: boundary — each refined-type input/output parameter → 2 edge cases.
        for param in intent.inputs.iter().chain(intent.outputs.iter()) {
            if matches!(&param.ty, TypeNode::Refined { .. }) {
                count += 2;
            }
        }

        // Rule 1b: boundary — each precondition line → 2 edge cases.
        count += intent.preconditions.len() * 2;

        // Rule 2: forbidden behavior — each never identifier → 1 edge case.
        count += intent.never.len();

        // Rule 3: old-value delta — each postcondition referencing old() → 3 edge cases.
        for post in &intent.postconditions {
            if super::predicate_contains_old(post) {
                count += 3;
            }
        }

        // Rule 4: context disjointness — pairs of contextual input params with
        // the same base type → 1 edge case per pair.
        let contextual_bases: Vec<&str> = intent
            .inputs
            .iter()
            .filter_map(|p| {
                if let TypeNode::Contextual { base, .. } = &p.ty {
                    Some(base.as_str())
                } else {
                    None
                }
            })
            .collect();
        for i in 0..contextual_bases.len() {
            for j in (i + 1)..contextual_bases.len() {
                if contextual_bases[i] == contextual_bases[j] {
                    count += 1;
                }
            }
        }
    }
    count
}

// ── weight computation ────────────────────────────────────────────────────────

fn assumption_contribution(assumption: &AssumptionNode, _e: usize) -> f64 {
    let linked = assumption
        .sections
        .iter()
        .any(|s| matches!(s, AssumptionSection::ContextSource(refs) if !refs.is_empty()));

    let (method_w, confidence) = extract_method_and_confidence(assumption);

    // Effective weight per assumption string.
    let w = if linked {
        (method_w * confidence * 1.15_f64).min(1.0)
    } else {
        0.10
    };

    // Sum over string assumptions with novelty decay.
    // The n-th string covering the same edge cases contributes w/n to the ACS.
    // With simplified full-coverage model: contribution per string i (0-indexed) = w/(i+1).
    let strings: Vec<_> = assumption
        .sections
        .iter()
        .filter(|s| matches!(s, AssumptionSection::StringAssumption(_)))
        .collect();

    strings
        .iter()
        .enumerate()
        .map(|(i, _)| w / (i + 1) as f64)
        .sum()
}

fn extract_method_and_confidence(assumption: &AssumptionNode) -> (f64, f64) {
    let mut method_weight = 0.10_f64; // default: unlinked string weight
    let mut confidence = 1.0_f64;

    for section in &assumption.sections {
        if let AssumptionSection::ValidatedBy(vb) = section {
            for field in &vb.fields {
                match field {
                    ValidatedByField::Method(m) => {
                        method_weight = method_base_weight(m);
                    }
                    ValidatedByField::Confidence(c) => {
                        confidence = *c;
                    }
                    _ => {}
                }
            }
        }
    }

    (method_weight, confidence)
}

fn method_base_weight(method: &ValidationMethod) -> f64 {
    match method {
        ValidationMethod::FormalAudit => 1.00,
        ValidationMethod::DocumentReview => 0.80,
        ValidationMethod::PeerReview => 0.60,
        ValidationMethod::Interview => 0.35,
    }
}
