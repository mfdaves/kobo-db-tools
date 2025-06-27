use crate::model::{ReadingSession, ReadingSessions,DictionaryWord};
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use std::collections::HashMap;
use std::str::FromStr;
use thiserror::Error;
// === Errori ===

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Event is not valid")]
    InvalidEventType,
    #[error("Error during session completation")]
    SessionCompletionFailed,
    #[error("Error during deserialize")]
    DeserializationError,
}

// === Dati degli attributi per ReadingSession ===

#[derive(serde::Deserialize, Clone)]
struct ReadingSessionAttributes {
    progress: String,
    volumeid: Option<String>,
    title: Option<String>,
    Monetization: String,
    author: Option<String>,
}

#[derive(serde::Deserialize)]
struct LeaveContentMetrics {
    #[serde(rename = "ButtonPressCount")]
    button_press_count: usize,
    #[serde(rename = "IdleTime")]
    idle_time: usize,
    #[serde(rename = "PagesTurned")]
    pages_turned: usize,
    #[serde(rename = "PercentageLandscapePageTurns")]
    prc_landscape_page_turns: u8,
    #[serde(rename = "SecondsRead")]
    seconds_read: usize,
}
#[derive(serde::Deserialize)]
struct LightAttributes{
    #[serde(alias = "NewNaturalLight")]
    #[serde(alias = "NewBrightness")]
    new_light:u8,
    #[serde(alias = "OldNaturalLight")]
    #[serde(alias = "OldBrightness")]
    old_light:u8
}

// === Dictionary ===

//in realtà sarebbe interessante per esempio
//sapere a quale sessione di lettura è associata la word e il term cosi da inserire poi un
//session_id attribute che permette la sua identificazione
#[derive(serde::Deserialize)]
struct DicitonaryAttributes {
    #[serde(rename = "Dictionary")]
    lang: String,
    #[serde(rename = "Word")]
    word: String,
}

// === Risultato aggregato ===
#[derive(Debug)]
pub struct EventAnalysis {
    pub sessions: ReadingSessions,
    pub terms: HashMap<Term, usize>,
}

// === Funzione principale ===

pub fn get_events(db: &Connection) -> rusqlite::Result<EventAnalysis> {
    let q = "SELECT Id, Type, Timestamp, Attributes, Metrics
             FROM AnalyticsEvents
             WHERE Type IN 
             (  'OpenContent', 'LeaveContent', 
                'DictionaryLookup', 
                'BrightnessAdjusted','NaturalLightAdjusted'
             )
             ORDER BY Timestamp ASC;";

    let mut stmt = db.prepare(q)?;
    let mut rows = stmt.query([])?;

    let mut analysis = EventAnalysis {
        sessions: ReadingSessions::new(),
        terms: HashMap::new(),
    };

    let mut current_session: Option<ReadingSession> = None;

    while let Some(row) = rows.next()? {
        let event_id: String = row.get("Id")?;
        let event_type: String = row.get("Type")?;
        let ts_str: String = row.get("Timestamp")?;
        let attr_json: String = row.get("Attributes")?;
        let ts = DateTime::<Utc>::from_str(&ts_str).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?;

        // Parsing attributi in base al tipo evento
        match event_type.as_str() {
            "OpenContent" | "LeaveContent" => {
                let attr: ReadingSessionAttributes =
                    serde_json::from_str(&attr_json).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            1,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;
                let progress = attr.progress.parse::<u8>().unwrap_or(0);

                let metrics = if event_type == "LeaveContent" {
                    let metr_json: String = row.get("Metrics")?;
                    Some(
                        serde_json::from_str::<LeaveContentMetrics>(&metr_json).map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                2,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })?,
                    )
                } else {
                    None
                };

                match handle_reading_session_event(
                    &event_type,
                    &event_id,
                    &mut current_session,
                    ts,
                    progress,
                    &attr,
                    metrics,
                ) {
                    Ok(Some(session)) => analysis.sessions.add_session(session),
                    Ok(None) => {}
                    Err(e) => eprintln!("Errore evento {}: {:?}", &event_id, e),
                }
            }
            "DictionaryLookup" => {
                *analysis
                    .terms
                    .entry(on_dictionary_lookup(attr_json)?)
                    .or_insert(0) += 1;
            }
            "BrightnessAdjusted" => {
                // TODO: handle_light_event(...)
            }
            "NaturalLightAdjusted" => {}
            _ => {
                eprintln!("Unknown event: {}", event_type);
            }
        }
    }
    Ok(analysis)
}

fn handle_reading_session_event(
    event_type: &str,
    event_id: &str,
    current_session: &mut Option<ReadingSession>,
    ts: DateTime<Utc>,
    progress: u8,
    attr: &ReadingSessionAttributes,
    metrics: Option<LeaveContentMetrics>,
) -> Result<Option<ReadingSession>, ParseError> {
    match event_type {
        "OpenContent" => {
            *current_session = Some(ReadingSession::new(
                ts,
                progress,
                attr.title.clone(),
                attr.volumeid.clone(),
                event_id.to_string(),
            ));
            Ok(None)
        }
        "LeaveContent" => {
            // println!("{:?}", current_session);
            if let Some(ref mut session) = current_session {
                let open_content_id = session.open_content_id.clone();
                let m = metrics.ok_or(ParseError::SessionCompletionFailed)?;
                session
                    .complete_session(
                        ts,
                        progress,
                        m.button_press_count as u64,
                        m.seconds_read as u64,
                        m.pages_turned as u64,
                        event_id.to_string(),
                    )
                    .map_err(|_| {
                        println!("START {:?} => END {:?}", open_content_id, event_id);
                        ParseError::SessionCompletionFailed
                    })?;

                let completed = std::mem::take(session);
                *current_session = None;
                Ok(Some(completed))
            } else {
                Err(ParseError::SessionCompletionFailed)
            }
        }
        _ => Err(ParseError::InvalidEventType),
    }
}

fn on_dictionary_lookup(attr_json: String) -> rusqlite::Result<Term> {
    let attr: DicitonaryAttributes = serde_json::from_str(&attr_json).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(e))
    })?;
    Ok(Term::new(attr.word, attr.lang))
}

