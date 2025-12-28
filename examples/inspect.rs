use kobo_db_tools::{AppEventKind, ParseOption, Parser, Statistics};
use rusqlite::{Connection, OpenFlags};
use std::env;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let db_path = env::args().nth(1).or_else(|| env::var("KOBO_DB_PATH").ok());
    let db_path = db_path.ok_or_else(|| {
        "Usage: cargo run --example inspect -- /path/to/KoboReader.sqlite (or set KOBO_DB_PATH)"
            .to_string()
    })?;

    let uri = format!("file:{}?immutable=1", db_path);
    let conn = Connection::open_with_flags(
        uri,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
    )?;
    let analysis = Parser::parse_events(&conn, ParseOption::All)?;

    if let Some(sessions) = &analysis.sessions {
        println!("sessions: {}", sessions.sessions_count());
        println!("sessions avg seconds: {:.2}", sessions.avg());
    } else {
        println!("sessions: none");
    }

    if let Some(terms) = &analysis.terms {
        println!("dictionary terms: {}", terms.len());
    } else {
        println!("dictionary terms: none");
    }

    if let Some(history) = &analysis.brightness_history {
        println!("brightness events: {}", history.events.len());
        println!("brightness avg: {:.2}", history.avg());
    } else {
        println!("brightness events: none");
    }

    if let Some(history) = &analysis.natural_light_history {
        println!("natural light events: {}", history.events.len());
        println!("natural light avg: {:.2}", history.avg());
    } else {
        println!("natural light events: none");
    }

    if let Some(bookmarks) = &analysis.bookmarks {
        println!("bookmarks: {}", bookmarks.len());
    } else {
        println!("bookmarks: none");
    }

    if let Some(books) = &analysis.books {
        println!("books: {}", books.len());
    } else {
        println!("books: none");
    }

    if let Some(app_events) = &analysis.app_events {
        let app_starts = app_events
            .iter()
            .filter(|event| event.kind == AppEventKind::AppStart)
            .count();
        let plugged_in = app_events
            .iter()
            .filter(|event| event.kind == AppEventKind::PluggedIn)
            .count();
        println!("app starts: {}", app_starts);
        println!("plugged in: {}", plugged_in);
    } else {
        println!("app events: none");
    }

    Ok(())
}
