use crate::model;
use std::{
    collections::HashMap,
    fs::File,
    io::{self},
    path::Path,
};

use tiny_http::{Header, Method, Request, Response};

fn serve_static_file(file_path: &str, request: Request, content_type: &str) -> io::Result<()> {
    let file_content = File::open(file_path)?;
    let content_type_text_html = Header::from_bytes("Content-Type", content_type).unwrap();
    let response = Response::from_file(file_content).with_header(content_type_text_html.clone());
    request.respond(response)?;
    Ok(())
}
fn serve_404_err(request: Request) -> io::Result<()> {
    let response = Response::from_string("404 - Page dosnt exist").with_status_code(404);
    request.respond(response)?;
    Ok(())
}

pub fn serve_request(
    inverted_index_data: &model::InvertedIndexData,
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
            let mut buf: Vec<u8> = Vec::new();
            request.as_reader().read_to_end(&mut buf)?;
            let body = str::from_utf8(&buf).unwrap().chars().collect::<Vec<_>>();

            let mut scores: HashMap<&Path, f32> = HashMap::new();
            for token in model::Lexer::new(&body) {
                if let Some(posting) = inverted_index_data.index_freq.get(&token) {
                    let idf = &*idf_map.get(&token).unwrap_or(&0f32);

                    for (doc, freq) in posting {
                        // TF using your stored doc length
                        let len = *inverted_index_data.doc_len.get(doc).unwrap_or(&1) as f32;
                        let tf = *freq as f32 / len;

                        *scores.entry(doc.as_path()).or_insert(0.0) += (tf * idf).sqrt();
                    }
                }
            }

            let mut score: Vec<_> = scores.into_iter().collect();
            score.sort_by(|(_, a), (_, b)| b.partial_cmp(&a).unwrap_or(std::cmp::Ordering::Equal));

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
