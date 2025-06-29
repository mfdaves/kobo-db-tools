use crate::model::brightness::brightness_event::BrightnessEvent;

#[derive(Debug, Clone, Default)]
pub struct BrightnessHistory {
    pub events: Vec<BrightnessEvent>,
}

impl BrightnessHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, brightness: BrightnessEvent) {
        self.events.push(brightness);
    }
}
