use crate::model::brightness::brightness_event::BrightnessEvent;

#[derive(Debug, Clone, Default)]
pub struct NaturalLightHistory {
    pub events: Vec<BrightnessEvent>,
}

impl NaturalLightHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, brightness: BrightnessEvent) {
        self.events.push(brightness);
    }
}
