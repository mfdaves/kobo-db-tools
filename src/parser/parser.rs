use crate::{
    get_bookmarks, Book, Bookmark, Brightness, BrightnessEvent, BrightnessHistory, DictionaryWord,
    NaturalLightHistory, ReadingSession, ReadingSessions,
};
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::str::FromStr;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Event is not valid")]
    InvalidEventType,
    #[error("Error during session completation")]
    SessionCompletionFailed,
    #[error("Error during deserialize")]
    DeserializationError,
}

#[derive(serde::Deserialize, Clone)]
struct ReadingSessionAttributes {
    progress: String,
    volumeid: Option<String>,
    title: Option<String>,
    #[serde(rename = "attribution")]
    author: Option<String>,
}

#[derive(serde::Deserialize)]
struct LeaveContentMetrics {
    #[serde(rename = "ButtonPressCount")]
    button_press_count: usize,
    #[serde(rename = "SecondsRead")]
    seconds_read: usize,
    #[serde(rename = "PagesTurned")]
    pages_turned: usize,
}

#[derive(serde::Deserialize)]
struct LightAttributes {
    #[serde(rename = "Method")]
    method: String,
}

#[derive(serde::Deserialize)]
struct LightMetrics {
    #[serde(alias = "NewNaturalLight")]
    #[serde(alias = "NewBrightness")]
    new_light: u8,
}

#[derive(serde::Deserialize)]
struct DicitonaryAttributes {
    #[serde(rename = "Dictionary")]
    lang: String,
    #[serde(rename = "Word")]
    word: String,
}

#[derive(Debug, PartialEq)]
pub enum ParseOption {
    All,
    ReadingSessions,
    DictionaryLookups,
    BrightnessHistory,
    NaturalLightHistory,
    Bookmarks,
    AppStart,
    PluggedIn,
}

#[derive(Debug, Default)]
pub struct EventAnalysis {
    pub sessions: Option<ReadingSessions>,
    pub terms: Option<HashMap<DictionaryWord, usize>>,
    pub brightness_history: Option<BrightnessHistory>,
    pub natural_light_history: Option<NaturalLightHistory>,
    pub bookmarks: Option<Vec<Bookmark>>,
    pub books: Option<Vec<Book>>,
}

pub struct Parser;

