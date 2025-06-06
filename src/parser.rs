use rusqlite::{Connection, Result};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use chrono::{DateTime, Utc};

use crate::model::session::ReadingSession;
use std::error::Error;


pub fn get_sessions(db: &Connection) -> Result<(), Box<dyn Error>> {
    let q = "SELECT Id, Type, Timestamp, Attributes, Metrics
             FROM AnalyticsEvents
             WHERE Type IN ('OpenContent','LeaveContent')
             ORDER BY Timestamp ASC;";
    
    let mut stmt = db.prepare(q)?;
    let mut c_session; 
    let events = stmt.query_map([], |row| {
        let event_type: String = row.get("Type")?;
        let timestamp:String = row.get("Timestamp")?;
        let attributes : serde_json::Value = row.get("Attributes")?;
        match event_type {
            "OpenContent" => {
                let progress = attributes.get("progress").ok_or("Missing progress field.");
                let ts = DateTime::<Utc>::from_str(timestamp)?;
                c_session = ReadingSession::new(ts,progress)
            }
        }
    })?;

    let mut count = 0;
    for event in events {
        let (event_type, timestamp, attributes, metrics) = event?;
        count += 1;
        println!("[{}] Type: {}, Timestamp: {}", count, event_type, timestamp);
    }
    Ok(())
}

