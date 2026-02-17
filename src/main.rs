use std::{
    env::current_dir,
    io::{self, Write},
};

fn main() {
    loop {
        let cur = current_dir().unwrap();
        let last = cur.components().last().unwrap().as_os_str();
        print!("{} ❯ ", last.display());
        //println!();
        //println!("{}", current_dir().unwrap().display());
        //print!("$ ");
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