impl Parser {
    pub fn parse_events(db: &Connection, option: ParseOption) -> rusqlite::Result<EventAnalysis> {
        let mut analysis = EventAnalysis::default();

        let mut event_types_to_query = Vec::new();
        let mut get_bookmarks_flag = false;

        match option {
            ParseOption::All => {
                event_types_to_query.extend_from_slice(&[
                    "'OpenContent'",
                    "'LeaveContent'",
                    "'DictionaryLookup'",
                    "'BrightnessAdjusted'",
                    "'NaturalLightAdjusted'",
                ]);
                get_bookmarks_flag = true;
            }
            ParseOption::ReadingSessions => {
                event_types_to_query.extend_from_slice(&["'OpenContent'", "'LeaveContent'"]);
            }
            ParseOption::DictionaryLookups => {
                event_types_to_query.push("'DictionaryLookup'");
            }
            ParseOption::BrightnessHistory => {
                event_types_to_query.push("'BrightnessAdjusted'");
            }
            ParseOption::NaturalLightHistory => {
                event_types_to_query.push("'NaturalLightAdjusted'");
            }
            ParseOption::Bookmarks => {
                get_bookmarks_flag = true;
            }
            ParseOption::AppStart | ParseOption::PluggedIn => {
                // Do nothing for now
            }
        }

        if get_bookmarks_flag {
            analysis.bookmarks = Some(get_bookmarks(db)?);
        }

        if !event_types_to_query.is_empty() {
            let q = format!(
                "SELECT Id, Type, Timestamp, Attributes, Metrics FROM AnalyticsEvents WHERE Type IN ({}) ORDER BY Timestamp ASC;",
                event_types_to_query.join(", ")
            );

            let mut stmt = db.prepare(&q)?;
            let mut rows = stmt.query([])?;

            let mut current_session: Option<ReadingSession> = None;
            let mut sessions_vec = ReadingSessions::new();
            let mut terms_map = HashMap::new();
            let mut brightness_hist = BrightnessHistory::new();
            let mut natural_light_hist = NaturalLightHistory::new();
            let mut volume_ids_to_query = HashSet::new();
            let mut books_from_events = HashMap::new();

            while let Some(row) = rows.next()? {
                let event_id: String = row.get("Id")?;
                let event_type: String = row.get("Type")?;
                let ts_str: String = row.get("Timestamp")?;
                let ts = DateTime::<Utc>::from_str(&ts_str).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;

                match event_type.as_str() {
                    "OpenContent" | "LeaveContent" => {
                        if option == ParseOption::All || option == ParseOption::ReadingSessions {
                            let attr_json: String = row.get("Attributes")?;
                            let attr: ReadingSessionAttributes = serde_json::from_str(&attr_json)
                                .map_err(|e| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    1,
                                    rusqlite::types::Type::Text,
                                    Box::new(e),
                                )
                            })?;
                            let progress = attr.progress.parse::<u8>().unwrap_or(0);

                            if attr.volumeid.is_none() {
                                if let (Some(title), Some(author)) = (attr.title.clone(), attr.author.clone()) {
                                    if !books_from_events.contains_key(&title) {
                                        books_from_events.insert(title.clone(), Book::new(author, title, None, "".to_string()));
                                    }
                                }
                            } else if attr.title.is_none() {
                                if let Some(volume_id) = &attr.volumeid {
                                    volume_ids_to_query.insert(volume_id.clone());
                                }
                            }

                            let metrics = if event_type == "LeaveContent" {
                                let metr_json: String = row.get("Metrics")?;
                                Some(
                                    serde_json::from_str::<LeaveContentMetrics>(&metr_json)
                                        .map_err(|e| {
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
                                Ok(Some(session)) => sessions_vec.add_session(session),
                                Ok(None) => {}
                                Err(e) => eprintln!("Errore evento {}: {:?}", &event_id, e),
                            }
                        }
                    }
                    "DictionaryLookup" => {
                        if option == ParseOption::All || option == ParseOption::DictionaryLookups {
                            let session_id = current_session.as_ref().map(|s| s.id);
                            let attr_json: String = row.get("Attributes")?;
                            *terms_map
                                .entry(on_dictionary_lookup(attr_json, session_id)?)
                                .or_insert(0) += 1;
                        }
                    }
                    "BrightnessAdjusted" => {
                        if option == ParseOption::All || option == ParseOption::BrightnessHistory {
                            let attr_json: String = row.get("Attributes")?;
                            let metr_json: String = row.get("Metrics")?;
                            let event = on_light_adjusted(attr_json, metr_json, ts)?;
                            brightness_hist.insert(event);
                        }
                    }
                    "NaturalLightAdjusted" => {
                        if option == ParseOption::All || option == ParseOption::NaturalLightHistory
                        {
                            let attr_json: String = row.get("Attributes")?;
                            let metr_json: String = row.get("Metrics")?;
                            let event = on_light_adjusted(attr_json, metr_json, ts)?;
                            natural_light_hist.insert(event);
                        }
                    }
                    _ => {
                        eprintln!("Unknown event: {}", event_type);
                    }
                }
            }

            let mut books_from_db = get_books_by_volume_id(db, &volume_ids_to_query)?;
            books_from_db.extend(books_from_events);
            analysis.books = Some(books_from_db.values().cloned().collect());

            for session in sessions_vec.get_mut_sessions() {
                if let Some(volume_id) = &session.volume_id {
                    if let Some(book) = books_from_db.get(volume_id) {
                        session.book_title = Some(book.title.clone());
                    }
                }
            }

            if option == ParseOption::All || option == ParseOption::ReadingSessions {
                analysis.sessions = Some(sessions_vec);
            }
            if option == ParseOption::All || option == ParseOption::DictionaryLookups {
                analysis.terms = Some(terms_map);
            }
            if option == ParseOption::All || option == ParseOption::BrightnessHistory {
                analysis.brightness_history = Some(brightness_hist);
            }
            if option == ParseOption::All || option == ParseOption::NaturalLightHistory {
                analysis.natural_light_history = Some(natural_light_hist);
            }
        }
        Ok(analysis)
    }
    pub fn parse_from_str<P: AsRef<Path>>(
        path: P,
        option: ParseOption,
    ) -> rusqlite::Result<EventAnalysis> {
        let conn = Connection::open(path)?;
        Self::parse_events(&conn, option)
    }
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
            if let Some(ref mut session) = current_session {
                let _open_content_id = session.open_content_id.clone();
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
                    .map_err(|_| ParseError::SessionCompletionFailed)?;

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

fn on_dictionary_lookup(
    attr_json: String,
    session_id: Option<Uuid>,
) -> rusqlite::Result<DictionaryWord> {
    let attr: DicitonaryAttributes = serde_json::from_str(&attr_json).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(e))
    })?;
    Ok(DictionaryWord::new(attr.word, attr.lang, session_id))
}

