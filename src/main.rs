use std::io::{stdin, stdout};

use clap::Parser;

use brainfuck::interpreter::Interpreter;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    path: std::path::PathBuf,
}

fn main() {
    let args = Cli::parse();
    let code = std::fs::read_to_string(&args.path).expect("could not read file");
    let interpreter = Interpreter::build(&code).unwrap();
    interpreter.execute(&mut stdin(), &mut stdout()).unwrap();
}
