/// Stage 2: Pairs → FIR lowering.
///
/// Converts the pest `Pairs<Rule>` tree produced by the parser into a typed
/// `Program` FIR node.
use super::{
    AssignOp, AssumptionNode, AssumptionSection, BinOpKind, CertificatePlaceholder, ComparisonOp,
    ContextDecl, ContextField, ContextFieldValue, Declaration, Duration, ExprNode, IntentNode,
    LemmaDecl, LetBinding, Number, Param, PredicateNode, Program, ProofMethod, ProofNode,
    ProofTechnique, SourceRef, SourceType, StrategyExpr, StrategyName, TemporalType, TimeUnit,
    TypeNode, ValidatedBy, ValidatedByField, ValidationMethod, VerifyDecl, VerifyStatement,
};
use crate::errors::CompilerError;
use crate::parser::Rule;
use pest::iterators::Pair;

// ── entry point ─────────────────────────────────────────────────────────────

pub fn lower(pairs: pest::iterators::Pairs<crate::parser::Rule>) -> Result<Program, CompilerError> {
    let program_pair = pairs
        .into_iter()
        .next()
        .ok_or_else(|| lerr("empty pairs: expected program rule"))?;
    lower_program_pair(program_pair)
}

// ── top-level ────────────────────────────────────────────────────────────────

fn lower_program_pair(pair: Pair<Rule>) -> Result<Program, CompilerError> {
    let mut contexts = Vec::new();
    let mut lets = Vec::new();
    let mut declarations = Vec::new();

    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::context_decl => contexts.push(lower_context_decl(item)?),
            Rule::let_stmt => lets.push(lower_let_stmt(item)?),
            Rule::declaration => declarations.push(lower_declaration(item)?),
            Rule::EOI => {}
            r => return Err(lerr(&format!("program: unexpected rule {r:?}"))),
        }
    }

    Ok(Program {
        contexts,
        lets,
        declarations,
    })
}

// ── context_decl ─────────────────────────────────────────────────────────────

fn lower_context_decl(pair: Pair<Rule>) -> Result<ContextDecl, CompilerError> {
    let mut inner = pair.into_inner();
    let type_name = next_str(&mut inner, "context_decl", "type_name")?;
    let context_name = next_str(&mut inner, "context_decl", "context_name")?;
    let fields = inner
        .map(lower_context_field)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ContextDecl {
        type_name,
        context_name,
        fields,
    })
}

fn lower_context_field(pair: Pair<Rule>) -> Result<ContextField, CompilerError> {
    let mut inner = pair.into_inner();
    let name = next_str(&mut inner, "context_field", "name")?;
    let value_pair = inner
        .next()
        .ok_or_else(|| lerr("context_field: missing value"))?;
    let value = match value_pair.as_rule() {
        Rule::string_literal => ContextFieldValue::StringVal(unquote(value_pair.as_str())),
        Rule::number => {
            let n = lower_number(value_pair)?;
            match n {
                Number::Integer(v) => ContextFieldValue::Integer(v),
                Number::Decimal(v) => ContextFieldValue::Decimal(v),
            }
        }
        Rule::boolean_literal => {
            ContextFieldValue::Boolean(value_pair.as_str().starts_with("true"))
        }
        r => return Err(lerr(&format!("context_field: unexpected value rule {r:?}"))),
    };
    Ok(ContextField { name, value })
}

// ── let_stmt ──────────────────────────────────────────────────────────────────

fn lower_let_stmt(pair: Pair<Rule>) -> Result<LetBinding, CompilerError> {
    let mut inner = pair.into_inner();
    let name = next_str(&mut inner, "let_stmt", "name")?;

    let mut ty = None;
    let next = inner.next().ok_or_else(|| lerr("let_stmt: missing expr"))?;

    let expr_pair = if next.as_rule() == Rule::type_expr {
        ty = Some(lower_type_expr(next)?);
        inner
            .next()
            .ok_or_else(|| lerr("let_stmt: missing expr after type"))?
    } else {
        next
    };

    let expr = lower_expr(expr_pair)?;
    Ok(LetBinding { name, ty, expr })
}

