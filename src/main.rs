use brainfuck::interpreter::Interpreter;
use clap::Parser;

#[derive(Parser)]
struct Cli {
    path: std::path::PathBuf,
}

fn main() {
    let args = Cli::parse();
    let code = std::fs::read_to_string(&args.path).expect("could not read file");
    let result = Interpreter::new(&code).execute();
    println!("{}", unsafe { String::from_utf8_unchecked(result) });
}
