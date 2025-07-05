use super::{error::ExportError, Export};
use crate::model::Bookmark;
use chrono::{DateTime, Utc};
use csv::Writer;
use std::io::Write;
use std::str::FromStr;
use serde_json;

impl Export for [Bookmark] {
    fn to_csv(&self) -> Result<String, ExportError> {
        let mut wtr = Writer::from_writer(vec![]);
        for bookmark in self {
            wtr.serialize(bookmark)?;
        }
        Ok(String::from_utf8(wtr.into_inner()?)?)
    }

    fn to_md(&self) -> Result<String, ExportError> {
        let mut buffer = Vec::new();
        for bookmark in self {
            writeln!(buffer, "### {}", bookmark.book_title)?;
            writeln!(buffer, "\n> {}", bookmark.content)?;
            writeln!(
                buffer,
                "\n**Chapter Progress:** {:.2}%",
                bookmark.chapter_progress * 100.0
            )?;
            let formatted_date = DateTime::<Utc>::from_str(&bookmark.create_date)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                .unwrap_or_else(|_| bookmark.create_date.clone());
            writeln!(buffer, "**Created:** {}", formatted_date)?;
            writeln!(buffer, "\n---\n")?;
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
    use crate::model::Bookmark;

    #[test]
    fn test_to_csv() {
        let bookmarks = vec![
            Bookmark {
                content_id: "content1".to_string(),
                content: "content text".to_string(),
                book_id: "book1".to_string(),
                book_title: "Book Title".to_string(),
                color: 1,
                chapter_progress: 0.5,
                create_date: "2025-07-05T10:00:00Z".to_string(),
                write_date: "2025-07-05T10:00:00Z".to_string(),
            },
        ];

        let expected_csv = "content_id,content,book_id,book_title,color,chapter_progress,create_date,write_date\ncontent1,content text,book1,Book Title,1,0.5,2025-07-05T10:00:00Z,2025-07-05T10:00:00Z\n".to_string();
        let result = bookmarks.to_csv().unwrap();
        assert_eq!(result, expected_csv);
    }

    #[test]
    fn test_to_json() {
        let bookmarks = vec![
            Bookmark {
                content_id: "content1".to_string(),
                content: "content text".to_string(),
                book_id: "book1".to_string(),
                book_title: "Book Title".to_string(),
                color: 1,
                chapter_progress: 0.5,
                create_date: "2025-07-05T10:00:00Z".to_string(),
                write_date: "2025-07-05T10:00:00Z".to_string(),
            },
        ];

        let expected_json = serde_json::to_string(&bookmarks).unwrap();
        let result = bookmarks.to_json().unwrap();
        assert_eq!(result, expected_json);
    }
}