use crate::Statistics;           
use crate::{ReadingSessions, ReadingMetric}; 
use std::cmp::Ordering; 

impl Statistics for ReadingSessions{

    type Metric = ReadingMetric; 
    
    fn avg(&self) -> f64 {
	    let valid_sessions_seconds: Vec<f64> = self
	        .valid_sessions()
	        .filter_map(|s| s.seconds_read.map(|sec| sec as f64))
	        .collect();

	    if valid_sessions_seconds.is_empty() {
	        0.0
	    } else {
	        valid_sessions_seconds.iter().sum::<f64>() / valid_sessions_seconds.len() as f64
	    }
    }
    fn calculate_percentile(&self, metric: ReadingMetric, percentiles: &[f64]) -> Vec<f64> {
        let mut values: Vec<f64> = self
            .valid_sessions()
            .map(|s| match metric {
                ReadingMetric::SecondsRead => s.seconds_read.unwrap_or(0) as f64,
                ReadingMetric::PagesTurned => s.pages_turned.unwrap_or(0) as f64,
                ReadingMetric::ButtonPressCount => s.button_press_count.unwrap_or(0) as f64,
                ReadingMetric::Progress => (s.end_progress.unwrap_or(0) - s.start_progress) as f64,
            })
            .collect();

        if values.is_empty() {
            return vec![0.0];
        }

        values.sort_by(|a, b| a.partial_cmp(&b).unwrap_or(Ordering::Less));
        percentiles
            .iter()
            .map(|&p| {
                let idx = ((p.clamp(0.0, 1.0)) * ((values.len() - 1) as f64)).round() as usize;
                values.get(idx).copied().unwrap_or(0.0)
            })
            .collect()
    }
}