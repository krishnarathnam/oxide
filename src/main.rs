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

            model::save_folder_to_model(dir_path.as_str(), &mut index_data).map_err(|e| {
                eprintln!("ERROR: cannot read directory `{dir_path}`: {e}");
                ()
            })?;
            model::save_index_to_json(&mut index_data, "index.json").map_err(|e| {
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

            let index_file = File::open(index_path).map_err(|e| {
                eprintln!("ERROR: cannot open index file: {e}");
                ()
            })?;

            let index_data: model::IndexData =
                serde_json::from_reader(index_file).map_err(|e| {
                    eprintln!("ERROR: cannot parse index file: {e}");
                    ()
                })?;

            let address = args.next().unwrap_or("127.0.0.1:6969".to_string());
            let server = Server::http(&address).map_err(|e| {
                eprintln!("ERROR: Could not start server at {address}: {e}");
                ()
            })?;
            let idf_map = model::build_idf_index(&index_data);
            println!("Running server at: http://{address}");
            for request in server.incoming_requests() {
                let _ = server::serve_request(&index_data, request, &idf_map);
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
