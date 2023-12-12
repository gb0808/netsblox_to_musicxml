pub struct Note {
    pitch: String,
    duration: String,
}

impl Note {
    pub fn to_string(&self) -> String {
        return String::from(self.pitch.clone() + " - " + &self.duration);
    }
}

pub fn note_builder(pitch: String, duration: String) -> Note {
    Note {
        pitch,
        duration,
    }
}