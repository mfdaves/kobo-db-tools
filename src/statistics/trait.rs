pub trait Statistics{
	type Metric;
	fn avg(&self)->f64;
	fn calculate_percentile(&self,metric:Self::Metric,percentiles:&[f64])->Vec<f64>; 
}