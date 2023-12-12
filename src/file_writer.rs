use std::fs::File;
use std::io;
use simple_xml_builder::XMLElement;

pub mod note;

pub struct MusicXML {
    title: String,
    src: io::Result<File>,
}

impl MusicXML {
    pub fn add_note(&mut self) {
        unimplemented!();
    }
}

pub fn musicxml_builder(title: String) -> MusicXML {
    let file_name = title.clone() + ".musicxml";
    MusicXML {
        title: title,
        src: File::create(file_name),
    }
}