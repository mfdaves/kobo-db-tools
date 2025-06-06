pub struct DictionaryWord{
	term:String,
	lang:String
}

impl DictionaryWord {
    pub fn new(term:String,lang:String) -> Self {
        Self {
        	term,
        	lang
        }
    }
}