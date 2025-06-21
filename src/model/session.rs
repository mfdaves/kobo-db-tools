use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;
use std::cmp::Ordering;

const MIN_VALID_SESSION_TIME: u64 = 60;

#[derive(Debug,Clone,Copy)]
pub enum ReadingMetric {
    SecondsRead,
    PagesTurned,
    ButtonPressCount,
    Progress,
}

#[derive(Debug)]
pub enum ReadingSessionError {
    InvalidEndTime,
    InvalidProgressValue,
}

#[derive(Debug, Default)]
pub struct ReadingSession {
    pub id: Uuid,
    pub open_content_id: String,
    pub leave_content_id: Option<String>,
    pub time_start: DateTime<Utc>,
    pub time_end: Option<DateTime<Utc>>,
    pub volume_id: Option<String>,
    pub start_progress: u8,
    pub end_progress: Option<u8>,
    pub book_title: Option<String>,
    pub button_press_count: Option<u64>,
    pub seconds_read: Option<u64>,
    pub pages_turned: Option<u64>,
}

impl ReadingSession {
    pub fn new(
        ts: DateTime<Utc>,
        start_progress: u8,
        book_title: Option<String>,
        volume_id: Option<String>,
        open_content_id: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            time_start: ts,
            time_end: None,
            open_content_id,
            leave_content_id: None,
            volume_id,
            start_progress,
            end_progress: None,
            book_title,
            button_press_count: None,
            seconds_read: None,
            pages_turned: None,
        }
    }

    pub fn complete_session(
        &mut self,
        time_end: DateTime<Utc>,
        end_progress: u8,
        button_press_count: u64,
        seconds_read: u64,
        pages_turned: u64,
        leave_content_id: String,
    ) -> Result<&mut Self, ReadingSessionError> {
        if self.time_start > time_end {
            return Err(ReadingSessionError::InvalidEndTime);
        }

        if end_progress > 100 || self.start_progress > 100 || end_progress < self.start_progress {
            return Err(ReadingSessionError::InvalidProgressValue);
        }

        self.time_end = Some(time_end);
        self.end_progress = Some(end_progress);
        self.button_press_count = Some(button_press_count);
        self.seconds_read = Some(seconds_read);
        self.pages_turned = Some(pages_turned);
        self.leave_content_id = Some(leave_content_id);
        Ok(self)
    }

    pub fn is_complete(&self) -> bool {
        self.end_progress.is_some()
    }

    pub fn duration(&self) -> Option<Duration> {
        self.time_end.map(|end| end - self.time_start)
    }
}

#[derive(Debug)]
pub struct ReadingSessions {
    sessions: Vec<ReadingSession>,
}

impl ReadingSessions {
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
        }
    }

    pub fn add_session(&mut self, session: ReadingSession) {
        self.sessions.push(session);
    }

    pub fn iter(&self) -> impl Iterator<Item = &ReadingSession> {
        self.sessions.iter().filter(|s| {
            s.is_complete()
                && s.seconds_read
                    .map(|sec| sec >= MIN_VALID_SESSION_TIME)
                    .unwrap_or(false)
        })
    }


    // FILTRO SULLE SESSIONI VALIDE
    fn valid_sessions(&self) -> impl Iterator<Item = &ReadingSession> {
        self.sessions.iter().filter(|s| {
            s.is_complete() && s.seconds_read.unwrap_or(0) >= MIN_VALID_SESSION_TIME
        })
    }

    pub fn avg_seconds_read(&self) -> f64 {
        let valid_sessions_seconds: Vec<f64> = self
            .valid_sessions()
            .filter_map(|s| s.seconds_read.map(|sec| sec as f64))
            .collect();

        if valid_sessions_seconds.is_empty() {
            0.0
        } else {
            valid_sessions_seconds.iter().sum::<f64>() / valid_sessions_seconds.len() as f64
        }
    }

    pub fn sessions_count(&self) -> usize {
        self.sessions.len()
    }

    pub fn calculate_percentile(&self, metric: ReadingMetric, percentiles: &[f64]) -> Vec<f64> {
        let mut values: Vec<f64> = self
            .valid_sessions()
            .map(|s| match metric {
                ReadingMetric::SecondsRead => s.seconds_read.unwrap_or(0) as f64,
                ReadingMetric::PagesTurned => s.pages_turned.unwrap_or(0) as f64,
                ReadingMetric::ButtonPressCount => s.button_press_count.unwrap_or(0) as f64,
                ReadingMetric::Progress => (s.end_progress.unwrap_or(0) - s.start_progress) as f64,
            })
            .collect();

        if values.is_empty() {
            return vec![0.0];
        }

        values.sort_by(|a, b| a.partial_cmp(&b).unwrap_or(Ordering::Less));

        percentiles
            .iter()
            .map(|&p| {
                let idx = ((p.clamp(0.0, 1.0)) * ((values.len() - 1) as f64)).round() as usize;
                values.get(idx).copied().unwrap_or(0.0)
            })
            .collect()
    }
}
