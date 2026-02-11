use std::{
    collections::HashMap,
    fs::{self, File},
    io,
    path::{Path, PathBuf},
    process::ExitCode,
};
use tiny_http::{Header, Method, Request, Response, Server};
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

    fn chop_while<P>(&mut self, _predicate: P) -> &'a [char]
    where
        P: FnMut(&char) -> bool,
    {
        let mut n = 0;
        while n < self.content.len() && self.content[n].is_alphanumeric() {
            n += 1;
        }

        self.chop(n)
    }

    fn next_token(&mut self) -> Option<String> {
        self.trim_left_whitespace();
        if self.content.len() == 0 {
            return None;
        }

        if self.content[0].is_numeric() {
            return Some(self.chop_while(|x| x.is_numeric()).iter().collect());
        }

        if self.content[0].is_alphabetic() {
            return Some(
                self.chop_while(|x| x.is_alphanumeric())
                    .iter()
                    .map(|x| x.to_ascii_uppercase())
                    .collect(),
            );
        }

        return Some(self.chop(1).iter().collect());
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = String;

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

        for term in Lexer::new(&document) {
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

fn serve_static_file(file_path: &str, request: Request, content_type: &str) -> io::Result<()> {
    let file_content = File::open(file_path)?;
    let content_type_text_html = Header::from_bytes("Content-Type", content_type).unwrap();
    let response = Response::from_file(file_content).with_header(content_type_text_html.clone());
    request.respond(response)?;
    Ok(())
}

fn usage(program: &str) {
    eprintln!("Usage: {program} [SUBCOMMAND] [OPTIONS]");
    eprintln!("Subcommands:");
    eprintln!("    serve <folder> [address]       start local HTTP server with Web Interface");
}

fn serve_404_err(request: Request) -> io::Result<()> {
    let response = Response::from_string("404 - Page dosnt exist").with_status_code(404);
    request.respond(response)?;
    Ok(())
}

fn tf_search(term: &str, d: &TF) -> f32 {
    let a = d.get(term).cloned().unwrap_or(0) as f32;
    let b = d.iter().map(|(_, f)| *f).sum::<usize>() as f32;
    a / b
}

fn serve_request(tf_index: &TermFreqIndex, mut request: Request) -> io::Result<()> {
    println!(
        "INFO: received request! method {:?} url: {:?}",
        request.method(),
        request.url()
    );

    match (request.method(), request.url()) {
        (Method::Post, "/api/search") => {
            let mut buf1: Vec<u8> = Vec::new();
            request.as_reader().read_to_end(&mut buf1)?;
            let body = str::from_utf8(&buf1).unwrap().chars().collect::<Vec<_>>();

            for (path, tf_table) in tf_index {
                let mut total_tf: f32 = 0.0;
                for token in Lexer::new(&body) {
                    total_tf += tf_search(&token.to_string(), &tf_table);
                }

                println!(
                    "{path} total score for this document: {total_tf}",
                    path = path.display()
                );
            }
        }
        (Method::Get, "/") | (Method::Get, "/index.html") => {
            serve_static_file("./index.html", request, "text/html; charset=uts-8")?
        }

        (Method::Get, "/index.js") => {
            serve_static_file("./index.js", request, "text/javascript; charset=uts-8")?
        }

        _ => serve_404_err(request)?,
    }

    Ok(())
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
            let index_path = args.next().ok_or_else(|| {
                usage(&program);
                eprintln!("ERROR: no directory is provided for {subcommand} subcommand");
            })?;

            let index_file = File::open(index_path).map_err(|e| {
                eprintln!("ERROR: cannot open index file: {e}");
                ()
            })?;

            let tf_index: TermFreqIndex = serde_json::from_reader(index_file).map_err(|e| {
                eprintln!("ERROR: cannot parse index file: {e}");
                ()
            })?;

            let address = args.next().unwrap_or("127.0.0.1:6969".to_string());
            let server = Server::http(&address).map_err(|e| {
                eprintln!("ERROR: Could not start server at {address}: {e}");
                ()
            })?;

            println!("Running server at: http://{address}");
            for request in server.incoming_requests() {
                let _ = serve_request(&tf_index, request);
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
