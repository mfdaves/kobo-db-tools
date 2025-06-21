use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Bookmark {
    content_id: String,
    content: String,
    book_id: String,
    book_title: String,
    color: u8,
    chapter_progress: f64,
    create_date: String,
    write_date: String,
}

// fn get_bookmarks(db: &Connection) -> Result<Vec<Bookmark>> {
//     let mut stmt = db.prepare(
//         "
//         SELECT bm.BookmarkID, bm.Text, bm.VolumeID, bm.Color, bm.ChapterProgress, bm.DateCreated, bm.DateModified, c.Title
//         FROM Bookmark bm
//         LEFT JOIN content c ON c.ContentID = bm.VolumeID
//         WHERE bm.Text IS NOT NULL AND bm.Text != ''
//         ",
//     )?;
//     let bookmarks: Result<Vec<Bookmark>> = stmt
//         .query_map([], |row| {
//             Ok(Bookmark {
//                 content_id: row.get("BookmarkID")?,
//                 content: row.get("Text")?,
//                 book_id: row.get("VolumeID")?,
//                 book_title: row.get("Title")?,
//                 color: row.get("Color")?,
//                 chapter_progress: row.get("ChapterProgress")?,
//                 create_date: row.get("DateCreated")?,
//                 write_date: row.get("DateModified")?,
//             })
//         })?
//         .collect();
//     bookmarks
// }

// fn export_bookmarks(bookmarks: &Vec<Bookmark>, format: Format, path: &str) -> Result<(), Box<dyn std::error::Error>> {
//     match format {
//         Format::Csv => {
//             let file = File::create(path)?;
//             let mut wtr = Writer::from_writer(file);
//             for bookmark in bookmarks {
//                 wtr.serialize(bookmark)?;
//             }
//             wtr.flush()?;
//         }
//         _ => eprintln!("This format: {:?} is not impl yet.", format),
//     }
//     Ok(())
// }