fn on_light_adjusted(
    attr_json: String,
    metr_json: String,
    ts: DateTime<Utc>,
) -> rusqlite::Result<BrightnessEvent> {
    let attributes: LightAttributes = serde_json::from_str(&attr_json).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(e))
    })?;
    let metrics: LightMetrics = serde_json::from_str(&metr_json).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(e))
    })?;
    let brightness = Brightness::new(attributes.method, metrics.new_light);
    Ok(BrightnessEvent::new(brightness, ts))
}

fn get_books_by_volume_id(
    db: &Connection,
    volume_ids: &HashSet<String>,
) -> rusqlite::Result<HashMap<String, Book>> {
    let mut books = HashMap::new();
    if volume_ids.is_empty() {
        return Ok(books);
    }
    let mut stmt = db.prepare("SELECT BookID, Title, Attribution as Authors FROM content WHERE ContentType=6 AND BookID = ?1")?;

    for volume_id in volume_ids {
        let mut rows = stmt.query([volume_id])?;
        if let Some(row) = rows.next()? {
            let title: String = row.get("Title")?;
            let authors: String = row.get("Authors")?;
            let book_id: String = row.get("BookID")?;
            books.insert(
                volume_id.clone(),
                Book::new(authors, title, None, book_id),
            );
        }
    }

    Ok(books)
}

#[cfg(test)]
mod tests {
    use super::{Parser, ParseOption};
    use rusqlite::Connection;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE AnalyticsEvents (\n                Id TEXT PRIMARY KEY,\n                Type TEXT NOT NULL,\n                Timestamp TEXT NOT NULL,\n                Attributes TEXT,\n                Metrics TEXT\n            );\n            CREATE TABLE content (\n                ContentID TEXT PRIMARY KEY,\n                ContentType INTEGER,\n                Title TEXT,\n                Attribution TEXT,\n                BookID TEXT\n            );\n            CREATE TABLE Bookmark (\n                BookmarkID TEXT PRIMARY KEY,\n                Text TEXT,\n                VolumeID TEXT,\n                Color INTEGER,\n                ChapterProgress REAL,\n                DateCreated TEXT,\n                DateModified TEXT\n            );",
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_parse_events_all() {
        let db = setup_test_db();

        // Insert sample data
        db.execute(
            "INSERT INTO AnalyticsEvents (Id, Type, Timestamp, Attributes, Metrics) VALUES (?, ?, ?, ?, ?)",
            &[
                "session1_open",
                "OpenContent",
                "2023-01-01T10:00:00Z",
                "{\"progress\":\"0\",\"volumeid\":\"book1\"}", ""
            ],
        ).unwrap();
        db.execute(
            "INSERT INTO AnalyticsEvents (Id, Type, Timestamp, Attributes, Metrics) VALUES (?, ?, ?, ?, ?)",
            &[
                "session1_leave",
                "LeaveContent",
                "2023-01-01T10:05:00Z",
                "{\"progress\":\"10\",\"volumeid\":\"book1\"}", "{\"ButtonPressCount\":10,\"SecondsRead\":300,\"PagesTurned\":5}"
            ],
        ).unwrap();
        db.execute(
            "INSERT INTO AnalyticsEvents (Id, Type, Timestamp, Attributes, Metrics) VALUES (?, ?, ?, ?, ?)",
            &[
                "dict_lookup1",
                "DictionaryLookup",
                "2023-01-01T10:01:00Z",
                "{\"Dictionary\":\"en\",\"Word\":\"test\"}", ""
            ],
        ).unwrap();
        db.execute(
            "INSERT INTO AnalyticsEvents (Id, Type, Timestamp, Attributes, Metrics) VALUES (?, ?, ?, ?, ?)",
            &[
                "brightness_adj1",
                "BrightnessAdjusted",
                "2023-01-01T10:02:00Z",
                "{\"Method\":\"manual\"}", "{\"NewBrightness\":50}"
            ],
        ).unwrap();
        db.execute(
            "INSERT INTO AnalyticsEvents (Id, Type, Timestamp, Attributes, Metrics) VALUES (?, ?, ?, ?, ?)",
            &[
                "natural_light_adj1",
                "NaturalLightAdjusted",
                "2023-01-01T10:03:00Z",
                "{\"Method\":\"auto\"}", "{\"NewNaturalLight\":70}"
            ],
        ).unwrap();

        db.execute(
            "INSERT INTO content (ContentID, Title, ContentType, Attribution, BookID) VALUES (?, ?, ?, ?, ?)",
            &["book1", "Book One", "6", "Author One", "book1"],
        )
        .unwrap();
        db.execute(
            "INSERT INTO Bookmark (BookmarkID, Text, VolumeID, Color, ChapterProgress, DateCreated, DateModified) VALUES (?, ?, ?, ?, ?, ?, ?)",
            &["bookmark1", "Some text", "book1", "1", "0.5", "2023-01-01T10:06:00Z", "2023-01-01T10:06:00Z"],
        ).unwrap();

        let analysis = Parser::parse_events(&db, ParseOption::All).unwrap();

        assert!(analysis.sessions.is_some());
        assert_eq!(analysis.sessions.unwrap().sessions_count(), 1);

        assert!(analysis.terms.is_some());
        assert_eq!(analysis.terms.unwrap().len(), 1);

        assert!(analysis.brightness_history.is_some());
        assert_eq!(analysis.brightness_history.unwrap().events.len(), 1);

        assert!(analysis.natural_light_history.is_some());
        assert_eq!(analysis.natural_light_history.unwrap().events.len(), 1);

        assert!(analysis.bookmarks.is_some());
        assert_eq!(analysis.bookmarks.unwrap().len(), 1);

        assert!(analysis.books.is_some());
        assert_eq!(analysis.books.unwrap().len(), 1);
    }

