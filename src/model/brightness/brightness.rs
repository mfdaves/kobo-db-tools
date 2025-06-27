#[derive(Debug,Clone)]
struct Brightness{
	method:String,
	percentage:u8
}


impl Brightness {
	pub new(method:String,percentage:u8)->Self{
		Self{
			method,
			percentage
		}
	}
}