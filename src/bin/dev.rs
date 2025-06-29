use kobo_reader_db::export::{export_bookmarks, ExportFormat};
use kobo_reader_db::{statistics::Statistics, Parser};
use rusqlite::{Connection, OpenFlags};

fn main() -> Result<(), ()> {
    let path = "/home/mfdaves/personal/kobo/db/KoboReader.sqlite";
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let ae = Parser::parse_events(&conn).unwrap();

    println!("Average Brightness: {}", ae.brightness_history.avg());
    println!("Average Natural Light: {}", ae.natural_light_history.avg());

    // Export bookmarks to Markdown
    export_bookmarks(&ae.bookmarks, ExportFormat::Markdown, "./bookmarks.md")
        .expect("Failed to export bookmarks to Markdown");
    println!("Bookmarks exported to bookmarks.md");

    Ok(())
}
