use crate::errors::CompilerError;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "firmum", about = "Firmum formal verification compiler")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Type check and compute ACS — no Z3
    Check { file: std::path::PathBuf },
    /// Full build: type check + Z3 verification + certificate emission
    Build { file: std::path::PathBuf },
    /// Print proof certificate for a module
    Proof { module_id: String },
    /// Print ACS score and coverage gap report
    Acs { file: std::path::PathBuf },
    /// Print the raw parse tree (debugging aid)
    Ast { file: std::path::PathBuf },
}

impl Cli {
    pub fn run(&self) -> Result<(), CompilerError> {
        match &self.command {
            Command::Check { file } => run_check(file),
            Command::Build { file } => run_build(file),
            Command::Proof { module_id } => run_proof(module_id),
            Command::Acs { file } => run_acs(file),
            Command::Ast { file } => run_ast(file),
        }
    }
}

fn read_source(file: &std::path::Path) -> Result<String, CompilerError> {
    std::fs::read_to_string(file).map_err(CompilerError::from)
}

fn parse_source(
    source: &str,
) -> Result<pest::iterators::Pairs<'_, crate::parser::Rule>, CompilerError> {
    crate::parser::parse(source).map_err(|e| CompilerError::ParseError(e.to_string()))
}

fn run_check(file: &std::path::Path) -> Result<(), CompilerError> {
    let source = read_source(file)?;
    let pairs = parse_source(&source)?;
    let program = crate::fir::lower::lower(pairs)?;
    crate::typeck::check(&program)
}

fn run_build(file: &std::path::Path) -> Result<(), CompilerError> {
    let source = read_source(file)?;
    let pairs = parse_source(&source)?;
    let program = crate::fir::lower::lower(pairs)?;
    crate::typeck::check(&program)?;
    crate::smt::orchestrate(&program)
}

fn run_proof(_module_id: &str) -> Result<(), CompilerError> {
    Err(CompilerError::NotYetImplemented("proof command (Stage 4)"))
}

fn run_acs(file: &std::path::Path) -> Result<(), CompilerError> {
    let source = read_source(file)?;
    let pairs = parse_source(&source)?;
    let program = crate::fir::lower::lower(pairs)?;
    crate::typeck::acs::compute(&program).map(|score| {
        println!("ACS: {score:.4}");
    })
}

fn run_ast(file: &std::path::Path) -> Result<(), CompilerError> {
    let source = read_source(file)?;
    let pairs = parse_source(&source)?;
    println!("{pairs:#?}");
    Ok(())
}
