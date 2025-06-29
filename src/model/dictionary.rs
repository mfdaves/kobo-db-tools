use uuid::Uuid;


#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct DictionaryWord {
    term: String,
    lang: String,
    session_id: Option<Uuid>
}

impl DictionaryWord {
    pub fn new(term: String, lang: String, session_id:Option<Uuid>) -> Self {
        Self { term, lang, session_id }
    }
}
