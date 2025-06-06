pub enum EventType {
    DictionaryLookup,
    ReadingSession,
    BrightnessAdjusted,
    Unknown(String),
}

impl EventType {
    pub fn db_types(&self) -> Vec<&'static str> {
        match self {
            EventType::DictionaryLookup => vec!["dictionarylookup"],
            EventType::ReadingSession => vec!["opencontent", "leavecontent"],
            EventType::BrightnessAdjusted => vec!["brightnessadjusted"],
            EventType::Unknown(_) => vec!["unknown"],  // valore statico di fallback
        }
    }
}
