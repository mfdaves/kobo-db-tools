use crate::{
    get_bookmarks, AppEvent, AppEventKind, Book, Bookmark, Brightness, BrightnessEvent,
    BrightnessHistory, ChargeCycle, ChargeCycleMetrics, CorrelatedAnalysis, CorrelatedSession,
    DictionaryWord, NaturalLightHistory, OrphanEvents, ReadingSession, ReadingSessions,
};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params_from_iter, Connection, OpenFlags};
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
struct DictionaryAttributes {
    #[serde(rename = "Dictionary")]
    lang: String,
    #[serde(rename = "Word")]
    word: String,
}

struct TimedDictionaryWord {
    timestamp: DateTime<Utc>,
    word: DictionaryWord,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    pub app_events: Option<Vec<AppEvent>>,
}

pub struct Parser;

impl Parser {
    pub fn parse_events(db: &Connection, option: ParseOption) -> rusqlite::Result<EventAnalysis> {
        let mut analysis = EventAnalysis::default();
        let event_types = event_types_for_option(option);
        let include_bookmarks = matches!(option, ParseOption::All | ParseOption::Bookmarks);
        let include_sessions = matches!(option, ParseOption::All | ParseOption::ReadingSessions);
        let include_dictionary =
            matches!(option, ParseOption::All | ParseOption::DictionaryLookups);
        let include_brightness =
            matches!(option, ParseOption::All | ParseOption::BrightnessHistory);
        let include_natural_light =
            matches!(option, ParseOption::All | ParseOption::NaturalLightHistory);
        let include_app_events = matches!(
            option,
            ParseOption::All | ParseOption::AppStart | ParseOption::PluggedIn
        );

        if include_bookmarks {
            analysis.bookmarks = Some(get_bookmarks(db)?);
        }

        if !event_types.is_empty() {
            let q = build_event_query(event_types.len());

            let mut stmt = db.prepare(&q)?;
            let mut rows = stmt.query(params_from_iter(event_types.iter().copied()))?;

            let mut current_session: Option<ReadingSession> = None;
            let mut sessions_vec = ReadingSessions::new();
            let mut terms_map = HashMap::new();
            let mut brightness_hist = BrightnessHistory::new();
            let mut natural_light_hist = NaturalLightHistory::new();
            let mut volume_ids_to_query = HashSet::new();
            let mut books_from_events = HashMap::new();
            let mut app_events = Vec::new();

            while let Some(row) = rows.next()? {
                let event_id: String = row.get("Id")?;
                let event_type: String = row.get("Type")?;
                let ts_str: String = row.get("Timestamp")?;
                let ts = parse_timestamp(&ts_str, 2)?;

                match event_type.as_str() {
                    "OpenContent" | "LeaveContent" if include_sessions => {
                        let attr_json: String = row.get("Attributes")?;
                        let attr: ReadingSessionAttributes = from_json(&attr_json, 1)?;
                        let progress = attr.progress.parse::<u8>().unwrap_or(0);

                        match (&attr.volumeid, &attr.title, &attr.author) {
                            (None, Some(title), Some(author)) => {
                                books_from_events.entry(title.clone()).or_insert_with(|| {
                                    Book::new(author.clone(), title.clone(), None, String::new())
                                });
                            }
                            (Some(volume_id), None, _) => {
                                volume_ids_to_query.insert(volume_id.clone());
                            }
                            _ => {}
                        }

                        let metrics = if event_type == "LeaveContent" {
                            let metr_json: String = row.get("Metrics")?;
                            Some(from_json::<LeaveContentMetrics>(&metr_json, 2)?)
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
                    "DictionaryLookup" if include_dictionary => {
                        let session_id = current_session.as_ref().map(|s| s.id);
                        let attr_json: String = row.get("Attributes")?;
                        *terms_map
                            .entry(on_dictionary_lookup(&attr_json, session_id)?)
                            .or_insert(0) += 1;
                    }
                    "BrightnessAdjusted" if include_brightness => {
                        let attr_json: String = row.get("Attributes")?;
                        let metr_json: String = row.get("Metrics")?;
                        let event = on_light_adjusted(&attr_json, &metr_json, ts)?;
                        brightness_hist.insert(event);
                    }
                    "NaturalLightAdjusted" if include_natural_light => {
                        let attr_json: String = row.get("Attributes")?;
                        let metr_json: String = row.get("Metrics")?;
                        let event = on_light_adjusted(&attr_json, &metr_json, ts)?;
                        natural_light_hist.insert(event);
                    }
                    "AppStart" | "PluggedIn" if include_app_events => {
                        let attr_json: Option<String> = row.get("Attributes")?;
                        let attributes = parse_optional_json_value(attr_json, 1)?;
                        let kind = match event_type.as_str() {
                            "AppStart" => AppEventKind::AppStart,
                            "PluggedIn" => AppEventKind::PluggedIn,
                            _ => continue,
                        };
                        app_events.push(AppEvent::new(kind, ts, attributes));
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

            if include_sessions {
                analysis.sessions = Some(sessions_vec);
            }
            if include_dictionary {
                analysis.terms = Some(terms_map);
            }
            if include_brightness {
                analysis.brightness_history = Some(brightness_hist);
            }
            if include_natural_light {
                analysis.natural_light_history = Some(natural_light_hist);
            }
            if include_app_events {
                analysis.app_events = Some(app_events);
            }
        }
        Ok(analysis)
    }

    pub fn parse_correlated(db: &Connection) -> rusqlite::Result<CorrelatedAnalysis> {
        const TOLERANCE_SECONDS: i64 = 30;
        let tolerance = Duration::seconds(TOLERANCE_SECONDS);
        let event_types = event_types_for_option(ParseOption::All);
        let q = build_event_query(event_types.len());

        let mut stmt = db.prepare(&q)?;
        let mut rows = stmt.query(params_from_iter(event_types.iter().copied()))?;

        let mut current_session: Option<ReadingSession> = None;
        let mut sessions_vec = ReadingSessions::new();
        let mut dictionary_events = Vec::new();
        let mut brightness_events = Vec::new();
        let mut natural_light_events = Vec::new();
        let mut app_events = Vec::new();
        let mut volume_ids_to_query = HashSet::new();
        let mut books_from_events = HashMap::new();

        while let Some(row) = rows.next()? {
            let event_id: String = row.get("Id")?;
            let event_type: String = row.get("Type")?;
            let ts_str: String = row.get("Timestamp")?;
            let ts = parse_timestamp(&ts_str, 2)?;

            match event_type.as_str() {
                "OpenContent" | "LeaveContent" => {
                    let attr_json: String = row.get("Attributes")?;
                    let attr: ReadingSessionAttributes = from_json(&attr_json, 1)?;
                    let progress = attr.progress.parse::<u8>().unwrap_or(0);

                    match (&attr.volumeid, &attr.title, &attr.author) {
                        (None, Some(title), Some(author)) => {
                            books_from_events.entry(title.clone()).or_insert_with(|| {
                                Book::new(author.clone(), title.clone(), None, String::new())
                            });
                        }
                        (Some(volume_id), None, _) => {
                            volume_ids_to_query.insert(volume_id.clone());
                        }
                        _ => {}
                    }

                    let metrics = if event_type == "LeaveContent" {
                        let metr_json: String = row.get("Metrics")?;
                        Some(from_json::<LeaveContentMetrics>(&metr_json, 2)?)
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
                "DictionaryLookup" => {
                    let attr_json: String = row.get("Attributes")?;
                    let word = on_dictionary_lookup(&attr_json, None)?;
                    dictionary_events.push(TimedDictionaryWord {
                        timestamp: ts,
                        word,
                    });
                }
                "BrightnessAdjusted" => {
                    let attr_json: String = row.get("Attributes")?;
                    let metr_json: String = row.get("Metrics")?;
                    let event = on_light_adjusted(&attr_json, &metr_json, ts)?;
                    brightness_events.push(event);
                }
                "NaturalLightAdjusted" => {
                    let attr_json: String = row.get("Attributes")?;
                    let metr_json: String = row.get("Metrics")?;
                    let event = on_light_adjusted(&attr_json, &metr_json, ts)?;
                    natural_light_events.push(event);
                }
                "AppStart" | "PluggedIn" => {
                    let attr_json: Option<String> = row.get("Attributes")?;
                    let attributes = parse_optional_json_value(attr_json, 1)?;
                    let kind = match event_type.as_str() {
                        "AppStart" => AppEventKind::AppStart,
                        "PluggedIn" => AppEventKind::PluggedIn,
                        _ => continue,
                    };
                    app_events.push(AppEvent::new(kind, ts, attributes));
                }
                _ => {}
            }
        }

        let mut books_from_db = get_books_by_volume_id(db, &volume_ids_to_query)?;
        books_from_db.extend(books_from_events);

        for session in sessions_vec.get_mut_sessions() {
            if let Some(volume_id) = &session.volume_id {
                if let Some(book) = books_from_db.get(volume_id) {
                    session.book_title = Some(book.title.clone());
                }
            }
        }

        let sessions = std::mem::take(sessions_vec.get_mut_sessions());
        let mut correlated_sessions: Vec<CorrelatedSession> =
            sessions.into_iter().map(CorrelatedSession::new).collect();

        let mut orphan_dictionary = Vec::new();
        for timed in dictionary_events {
            if let Some(index) =
                find_session_index(&correlated_sessions, timed.timestamp, tolerance)
            {
                let session_id = correlated_sessions[index].session.id;
                let word = DictionaryWord::new(
                    timed.word.term().to_string(),
                    timed.word.lang().to_string(),
                    Some(session_id),
                );
                correlated_sessions[index].dictionary.push(word);
            } else {
                orphan_dictionary.push(DictionaryWord::new(
                    timed.word.term().to_string(),
                    timed.word.lang().to_string(),
                    None,
                ));
            }
        }

        let mut orphan_brightness = Vec::new();
        for event in brightness_events {
            if let Some(index) =
                find_session_index(&correlated_sessions, event.timestamp, tolerance)
            {
                correlated_sessions[index].brightness.push(event);
            } else {
                orphan_brightness.push(event);
            }
        }

        let mut orphan_natural_light = Vec::new();
        for event in natural_light_events {
            if let Some(index) =
                find_session_index(&correlated_sessions, event.timestamp, tolerance)
            {
                correlated_sessions[index].natural_light.push(event);
            } else {
                orphan_natural_light.push(event);
            }
        }

        let mut orphan_app_events = Vec::new();
        for event in app_events {
            if let Some(index) =
                find_session_index(&correlated_sessions, event.timestamp, tolerance)
            {
                correlated_sessions[index].app_events.push(event);
            } else {
                orphan_app_events.push(event);
            }
        }

        let orphans = OrphanEvents {
            dictionary: orphan_dictionary,
            brightness: orphan_brightness,
            natural_light: orphan_natural_light,
            app_events: orphan_app_events,
        };

        let mut all_app_events: Vec<AppEvent> = correlated_sessions
            .iter()
            .flat_map(|session| session.app_events.iter().cloned())
            .collect();
        all_app_events.extend(orphans.app_events.iter().cloned());

        let cycles = build_charge_cycles(&correlated_sessions, &all_app_events);
        let mut app_start_counts_by_day = HashMap::new();
        for event in all_app_events
            .iter()
            .filter(|event| event.kind == AppEventKind::AppStart)
        {
            let day = event.timestamp.date_naive();
            *app_start_counts_by_day.entry(day).or_insert(0) += 1;
        }

        Ok(CorrelatedAnalysis {
            sessions: correlated_sessions,
            orphans,
            cycles,
            app_start_counts_by_day,
        })
    }
    pub fn parse_from_str<P: AsRef<Path>>(
        path: P,
        option: ParseOption,
    ) -> rusqlite::Result<EventAnalysis> {
        let path_ref = path.as_ref();
        let conn = Connection::open(path_ref).or_else(|err| {
            Connection::open_with_flags(path_ref, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|_| err)
        })?;
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
            let mut session = current_session
                .take()
                .ok_or(ParseError::SessionCompletionFailed)?;
            let m = metrics.ok_or(ParseError::SessionCompletionFailed)?;
            if session
                .complete_session(
                    ts,
                    progress,
                    m.button_press_count as u64,
                    m.seconds_read as u64,
                    m.pages_turned as u64,
                    event_id.to_string(),
                )
                .is_err()
            {
                *current_session = Some(session);
                return Err(ParseError::SessionCompletionFailed);
            }
            Ok(Some(session))
        }
        _ => Err(ParseError::InvalidEventType),
    }
}

fn on_dictionary_lookup(
    attr_json: &str,
    session_id: Option<Uuid>,
) -> rusqlite::Result<DictionaryWord> {
    let attr: DictionaryAttributes = from_json(attr_json, 1)?;
    Ok(DictionaryWord::new(attr.word, attr.lang, session_id))
}

fn on_light_adjusted(
    attr_json: &str,
    metr_json: &str,
    ts: DateTime<Utc>,
) -> rusqlite::Result<BrightnessEvent> {
    let attributes: LightAttributes = from_json(attr_json, 1)?;
    let metrics: LightMetrics = from_json(metr_json, 1)?;
    let brightness = Brightness::new(attributes.method, metrics.new_light);
    Ok(BrightnessEvent::new(brightness, ts))
}

fn from_json<T: serde::de::DeserializeOwned>(
    json: &str,
    column_index: usize,
) -> rusqlite::Result<T> {
    serde_json::from_str(json).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(
            column_index,
            rusqlite::types::Type::Text,
            Box::new(e),
        )
    })
}

fn parse_timestamp(ts: &str, column_index: usize) -> rusqlite::Result<DateTime<Utc>> {
    DateTime::<Utc>::from_str(ts).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(
            column_index,
            rusqlite::types::Type::Text,
            Box::new(e),
        )
    })
}

fn parse_optional_json_value(
    json: Option<String>,
    column_index: usize,
) -> rusqlite::Result<Option<serde_json::Value>> {
    match json {
        None => Ok(None),
        Some(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            serde_json::from_str(trimmed).map(Some).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    column_index,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })
        }
    }
}

