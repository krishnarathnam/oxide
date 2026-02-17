use std::env;
use std::io::Write;
use std::path::Path;

pub enum ShellCommand<'a> {
    Exit,
    Echo(Vec<&'a str>),
    Pwd,
    Type(&'a str, Vec<&'a str>),
    Cd(&'a str, Vec<&'a str>),
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
                .map(|x| ShellCommand::Type(x, tokens[1..].to_vec()))
                .unwrap_or(ShellCommand::Empty),

            "cd" => tokens
                .get(1)
                .map(|x| ShellCommand::Cd(x, tokens[1..].to_vec()))
                .unwrap_or(ShellCommand::Empty),

            cmd => ShellCommand::External(cmd, tokens[1..].to_vec()),
        }
    }

    pub fn execute(self) -> bool {
        // capture output buffer

        match self {
            ShellCommand::Exit => return false,

            ShellCommand::Echo(args) => {
                let (real_args, redirect, is_stderr) = split_redirect(&args);
                let output = real_args.join(" ");
                if is_stderr {
                    println!("{output}");
                    if let Some(file) = redirect {
                        std::fs::write(file, b"").unwrap();
                    }
                } else {
                    write_or_print(&output.to_string(), redirect);
                }
            }

            ShellCommand::Pwd => {
                let path = env::current_dir().unwrap();
                let output = format!("{}", path.display());
                write_or_print(&output, None);
            }

            ShellCommand::Type(name, args) => {
                let output;
                let (_, file, print_error) = split_redirect(&args);
                if is_builtin(name) {
                    output = format!("{name} is a shell builtin");
                } else if let Some(exe) = pathsearch::find_executable_in_path(name) {
                    output = format!("{name} is {}", exe.display());
                } else {
                    output = format!("{name}: not found");
                }
                if print_error {
                    write_or_print_error(&output, file);
                } else {
                    write_or_print(&output, None);
                }
            }

            ShellCommand::Cd(path, args) => {
                let (_, file, print_error) = split_redirect(&args);
                let target = if path == "~" {
                    env::var("HOME").unwrap_or_else(|_| "/".into())
                } else {
                    path.to_string()
                };

                if Path::new(&target).is_dir() {
                    if let Err(e) = env::set_current_dir(&target) {
                        if print_error {
                            let output = format!("cd: {e}");
                            write_or_print_error(&output, file);
                        } else {
                            eprintln!("cd: {e}");
                        }
                    }
                } else {
                    if print_error {
                        let output = format!("cd: {}: No such file or directory", target);
                        write_or_print_error(&output, file);
                    } else {
                        eprintln!("cd: {}: No such file or directory", target);
                    }
                }
            }

            ShellCommand::External(cmd, args) => {
                let (real_args, redirect, print_error) = split_redirect(&args);

                if pathsearch::find_executable_in_path(cmd).is_none() {
                    println!("{cmd}: command not found");
                    return true;
                }

                let output = std::process::Command::new(cmd)
                    .args(&real_args)
                    .output()
                    .unwrap();

                match (redirect, print_error) {
                    (Some(file), true) => {
                        // 2> : stdout→terminal, stderr→file
                        print!("{}", String::from_utf8_lossy(&output.stdout));
                        if !output.stderr.is_empty() {
                            std::fs::write(file, &output.stderr).unwrap();
                        }
                    }
                    (Some(file), false) => {
                        // > : stdout→file, stderr→terminal
                        std::fs::write(file, &output.stdout).unwrap();
                        if !output.stderr.is_empty() {
                            eprint!("{}", String::from_utf8_lossy(&output.stderr));
                        }
                    }
                    (None, _) => {
                        // No redirect
                        print!("{}", String::from_utf8_lossy(&output.stdout));
                        if !output.stderr.is_empty() {
                            eprint!("{}", String::from_utf8_lossy(&output.stderr));
                        }
                    }
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

pub fn tokenize(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut in_blackslash = false;

    for c in input.chars() {
        if in_blackslash {
            current.push(c);
            in_blackslash = false;
            continue;
        }

        match c {
            '\'' if !in_double => {
                in_single = !in_single;
            }

            '"' if !in_single => {
                in_double = !in_double;
            }

            '\\' if !in_single => {
                in_blackslash = true;
            }

            ' ' | '\t' if !in_single && !in_double => {
                if !current.is_empty() {
                    args.push(current.clone());
                    current.clear();
                }
            }

            _ => current.push(c),
        }
    }

    if !current.is_empty() {
        args.push(current);
    }

    args
}

fn split_redirect<'a>(args: &'a [&'a str]) -> (Vec<&'a str>, Option<&'a str>, bool) {
    if let Some(pos) = args.iter().position(|a| *a == "2>") {
        if pos + 1 < args.len() {
            return (args[..pos].to_vec(), Some(args[pos + 1]), true);
        }
    }
    if let Some(pos) = args.iter().position(|a| *a == ">" || *a == "1>") {
        if pos + 1 < args.len() {
            return (args[..pos].to_vec(), Some(args[pos + 1]), false);
        }
    }
    (args.to_vec(), None, false)
}

fn write_or_print(text: &String, redirect: Option<&str>) {
    if let Some(file) = redirect {
        let mut f = std::fs::File::create(file).unwrap();
        writeln!(f, "{text}").unwrap();
    } else {
        println!("{text}");
    }
}

fn write_or_print_error(text: &String, redirect: Option<&str>) {
    if let Some(file) = redirect {
        let mut f = std::fs::File::create(file).unwrap();
        writeln!(f, "{text}").unwrap();
    } else {
        println!("{text}");
    }
}
