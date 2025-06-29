use chrono::{DateTime, Utc};

use super::brightness_value::Brightness;
#[derive(Debug, Clone)]
pub struct BrightnessEvent {
    pub brightness: Brightness,
    pub timestamp: DateTime<Utc>,
}

impl BrightnessEvent {
    pub fn new(brightness: Brightness, timestamp: DateTime<Utc>) -> Self {
        Self {
            brightness,
            timestamp,
        }
    }
}
