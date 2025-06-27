#[derive(Debug,Clone)]
struct BrightnessHistory{
	events:Vec<BrightnessEvent>
}

impl BrightnessHistory {
	pub fn new(events:Vec<BrightnessEvent>)->Self{
		Self{
			events
		}
	}
}

