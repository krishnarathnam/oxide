use std::{
    collections::HashMap,
    fs::{self, File},
    io,
    path::{Path, PathBuf},
    process::exit,
};
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
    fn next_token(&mut self) -> Option<&'a [char]> {
        self.trim_left_whitespace();
        if self.content.len() == 0 {
            return None;
        }

        if self.content[0].is_alphabetic() {
            let mut n = 0;
            while n < self.content.len() && self.content[n].is_alphanumeric() {
                n += 1;
            }

            let result = &self.content[0..n];
            self.content = &self.content[n..];
            return Some(result);
        }

        todo!()
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = &'a [char];

    fn next(&mut self) -> Option<Self::Item> {
        self.next_token()
    }
}

fn index_document(doc_content: &str) -> HashMap<String, usize> {
    todo!("Not Implemented")
}

fn read_entire_xml_file<P: AsRef<Path>>(file_path: P) -> io::Result<String> {
    let file = File::open(file_path).unwrap_or_else(|err| {
        eprintln!("ERROR: cound not read file : {err}");
        exit(1);
    });

    let er = EventReader::new(file);
    let mut content = String::new();

    for event in er {
        let event = event.unwrap_or_else(|err| {
            eprintln!("ERROR: cannot read next xml file: {err}");
            exit(1)
        });

        if let XmlEvent::Characters(text) = event {
            content.push_str(&text);
            content.push_str(" ");
        }
    }

    Ok(content)
}

fn main() -> io::Result<()> {
    let file_path = "./docs.gl/gl3/glActiveTexture.xhtml";
    let document = read_entire_xml_file(file_path)?.chars().collect::<Vec<_>>();

    for lexer in Lexer::new(&document) {
        println!("{lexer}", lexer = lexer.iter().collect::<String>());
    }
    //let all_document = HashMap::<PathBuf, HashMap<String, usize>>::new();
    //let file_path = "./docs.gl/gl4";
    //let dir = fs::read_dir(file_path)?;

    //for file in dir {
    //    let path = file?.path();
    //    let content = read_entire_xml_file(&path);
    //    println!(
    //        "length of content in {path:?}: {length}",
    //        length = content?.len()
    //    );
    //}
    Ok(())
}
