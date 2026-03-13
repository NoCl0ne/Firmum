/// Firmum Intermediate Representation (FIR).
///
/// Six node families as specified in firmum_arXiv_paper.txt §4.1:
/// IntentNode, AssumptionNode, ProofNode, TypeNode, PredicateNode, OwnershipNode.
///
/// These types are defined before the lowering pass (Stage 2) as required by
/// the autonomous development spec. Lowering code must not be written against
/// undefined types.
pub mod lower;

// ============================================================
// Lexical value types
// ============================================================

#[derive(Debug, Clone)]
pub enum Number {
    Integer(u64),
    Decimal(f64),
}

#[derive(Debug, Clone)]
pub struct Duration {
    pub value: Number,
    pub unit: TimeUnit,
}

#[derive(Debug, Clone)]
pub enum TimeUnit {
    Millisecond,
    Minute,
    Second,
    Hour,
    Day,
}

// ============================================================
// TypeNode family
// ============================================================

#[derive(Debug, Clone)]
pub enum TypeNode {
    Base(String),
    Refined {
        base: String,
        predicate: Box<PredicateNode>,
    },
    Dependent {
        base: String,
        type_param: Box<TypeNode>,
        value_param_name: String,
        value_param_type: String,
    },
    Contextual {
        base: String,
        context: String,
    },
    Temporal(TemporalType),
}

#[derive(Debug, Clone)]
pub enum TemporalType {
    Fresh {
        inner: Box<TypeNode>,
        duration: Duration,
    },
    Expiring {
        inner: Box<TypeNode>,
        duration: Duration,
    },
    Stale(Box<TypeNode>),
}

// ============================================================
// PredicateNode family
// ============================================================

#[derive(Debug, Clone)]
pub enum PredicateNode {
    Or(Box<PredicateNode>, Box<PredicateNode>),
    And(Box<PredicateNode>, Box<PredicateNode>),
    Not(Box<PredicateNode>),
    Forall {
        var: String,
        ty: Box<TypeNode>,
        body: Box<PredicateNode>,
    },
    Exists {
        var: String,
        ty: Box<TypeNode>,
        body: Box<PredicateNode>,
    },
    Comparison {
        left: ExprNode,
        op: ComparisonOp,
        right: ExprNode,
    },
}

#[derive(Debug, Clone)]
pub enum ComparisonOp {
    Eq,
    Ne,
    Le,
    Ge,
    Lt,
    Gt,
}

// ============================================================
// Expression nodes
// ============================================================

#[derive(Debug, Clone)]
pub enum ExprNode {
    Number(Number),
    StringLit(String),
    /// old(qualified_identifier) — valid only in postcondition and verify blocks.
    OldValue(String),
    FunctionCall {
        name: String,
        args: Vec<ExprNode>,
    },
    /// qualified_identifier (foo or foo.bar.baz)
    Identifier(String),
    BinOp {
        left: Box<ExprNode>,
        op: BinOpKind,
        right: Box<ExprNode>,
    },
}

#[derive(Debug, Clone)]
pub enum BinOpKind {
    Add,
    Sub,
    Mul,
    Div,
}

// ============================================================
// IntentNode family
// ============================================================

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: TypeNode,
}

#[derive(Debug, Clone)]
pub struct IntentNode {
    pub name: String,
    pub inputs: Vec<Param>,
    pub outputs: Vec<Param>,
    pub preconditions: Vec<PredicateNode>,
    pub postconditions: Vec<PredicateNode>,
    pub invariants: Vec<PredicateNode>,
    pub never: Vec<String>,
}

// ============================================================
// AssumptionNode family
// ============================================================

#[derive(Debug, Clone)]
pub struct AssumptionNode {
    pub name: String,
    pub sections: Vec<AssumptionSection>,
}

#[derive(Debug, Clone)]
pub enum AssumptionSection {
    StringAssumption(String),
    ContextSource(Vec<SourceRef>),
    OutOfScope(Vec<String>),
    ValidatedBy(ValidatedBy),
}