// ── declaration ───────────────────────────────────────────────────────────────

fn lower_declaration(pair: Pair<Rule>) -> Result<Declaration, CompilerError> {
    let mut inner = pair.into_inner();
    let intent = lower_intent_block(
        inner
            .next()
            .ok_or_else(|| lerr("declaration: missing intent"))?,
    )?;
    let assumption = lower_assumption_block(
        inner
            .next()
            .ok_or_else(|| lerr("declaration: missing assumption"))?,
    )?;
    let proof = lower_proof_block(
        inner
            .next()
            .ok_or_else(|| lerr("declaration: missing proof"))?,
    )?;
    Ok(Declaration {
        intent,
        assumption,
        proof,
    })
}

// ── intent_block ──────────────────────────────────────────────────────────────

fn lower_intent_block(pair: Pair<Rule>) -> Result<IntentNode, CompilerError> {
    let mut inner = pair.into_inner();
    let name = next_str(&mut inner, "intent_block", "name")?;

    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    let mut preconditions = Vec::new();
    let mut postconditions = Vec::new();
    let mut invariants = Vec::new();
    let mut never = Vec::new();

    for section in inner {
        match section.as_rule() {
            Rule::input_section => inputs = lower_param_section(section)?,
            Rule::output_section => outputs = lower_param_section(section)?,
            Rule::precondition_section => preconditions = lower_predicate_section(section)?,
            Rule::postcondition_section => postconditions = lower_predicate_section(section)?,
            Rule::invariant_section => invariants.extend(lower_predicate_section(section)?),
            Rule::never_section => never = lower_never_section(section)?,
            r => return Err(lerr(&format!("intent_block: unexpected rule {r:?}"))),
        }
    }

    Ok(IntentNode {
        name,
        inputs,
        outputs,
        preconditions,
        postconditions,
        invariants,
        never,
    })
}

fn lower_param_section(pair: Pair<Rule>) -> Result<Vec<Param>, CompilerError> {
    let mut params = Vec::new();
    for param_pair in pair.into_inner() {
        let mut pi = param_pair.into_inner();
        let name = next_str(&mut pi, "param", "name")?;
        let ty = lower_type_expr(pi.next().ok_or_else(|| lerr("param: missing type"))?)?;
        params.push(Param { name, ty });
    }
    Ok(params)
}

fn lower_predicate_section(pair: Pair<Rule>) -> Result<Vec<PredicateNode>, CompilerError> {
    let mut preds = Vec::new();
    for line in pair.into_inner() {
        // predicate_line = { !intent_section_kw ~ predicate_or }
        let pred_or = line
            .into_inner()
            .next()
            .ok_or_else(|| lerr("predicate_line: missing predicate_or"))?;
        preds.push(lower_predicate_or(pred_or)?);
    }
    Ok(preds)
}

fn lower_never_section(pair: Pair<Rule>) -> Result<Vec<String>, CompilerError> {
    let mut ids = Vec::new();
    for nid in pair.into_inner() {
        // never_id = { !"}" ~ identifier }
        let id = nid
            .into_inner()
            .next()
            .ok_or_else(|| lerr("never_id: missing identifier"))?
            .as_str()
            .to_string();
        ids.push(id);
    }
    Ok(ids)
}

// ── type_expr ─────────────────────────────────────────────────────────────────

fn lower_type_expr(pair: Pair<Rule>) -> Result<TypeNode, CompilerError> {
    let child = pair
        .into_inner()
        .next()
        .ok_or_else(|| lerr("type_expr: empty"))?;
    match child.as_rule() {
        Rule::temporal_type => lower_temporal_type(child),
        Rule::contextual_type => lower_contextual_type(child),
        Rule::dependent_type => lower_dependent_type(child),
        Rule::refined_type => lower_refined_type(child),
        Rule::base_type => {
            let id = child
                .into_inner()
                .next()
                .ok_or_else(|| lerr("base_type: missing identifier"))?
                .as_str()
                .to_string();
            Ok(TypeNode::Base(id))
        }
        r => Err(lerr(&format!("type_expr: unexpected rule {r:?}"))),
    }
}

