pub struct Note {
    pitch: String,
    duration: String,
}

pub fn note_builder(pitch: String, duration: String) -> Note {
    Note {
        pitch,
        duration,
    }
}

pub fn to_string(note: &Note) -> String {
    return String::from(note.pitch.clone() + " - " + &note.duration);
}