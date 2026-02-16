use std::env;
use std::path::Path;

pub enum ShellCommand<'a> {
    Exit,
    Echo(Vec<&'a str>),
    Pwd,
    Type(&'a str),
    Cd(&'a str),
    External(&'a str, Vec<&'a str>),
    Empty,
}

impl<'a> ShellCommand<'a> {
    pub fn parse(tokens: &'a [&'a str]) -> Self {
        match tokens[0] {
            "exit" => ShellCommand::Exit,
            "echo" => ShellCommand::Echo(tokens[1..].to_vec()),
            "pwd" => ShellCommand::Pwd,
            "type" => tokens
                .get(1)
                .map(|x| ShellCommand::Type(x))
                .unwrap_or(ShellCommand::Empty),

            "cd" => tokens
                .get(1)
                .map(|x| ShellCommand::Cd(x))
                .unwrap_or(ShellCommand::Empty),

            cmd => ShellCommand::External(cmd, tokens[1..].to_vec()),
        }
    }

    pub fn execute(self) -> bool {
        match self {
            ShellCommand::Exit => return false,

            ShellCommand::Echo(args) => {
                println!("{}", args.join(" "));
            }

            ShellCommand::Pwd => {
                if let Ok(path) = env::current_dir() {
                    println!("{}", path.display());
                }
            }

            ShellCommand::Type(name) => {
                if is_builtin(name) {
                    println!("{name} is a shell builtin");
                } else if let Some(exe) = pathsearch::find_executable_in_path(name) {
                    println!("{name} is {}", exe.display());
                } else {
                    println!("{name}: not found");
                }
            }

            ShellCommand::Cd(path) => {
                let target = if path == "~" {
                    env::var("HOME").unwrap_or_else(|_| "/".into())
                } else {
                    path.to_string()
                };

                if Path::new(&target).is_dir() {
                    if let Err(e) = env::set_current_dir(&target) {
                        eprintln!("cd: {e}");
                    }
                } else {
                    println!("cd: {}: No such directory", target);
                }
            }

            ShellCommand::External(cmd, args) => {
                if pathsearch::find_executable_in_path(cmd).is_some() {
                    let _ = std::process::Command::new(cmd).args(args).status();
                } else {
                    println!("{cmd}: command not found");
                }
            }

            ShellCommand::Empty => {}
        }

        true
    }
}

fn is_builtin(cmd: &str) -> bool {
    matches!(cmd, "exit" | "echo" | "pwd" | "type" | "cd")
}