fn lower_temporal_type(pair: Pair<Rule>) -> Result<TypeNode, CompilerError> {
    let text = pair.as_str();
    let mut inner = pair.into_inner();
    let inner_type = lower_type_expr(
        inner
            .next()
            .ok_or_else(|| lerr("temporal_type: missing inner type"))?,
    )?;

    if text.starts_with("Stale") {
        Ok(TypeNode::Temporal(TemporalType::Stale(Box::new(
            inner_type,
        ))))
    } else {
        let duration = lower_duration(
            inner
                .next()
                .ok_or_else(|| lerr("temporal_type: missing duration"))?,
        )?;
        if text.starts_with("Fresh") {
            Ok(TypeNode::Temporal(TemporalType::Fresh {
                inner: Box::new(inner_type),
                duration,
            }))
        } else {
            Ok(TypeNode::Temporal(TemporalType::Expiring {
                inner: Box::new(inner_type),
                duration,
            }))
        }
    }
}

fn lower_duration(pair: Pair<Rule>) -> Result<Duration, CompilerError> {
    let mut inner = pair.into_inner();
    let number = lower_number(
        inner
            .next()
            .ok_or_else(|| lerr("duration: missing number"))?,
    )?;
    let unit_text = inner
        .next()
        .ok_or_else(|| lerr("duration: missing time_unit"))?
        .as_str();
    let unit = match unit_text {
        "ms" => TimeUnit::Millisecond,
        "min" => TimeUnit::Minute,
        "s" => TimeUnit::Second,
        "h" => TimeUnit::Hour,
        "d" => TimeUnit::Day,
        u => return Err(lerr(&format!("duration: unknown time unit '{u}'"))),
    };
    Ok(Duration {
        value: number,
        unit,
    })
}

fn lower_number(pair: Pair<Rule>) -> Result<Number, CompilerError> {
    let inner = pair
        .into_inner()
        .next()
        .ok_or_else(|| lerr("number: empty"))?;
    match inner.as_rule() {
        Rule::decimal => inner
            .as_str()
            .parse::<f64>()
            .map(Number::Decimal)
            .map_err(|_| lerr(&format!("invalid decimal '{}'", inner.as_str()))),
        Rule::integer => inner
            .as_str()
            .parse::<u64>()
            .map(Number::Integer)
            .map_err(|_| lerr(&format!("invalid integer '{}'", inner.as_str()))),
        r => Err(lerr(&format!("number: unexpected rule {r:?}"))),
    }
}

fn lower_contextual_type(pair: Pair<Rule>) -> Result<TypeNode, CompilerError> {
    let mut inner = pair.into_inner();
    let base = next_str(&mut inner, "contextual_type", "base")?;
    let context = next_str(&mut inner, "contextual_type", "context")?;
    Ok(TypeNode::Contextual { base, context })
}

fn lower_dependent_type(pair: Pair<Rule>) -> Result<TypeNode, CompilerError> {
    let mut inner = pair.into_inner();
    let base = next_str(&mut inner, "dependent_type", "base")?;
    let type_param = lower_type_expr(
        inner
            .next()
            .ok_or_else(|| lerr("dependent_type: missing type_param"))?,
    )?;
    let value_param_name = next_str(&mut inner, "dependent_type", "value_param_name")?;
    let value_param_type = inner
        .next()
        .ok_or_else(|| lerr("dependent_type: missing value_param_type"))?
        .into_inner()
        .next()
        .ok_or_else(|| lerr("dependent_type: base_type empty"))?
        .as_str()
        .to_string();
    Ok(TypeNode::Dependent {
        base,
        type_param: Box::new(type_param),
        value_param_name,
        value_param_type,
    })
}

fn lower_refined_type(pair: Pair<Rule>) -> Result<TypeNode, CompilerError> {
    let mut inner = pair.into_inner();
    let base = next_str(&mut inner, "refined_type", "base")?;
    let pred = lower_predicate_or(
        inner
            .next()
            .ok_or_else(|| lerr("refined_type: missing predicate"))?,
    )?;
    Ok(TypeNode::Refined {
        base,
        predicate: Box::new(pred),
    })
}

