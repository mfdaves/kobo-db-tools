use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Default, Clone)]
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
