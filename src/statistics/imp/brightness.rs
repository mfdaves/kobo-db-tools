use crate::model::{BrightnessHistory, NaturalLightHistory};
use crate::statistics::Statistics;

impl Statistics for BrightnessHistory {
    type Metric = ();

    fn avg(&self) -> f64 {
        if self.events.len() < 2 {
            return self
                .events
                .first()
                .map_or(0.0, |e| e.brightness.percentage as f64);
        }

        let mut weighted_sum = 0.0;
        let mut total_duration = 0.0;

        for i in 0..self.events.len() - 1 {
            let current_event = &self.events[i];
            let next_event = &self.events[i + 1];

            let duration = (next_event.timestamp - current_event.timestamp).num_seconds() as f64;
            if duration > 0.0 {
                weighted_sum += current_event.brightness.percentage as f64 * duration;
                total_duration += duration;
            }
        }

        if total_duration == 0.0 {
            self.events
                .last()
                .map_or(0.0, |e| e.brightness.percentage as f64)
        } else {
            weighted_sum / total_duration
        }
    }

    fn calculate_percentile(&self, _metric: Self::Metric, _percentiles: &[f64]) -> Vec<f64> {
        // Placeholder implementation
        unimplemented!("Percentile calculation is not implemented for BrightnessHistory");
    }
}

impl Statistics for NaturalLightHistory {
    type Metric = ();

    fn avg(&self) -> f64 {
        if self.events.len() < 2 {
            return self
                .events
                .first()
                .map_or(0.0, |e| e.brightness.percentage as f64);
        }

        let mut weighted_sum = 0.0;
        let mut total_duration = 0.0;

        for i in 0..self.events.len() - 1 {
            let current_event = &self.events[i];
            let next_event = &self.events[i + 1];

            let duration = (next_event.timestamp - current_event.timestamp).num_seconds() as f64;
            if duration > 0.0 {
                weighted_sum += current_event.brightness.percentage as f64 * duration;
                total_duration += duration;
            }
        }

        if total_duration == 0.0 {
            self.events
                .last()
                .map_or(0.0, |e| e.brightness.percentage as f64)
        } else {
            weighted_sum / total_duration
        }
    }

    fn calculate_percentile(&self, _metric: Self::Metric, percentiles: &[f64]) -> Vec<f64> {
        let mut values: Vec<f64> = self
            .events
            .iter()
            .map(|e| e.brightness.percentage as f64)
            .collect();

        if values.is_empty() {
            return vec![0.0];
        }

        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Less));
        percentiles
            .iter()
            .map(|&p| {
                let idx = ((p.clamp(0.0, 1.0)) * ((values.len() - 1) as f64)).round() as usize;
                values.get(idx).copied().unwrap_or(0.0)
            })
            .collect()
    }
}
