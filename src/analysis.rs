use crate::data::SensorSample;

/// Statistics for one measured value over a set of readings.
pub struct FieldStats {
    pub count: usize,
    pub min: f64,
    pub max: f64,
    pub average: f64,
    /// The average after readings more than two standard deviations from the
    /// mean have been removed.
    pub average_without_outliers: f64,
    pub outliers_removed: usize,
}

/// Calculates the statistics for one list of values.
pub fn field_stats(values: &[f64]) -> Option<FieldStats> {
    if values.is_empty() {
        return None;
    }

    let count = values.len();
    let average = values.iter().sum::<f64>() / count as f64;

    let variance = values
        .iter()
        .map(|value| (value - average).powi(2))
        .sum::<f64>()
        / count as f64;

    let deviation = variance.sqrt();

    // A sensor that glitches usually reports a value far from the rest, so
    // readings outside two standard deviations are treated as outliers. When
    // every reading is identical the deviation is zero and nothing is removed.
    let kept: Vec<f64> = if deviation > 0.0 {
        values
            .iter()
            .copied()
            .filter(|value| (value - average).abs() <= 2.0 * deviation)
            .collect()
    } else {
        values.to_vec()
    };

    Some(FieldStats {
        count,
        min: values.iter().copied().fold(f64::MAX, f64::min),
        max: values.iter().copied().fold(f64::MIN, f64::max),
        average,
        average_without_outliers: kept.iter().sum::<f64>() / kept.len() as f64,
        outliers_removed: count - kept.len(),
    })
}

/// The statistics shown on the daily statistics screen.
pub struct DailyStats {
    pub readings: usize,
    pub temperature: Option<FieldStats>,
    pub humidity: Option<FieldStats>,
    pub pressure: Option<FieldStats>,
    pub sunlight: Option<FieldStats>,
}

impl DailyStats {
    pub fn from_samples(samples: &[SensorSample]) -> Option<Self> {
        if samples.is_empty() {
            return None;
        }

        let field = |pick: fn(&SensorSample) -> Option<f32>| {
            let values: Vec<f64> = samples
                .iter()
                .filter_map(|sample| pick(sample))
                .map(f64::from)
                .collect();

            field_stats(&values)
        };

        Some(Self {
            readings: samples.len(),
            temperature: field(|sample| sample.temperature),
            humidity: field(|sample| sample.humidity),
            pressure: field(|sample| sample.pressure),
            sunlight: field(|sample| sample.visible_light.map(|value| value as f32)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_list_has_no_statistics() {
        assert!(field_stats(&[]).is_none());
    }

    #[test]
    fn the_average_ignores_a_glitched_reading() {
        // Ten readings around 20 plus one impossible reading of 900.
        let mut values = vec![20.0; 10];
        values.push(900.0);

        let stats = field_stats(&values).unwrap();

        assert_eq!(stats.count, 11);
        assert_eq!(stats.outliers_removed, 1);
        assert_eq!(stats.average_without_outliers, 20.0);
        assert_eq!(stats.min, 20.0);
        assert_eq!(stats.max, 900.0);
    }

    #[test]
    fn identical_readings_are_all_kept() {
        let stats = field_stats(&[15.0, 15.0, 15.0]).unwrap();

        assert_eq!(stats.outliers_removed, 0);
        assert_eq!(stats.average, 15.0);
        assert_eq!(stats.average_without_outliers, 15.0);
    }
}
