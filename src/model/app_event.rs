use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppEventKind {
    AppStart,
    PluggedIn,
}

#[derive(Debug, Clone)]
pub struct AppEvent {
    pub kind: AppEventKind,
    pub timestamp: DateTime<Utc>,
    pub attributes: Option<serde_json::Value>,
}

impl AppEvent {
    pub fn new(
        kind: AppEventKind,
        timestamp: DateTime<Utc>,
        attributes: Option<serde_json::Value>,
    ) -> Self {
        Self {
            kind,
            timestamp,
            attributes,
        }
    }
}