// ── predicates ───────────────────────────────────────────────────────────────

fn lower_predicate_or(pair: Pair<Rule>) -> Result<PredicateNode, CompilerError> {
    // predicate_or = { predicate_and ~ ("OR" ~ predicate_and)* }
    // "OR" is anonymous → inner pairs are predicate_and, predicate_and, ...
    let mut children = pair
        .into_inner()
        .map(lower_predicate_and)
        .collect::<Result<Vec<_>, _>>()?;
    let first = children.remove(0);
    Ok(children.into_iter().fold(first, |acc, next| {
        PredicateNode::Or(Box::new(acc), Box::new(next))
    }))
}

fn lower_predicate_and(pair: Pair<Rule>) -> Result<PredicateNode, CompilerError> {
    // predicate_and = { predicate_atom ~ ("AND" ~ predicate_atom)* }
    // "AND" is anonymous → inner pairs are predicate_atom, predicate_atom, ...
    let mut children = pair
        .into_inner()
        .map(lower_predicate_atom)
        .collect::<Result<Vec<_>, _>>()?;
    let first = children.remove(0);
    Ok(children.into_iter().fold(first, |acc, next| {
        PredicateNode::And(Box::new(acc), Box::new(next))
    }))
}

fn lower_predicate_atom(pair: Pair<Rule>) -> Result<PredicateNode, CompilerError> {
    let text = pair.as_str().trim_start();
    let mut inner = pair.into_inner();

    if text.starts_with('!') {
        let inner_atom = inner
            .next()
            .ok_or_else(|| lerr("predicate_atom(!): missing inner"))?;
        return Ok(PredicateNode::Not(Box::new(lower_predicate_atom(
            inner_atom,
        )?)));
    }

    if text.starts_with("forall") {
        let var = next_str(&mut inner, "forall", "var")?;
        let ty = lower_type_expr(inner.next().ok_or_else(|| lerr("forall: missing type"))?)?;
        let body = lower_predicate_atom(inner.next().ok_or_else(|| lerr("forall: missing body"))?)?;
        return Ok(PredicateNode::Forall {
            var,
            ty: Box::new(ty),
            body: Box::new(body),
        });
    }

    if text.starts_with("exists") {
        let var = next_str(&mut inner, "exists", "var")?;
        let ty = lower_type_expr(inner.next().ok_or_else(|| lerr("exists: missing type"))?)?;
        let body = lower_predicate_atom(inner.next().ok_or_else(|| lerr("exists: missing body"))?)?;
        return Ok(PredicateNode::Exists {
            var,
            ty: Box::new(ty),
            body: Box::new(body),
        });
    }

    let first = inner
        .next()
        .ok_or_else(|| lerr("predicate_atom: empty inner"))?;
    match first.as_rule() {
        Rule::predicate_or => lower_predicate_or(first), // "(" ~ predicate_or ~ ")"
        Rule::comparison => lower_comparison(first),
        r => Err(lerr(&format!("predicate_atom: unexpected rule {r:?}"))),
    }
}

fn lower_comparison(pair: Pair<Rule>) -> Result<PredicateNode, CompilerError> {
    let mut inner = pair.into_inner();
    let left = lower_expr(
        inner
            .next()
            .ok_or_else(|| lerr("comparison: missing left"))?,
    )?;
    let op_pair = inner.next().ok_or_else(|| lerr("comparison: missing op"))?;
    let right = lower_expr(
        inner
            .next()
            .ok_or_else(|| lerr("comparison: missing right"))?,
    )?;

    let op = match op_pair.as_str() {
        "==" => ComparisonOp::Eq,
        "!=" => ComparisonOp::Ne,
        "<=" => ComparisonOp::Le,
        ">=" => ComparisonOp::Ge,
        "<" => ComparisonOp::Lt,
        ">" => ComparisonOp::Gt,
        o => return Err(lerr(&format!("comparison: unknown op '{o}'"))),
    };

    Ok(PredicateNode::Comparison { left, op, right })
}