fn session_contains(session: &ReadingSession, ts: DateTime<Utc>, tolerance: Duration) -> bool {
    match session.time_end {
        Some(end) => ts >= session.time_start - tolerance && ts <= end + tolerance,
        None => false,
    }
}

fn find_session_index(
    sessions: &[CorrelatedSession],
    ts: DateTime<Utc>,
    tolerance: Duration,
) -> Option<usize> {
    sessions
        .iter()
        .position(|session| session_contains(&session.session, ts, tolerance))
}

fn session_in_cycle(
    session: &ReadingSession,
    start: DateTime<Utc>,
    end: Option<DateTime<Utc>>,
) -> bool {
    match end {
        Some(end) => session.time_start >= start && session.time_start < end,
        None => session.time_start >= start,
    }
}

fn app_event_in_cycle(event: &AppEvent, start: DateTime<Utc>, end: Option<DateTime<Utc>>) -> bool {
    match end {
        Some(end) => event.timestamp >= start && event.timestamp < end,
        None => event.timestamp >= start,
    }
}

fn compute_cycle_metrics(
    sessions: &[CorrelatedSession],
    app_events: &[AppEvent],
) -> ChargeCycleMetrics {
    let total_seconds_read = sessions
        .iter()
        .map(|session| session.session.seconds_read.unwrap_or(0))
        .sum();
    let total_pages = sessions
        .iter()
        .map(|session| session.session.pages_turned.unwrap_or(0))
        .sum();
    let total_button_presses = sessions
        .iter()
        .map(|session| session.session.button_press_count.unwrap_or(0))
        .sum();
    let dictionary_lookups = sessions
        .iter()
        .map(|session| session.dictionary.len())
        .sum();
    let brightness_events = sessions
        .iter()
        .map(|session| session.brightness.len() + session.natural_light.len())
        .sum();
    let app_starts = app_events
        .iter()
        .filter(|event| event.kind == AppEventKind::AppStart)
        .count();
    ChargeCycleMetrics {
        total_seconds_read,
        total_pages,
        total_button_presses,
        dictionary_lookups,
        brightness_events,
        app_starts,
    }
}

