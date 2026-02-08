use std::{
    fs::{self, File},
    io,
    path::Path,
    process::exit,
};
use xml::reader::{EventReader, XmlEvent};

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
        }
    }

    Ok(content)
}

fn main() -> io::Result<()> {
    let file_path = "./docs.gl/gl4";
    let dir = fs::read_dir(file_path)?;

    for file in dir {
        let path = file?.path();
        let content = read_entire_xml_file(&path);
        println!(
            "length of content in {path:?}: {length}",
            length = content?.len()
        );
    }
    // println!(
    //     "{content}",
    //     content = read_entire_xml_file(file_path).expect("ERROR: cannot read dir")
    //
    // );

    Ok(())
}
