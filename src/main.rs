use std::{
    collections::HashMap,
    fs::{self, File},
    hash::Hash,
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
    let file_path = "./docs.gl/gl4";
    let dir = fs::read_dir(file_path)?;
    let top_n = 20;

    for file in dir {
        let file = file?.path();
        let document = read_entire_xml_file(&file)?.chars().collect::<Vec<_>>();

        let mut tf = HashMap::<String, usize>::new();

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

        let mut stats: Vec<_> = tf.iter().collect();
        stats.sort_by_key(|(_, f)| *f);
        stats.reverse();

        println!("{file:?}");
        for (t, r) in stats.iter().take(top_n) {
            println!("{t} => {r}");
        }
    }
    Ok(())
}