#[derive(Debug, Clone)]
pub struct SourceRef {
    pub source_type: SourceType,
    pub path: String,
}

#[derive(Debug, Clone)]
pub enum SourceType {
    Ref,
    Slack,
    Email,
    Github,
    Jira,
    Doc,
}

#[derive(Debug, Clone)]
pub struct ValidatedBy {
    pub fields: Vec<ValidatedByField>,
}

#[derive(Debug, Clone)]
pub enum ValidatedByField {
    DomainExpert(String),
    Date(String),
    Confidence(f64),
    Method(ValidationMethod),
}

#[derive(Debug, Clone)]
pub enum ValidationMethod {
    Interview,
    DocumentReview,
    FormalAudit,
    PeerReview,
}

// ============================================================
// ProofNode family
// ============================================================

#[derive(Debug, Clone)]
pub struct ProofNode {
    pub name: String,
    pub strategy: StrategyExpr,
    pub lemmas: Vec<LemmaDecl>,
    pub verify_decls: Vec<VerifyDecl>,
    pub certificate: Option<CertificatePlaceholder>,
}

#[derive(Debug, Clone)]
pub struct StrategyExpr {
    pub primary: StrategyName,
    pub fallback: Option<StrategyName>,
}

#[derive(Debug, Clone)]
pub enum StrategyName {
    SmtSolverZ3,
    BoundedModelChecking,
    Induction,
    AiAssisted,
}

#[derive(Debug, Clone)]
pub struct LemmaDecl {
    pub name: String,
    pub predicates: Vec<PredicateNode>,
    pub proof_method: Option<ProofMethod>,
}

#[derive(Debug, Clone)]
pub struct ProofMethod {
    pub technique: ProofTechnique,
}

#[derive(Debug, Clone)]
pub enum ProofTechnique {
    Induction { on: String },
    Contradiction,
    Direct,
}

#[derive(Debug, Clone)]
pub struct VerifyDecl {
    pub target: String,
    pub using: Option<String>,
    pub statements: Vec<VerifyStatement>,
}

#[derive(Debug, Clone)]
pub enum VerifyStatement {
    Assert(PredicateNode),
    Atomic(Vec<VerifyStatement>),
    Assign {
        target: String,
        op: AssignOp,
        expr: ExprNode,
    },
}

#[derive(Debug, Clone)]
pub enum AssignOp {
    Assign,
    AddAssign,
    SubAssign,
}

/// The string value in source is a placeholder; the compiler emits the real
/// ModuleCertificate (Ed25519-signed) during `firmum build`.
#[derive(Debug, Clone)]
pub struct CertificatePlaceholder {
    pub value: String,
}

// ============================================================
// OwnershipNode family
// ============================================================

/// Ownership is encoded as a logical precondition, not a runtime check.
/// Input parameters receive an implicit Owned node.
/// Postconditions containing old(x) generate an OldBorrow node for x.
#[derive(Debug, Clone)]
pub enum OwnershipNode {
    Owned(String),
    OldBorrow(String),
}

// ============================================================
// Top-level program structures
// ============================================================

#[derive(Debug, Clone)]
pub struct ContextDecl {
    pub type_name: String,
    pub context_name: String,
    pub fields: Vec<ContextField>,
}

#[derive(Debug, Clone)]
pub struct ContextField {
    pub name: String,
    pub value: ContextFieldValue,
}

#[derive(Debug, Clone)]
pub enum ContextFieldValue {
    StringVal(String),
    Integer(u64),
    Decimal(f64),
    Boolean(bool),
}

#[derive(Debug, Clone)]
pub struct LetBinding {
    pub name: String,
    pub ty: Option<TypeNode>,
    pub expr: ExprNode,
}

#[derive(Debug, Clone)]
pub struct Declaration {
    pub intent: IntentNode,
    pub assumption: AssumptionNode,
    pub proof: ProofNode,
}

/// The top-level program after FIR lowering.
#[derive(Debug, Clone)]
pub struct Program {
    pub contexts: Vec<ContextDecl>,
    pub lets: Vec<LetBinding>,
    pub declarations: Vec<Declaration>,
}
