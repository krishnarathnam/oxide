use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{self, BufReader, BufWriter},
    path::{Path, PathBuf},
    process::ExitCode,
    usize,
};
use tiny_http::{Header, Method, Request, Response, Server};
use xml::reader::{EventReader, XmlEvent};

type TermFreq = HashMap<String, usize>;
type DocFreq = HashMap<String, usize>;
type TermFreqIndex = HashMap<PathBuf, TermFreq>;

#[derive(Deserialize, Serialize, Default, Debug)]
struct IndexData {
    tfi: TermFreqIndex,
    df: DocFreq,
}

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

fn build_df_index(index_data: &mut IndexData) {
    for tf_table in index_data.tfi.values() {
        for term in tf_table.keys() {
            *index_data.df.entry(term.clone()).or_insert(0) += 1;
        }
    }
}

fn build_idf_index(index_data: &IndexData) -> HashMap<String, f32> {
    let mut idf_map: HashMap<String, f32> = HashMap::new();

    for (term, count) in &index_data.df {
        let idf = (index_data.tfi.len() as f32 / *count as f32).log10();
        idf_map.insert(term.to_string(), idf);
    }

    idf_map
}

fn save_index_to_json(index_data: &mut IndexData, index_path: &str) -> io::Result<()> {
    build_df_index(index_data);
    let index_file = File::create(index_path)?;
    serde_json::to_writer(BufWriter::new(index_file), index_data)?;
    Ok(())
}

fn save_folder_to_model(dir_path: &str, index_data: &mut IndexData) -> io::Result<()> {
    let dir = fs::read_dir(dir_path)?;
    for entry in dir {
        let path = entry?.path();

        if path.is_dir() {
            save_folder_to_model(path.to_str().unwrap(), index_data)?;
        } else {
            let document = match read_entire_xml_file(&path) {
                Ok(text) => {
                    println!("converted the file: {}", path.display());
                    text.chars().collect::<Vec<_>>()
                }
                Err(_) => {
                    println!("Skipping non-XML file {}", path.display());
                    continue;
                }
            };

            let mut tf = TermFreq::new();

            for term in Lexer::new(&document) {
                if let Some(freq) = tf.get_mut(&term) {
                    *freq += 1;
                } else {
                    tf.insert(term, 1);
                }
            }
            index_data.tfi.insert(path, tf);
        }
    }
    Ok(())
}

fn read_entire_xml_file<P: AsRef<Path>>(file_path: P) -> io::Result<String> {
    let file = File::open(file_path)?;
    let er = EventReader::new(BufReader::new(file));
    let mut content = String::new();

    if content.starts_with('\u{feff}') {
        content = content.replacen('\u{feff}', "", 1);
    }
    for event in er {
        let event = event.map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        if let XmlEvent::Characters(text) = event {
            content.push_str(&text);
            content.push_str(" ");
        } else {
            continue;
        }
    }

    Ok(content)
}

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
    eprintln!("    index <folder>                 Turn the folder and file into index.json");
    eprintln!("    search <folder>                search how many files are in index.json");
    eprintln!("    serve <folder> [address]       start local HTTP server with Web Interface");
}

fn serve_404_err(request: Request) -> io::Result<()> {
    let response = Response::from_string("404 - Page dosnt exist").with_status_code(404);
    request.respond(response)?;
    Ok(())
}

fn calculate_tf(term: &str, d: &TermFreq) -> f32 {
    let a = d.get(term).cloned().unwrap_or(0) as f32;
    let b = d.iter().map(|(_, f)| *f).sum::<usize>() as f32;
    a / b
}

fn serve_request(
    index_data: &IndexData,
    mut request: Request,
    idf_map: &HashMap<String, f32>,
) -> io::Result<()> {
    println!(
        "info: received request! method {:?} url: {:?}",
        request.method(),
        request.url()
    );

    match (request.method(), request.url()) {
        (Method::Post, "/api/search") => {
            let mut buf1: Vec<u8> = Vec::new();
            request.as_reader().read_to_end(&mut buf1)?;
            let body = str::from_utf8(&buf1).unwrap().chars().collect::<Vec<_>>();

            let mut score: Vec<(&Path, f32)> = Vec::new();
            for (path, tf_table) in &index_data.tfi {
                let mut doc_score: f32 = 0.0;
                for token in Lexer::new(&body) {
                    let tf = calculate_tf(&token, &tf_table);
                    let idf = *idf_map.get(&token).unwrap_or(&0.0);
                    doc_score += tf * idf;
                }

                score.push((&path, doc_score));
            }
            score.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            let top_results: Vec<_> = score
                .iter()
                .take(10)
                .map(|(path, rank)| format!("{} => {}", path.display(), rank))
                .collect();

            let body = top_results.join("\n");

            let response = Response::from_string(body)
                .with_header(Header::from_bytes("Content-Type", "text/plain").unwrap());

            request.respond(response)?;
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

            let mut index_data: IndexData = Default::default();

            save_folder_to_model(dir_path.as_str(), &mut index_data).map_err(|e| {
                eprintln!("ERROR: cannot read directory `{dir_path}`: {e}");
                ()
            })?;
            save_index_to_json(&mut index_data, "index.json").map_err(|e| {
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

            let index_data: IndexData = serde_json::from_reader(index_file).map_err(|e| {
                eprintln!("ERROR: cannot parse index file: {e}");
                ()
            })?;

            let address = args.next().unwrap_or("127.0.0.1:6969".to_string());
            let server = Server::http(&address).map_err(|e| {
                eprintln!("ERROR: Could not start server at {address}: {e}");
                ()
            })?;
            let idf_map = build_idf_index(&index_data);
            println!("Running server at: http://{address}");
            for request in server.incoming_requests() {
                let _ = serve_request(&index_data, request, &idf_map);
            }
        }

        _ => {
            usage(&program);
        }
    }

    Ok(())
}

fn main() -> ExitCode {
    //main2()?;
    match entry() {
        Ok(()) => ExitCode::SUCCESS,
        Err(()) => ExitCode::FAILURE,
    }
}
