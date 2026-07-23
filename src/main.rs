use anyhow::{Context, Result};
use sierra::{analysis::Analysis, compiler::Compiler, lexer::Lexer, parser::Parser};

fn main() -> Result<()> {
    let path = std::env::args().nth(1).context("no input file")?;
    let input = std::fs::read(&path)
        .with_context(|| format!("{} file not found", path))?;
    let lexer = Lexer::new(input);
    let parser = Parser::new(lexer)?;
    let file = parser.parse_file()?;
    let analysis = Analysis::new(&file)?;
    let compiler = Compiler::new(file, analysis);
    std::fs::write(format!("{}.c", path), compiler.compile())?;

    Ok(())
}
