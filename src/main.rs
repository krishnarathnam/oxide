use std::{
    collections::HashMap,
    fs::{self, File},
    io,
    path::{Path, PathBuf},
    process::ExitCode,
};
use tiny_http::{Header, Response, Server};
use xml::reader::{EventReader, XmlEvent};

#[derive(Debug)]
struct Lexer<'a> {
    content: &'a [char],
}

impl<'a> Lexer<'a> {
    fn new(content: &'a [char]) -> Self {
        Self { content }
    }

    fn trim_left_whitespace(&mut self) {
        while self.content.len() > 0 && self.content[0].is_whitespace() {
            self.content = &self.content[1..];
        }
    }

    fn chop(&mut self, n: usize) -> &'a [char] {
        let token = &self.content[0..n];
        self.content = &self.content[n..];
        token
    }

    fn next_token(&mut self) -> Option<&'a [char]> {
        self.trim_left_whitespace();
        if self.content.len() == 0 {
            return None;
        }

        if self.content[0].is_numeric() {
            let mut n = 0;
            while n < self.content.len() && self.content[n].is_alphanumeric() {
                n += 1;
            }

            return Some(self.chop(n));
        }

        if self.content[0].is_alphabetic() {
            let mut n = 0;
            while n < self.content.len() && self.content[n].is_alphanumeric() {
                n += 1;
            }

            return Some(self.chop(n));
        }

        return Some(self.chop(1));
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = &'a [char];

    fn next(&mut self) -> Option<Self::Item> {
        self.next_token()
    }
}

fn check_index(index_path: &str) -> io::Result<()> {
    let index_file = File::open(index_path)?;
    let tf_index: TermFreqIndex = serde_json::from_reader(index_file)?;
    println!(
        "Index.json contains: {length} files",
        length = tf_index.len()
    );

    Ok(())
}

fn save_tf_index(tf_index: &TermFreqIndex, index_path: &str) -> io::Result<()> {
    let index_file = File::create(index_path)?;
    serde_json::to_writer(index_file, tf_index)?;
    Ok(())
}

fn tf_index_of_folder(file_path: &str, tf_index: &mut TermFreqIndex) -> io::Result<()> {
    let dir = fs::read_dir(file_path)?;
    for file in dir {
        let file = file?.path();
        let document = read_entire_xml_file(&file)?.chars().collect::<Vec<_>>();

        let mut tf = TF::new();

        for lexer in Lexer::new(&document) {
            let term = lexer
                .iter()
                .map(|x| x.to_ascii_uppercase())
                .collect::<String>();

            if let Some(freq) = tf.get_mut(&term) {
                *freq += 1;
            } else {
                tf.insert(term, 1);
            }
        }

        tf_index.insert(file, tf);
    }
    Ok(())
}

fn read_entire_xml_file<P: AsRef<Path>>(file_path: P) -> io::Result<String> {
    let file = File::open(file_path)?;

    let er = EventReader::new(file);
    let mut content = String::new();

    for event in er {
        //let event = event.unwrap_or_else(|err| {
        //    eprintln!("ERROR: cannot read next xml file: {err}");
        //    exit(1)
        //});

        if let XmlEvent::Characters(text) = event.expect("ERROR: cannot read next xml file: {err}")
        {
            content.push_str(&text);
            content.push_str(" ");
        }
    }

    Ok(content)
}
type TF = HashMap<String, usize>;
type TermFreqIndex = HashMap<PathBuf, TF>;

fn usage(program: &str) {
    eprintln!("Usage: {program} [SUBCOMMAND] [OPTIONS]");
    eprintln!("Subcommands:");
    eprintln!("    serve <folder> [address]       start local HTTP server with Web Interface");
}

fn entry() -> Result<(), ()> {
    let mut args = std::env::args();
    let program = args.next().expect("Path to prgram is provided");

    let subcommand = args.next().ok_or_else(|| {
        usage(&program);
        eprintln!("ERROR: no address provided");
        ()
    })?;

    match subcommand.as_str() {
        "index" => {
            let dir_path = args.next().ok_or_else(|| {
                usage(&program);
                eprintln!("ERROR: no directory is provided for {subcommand} subcommand");
            })?;

            let mut tf_index = TermFreqIndex::new();

            tf_index_of_folder(dir_path.as_str(), &mut tf_index).map_err(|e| {
                eprintln!("ERROR: cannot read directory `{dir_path}`: {e}");
                ()
            })?;
            save_tf_index(&tf_index, "index.json").map_err(|e| {
                eprintln!("ERROR: cannot read directory `{dir_path}`: {e}");
                ()
            })?;
        }

        "search" => {
            let index_path = args.next().ok_or_else(|| {
                usage(&program);
                eprintln!("ERROR: no directory is provided for {subcommand} subcommand");
            })?;

            check_index(&index_path).map_err(|e| {
                eprintln!("ERROR: {e}");
                ()
            })?;
        }

        "serve" => {
            let address = args.next().unwrap_or("127.0.0.1:6969".to_string());
            let server = Server::http(&address).map_err(|e| {
                eprintln!("ERROR: Could not start server at {address}: {e}");
                ()
            })?;

            let file_content = fs::read_to_string("./index.html").map_err(|e| {
                eprintln!("ERROR: Could not start server at {address}: {e}");
                ()
            })?;

            let content_type_text_html =
                Header::from_bytes("Content-Type", "text/html; charset=uts-8").unwrap();
            println!("Running server at: http://{address}");

            for request in server.incoming_requests() {
                println!(
                    "INFO: received request! method {:?} url: {:?}",
                    request.method(),
                    request.url()
                );
                let response = Response::from_string(&file_content)
                    .with_header(content_type_text_html.clone());
                request.respond(response).map_err(|e| {
                    eprintln!("ERROR: Could not send response at {address}: {e}");
                })?;
            }
        }

        _ => {
            usage(&program);
        }
    }

    todo!();
}

fn main() -> ExitCode {
    //main2()?;
    match entry() {
        Ok(()) => ExitCode::SUCCESS,
        Err(()) => ExitCode::FAILURE,
    }
}