// ── expressions ───────────────────────────────────────────────────────────────

fn lower_expr(pair: Pair<Rule>) -> Result<ExprNode, CompilerError> {
    // expr = { term ~ (("+"|"-") ~ term)* }
    // "+" and "-" are anonymous → inner pairs are term, term, ...
    // Operator is recovered via span arithmetic within the parent's text.
    let full = pair.as_str();
    let base = pair.as_span().start();
    let mut inner = pair.into_inner();

    let first = inner.next().ok_or_else(|| lerr("expr: empty"))?;
    let mut prev_end = first.as_span().end() - base;
    let mut result = lower_term(first)?;

    for term_pair in inner {
        let term_start = term_pair.as_span().start() - base;
        let op = match full[prev_end..term_start].trim() {
            "+" => BinOpKind::Add,
            "-" => BinOpKind::Sub,
            o => return Err(lerr(&format!("expr: unknown op '{o}'"))),
        };
        prev_end = term_pair.as_span().end() - base;
        let right = lower_term(term_pair)?;
        result = ExprNode::BinOp {
            left: Box::new(result),
            op,
            right: Box::new(right),
        };
    }

    Ok(result)
}

fn lower_term(pair: Pair<Rule>) -> Result<ExprNode, CompilerError> {
    // term = { factor ~ (("*"|"/") ~ factor)* }
    let full = pair.as_str();
    let base = pair.as_span().start();
    let mut inner = pair.into_inner();

    let first = inner.next().ok_or_else(|| lerr("term: empty"))?;
    let mut prev_end = first.as_span().end() - base;
    let mut result = lower_factor(first)?;

    for factor_pair in inner {
        let factor_start = factor_pair.as_span().start() - base;
        let op = match full[prev_end..factor_start].trim() {
            "*" => BinOpKind::Mul,
            "/" => BinOpKind::Div,
            o => return Err(lerr(&format!("term: unknown op '{o}'"))),
        };
        prev_end = factor_pair.as_span().end() - base;
        let right = lower_factor(factor_pair)?;
        result = ExprNode::BinOp {
            left: Box::new(result),
            op,
            right: Box::new(right),
        };
    }

    Ok(result)
}

fn lower_factor(pair: Pair<Rule>) -> Result<ExprNode, CompilerError> {
    let child = pair
        .into_inner()
        .next()
        .ok_or_else(|| lerr("factor: empty"))?;
    match child.as_rule() {
        Rule::number => lower_number(child).map(ExprNode::Number),
        Rule::string_literal => Ok(ExprNode::StringLit(unquote(child.as_str()))),
        Rule::old_expr => {
            // old_expr = { "old" ~ "(" ~ qualified_identifier ~ ")" }
            let qi = child
                .into_inner()
                .next()
                .ok_or_else(|| lerr("old_expr: missing qualified_identifier"))?
                .as_str()
                .to_string();
            Ok(ExprNode::OldValue(qi))
        }
        Rule::function_call => lower_function_call(child),
        Rule::qualified_identifier => Ok(ExprNode::Identifier(child.as_str().to_string())),
        Rule::expr => lower_expr(child),
        r => Err(lerr(&format!("factor: unexpected rule {r:?}"))),
    }
}

fn lower_function_call(pair: Pair<Rule>) -> Result<ExprNode, CompilerError> {
    // function_call = { identifier ~ "(" ~ (expr ~ ("," ~ expr)*)? ~ ")" }
    // "(" ")" "," are anonymous → inner pairs: identifier, expr, expr, ...
    let mut inner = pair.into_inner();
    let name = next_str(&mut inner, "function_call", "name")?;
    let args = inner.map(lower_expr).collect::<Result<Vec<_>, _>>()?;
    Ok(ExprNode::FunctionCall { name, args })
}

// ── assumption_block ──────────────────────────────────────────────────────────

