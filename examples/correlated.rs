use kobo_db_tools::Parser;
use rusqlite::{Connection, OpenFlags};
use std::env;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let db_path = env::args().nth(1).or_else(|| env::var("KOBO_DB_PATH").ok());
    let db_path = db_path.ok_or_else(|| {
        "Usage: cargo run --example correlated -- /path/to/KoboReader.sqlite (or set KOBO_DB_PATH)"
            .to_string()
    })?;

    let uri = format!("file:{}?immutable=1", db_path);
    let conn = Connection::open_with_flags(
        uri,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
    )?;
    let analysis = Parser::parse_correlated(&conn)?;

    println!("correlated sessions: {}", analysis.sessions.len());
    if let Some(session) = analysis.sessions.first() {
        println!(
            "sample session: {} -> {:?}",
            session.session.time_start.to_rfc3339(),
            session.session.time_end.map(|end| end.to_rfc3339())
        );
        println!("  dictionary events: {}", session.dictionary.len());
        println!(
            "  brightness events: {}",
            session.brightness.len() + session.natural_light.len()
        );
        println!("  app events: {}", session.app_events.len());
    }

    println!(
        "orphans: dict {} brightness {} natural {} app {}",
        analysis.orphans.dictionary.len(),
        analysis.orphans.brightness.len(),
        analysis.orphans.natural_light.len(),
        analysis.orphans.app_events.len()
    );

    println!("charge cycles: {}", analysis.cycles.len());
    for (index, cycle) in analysis.cycles.iter().take(3).enumerate() {
        let end = cycle
            .end
            .map(|ts| ts.to_rfc3339())
            .unwrap_or_else(|| "open".to_string());
        println!(
            "cycle {}: {} -> {} | sessions {} | app starts {}",
            index + 1,
            cycle.start.to_rfc3339(),
            end,
            cycle.sessions.len(),
            cycle.metrics.app_starts
        );
    }

    if !analysis.app_start_counts_by_day.is_empty() {
        let mut days: Vec<_> = analysis.app_start_counts_by_day.iter().collect();
        days.sort_by_key(|(day, _)| **day);
        println!("app starts by day (first 5):");
        for (day, count) in days.into_iter().take(5) {
            println!("  {}: {}", day, count);
        }
    }

    Ok(())
}
