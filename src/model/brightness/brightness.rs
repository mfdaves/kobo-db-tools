#[derive(Debug,Clone)]
pub struct Brightness{
	pub method:String,
	pub percentage:u8
}


impl Brightness {
	pub fn new(method:String,percentage:u8)->Self{
		Self{
			method,
			percentage
		}
	}
}