fn lower_assumption_block(pair: Pair<Rule>) -> Result<AssumptionNode, CompilerError> {
    let mut inner = pair.into_inner();
    let name = next_str(&mut inner, "assumption_block", "name")?;
    let sections = inner
        .map(lower_assumption_section)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(AssumptionNode { name, sections })
}

fn lower_assumption_section(pair: Pair<Rule>) -> Result<AssumptionSection, CompilerError> {
    let child = pair
        .into_inner()
        .next()
        .ok_or_else(|| lerr("assumption_section: empty"))?;
    match child.as_rule() {
        Rule::assumption_string => {
            let sl = child
                .into_inner()
                .next()
                .ok_or_else(|| lerr("assumption_string: missing string_literal"))?;
            Ok(AssumptionSection::StringAssumption(unquote(sl.as_str())))
        }
        Rule::context_source_section => {
            let refs = child
                .into_inner()
                .map(lower_source_ref)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(AssumptionSection::ContextSource(refs))
        }
        Rule::out_of_scope_section => {
            let strings = child.into_inner().map(|sl| unquote(sl.as_str())).collect();
            Ok(AssumptionSection::OutOfScope(strings))
        }
        Rule::validated_by_section => {
            let fields = child
                .into_inner()
                .map(lower_validated_by_field)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(AssumptionSection::ValidatedBy(ValidatedBy { fields }))
        }
        r => Err(lerr(&format!("assumption_section: unexpected rule {r:?}"))),
    }
}

fn lower_source_ref(pair: Pair<Rule>) -> Result<SourceRef, CompilerError> {
    let mut inner = pair.into_inner();
    let st = inner
        .next()
        .ok_or_else(|| lerr("source_ref: missing source_type"))?;
    let sp = inner
        .next()
        .ok_or_else(|| lerr("source_ref: missing source_path"))?;

    let source_type = match st.as_str() {
        "ref" => SourceType::Ref,
        "slack" => SourceType::Slack,
        "email" => SourceType::Email,
        "github" => SourceType::Github,
        "jira" => SourceType::Jira,
        "doc" => SourceType::Doc,
        s => return Err(lerr(&format!("source_ref: unknown type '{s}'"))),
    };

    Ok(SourceRef {
        source_type,
        path: sp.as_str().to_string(),
    })
}

fn lower_validated_by_field(pair: Pair<Rule>) -> Result<ValidatedByField, CompilerError> {
    // validated_by_field alternatives are anonymous keyword + ":" + value pairs.
    // The keywords ("domain_expert", "date", "confidence", "method") are not emitted.
    // Only the value child is emitted; its rule type disambiguates the alternative.
    let child = pair
        .into_inner()
        .next()
        .ok_or_else(|| lerr("validated_by_field: empty"))?;
    match child.as_rule() {
        Rule::string_literal => Ok(ValidatedByField::DomainExpert(unquote(child.as_str()))),
        Rule::date_literal => Ok(ValidatedByField::Date(child.as_str().to_string())),
        Rule::confidence_value => {
            let v: f64 = child
                .as_str()
                .parse()
                .map_err(|_| lerr(&format!("invalid confidence '{}'", child.as_str())))?;
            Ok(ValidatedByField::Confidence(v))
        }
        Rule::validation_method => {
            let method = match child.as_str() {
                "interview" => ValidationMethod::Interview,
                "document_review" => ValidationMethod::DocumentReview,
                "formal_audit" => ValidationMethod::FormalAudit,
                "peer_review" => ValidationMethod::PeerReview,
                m => return Err(lerr(&format!("unknown validation method '{m}'"))),
            };
            Ok(ValidatedByField::Method(method))
        }
        r => Err(lerr(&format!("validated_by_field: unexpected rule {r:?}"))),
    }
}

// ── proof_block ───────────────────────────────────────────────────────────────

