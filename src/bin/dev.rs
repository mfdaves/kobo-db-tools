use rusqlite::{Connection,OpenFlags};
use kobo_reader_db::get_events;
use kobo_reader_db::statistics::Statistics;

fn main()->Result<(),()>{ 
    let path = "/home/mfdaves/personal/kobo/db/KoboReader.sqlite"; 
    let conn = Connection::open_with_flags(&path,OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let ae = get_events(&conn).unwrap();

    println!("Average Brightness: {}",ae.brightness_history.avg());
    println!("Average Natural Light: {}",ae.natural_light_history.avg());

    Ok(()) 
}