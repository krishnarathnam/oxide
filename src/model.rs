use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;
use std::{collections::HashMap, path::PathBuf};
use std::{fs, io};
use xml::reader::{EventReader, XmlEvent};

pub type TermFreq = HashMap<String, usize>;
pub type DocFreq = HashMap<String, usize>;
pub type TermFreqIndex = HashMap<PathBuf, TermFreq>;

#[derive(Deserialize, Serialize, Default, Debug)]
pub struct IndexData {
    pub tfi: TermFreqIndex,
    pub df: DocFreq,
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

        return Some(self.chop(1).iter().collect());
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_token()
    }
}

pub fn check_index(index_path: &str) -> io::Result<()> {
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

pub fn build_idf_index(index_data: &IndexData) -> HashMap<String, f32> {
    let mut idf_map: HashMap<String, f32> = HashMap::new();

    for (term, count) in &index_data.df {
        let idf = (index_data.tfi.len() as f32 / *count as f32).log10();
        idf_map.insert(term.to_string(), idf);
    }

    idf_map
}

pub fn save_index_to_json(index_data: &mut IndexData, index_path: &str) -> io::Result<()> {
    build_df_index(index_data);
    let index_file = File::create(index_path)?;
    serde_json::to_writer(BufWriter::new(index_file), index_data)?;
    Ok(())
}

pub fn save_folder_to_model(dir_path: &str, index_data: &mut IndexData) -> io::Result<()> {
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
