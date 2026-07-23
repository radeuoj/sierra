use anyhow::Result;
use sierra::{analysis::Analysis, lexer::Lexer, parser::Parser};

fn main() -> Result<()> {
    let lexer = Lexer::new(br#"
        fn put(ch: i32) -> i32 {
            // ..
        }

        fn main() -> i32 {
            let a: i32 = 32
            put(a)
        }
    "#.into());

    let parser = Parser::new(lexer)?;

    let file = parser.parse_file()?;
    println!("{:?}", file);

    let analysis = Analysis::from(&file)?;
    println!("{:?}", analysis);

    Ok(())
}
