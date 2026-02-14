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

fn calculate_tf(term: &str, d: &model::TermFreq) -> f32 {
    let a = d.get(term).cloned().unwrap_or(0) as f32;
    let b = d.iter().map(|(_, f)| *f).sum::<usize>() as f32;
    a / b
}

pub fn serve_request(
    index_data: &model::IndexData,
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

            let mut score: Vec<(&Path, f32)> = Vec::new();
            for (path, tf_table) in &index_data.tfi {
                let mut doc_score: f32 = 0.0;
                for token in model::Lexer::new(&body) {
                    let tf = calculate_tf(&token, &tf_table);

                    let idf = *idf_map.get(&token).unwrap_or(&0.0);
                    doc_score += tf * idf;
                }

                score.push((&path, doc_score));
            }
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
