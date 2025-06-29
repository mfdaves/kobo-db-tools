use crate::model::Bookmark;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use chrono::{DateTime, Utc};
use std::str::FromStr;

pub enum ExportFormat {
    Csv,
    Markdown,
}

pub fn export_bookmarks(bookmarks: &[Bookmark], format: ExportFormat, path: &str) -> Result<(), Box<dyn Error>> {
    match format {
        ExportFormat::Csv => {
            let file = File::create(path)?;
            let mut wtr = csv::Writer::from_writer(file);
            for bookmark in bookmarks {
                wtr.serialize(bookmark)?;
            }
            wtr.flush()?;
        }
        ExportFormat::Markdown => {
            let mut file = File::create(path)?;
            for bookmark in bookmarks {
                writeln!(file, "### {}", bookmark.book_title)?;
                writeln!(file, "\n> {}", bookmark.content)?;
                writeln!(file, "\n**Chapter Progress:** {:.2}%", bookmark.chapter_progress * 100.0)?;
                let formatted_date = DateTime::<Utc>::from_str(&bookmark.create_date)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                    .unwrap_or_else(|_| bookmark.create_date.clone());
                writeln!(file, "**Created:** {}", formatted_date)?;
                writeln!(file, "\n---\n")?;
            }
        }
    }
    Ok(())
}
