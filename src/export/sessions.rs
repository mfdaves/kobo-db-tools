use chrono::{DateTime, SecondsFormat, Utc};
use serde::Serialize;

use crate::export::{Export, ExportError};
use crate::model::CorrelatedSession;

#[derive(Serialize)]
struct SessionExportRow {
    session_id: String,
    open_content_id: String,
    leave_content_id: Option<String>,
    start_time: String,
    end_time: Option<String>,
    duration_seconds: Option<i64>,
    start_progress: u8,
    end_progress: Option<u8>,
    progress_delta: Option<i16>,
    seconds_read: Option<u64>,
    pages_turned: Option<u64>,
    button_press_count: Option<u64>,
    book_title: Option<String>,
    volume_id: Option<String>,
    dictionary_lookups: usize,
    brightness_events: usize,
    natural_light_events: usize,
    app_events: usize,
}

fn format_time(ts: DateTime<Utc>) -> String {
    ts.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn to_export_row(session: &CorrelatedSession) -> SessionExportRow {
    let end_progress = session.session.end_progress;
    let progress_delta = end_progress.map(|end| end as i16 - session.session.start_progress as i16);
    SessionExportRow {
        session_id: session.session.id.to_string(),
        open_content_id: session.session.open_content_id.clone(),
        leave_content_id: session.session.leave_content_id.clone(),
        start_time: format_time(session.session.time_start),
        end_time: session.session.time_end.map(format_time),
        duration_seconds: session.session.duration().map(|d| d.num_seconds()),
        start_progress: session.session.start_progress,
        end_progress,
        progress_delta,
        seconds_read: session.session.seconds_read,
        pages_turned: session.session.pages_turned,
        button_press_count: session.session.button_press_count,
        book_title: session.session.book_title.clone(),
        volume_id: session.session.volume_id.clone(),
        dictionary_lookups: session.dictionary.len(),
        brightness_events: session.brightness.len(),
        natural_light_events: session.natural_light.len(),
        app_events: session.app_events.len(),
    }
}

impl Export for [CorrelatedSession] {
    fn to_csv(&self) -> Result<String, ExportError> {
        let mut wtr = csv::Writer::from_writer(vec![]);
        for session in self {
            wtr.serialize(to_export_row(session))?;
        }
        Ok(String::from_utf8(wtr.into_inner()?)?)
    }

    fn to_md(&self) -> Result<String, ExportError> {
        let mut buffer = Vec::new();
        use std::io::Write;

        writeln!(
            buffer,
            "| Start | End | Progress Δ | Pages | Buttons | Book | Dictionary | Brightness |"
        )?;
        writeln!(
            buffer,
            "|-------|-----|------------|-------|---------|------|------------|------------|"
        )?;

        for session in self {
            let row = to_export_row(session);
            let end_time = row.end_time.unwrap_or_else(|| "N/A".to_string());
            let progress_delta = row
                .progress_delta
                .map(|delta| delta.to_string())
                .unwrap_or_else(|| "N/A".to_string());
            let pages = row
                .pages_turned
                .map(|pages| pages.to_string())
                .unwrap_or_else(|| "N/A".to_string());
            let buttons = row
                .button_press_count
                .map(|count| count.to_string())
                .unwrap_or_else(|| "N/A".to_string());
            let book_title = row.book_title.unwrap_or_else(|| "N/A".to_string());
            let brightness_count = row.brightness_events + row.natural_light_events;

            writeln!(
                buffer,
                "| {} | {} | {} | {} | {} | {} | {} | {} |",
                row.start_time,
                end_time,
                progress_delta,
                pages,
                buttons,
                book_title,
                row.dictionary_lookups,
                brightness_count
            )?;
        }

        Ok(String::from_utf8(buffer)?)
    }

    fn to_json(&self) -> Result<String, ExportError> {
        let rows: Vec<SessionExportRow> = self.iter().map(to_export_row).collect();
        serde_json::to_string(&rows).map_err(ExportError::JsonToString)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AppEvent, AppEventKind, Brightness, BrightnessEvent, CorrelatedSession, DictionaryWord,
        ReadingSession,
    };
    use chrono::{DateTime, Utc};
    use std::str::FromStr;
    use uuid::Uuid;

    fn build_session() -> CorrelatedSession {
        let start = DateTime::<Utc>::from_str("2023-01-01T10:00:00Z").unwrap();
        let end = DateTime::<Utc>::from_str("2023-01-01T10:10:00Z").unwrap();
        let mut session = ReadingSession::new(
            start,
            10,
            Some("Book Title".to_string()),
            Some("book1".to_string()),
            "open1".to_string(),
        );
        session.id = Uuid::nil();
        session
            .complete_session(end, 20, 3, 600, 5, "leave1".to_string())
            .unwrap();
        let mut correlated = CorrelatedSession::new(session);
        correlated.dictionary.push(DictionaryWord::new(
            "test".to_string(),
            "en".to_string(),
            Some(Uuid::nil()),
        ));
        correlated.brightness.push(BrightnessEvent::new(
            Brightness::new("manual".to_string(), 50),
            start,
        ));
        correlated.natural_light.push(BrightnessEvent::new(
            Brightness::new("auto".to_string(), 60),
            start,
        ));
        correlated
            .app_events
            .push(AppEvent::new(AppEventKind::AppStart, start, None));
        correlated
    }

    #[test]
    fn test_sessions_to_csv() {
        let sessions = [build_session()];
        let expected = [
            "session_id,open_content_id,leave_content_id,start_time,end_time,duration_seconds,start_progress,end_progress,progress_delta,seconds_read,pages_turned,button_press_count,book_title,volume_id,dictionary_lookups,brightness_events,natural_light_events,app_events",
            "00000000-0000-0000-0000-000000000000,open1,leave1,2023-01-01T10:00:00Z,2023-01-01T10:10:00Z,600,10,20,10,600,5,3,Book Title,book1,1,1,1,1",
            "",
        ]
        .join("\n");
        assert_eq!(sessions.to_csv().unwrap(), expected);
    }

    #[test]
    fn test_sessions_to_json() {
        let sessions = [build_session()];
        let json = sessions.to_json().unwrap();
        assert!(json.contains("\"session_id\":\"00000000-0000-0000-0000-000000000000\""));
        assert!(json.contains("\"dictionary_lookups\":1"));
    }
}
