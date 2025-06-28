use crate::model::brightness::brightness_event::BrightnessEvent;

#[derive(Debug,Clone)]
pub struct NaturalLightHistory{
	pub events:Vec<BrightnessEvent>
}

impl NaturalLightHistory {
	pub fn new()->Self{
		Self{
			events:Vec::new()
		}
	}

	pub fn insert(&mut self,brightness:BrightnessEvent){
		self.events.push(brightness);
	}
}

