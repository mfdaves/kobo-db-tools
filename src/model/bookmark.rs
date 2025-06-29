use rusqlite::{Connection, Result};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Bookmark {
    pub content_id: String,
    pub content: String,
    pub book_id: String,
    pub book_title: String,
    pub color: u8,
    pub chapter_progress: f64,
    pub create_date: String,
    pub write_date: String,
}

pub fn get_bookmarks(db: &Connection) -> Result<Vec<Bookmark>> {
    let mut stmt = db.prepare(
        "
        SELECT bm.BookmarkID, bm.Text, bm.VolumeID, bm.Color, bm.ChapterProgress, bm.DateCreated, bm.DateModified, c.Title
        FROM Bookmark bm
        LEFT JOIN content c ON c.ContentID = bm.VolumeID
        WHERE bm.Text IS NOT NULL AND bm.Text != ''
        ",
    )?;
    let bookmarks: Result<Vec<Bookmark>> = stmt
        .query_map([], |row| {
            Ok(Bookmark {
                content_id: row.get("BookmarkID")?,
                content: row.get("Text")?,
                book_id: row.get("VolumeID")?,
                book_title: row.get("Title")?,
                color: row.get("Color")?,
                chapter_progress: row.get("ChapterProgress")?,
                create_date: row.get("DateCreated")?,
                write_date: row.get("DateModified")?,
            })
        })?
        .collect();
    bookmarks
}
