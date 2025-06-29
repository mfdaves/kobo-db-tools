use crate::ReadingSession;

const MIN_VALID_SESSION_TIME: u64 = 60;

#[derive(Debug, Default)]
pub struct ReadingSessions {
    sessions: Vec<ReadingSession>,
}

impl ReadingSessions {
    pub fn new() -> Self {
        Self::default()
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
    pub fn valid_sessions(&self) -> impl Iterator<Item = &ReadingSession> {
        self.sessions
            .iter()
            .filter(|s| s.is_complete() && s.seconds_read.unwrap_or(0) >= MIN_VALID_SESSION_TIME)
    }
    pub fn sessions_count(&self) -> usize {
        self.sessions.len()
    }
}
