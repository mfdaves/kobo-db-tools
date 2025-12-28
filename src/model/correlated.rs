use chrono::{DateTime, NaiveDate, Utc};
use std::collections::HashMap;

use crate::{AppEvent, BrightnessEvent, DictionaryWord, ReadingSession};

#[derive(Debug, Clone)]
pub struct CorrelatedSession {
    pub session: ReadingSession,
    pub dictionary: Vec<DictionaryWord>,
    pub brightness: Vec<BrightnessEvent>,
    pub natural_light: Vec<BrightnessEvent>,
    pub app_events: Vec<AppEvent>,
}

impl CorrelatedSession {
    pub fn new(session: ReadingSession) -> Self {
        Self {
            session,
            dictionary: Vec::new(),
            brightness: Vec::new(),
            natural_light: Vec::new(),
            app_events: Vec::new(),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct OrphanEvents {
    pub dictionary: Vec<DictionaryWord>,
    pub brightness: Vec<BrightnessEvent>,
    pub natural_light: Vec<BrightnessEvent>,
    pub app_events: Vec<AppEvent>,
}

#[derive(Debug, Clone)]
pub struct CorrelatedAnalysis {
    pub sessions: Vec<CorrelatedSession>,
    pub orphans: OrphanEvents,
    pub cycles: Vec<ChargeCycle>,
    pub app_start_counts_by_day: HashMap<NaiveDate, usize>,
}

#[derive(Debug, Clone)]
pub struct ChargeCycle {
    pub start: DateTime<Utc>,
    pub end: Option<DateTime<Utc>>,
    pub sessions: Vec<CorrelatedSession>,
    pub app_events: Vec<AppEvent>,
    pub metrics: ChargeCycleMetrics,
}

#[derive(Debug, Default, Clone)]
pub struct ChargeCycleMetrics {
    pub total_seconds_read: u64,
    pub total_pages: u64,
    pub total_button_presses: u64,
    pub dictionary_lookups: usize,
    pub brightness_events: usize,
    pub app_starts: usize,
}
