use rusqlite::{Connection,OpenFlags};
use std::collections::HashMap; 
use kobo_reader_db::model::*; 
use kobo_reader_db::parser::*; 



fn main()->Result<(),()>{
    let path = "/home/mfdaves/personal/kobo/db/KoboReader.sqlite"; 
    let conn = Connection::open_with_flags(&path,OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let ae = get_events(&conn);

    let ss = ae.unwrap().sessions;
    Ok(()) 
}