fn build_charge_cycles(
    sessions: &[CorrelatedSession],
    app_events: &[AppEvent],
) -> Vec<ChargeCycle> {
    let mut plug_events: Vec<AppEvent> = app_events
        .iter()
        .filter(|event| event.kind == AppEventKind::PluggedIn)
        .cloned()
        .collect();
    plug_events.sort_by_key(|event| event.timestamp);

    let mut cycles = Vec::new();
    for (index, plug_event) in plug_events.iter().enumerate() {
        let start = plug_event.timestamp;
        let end = plug_events.get(index + 1).map(|event| event.timestamp);
        let cycle_sessions: Vec<CorrelatedSession> = sessions
            .iter()
            .filter(|session| session_in_cycle(&session.session, start, end))
            .cloned()
            .collect();
        let cycle_app_events: Vec<AppEvent> = app_events
            .iter()
            .filter(|event| app_event_in_cycle(event, start, end))
            .cloned()
            .collect();
        let metrics = compute_cycle_metrics(&cycle_sessions, &cycle_app_events);
        cycles.push(ChargeCycle {
            start,
            end,
            sessions: cycle_sessions,
            app_events: cycle_app_events,
            metrics,
        });
    }
    cycles
}

fn event_types_for_option(option: ParseOption) -> Vec<&'static str> {
    match option {
        ParseOption::All => vec![
            "OpenContent",
            "LeaveContent",
            "DictionaryLookup",
            "BrightnessAdjusted",
            "NaturalLightAdjusted",
            "AppStart",
            "PluggedIn",
        ],
        ParseOption::ReadingSessions => vec!["OpenContent", "LeaveContent"],
        ParseOption::DictionaryLookups => vec!["DictionaryLookup"],
        ParseOption::BrightnessHistory => vec!["BrightnessAdjusted"],
        ParseOption::NaturalLightHistory => vec!["NaturalLightAdjusted"],
        ParseOption::AppStart => vec!["AppStart"],
        ParseOption::PluggedIn => vec!["PluggedIn"],
        ParseOption::Bookmarks => Vec::new(),
    }
}

