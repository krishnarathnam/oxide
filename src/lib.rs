use std::env;
use std::io::Write;
use std::path::Path;
mod built_in_commands;

pub enum ShellCommand<'a> {
    Exit,
    Echo(Vec<&'a str>),
    Pwd,
    Type(&'a str, Vec<&'a str>),
    Cd(&'a str, Vec<&'a str>),
    External(&'a str, Vec<&'a str>),
    Empty,
}

pub enum Redirect<'a> {
    AppendStdout(&'a str),
    AppendStderr(&'a str),
    Stderr(&'a str),
    Stdout(&'a str),
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
                let (real_args, redirect) = split_redirect(&args);
                let mut output = real_args.join(" ");
                match &redirect {
                    Some(Redirect::Stderr(file)) => {
                        println!("{output}");
                        write_output("", Some(Redirect::Stderr(file)));
                    }
                    Some(Redirect::Stdout(file)) => {
                        output.push('\n');
                        write_output(&output, Some(Redirect::Stdout(file)));
                    }
                    Some(Redirect::AppendStdout(file)) => {
                        output.push('\n');
                        write_output(&output, Some(Redirect::AppendStdout(file)));
                    }
                    Some(Redirect::AppendStderr(file)) => {
                        println!("{output}");
                        write_output("", Some(Redirect::AppendStderr(file)));
                    }
                    None => println!("{output}"),
                }
            }

            ShellCommand::Pwd => {
                let path = env::current_dir().unwrap();
                let output = format!("{}", path.display());
                write_output(&output, None);
            }

            ShellCommand::Type(name, args) => {
                let output;
                let (_, redirect) = split_redirect(&args);
                if built_in_commands::is_builtin(name) {
                    output = format!("{name} is a shell builtin");
                } else if let Some(exe) = pathsearch::find_executable_in_path(name) {
                    output = format!("{name} is {}", exe.display());
                } else {
                    output = format!("{name}: not found");
                }

                match redirect {
                    Some(Redirect::Stderr(file)) => {
                        println!("{output}");
                        write_output("", Some(Redirect::Stderr(file)));
                    }
                    Some(Redirect::Stdout(file)) => {
                        write_output(&output, Some(Redirect::Stdout(file)));
                    }
                    Some(Redirect::AppendStdout(file)) => {
                        write_output(&output, Some(Redirect::AppendStdout(file)));
                    }
                    Some(Redirect::AppendStderr(file)) => {
                        write_output(&output, Some(Redirect::AppendStderr(file)));
                    }
                    None => println!("{output}"),
                }
            }

            ShellCommand::Cd(path, args) => {
                let (_, redirect) = split_redirect(&args);
                let target = if path == "~" {
                    env::var("HOME").unwrap_or_else(|_| "/".to_string())
                } else {
                    path.to_string()
                };

                if Path::new(&target).is_dir() {
                    if let Err(e) = env::set_current_dir(&target) {
                        let error_msg = format!("cd: {}", e);
                        match &redirect {
                            Some(Redirect::Stderr(file)) => {
                                write_output(&error_msg, Some(Redirect::Stderr(file)));
                            }
                            Some(Redirect::Stdout(file)) => {
                                write_output(&error_msg, Some(Redirect::Stdout(file)));
                            }
                            Some(Redirect::AppendStdout(file)) => {
                                write_output(&error_msg, Some(Redirect::AppendStdout(file)));
                            }
                            Some(Redirect::AppendStderr(file)) => {
                                write_output(&error_msg, Some(Redirect::AppendStderr(file)));
                            }
                            None => {
                                eprintln!("{error_msg}");
                            }
                        }
                    }
                } else {
                    let error_msg = format!("cd: {}: No such file or directory", target);
                    match &redirect {
                        Some(Redirect::Stderr(file)) => {
                            write_output(&error_msg, Some(Redirect::Stderr(file)));
                        }
                        Some(Redirect::Stdout(file)) => {
                            write_output(&error_msg, Some(Redirect::Stdout(file)));
                        }
                        Some(Redirect::AppendStdout(file)) => {
                            write_output(&error_msg, Some(Redirect::AppendStdout(file)));
                        }
                        Some(Redirect::AppendStderr(file)) => {
                            write_output(&error_msg, Some(Redirect::AppendStderr(file)));
                        }
                        None => {
                            eprintln!("{error_msg}");
                        }
                    }
                }
            }

            ShellCommand::External(cmd, args) => {
                let (real_args, redirect) = split_redirect(&args);
                if pathsearch::find_executable_in_path(cmd).is_none() {
                    println!("{cmd}: command not found");
                    return true;
                }

                let output = std::process::Command::new(cmd)
                    .args(&real_args)
                    .output()
                    .unwrap();

                match &redirect {
                    Some(Redirect::Stdout(file)) => {
                        std::fs::write(file, &output.stdout).unwrap();
                        eprint!("{}", String::from_utf8_lossy(&output.stderr));
                    }
                    Some(Redirect::Stderr(file)) => {
                        std::fs::write(file, &output.stderr).unwrap();
                        print!("{}", String::from_utf8_lossy(&output.stdout));
                    }
                    Some(Redirect::AppendStdout(file)) => {
                        std::fs::OpenOptions::new()
                            .write(true)
                            .append(true)
                            .create(true)
                            .open(file)
                            .unwrap()
                            .write_all(&output.stdout)
                            .unwrap();
                        eprint!("{}", String::from_utf8_lossy(&output.stderr));
                    }
                    Some(Redirect::AppendStderr(file)) => {
                        std::fs::OpenOptions::new()
                            .write(true)
                            .append(true)
                            .create(true)
                            .open(file)
                            .unwrap()
                            .write_all(&output.stderr)
                            .unwrap();
                        eprint!("{}", String::from_utf8_lossy(&output.stdout));
                    }
                    None => {
                        print!("{}", String::from_utf8_lossy(&output.stdout));
                        eprint!("{}", String::from_utf8_lossy(&output.stderr));
                    }
                }
            }

            ShellCommand::Empty => {}
        }

        true
    }
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

fn split_redirect<'a>(args: &'a [&'a str]) -> (Vec<&'a str>, Option<Redirect<'a>>) {
    if let Some(pos) = args.iter().position(|a| *a == ">>" || *a == "1>>") {
        if pos + 1 < args.len() {
            return (
                args[..pos].to_vec(),
                Some(Redirect::AppendStdout(&args[pos + 1])),
            );
        }
    }
    if let Some(pos) = args.iter().position(|a| *a == "2>>") {
        if pos + 1 < args.len() {
            return (
                args[..pos].to_vec(),
                Some(Redirect::AppendStderr(&args[pos + 1])),
            );
        }
    }
    if let Some(pos) = args.iter().position(|a| *a == "2>") {
        if pos + 1 < args.len() {
            return (args[..pos].to_vec(), Some(Redirect::Stderr(&args[pos + 1])));
        }
    }
    if let Some(pos) = args.iter().position(|a| *a == ">" || *a == "1>") {
        if pos + 1 < args.len() {
            return (args[..pos].to_vec(), Some(Redirect::Stdout(&args[pos + 1])));
        }
    }
    (args.to_vec(), None)
}

fn write_output(text: &str, redirect: Option<Redirect>) {
    match redirect {
        Some(Redirect::Stdout(file)) => {
            std::fs::write(file, text.as_bytes()).unwrap();
        }
        Some(Redirect::Stderr(file)) => {
            std::fs::write(file, text.as_bytes()).unwrap();
        }
        Some(Redirect::AppendStdout(file)) => {
            std::fs::OpenOptions::new()
                .write(true)
                .append(true)
                .create(true)
                .open(file)
                .unwrap()
                .write_all(text.as_bytes())
                .unwrap();
        }
        Some(Redirect::AppendStderr(file)) => {
            std::fs::OpenOptions::new()
                .write(true)
                .append(true)
                .create(true)
                .open(file)
                .unwrap()
                .write_all(text.as_bytes())
                .unwrap();
        }
        None => {
            println!("{text}");
        }
    }
}
