use pest::Parser;
use pest_derive::Parser;

/// Pest-derived parser for Firmum source files.
///
/// The grammar path is relative to the crate root (CARGO_MANIFEST_DIR).
/// firmum.pest lives at the repository root, one level above `src/`.
#[derive(Parser)]
#[grammar = "../firmum.pest"]
pub struct FirmumParser;

/// Parse a Firmum source string and return the top-level pest `Pairs`.
///
/// The returned iterator yields a single pair matching the `program` rule.
/// Callers that need the program's children should call
/// `.next().unwrap().into_inner()` — the outer `program` pair is always
/// present when `Ok` is returned.
pub fn parse(source: &str) -> Result<pest::iterators::Pairs<'_, Rule>, pest::error::Error<Rule>> {
    FirmumParser::parse(Rule::program, source)
}
