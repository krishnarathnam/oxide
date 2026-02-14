use poppler;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read};
use std::path::Path;
use std::{collections::HashMap, path::PathBuf};
use std::{fs, io};
use xml::reader::{EventReader, XmlEvent};

pub type Posting = HashMap<PathBuf, usize>;
pub type InvertedIndex = HashMap<String, Posting>;
pub type DocLen = HashMap<PathBuf, usize>;

#[derive(Deserialize, Serialize, Default, Debug)]
pub struct InvertedIndexData {
    pub index_freq: InvertedIndex,
    pub doc_len: DocLen,
}

pub struct Lexer<'a> {
    content: &'a [char],
}

impl<'a> Lexer<'a> {
    pub fn new(content: &'a [char]) -> Self {
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

        self.chop(1);
        return self.next_token();
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_token()
    }
}

pub fn build_idf_inverted_index(inverted_index_data: &InvertedIndexData) -> HashMap<String, f32> {
    let mut idf_map: HashMap<String, f32> = HashMap::new();
    let total_doc = inverted_index_data.doc_len.len() as f32;

    for (term, posting) in &inverted_index_data.index_freq {
        let df = posting.len() as f32;
        let score = (total_doc / df).log10();

        idf_map.insert(term.clone(), score);
    }

    idf_map
}

pub fn save_inverted_index_to_json(
    inverted_index_data: &mut InvertedIndexData,
    index_path: &str,
) -> io::Result<()> {
    let index_file = File::create(index_path)?;
    serde_json::to_writer(BufWriter::new(index_file), inverted_index_data)?;
    Ok(())
}

pub fn add_to_model(
    inverted_index_data: &mut InvertedIndexData,
    path: &PathBuf,
    document: &Vec<char>,
) -> usize {
    let mut count = 0;
    for token in Lexer::new(&document) {
        if let Some(_) = inverted_index_data.index_freq.get_mut(&token) {
            let posting = inverted_index_data
                .index_freq
                .entry(token)
                .or_insert_with(HashMap::new);

            *posting.entry(path.clone()).or_insert(0) += 1;
        } else {
            let mut temp = Posting::new();
            temp.insert(path.clone(), 1);
            inverted_index_data
                .index_freq
                .insert(token.to_string(), temp);
        }

        count += 1;
    }

    count
}

pub fn save_folder_to_index_data_model(
    dir_path: &str,
    inverted_index_data: &mut InvertedIndexData,
) -> io::Result<()> {
    let dir = fs::read_dir(dir_path)?;

    for entry in dir {
        let path = entry?.path();

        if path.is_dir() {
            save_folder_to_index_data_model(path.to_str().unwrap(), inverted_index_data)?;
        } else {
            let extenstion = match path.extension() {
                Some(ext) => ext.to_str().unwrap(),
                None => continue,
            };

            let document: Vec<char>;
            if extenstion == "xhtml" || extenstion == "xml" {
                document = match read_entire_xml_file(&path) {
                    Ok(text) => {
                        println!("converted the file: {}", path.display());
                        text.chars().collect::<Vec<_>>()
                    }
                    Err(_) => {
                        continue;
                    }
                };

                let count = add_to_model(inverted_index_data, &path, &document);
                inverted_index_data.doc_len.insert(path.clone(), count);
            } else if extenstion == "pdf" {
                document = match read_entire_pdf_file(&path) {
                    Ok(text) => {
                        println!("converted the file: {}", path.display());
                        text.chars().collect::<Vec<_>>()
                    }
                    Err(_) => {
                        continue;
                    }
                };

                let count = add_to_model(inverted_index_data, &path, &document);
                inverted_index_data.doc_len.insert(path.clone(), count);
            } else if extenstion == "md" || extenstion == "txt" {
                document = match fs::read_to_string(&path) {
                    Ok(text) => {
                        println!("converted the file: {}", path.display());
                        text.chars().collect::<Vec<_>>()
                    }
                    Err(_) => {
                        continue;
                    }
                };

                let count = add_to_model(inverted_index_data, &path, &document);
                inverted_index_data.doc_len.insert(path.clone(), count);
            }
        }
    }

    Ok(())
}

fn read_entire_pdf_file(file_path: &Path) -> io::Result<String> {
    // Fixed: use io::Result
    let mut content = Vec::new();

    // File I/O - now ? works automatically
    File::open(file_path)?.read_to_end(&mut content)?;

    // Parse PDF document
    let pdf = poppler::PopplerDocument::new_from_data(&mut content, None).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Poppler error {}: {}", file_path.display(), e),
        )
    })?; // Convert poppler error to io::Error

    let mut result = String::new();

    // Extract text from all pages
    let n = pdf.get_n_pages();
    for i in 0..n {
        if let Some(page) = pdf.get_page(i) {
            if let Some(text) = page.get_text() {
                result.push_str(&text);
                result.push(' ');
            }
        }
    }

    Ok(result)
}

fn read_entire_xml_file<P: AsRef<Path>>(file_path: P) -> io::Result<String> {
    let file = File::open(file_path)?;
    let er = EventReader::new(BufReader::new(file));
    let mut content = String::new();

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
