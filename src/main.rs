use std::io::{self, Write};

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            continue;
        }

        let tokens: Vec<&str> = input.trim().split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }

        let command = oxide::ShellCommand::parse(&tokens);

        if !command.execute() {
            break;
        }
    }
}
