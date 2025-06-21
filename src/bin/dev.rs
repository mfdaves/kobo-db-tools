use rusqlite::{Connection,OpenFlags};
use std::collections::HashMap; 


mod parser;
mod model;

use parser::*;
use model::*;


fn main()->Result<(),()>{
    let path = "/home/mfdaves/personal/kobo/db/KoboReader.sqlite"; 
    let conn = Connection::open_with_flags(&path,OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let ae = get_events(&conn);

    let ss = ae.unwrap().sessions;


    let interesting_metrics = vec![
        ReadingMetric::SecondsRead
    ];

    let percs: &[f64] = &[0.0,0.25,0.5,0.75,1.0];

    if ss.sessions_count() == 0 {
        return Err(());
    } else{
        let percentiles: Vec<Vec<f64>> = interesting_metrics
            .iter()
            .map(|m| ss.calculate_percentile(*m,percs))
            .collect();

        for p in percentiles.iter(){
            for px in p.iter(){
                println!("{:?}", px/60.0);
            }
        } 
    }





    Ok(()) 
}