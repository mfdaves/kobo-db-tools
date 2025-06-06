// use rusqlite::{Connection, Result};
// use csv::Writer;
// use std::fs::File;
// use std::collections::HashSet;
// use serde::{Serialize, Deserialize};
// use std::hash::{Hash, Hasher};

// #[derive(Debug)]
// enum Format {
//     Json,
//     Csv,
//     Markdown,
// }


// struct GenericEvent{
//     id:String,
//     timestamp:DateTime<Utc>,
//     type:EventType,
//     metrics:serde_json::Value,
//     attributes:Option<serde_json::Value>
// }



// fn main() -> Result<(), Box<dyn std::error::Error>> {
//     let path = "/home/mfdaves/personal/kobo/db/KoboReader.sqlite";
//     let db = Connection::open(path)?;

//     Ok(())
// }


// fn get_dictionary_lookup<'a>(
//     db: &Connection,
//     ae: &'a mut AnalyticsEvents,
// ) -> Result<&'a HashSet<Word>, Box<dyn std::error::Error>> {

//     let placeholders = INTERESTING_EVENTS.iter().map(|_| "?").collect()::<Vec<_>>().join(",");

//     let q = format!(
//          "
//         SELECT Id,Type,Timestamp,Attributes,Metrics
//         FROM AnalyticsEvents
//         WHERE Type IN ({})
//         ORDER BY Timestamp DESC;
//         ",
//         placeholders
//         );
//     let mut stmt = db.prepare(&q)?;
//     let events = stmt.query_map(INTERESTING_EVENTS, |row| {
//         let type = row.get("Type")?;
//         println!("{:?}",type);
//         match type {
//             "DictionaryLookup" => {
                
//             }
//             _=>()
//         }
//         let attr_str: String = row.get(0)?;
//         let entry: DictionaryEntry = serde_json::from_str(&attr_str).unwrap();
//         Ok(entry)
//     })?;

//     for attribute in attributes {
//         let entry = attribute?;
//         let word = Word::new(entry);
//         ae.dictionary_lookup.insert(word);
//     }

//     Ok(&ae.dictionary_lookup)
// }





//Dictionary Lookup --> vocaboli cercati per lingua
//Open Content/Leave Content --> Analisi sessione e attività, analisi temporale, heatmap e comportamentale
// --> vedere per esempio quando ci sono sessioni che durano pochi secondi
//Brightness --> luminosità media
//Settings --> Active setting e cambiamenti effettuati
//Review -- forse 




//---> Quindi io faccio il parsing degli eventi importanti
//--> parso tutta la tabella e poi dopo posso chiedere di vedere i dati, però devo anche lasciare la possibilità di avere le singole informazioni, usando gli eventType enum


// fn main() -> Result



use rusqlite::Connection;
use kobo_stats::model::{ReadingSession, EventType};
use kobo_stats::parser::get_sessions;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Percorso al database Kobo
    let path = "/home/mfdaves/personal/kobo/db/KoboReader.sqlite";
    let db = Connection::open(path)?;

    // Passiamo solo il tipo evento ReadingSession al parser
    let events = vec![EventType::ReadingSession];

    // Chiamata alla funzione parse_db e log delle sessioni
    let sessions = get_sessions(&db)?;

    Ok(())
}

