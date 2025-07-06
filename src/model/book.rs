use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Book {
    pub authors: String,
    pub title: String,
    pub size: Option<u64>,
    pub book_id: String,
}

impl Book {
    pub fn new(authors: String, title: String, size: Option<u64>, book_id: String) -> Self {
        Self {
            authors,
            title,
            size,
            book_id,
        }
    }

    pub fn authors(&self) -> &str {
        &self.authors
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn size(&self) -> Option<u64> {
        self.size
    }
}
