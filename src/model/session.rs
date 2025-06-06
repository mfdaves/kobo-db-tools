use chrono::{DateTime, Utc, Duration};
use uuid::Uuid;

/*
rendere option book_title volume_id ecc ecc, 
cosi da renderlo ok con tutto il resto, quando ho volume id posso ricercare il titolo fra i content table
*/



#[derive(Debug)]
pub enum ReadingSessionError {
    InvalidEndTime,
    InvalidProgressValue,
}

#[derive(Debug)]
pub struct ReadingSession {
    pub id: Uuid,
    pub time_start: DateTime<Utc>,
    pub time_end: Option<DateTime<Utc>>,
    pub start_progress: u8,
    pub end_progress: Option<u8>,
    pub volume_id: String,
    pub book_title: String,
    pub button_press_count: Option<u64>,
    pub seconds_read: Option<u64>,
    pub pages_turned: Option<u64>,
}


impl ReadingSession {
    pub fn new(
        ts: DateTime<Utc>,
        start_progress: u8,
        book_title: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            time_start: ts,
            time_end: None,
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
    ) -> Result<(), ReadingSessionError> {
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

        Ok(())
    }

    pub fn session_is_complete(&self) -> bool {
        self.end_progress.is_some()
    }

    pub fn duration(&self) -> Option<Duration> {
        self.time_end.map(|end| end - self.time_start)
    }
}