fn lower_proof_block(pair: Pair<Rule>) -> Result<ProofNode, CompilerError> {
    let mut inner = pair.into_inner();
    let name = next_str(&mut inner, "proof_block", "name")?;
    let strategy = lower_strategy_decl(
        inner
            .next()
            .ok_or_else(|| lerr("proof_block: missing strategy"))?,
    )?;

    let mut lemmas = Vec::new();
    let mut verify_decls = Vec::new();
    let mut certificate = None;

    for child in inner {
        match child.as_rule() {
            Rule::lemma_decl => lemmas.push(lower_lemma_decl(child)?),
            Rule::verify_decl => verify_decls.push(lower_verify_decl(child)?),
            Rule::certificate_decl => {
                // certificate_decl = { "certificate" ~ ":" ~ string_literal ~ "verified_at" ~ ":" ~ "compile_time" }
                let sl = child
                    .into_inner()
                    .next()
                    .ok_or_else(|| lerr("certificate_decl: missing string_literal"))?;
                certificate = Some(CertificatePlaceholder {
                    value: unquote(sl.as_str()),
                });
            }
            r => return Err(lerr(&format!("proof_block: unexpected rule {r:?}"))),
        }
    }

    Ok(ProofNode {
        name,
        strategy,
        lemmas,
        verify_decls,
        certificate,
    })
}

fn lower_strategy_decl(pair: Pair<Rule>) -> Result<StrategyExpr, CompilerError> {
    lower_strategy_expr(
        pair.into_inner()
            .next()
            .ok_or_else(|| lerr("strategy_decl: missing strategy_expr"))?,
    )
}

fn lower_strategy_expr(pair: Pair<Rule>) -> Result<StrategyExpr, CompilerError> {
    // strategy_expr = { strategy_name ~ ("with" ~ "fallback" ~ "(" ~ strategy_name ~ ")")? }
    // "with" "fallback" "(" ")" are anonymous → inner pairs: strategy_name, strategy_name?
    let mut inner = pair.into_inner();
    let primary = lower_strategy_name(
        inner
            .next()
            .ok_or_else(|| lerr("strategy_expr: missing primary"))?,
    )?;
    let fallback = inner.next().map(lower_strategy_name).transpose()?;
    Ok(StrategyExpr { primary, fallback })
}

fn lower_strategy_name(pair: Pair<Rule>) -> Result<StrategyName, CompilerError> {
    let text = pair.as_str().trim_start();
    if text.starts_with("smt_solver") {
        Ok(StrategyName::SmtSolverZ3)
    } else {
        match text {
            "bounded_model_checking" => Ok(StrategyName::BoundedModelChecking),
            "induction" => Ok(StrategyName::Induction),
            "ai_assisted" => Ok(StrategyName::AiAssisted),
            s => Err(lerr(&format!("unknown strategy name '{s}'"))),
        }
    }
}

fn lower_lemma_decl(pair: Pair<Rule>) -> Result<LemmaDecl, CompilerError> {
    let mut inner = pair.into_inner();
    let name = next_str(&mut inner, "lemma_decl", "name")?;

    let mut predicates = Vec::new();
    let mut proof_method = None;

    for child in inner {
        match child.as_rule() {
            Rule::predicate_or => predicates.push(lower_predicate_or(child)?),
            Rule::proof_method => proof_method = Some(lower_proof_method(child)?),
            r => return Err(lerr(&format!("lemma_decl: unexpected rule {r:?}"))),
        }
    }

    Ok(LemmaDecl {
        name,
        predicates,
        proof_method,
    })
}

fn lower_proof_method(pair: Pair<Rule>) -> Result<ProofMethod, CompilerError> {
    let technique_pair = pair
        .into_inner()
        .next()
        .ok_or_else(|| lerr("proof_method: missing proof_technique"))?;
    Ok(ProofMethod {
        technique: lower_proof_technique(technique_pair)?,
    })
}

fn lower_proof_technique(pair: Pair<Rule>) -> Result<ProofTechnique, CompilerError> {
    let text = pair.as_str().trim_start();
    if text.starts_with("induction") {
        let on = pair
            .into_inner()
            .next()
            .ok_or_else(|| lerr("proof_technique: induction missing identifier"))?
            .as_str()
            .to_string();
        Ok(ProofTechnique::Induction { on })
    } else if text == "contradiction" {
        Ok(ProofTechnique::Contradiction)
    } else if text == "direct" {
        Ok(ProofTechnique::Direct)
    } else {
        Err(lerr(&format!("unknown proof technique '{text}'")))
    }
}