    #[test]
    fn test_parse_events_reading_sessions() {
        let db = setup_test_db();
        db.execute(
            "INSERT INTO AnalyticsEvents (Id, Type, Timestamp, Attributes, Metrics) VALUES (?, ?, ?, ?, ?)",
            &[
                "session1_open",
                "OpenContent",
                "2023-01-01T10:00:00Z",
                "{\"progress\":\"0\",\"volumeid\":\"book1\"}", ""
            ],
        ).unwrap();
        db.execute(
            "INSERT INTO AnalyticsEvents (Id, Type, Timestamp, Attributes, Metrics) VALUES (?, ?, ?, ?, ?)",
            &[
                "session1_leave",
                "LeaveContent",
                "2023-01-01T10:05:00Z",
                "{\"progress\":\"10\",\"volumeid\":\"book1\"}", "{\"ButtonPressCount\":10,\"SecondsRead\":300,\"PagesTurned\":5}"
            ],
        ).unwrap();
        
        db.execute(
            "INSERT INTO content (ContentID, Title, ContentType, Attribution, BookID) VALUES (?, ?, ?, ?, ?)",
            &["book1", "The Real Book Title", "6", "Author One", "book1"],
        )
        .unwrap();

        let analysis = Parser::parse_events(&db, ParseOption::ReadingSessions).unwrap();

        assert!(analysis.sessions.is_some());
        let sessions = analysis.sessions.unwrap();
        assert_eq!(sessions.sessions_count(), 1);
        let session = sessions.get_sessions().get(0).unwrap();
        assert_eq!(session.book_title.as_deref(), Some("The Real Book Title"));
        assert!(analysis.terms.is_none());
        assert!(analysis.brightness_history.is_none());
        assert!(analysis.natural_light_history.is_none());
        assert!(analysis.bookmarks.is_none());
        assert!(analysis.books.is_some());
        assert_eq!(analysis.books.unwrap().len(), 1);
    }

    #[test]
    fn test_parse_events_bookmarks() {
        let db = setup_test_db();
        db.execute(
            "INSERT INTO content (ContentID, Title, ContentType, Attribution, BookID) VALUES (?, ?, ?, ?, ?)",
            &["book1", "Book One", "6", "Author One", "book1"],
        )
        .unwrap();
        db.execute(
            "INSERT INTO Bookmark (BookmarkID, Text, VolumeID, Color, ChapterProgress, DateCreated, DateModified) VALUES (?, ?, ?, ?, ?, ?, ?)",
            &["bookmark1", "Some text", "book1", "1", "0.5", "2023-01-01T10:06:00Z", "2023-01-01T10:06:00Z"],
        ).unwrap();

        let analysis = Parser::parse_events(&db, ParseOption::Bookmarks).unwrap();

        assert!(analysis.sessions.is_none());
        assert!(analysis.terms.is_none());
        assert!(analysis.brightness_history.is_none());
        assert!(analysis.natural_light_history.is_none());
        assert!(analysis.bookmarks.is_some());
        assert_eq!(analysis.bookmarks.unwrap().len(), 1);
        assert!(analysis.books.is_none());
    }
}
