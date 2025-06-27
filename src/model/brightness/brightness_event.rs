use super::brightness::Brightness;
#[derive(Debug,Clone)]
struct BrightnessEvent{
	brightness:Brightness, 
	timestamp:DateTime<Utc>
}

impl BrightnessEvent{
	pub fn new(brightness:Brightness,timestamp:DateTime<Utc>)->Self{
		Self{
			brightness,
			timestamp
		}
	}
}