fn lower_verify_decl(pair: Pair<Rule>) -> Result<VerifyDecl, CompilerError> {
    // verify_decl = { "verify" ~ identifier ~ ("using" ~ identifier)? ~ "{" ~ verify_statement* ~ "}" }
    // "verify" "using" "{" "}" are anonymous → inner pairs: identifier, identifier?, verify_statement*
    let mut inner = pair.into_inner();
    let target = next_str(&mut inner, "verify_decl", "target")?;

    let mut using = None;
    let mut statements = Vec::new();

    for child in inner {
        match child.as_rule() {
            Rule::identifier => using = Some(child.as_str().to_string()),
            Rule::verify_statement => statements.push(lower_verify_statement(child)?),
            r => return Err(lerr(&format!("verify_decl: unexpected rule {r:?}"))),
        }
    }

    Ok(VerifyDecl {
        target,
        using,
        statements,
    })
}

fn lower_verify_statement(pair: Pair<Rule>) -> Result<VerifyStatement, CompilerError> {
    let child = pair
        .into_inner()
        .next()
        .ok_or_else(|| lerr("verify_statement: empty"))?;
    match child.as_rule() {
        Rule::assert_stmt => {
            let pred = lower_predicate_or(
                child
                    .into_inner()
                    .next()
                    .ok_or_else(|| lerr("assert_stmt: missing predicate"))?,
            )?;
            Ok(VerifyStatement::Assert(pred))
        }
        Rule::atomic_stmt => {
            let stmts = child
                .into_inner()
                .map(lower_verify_statement)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(VerifyStatement::Atomic(stmts))
        }
        Rule::assign_stmt => lower_assign_stmt(child),
        r => Err(lerr(&format!("verify_statement: unexpected rule {r:?}"))),
    }
}

fn lower_assign_stmt(pair: Pair<Rule>) -> Result<VerifyStatement, CompilerError> {
    // assign_stmt = { qualified_identifier ~ ("+=" | "-=" | "=") ~ expr }
    // Operators are anonymous → inner pairs: qualified_identifier, expr.
    // Recover operator from text between the two children's spans.
    let full = pair.as_str();
    let mut inner = pair.into_inner();

    let target_pair = inner
        .next()
        .ok_or_else(|| lerr("assign_stmt: missing target"))?;
    let target = target_pair.as_str().to_string();
    let target_len = target.len();

    let expr_pair = inner
        .next()
        .ok_or_else(|| lerr("assign_stmt: missing expr"))?;

    let between = full[target_len..].trim_start();
    let op = if between.starts_with("+=") {
        AssignOp::AddAssign
    } else if between.starts_with("-=") {
        AssignOp::SubAssign
    } else if between.starts_with('=') {
        AssignOp::Assign
    } else {
        return Err(lerr(&format!(
            "assign_stmt: cannot parse operator from '{between}'"
        )));
    };

    let expr = lower_expr(expr_pair)?;
    Ok(VerifyStatement::Assign { target, op, expr })
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Extract the next identifier string from an inner iterator.
fn next_str(
    inner: &mut pest::iterators::Pairs<Rule>,
    ctx: &str,
    field: &str,
) -> Result<String, CompilerError> {
    Ok(inner
        .next()
        .ok_or_else(|| lerr(&format!("{ctx}: missing {field}")))?
        .as_str()
        .to_string())
}

/// Strip surrounding double-quotes and unescape `\"` → `"` and `\\` → `\`.
fn unquote(s: &str) -> String {
    let trimmed = s.strip_prefix('"').unwrap_or(s);
    let trimmed = trimmed.strip_suffix('"').unwrap_or(trimmed);
    trimmed.replace("\\\"", "\"").replace("\\\\", "\\")
}

/// Convenience: create a `CompilerError::LoweringError`.
fn lerr(msg: &str) -> CompilerError {
    CompilerError::LoweringError(msg.to_string())
}
