pub trait Statistics{
	fn avg(&self)->f64;
	fn calculate_percentile(&self,percentiles:&[f64])->Vec<f64>; 
}