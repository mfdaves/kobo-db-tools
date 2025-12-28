use uuid::Uuid;

#[derive(Debug, Clone, Eq, Hash, PartialEq, serde::Serialize)]
pub struct DictionaryWord {
    term: String,
    lang: String,
    session_id: Option<Uuid>,
}

impl DictionaryWord {
    pub fn new(term: String, lang: String, session_id: Option<Uuid>) -> Self {
        Self {
            term,
            lang,
            session_id,
        }
    }

    pub fn term(&self) -> &str {
        &self.term
    }

    pub fn lang(&self) -> &str {
        &self.lang
    }

    pub fn session_id(&self) -> Option<Uuid> {
        self.session_id
    }
}
