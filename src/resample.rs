use std::time::Instant;

/// Resample a series of samples taken at irregular time intervals into a fixed number of bins.
/// Use a dumb 'max' strategy that simply takes the maximum value in each bin.
pub fn resample(
    samples: &[f32],
    time_samples: &[Instant],
    start: Instant,
    end: Instant,
    num_bins: usize,
) -> Vec<Option<f32>> {
    if samples.is_empty() || time_samples.is_empty() || num_bins == 0 {
        return Vec::new();
    }

    if samples.len() != time_samples.len() {
        panic!("samples and time_samples must have the same length");
    }

    let mut result = vec![None; num_bins];
    let total_duration = end.duration_since(start);
    let bin_duration = total_duration / num_bins as u32;

    for i in 0..num_bins {
        let bin_start = start + bin_duration * i as u32;
        let bin_end = bin_start + bin_duration;

        let mut max_value: Option<f32> = None;

        for (j, &time_sample) in time_samples.iter().enumerate() {
            if time_sample > bin_start && time_sample <= bin_end {
                let sample_value = samples[j];
                max_value = match max_value {
                    Some(current_max) => Some(current_max.max(sample_value)),
                    None => Some(sample_value),
                };
            }
        }

        result[i] = max_value;
    }
    result
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::time::Duration;

    const EPSILON: f32 = 1e-6;

    fn assert_nearly_equal(a: f32, b: f32, msg: &str) {
        let delta = (a - b).abs();
        assert!(
            delta < EPSILON,
            "{}: expected {}, got {} (diff: {})",
            msg,
            b,
            a,
            delta
        );
    }

    fn assert_vec_nearly_equal(actual: &[Option<f32>], expected: &[Option<f32>], msg: &str) {
        assert_eq!(actual.len(), expected.len(), "{}: length mismatch", msg);
        for (i, (a, e)) in actual.iter().zip(expected.iter()).enumerate() {
            assert!(
                (a.is_some() && e.is_some()) || (a.is_none() && e.is_none()),
                "{} at index {}: mismatch between actual {:?} and expected {:?}",
                msg,
                i,
                a,
                e
            );
            match (a, e) {
                (Some(a_val), Some(e_val)) => {
                    assert_nearly_equal(*a_val, *e_val, &format!("{} at index {}", msg, i))
                }
                (None, None) => (),
                _ => panic!(
                    "{} at index {}: mismatch between {:?} and {:?}",
                    msg, i, a, e
                ),
            }
        }
    }

    #[test]
    fn test_exact_alignment() {
        // When bin midpoints align exactly with sample points
        let now = Instant::now();
        let time_samples = vec![
            now + Duration::from_millis(50),
            now + Duration::from_millis(150),
            now + Duration::from_millis(250),
            now + Duration::from_millis(350),
        ];
        let samples = vec![0.0, 1.0, 2.0, 3.0];

        let start = now;
        let end = now + Duration::from_millis(400);
        let num_bins = 4;

        let result = resample(&samples, &time_samples, start, end, num_bins);

        let expected = vec![Some(0.0), Some(1.0), Some(2.0), Some(3.0)];
        assert_vec_nearly_equal(&result, &expected, "exact alignment");
    }

    macro_rules! resample_tests {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (samples, time_samples, start_offset, end_offset, num_bins, expected) = $value;
                    let now = Instant::now();
                    let time_samples: Vec<Instant> = time_samples
                        .iter()
                        .map(|&offset| now + Duration::from_millis(offset))
                        .collect();
                    let start = now + Duration::from_millis(start_offset);
                    let end = now + Duration::from_millis(end_offset);

                    let result = resample(&samples, &time_samples, start, end, num_bins);
                    assert_vec_nearly_equal(&result, &expected, stringify!($name));
                }
            )*
        }
    }

    resample_tests! {
        single_sample_in_bin: (
            vec![42.0],
            vec![100],
            0,
            200,
            5,
            vec![None, None, Some(42.0), None, None],
        ),
        single_sample_at_start: (
            vec![10.0],
            vec![0],
            0,
            100,
            4,
            vec![None, None, None, None],
        ),
        single_sample_at_end: (
            vec![20.0],
            vec![100],
            0,
            100,
            4,
            vec![None, None, None, Some(20.0)],
        ),
        single_sample_outside_range_before: (
            vec![30.0],
            vec![50],
            100,
            200,
            3,
            vec![None, None, None],
        ),
        single_sample_outside_range_after: (
            vec![40.0],
            vec![250],
            100,
            200,
            3,
            vec![None, None, None],
        ),
        two_samples_in_one_bin: (
            vec![10.0, 20.0],
            vec![10, 20],
            0,
            100,
            4,
            vec![Some(20.0), None, None, None],
        ),
        //
        two_samples_in_adjacent_bins: (
            vec![10.0, 20.0],
            vec![20, 30],
            0,
            100,
            4,
            vec![Some(10.0),  Some(20.0), None, None],
        ),

        mixture_of_samples: (
            vec![5.0, 15.0, 25.0, 35.0, 45.0],
            vec![10, 50, 90, 160, 170],
            0,
            200,
            4,
            vec![Some(15.0), Some(25.0), None, Some(45.0)],
        ),
    }
}
