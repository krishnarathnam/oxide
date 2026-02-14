mod model;
mod server;

use std::{fs::File, process::ExitCode};
use tiny_http::Server;

fn usage(program: &str) {
    eprintln!("Usage: {program} [SUBCOMMAND] [OPTIONS]");
    eprintln!("Subcommands:");
    eprintln!("    index <folder>                 Turn the folder and file into index.json");
    eprintln!("    search <folder>                search how many files are in index.json");
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

            let mut index_data: model::IndexData = Default::default();
            let mut inverted_index_data: model::InvertedIndexData = Default::default();

            model::save_folder_to_model(dir_path.as_str(), &mut index_data).map_err(|e| {
                eprintln!("ERROR: cannot read directory `{dir_path}`: {e}");
                ()
            })?;
            model::save_index_to_json(&mut index_data, "index.json").map_err(|e| {
                eprintln!("ERROR: cannot read directory `{dir_path}`: {e}");
                ()
            })?;
            model::save_folder_to_index_data_model(dir_path.as_str(), &mut inverted_index_data)
                .map_err(|e| {
                    eprintln!("ERROR: cannot read directory `{dir_path}`: {e}");
                    ()
                })?;
            model::save_inverted_index_to_json(&mut inverted_index_data, "inverted_index.json")
                .map_err(|e| {
                    eprintln!("ERROR: cannot read directory `{dir_path}`: {e}");
                    ()
                })?;
        }

        "search" => {
            let index_path = args.next().ok_or_else(|| {
                usage(&program);
                eprintln!("ERROR: no directory is provided for {subcommand} subcommand");
            })?;

            model::check_index(&index_path).map_err(|e| {
                eprintln!("ERROR: {e}");
                ()
            })?;
        }

        "serve" => {
            let index_path = args.next().ok_or_else(|| {
                usage(&program);
                eprintln!("ERROR: no directory is provided for {subcommand} subcommand");
            })?;

            let inverted_index_file = File::open(index_path).map_err(|e| {
                eprintln!("ERROR: cannot open index file: {e}");
                ()
            })?;

            let inverted_index_data: model::InvertedIndexData =
                serde_json::from_reader(inverted_index_file).map_err(|e| {
                    eprintln!("ERROR: cannot parse index file: {e}");
                    ()
                })?;

            let address = args.next().unwrap_or("127.0.0.1:6969".to_string());
            let server = Server::http(&address).map_err(|e| {
                eprintln!("ERROR: Could not start server at {address}: {e}");
                ()
            })?;
            let idf_map = model::build_idf_inverted_index(&inverted_index_data);

            println!("Running server at: http://{address}");
            for request in server.incoming_requests() {
                let _ = server::serve_request(&inverted_index_data, request, &idf_map);
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
