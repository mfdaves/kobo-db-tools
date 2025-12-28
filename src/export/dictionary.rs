use crate::export::{Export, ExportError};
use crate::model::DictionaryWord;

impl Export for [DictionaryWord] {
    fn to_csv(&self) -> Result<String, ExportError> {
        let mut wtr = csv::Writer::from_writer(vec![]);
        for word in self {
            wtr.serialize(word)?;
        }
        Ok(String::from_utf8(wtr.into_inner()?)?)
    }

    fn to_md(&self) -> Result<String, ExportError> {
        let mut buffer = Vec::new();
        use std::io::Write;

        writeln!(buffer, "| Term | Language | Session ID |")?;
        writeln!(buffer, "|------|----------|------------|")?;

        for word in self {
            let session_id_str = word
                .session_id()
                .map(|id| id.to_string())
                .unwrap_or_else(|| "N/A".to_string());

            writeln!(
                buffer,
                "| {} | {} | {} |",
                word.term(),
                word.lang(),
                session_id_str
            )?;
        }
        Ok(String::from_utf8(buffer)?)
    }

    fn to_json(&self) -> Result<String, ExportError> {
        serde_json::to_string(self).map_err(ExportError::JsonToString)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::DictionaryWord;
    use uuid::Uuid;

    fn get_test_words() -> Vec<DictionaryWord> {
        vec![
            DictionaryWord::new("hello".to_string(), "en".to_string(), Some(Uuid::nil())),
            DictionaryWord::new("world".to_string(), "en".to_string(), None),
        ]
    }

    #[test]
    fn test_dict_to_csv() {
        let words = get_test_words();
        let expected = [
            "term,lang,session_id",
            "hello,en,00000000-0000-0000-0000-000000000000",
            "world,en,",
            "",
        ]
        .join("\n");
        assert_eq!(words.to_csv().unwrap(), expected);
    }

    #[test]
    fn test_dict_to_md() {
        let words = get_test_words();
        let expected = [
            "| Term | Language | Session ID |",
            "|------|----------|------------|",
            "| hello | en | 00000000-0000-0000-0000-000000000000 |",
            "| world | en | N/A |",
            "",
        ]
        .join("\n");
        assert_eq!(words.to_md().unwrap(), expected);
    }

    #[test]
    fn test_dict_to_json() {
        let words = get_test_words();
        let expected = serde_json::to_string(&words).unwrap();
        assert_eq!(words.to_json().unwrap(), expected);
    }
}