fn build_event_query(event_types_len: usize) -> String {
    let placeholders = std::iter::repeat_n("?", event_types_len)
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "SELECT Id, Type, Timestamp, Attributes, Metrics FROM AnalyticsEvents WHERE Type IN ({}) ORDER BY Timestamp ASC;",
        placeholders
    )
}

fn get_books_by_volume_id(
    db: &Connection,
    volume_ids: &HashSet<String>,
) -> rusqlite::Result<HashMap<String, Book>> {
    let mut books = HashMap::new();
    if volume_ids.is_empty() {
        return Ok(books);
    }
    let mut stmt = db.prepare(
        "SELECT BookID, Title, Attribution as Authors FROM content WHERE ContentType=6 AND (ContentID = ?1 OR BookID = ?1)",
    )?;

    for volume_id in volume_ids {
        let mut rows = stmt.query([volume_id])?;
        if let Some(row) = rows.next()? {
            let title: String = row.get("Title")?;
            let authors: String = row.get("Authors")?;
            let book_id: Option<String> = row.get("BookID")?;
            let book_id = book_id.unwrap_or_else(|| volume_id.clone());
            books.insert(volume_id.clone(), Book::new(authors, title, None, book_id));
        }
    }

    Ok(books)
}

#[cfg(test)]
mod tests {
    use super::{ParseOption, Parser};
    use crate::AppEventKind;
    use chrono::NaiveDate;
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
            [
                "session1_open",
                "OpenContent",
                "2023-01-01T10:00:00Z",
                "{\"progress\":\"0\",\"volumeid\":\"book1\"}", ""
            ],
        ).unwrap();
        db.execute(
            "INSERT INTO AnalyticsEvents (Id, Type, Timestamp, Attributes, Metrics) VALUES (?, ?, ?, ?, ?)",
            [
                "session1_leave",
                "LeaveContent",
                "2023-01-01T10:05:00Z",
                "{\"progress\":\"10\",\"volumeid\":\"book1\"}", "{\"ButtonPressCount\":10,\"SecondsRead\":300,\"PagesTurned\":5}"
            ],
        ).unwrap();
        db.execute(
            "INSERT INTO AnalyticsEvents (Id, Type, Timestamp, Attributes, Metrics) VALUES (?, ?, ?, ?, ?)",
            [
                "dict_lookup1",
                "DictionaryLookup",
                "2023-01-01T10:01:00Z",
                "{\"Dictionary\":\"en\",\"Word\":\"test\"}", ""
            ],
        ).unwrap();
        db.execute(
            "INSERT INTO AnalyticsEvents (Id, Type, Timestamp, Attributes, Metrics) VALUES (?, ?, ?, ?, ?)",
            [
                "brightness_adj1",
                "BrightnessAdjusted",
                "2023-01-01T10:02:00Z",
                "{\"Method\":\"manual\"}", "{\"NewBrightness\":50}"
            ],
        ).unwrap();
        db.execute(
            "INSERT INTO AnalyticsEvents (Id, Type, Timestamp, Attributes, Metrics) VALUES (?, ?, ?, ?, ?)",
            [
                "natural_light_adj1",
                "NaturalLightAdjusted",
                "2023-01-01T10:03:00Z",
                "{\"Method\":\"auto\"}", "{\"NewNaturalLight\":70}"
            ],
        ).unwrap();
        db.execute(
            "INSERT INTO AnalyticsEvents (Id, Type, Timestamp, Attributes, Metrics) VALUES (?, ?, ?, ?, ?)",
            [
                "app_start1",
                "AppStart",
                "2023-01-01T09:59:00Z",
                "{\"app\":\"nickel\"}",
                ""
            ],
        ).unwrap();
        db.execute(
            "INSERT INTO AnalyticsEvents (Id, Type, Timestamp, Attributes, Metrics) VALUES (?, ?, ?, ?, ?)",
            [
                "plugged_in1",
                "PluggedIn",
                "2023-01-01T10:04:00Z",
                "",
                ""
            ],
        ).unwrap();

        db.execute(
            "INSERT INTO content (ContentID, Title, ContentType, Attribution, BookID) VALUES (?, ?, ?, ?, ?)",
            ["book1", "Book One", "6", "Author One", "book1"],
        )
        .unwrap();
        db.execute(
            "INSERT INTO Bookmark (BookmarkID, Text, VolumeID, Color, ChapterProgress, DateCreated, DateModified) VALUES (?, ?, ?, ?, ?, ?, ?)",
            ["bookmark1", "Some text", "book1", "1", "0.5", "2023-01-01T10:06:00Z", "2023-01-01T10:06:00Z"],
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

        assert!(analysis.app_events.is_some());
        let app_events = analysis.app_events.unwrap();
        assert_eq!(app_events.len(), 2);
        assert!(app_events
            .iter()
            .any(|event| event.kind == AppEventKind::AppStart));
        assert!(app_events
            .iter()
            .any(|event| event.kind == AppEventKind::PluggedIn));
    }

    #[test]
    fn test_parse_events_reading_sessions() {
        let db = setup_test_db();
        db.execute(
            "INSERT INTO AnalyticsEvents (Id, Type, Timestamp, Attributes, Metrics) VALUES (?, ?, ?, ?, ?)",
            [
                "session1_open",
                "OpenContent",
                "2023-01-01T10:00:00Z",
                "{\"progress\":\"0\",\"volumeid\":\"book1\"}", ""
            ],
        ).unwrap();
        db.execute(
            "INSERT INTO AnalyticsEvents (Id, Type, Timestamp, Attributes, Metrics) VALUES (?, ?, ?, ?, ?)",
            [
                "session1_leave",
                "LeaveContent",
                "2023-01-01T10:05:00Z",
                "{\"progress\":\"10\",\"volumeid\":\"book1\"}", "{\"ButtonPressCount\":10,\"SecondsRead\":300,\"PagesTurned\":5}"
            ],
        ).unwrap();

        db.execute(
            "INSERT INTO content (ContentID, Title, ContentType, Attribution, BookID) VALUES (?, ?, ?, ?, ?)",
            ["book1", "The Real Book Title", "6", "Author One", "book1"],
        )
        .unwrap();

        let analysis = Parser::parse_events(&db, ParseOption::ReadingSessions).unwrap();

        assert!(analysis.sessions.is_some());
        let sessions = analysis.sessions.unwrap();
        assert_eq!(sessions.sessions_count(), 1);
        let session = sessions.get_sessions().first().unwrap();
        assert_eq!(session.book_title.as_deref(), Some("The Real Book Title"));
        assert!(analysis.terms.is_none());
        assert!(analysis.brightness_history.is_none());
        assert!(analysis.natural_light_history.is_none());
        assert!(analysis.bookmarks.is_none());
        assert!(analysis.books.is_some());
        assert_eq!(analysis.books.unwrap().len(), 1);
        assert!(analysis.app_events.is_none());
    }

    #[test]
    fn test_parse_events_bookmarks() {
        let db = setup_test_db();
        db.execute(
            "INSERT INTO content (ContentID, Title, ContentType, Attribution, BookID) VALUES (?, ?, ?, ?, ?)",
            ["book1", "Book One", "6", "Author One", "book1"],
        )
        .unwrap();
        db.execute(
            "INSERT INTO Bookmark (BookmarkID, Text, VolumeID, Color, ChapterProgress, DateCreated, DateModified) VALUES (?, ?, ?, ?, ?, ?, ?)",
            ["bookmark1", "Some text", "book1", "1", "0.5", "2023-01-01T10:06:00Z", "2023-01-01T10:06:00Z"],
        ).unwrap();

        let analysis = Parser::parse_events(&db, ParseOption::Bookmarks).unwrap();

        assert!(analysis.sessions.is_none());
        assert!(analysis.terms.is_none());
        assert!(analysis.brightness_history.is_none());
        assert!(analysis.natural_light_history.is_none());
        assert!(analysis.bookmarks.is_some());
        assert_eq!(analysis.bookmarks.unwrap().len(), 1);
        assert!(analysis.books.is_none());
        assert!(analysis.app_events.is_none());
    }

    #[test]
    fn test_parse_events_app_start() {
        let db = setup_test_db();
        db.execute(
            "INSERT INTO AnalyticsEvents (Id, Type, Timestamp, Attributes, Metrics) VALUES (?, ?, ?, ?, ?)",
            [
                "app_start1",
                "AppStart",
                "2023-01-01T09:59:00Z",
                "{\"app\":\"nickel\"}",
                ""
            ],
        ).unwrap();
        db.execute(
            "INSERT INTO AnalyticsEvents (Id, Type, Timestamp, Attributes, Metrics) VALUES (?, ?, ?, ?, ?)",
            [
                "plugged_in1",
                "PluggedIn",
                "2023-01-01T10:04:00Z",
                "",
                ""
            ],
        ).unwrap();

        let analysis = Parser::parse_events(&db, ParseOption::AppStart).unwrap();
        assert!(analysis.app_events.is_some());
        let app_events = analysis.app_events.unwrap();
        assert_eq!(app_events.len(), 1);
        assert_eq!(app_events[0].kind, AppEventKind::AppStart);
    }

    #[test]
    fn test_parse_events_plugged_in() {
        let db = setup_test_db();
        db.execute(
            "INSERT INTO AnalyticsEvents (Id, Type, Timestamp, Attributes, Metrics) VALUES (?, ?, ?, ?, ?)",
            [
                "app_start1",
                "AppStart",
                "2023-01-01T09:59:00Z",
                "{\"app\":\"nickel\"}",
                ""
            ],
        ).unwrap();
        db.execute(
            "INSERT INTO AnalyticsEvents (Id, Type, Timestamp, Attributes, Metrics) VALUES (?, ?, ?, ?, ?)",
            [
                "plugged_in1",
                "PluggedIn",
                "2023-01-01T10:04:00Z",
                "",
                ""
            ],
        ).unwrap();

        let analysis = Parser::parse_events(&db, ParseOption::PluggedIn).unwrap();
        assert!(analysis.app_events.is_some());
        let app_events = analysis.app_events.unwrap();
        assert_eq!(app_events.len(), 1);
        assert_eq!(app_events[0].kind, AppEventKind::PluggedIn);
    }

    #[test]
    fn test_parse_correlated_basic() {
        let db = setup_test_db();
        db.execute(
            "INSERT INTO AnalyticsEvents (Id, Type, Timestamp, Attributes, Metrics) VALUES (?, ?, ?, ?, ?)",
            [
                "session1_open",
                "OpenContent",
                "2023-01-01T10:00:00Z",
                "{\"progress\":\"0\",\"volumeid\":\"book1\"}",
                "",
            ],
        )
        .unwrap();
        db.execute(
            "INSERT INTO AnalyticsEvents (Id, Type, Timestamp, Attributes, Metrics) VALUES (?, ?, ?, ?, ?)",
            [
                "session1_leave",
                "LeaveContent",
                "2023-01-01T10:10:00Z",
                "{\"progress\":\"10\",\"volumeid\":\"book1\"}",
                "{\"ButtonPressCount\":10,\"SecondsRead\":600,\"PagesTurned\":5}",
            ],
        )
        .unwrap();
        db.execute(
            "INSERT INTO AnalyticsEvents (Id, Type, Timestamp, Attributes, Metrics) VALUES (?, ?, ?, ?, ?)",
            [
                "dict_lookup1",
                "DictionaryLookup",
                "2023-01-01T10:02:00Z",
                "{\"Dictionary\":\"en\",\"Word\":\"test\"}",
                "",
            ],
        )
        .unwrap();
        db.execute(
            "INSERT INTO AnalyticsEvents (Id, Type, Timestamp, Attributes, Metrics) VALUES (?, ?, ?, ?, ?)",
            [
                "dict_lookup2",
                "DictionaryLookup",
                "2023-01-01T11:00:00Z",
                "{\"Dictionary\":\"en\",\"Word\":\"orphan\"}",
                "",
            ],
        )
        .unwrap();
        db.execute(
            "INSERT INTO AnalyticsEvents (Id, Type, Timestamp, Attributes, Metrics) VALUES (?, ?, ?, ?, ?)",
            [
                "brightness_adj1",
                "BrightnessAdjusted",
                "2023-01-01T10:03:00Z",
                "{\"Method\":\"manual\"}",
                "{\"NewBrightness\":50}",
            ],
        )
        .unwrap();
        db.execute(
            "INSERT INTO AnalyticsEvents (Id, Type, Timestamp, Attributes, Metrics) VALUES (?, ?, ?, ?, ?)",
            [
                "natural_light_adj1",
                "NaturalLightAdjusted",
                "2023-01-01T10:04:00Z",
                "{\"Method\":\"auto\"}",
                "{\"NewNaturalLight\":70}",
            ],
        )
        .unwrap();
        db.execute(
            "INSERT INTO AnalyticsEvents (Id, Type, Timestamp, Attributes, Metrics) VALUES (?, ?, ?, ?, ?)",
            [
                "plugged_in1",
                "PluggedIn",
                "2023-01-01T09:50:00Z",
                "",
                "",
            ],
        )
        .unwrap();
        db.execute(
            "INSERT INTO AnalyticsEvents (Id, Type, Timestamp, Attributes, Metrics) VALUES (?, ?, ?, ?, ?)",
            [
                "app_start1",
                "AppStart",
                "2023-01-01T10:00:10Z",
                "{\"app\":\"nickel\"}",
                "",
            ],
        )
        .unwrap();

        let analysis = Parser::parse_correlated(&db).unwrap();

        assert_eq!(analysis.sessions.len(), 1);
        let session = &analysis.sessions[0];
        assert_eq!(session.dictionary.len(), 1);
        assert_eq!(session.brightness.len(), 1);
        assert_eq!(session.natural_light.len(), 1);
        assert_eq!(session.app_events.len(), 1);

        assert_eq!(analysis.orphans.dictionary.len(), 1);
        assert_eq!(analysis.orphans.app_events.len(), 1);

        assert_eq!(analysis.cycles.len(), 1);
        let cycle = &analysis.cycles[0];
        assert_eq!(cycle.metrics.total_seconds_read, 600);
        assert_eq!(cycle.metrics.total_pages, 5);
        assert_eq!(cycle.metrics.total_button_presses, 10);
        assert_eq!(cycle.metrics.dictionary_lookups, 1);
        assert_eq!(cycle.metrics.brightness_events, 2);
        assert_eq!(cycle.metrics.app_starts, 1);
        assert_eq!(
            analysis
                .app_start_counts_by_day
                .get(&NaiveDate::from_ymd_opt(2023, 1, 1).unwrap())
                .copied(),
            Some(1)
        );
    }
}
